pub mod user;

use diesel::{
    BoolExpressionMethods, ExpressionMethods, JoinOnDsl, OptionalExtension, QueryDsl, RunQueryDsl,
    SelectableHelper, SqliteConnection,
};
use talk_loco_client::talk::{
    channel::ChannelMetaType,
    session::{
        channel::{
            chat_on::{ChatOnChannelUsers, NormalChatOnChannel},
            normal,
        },
        TalkSession,
    },
};

use crate::{
    conn::Conn,
    database::{
        model::{
            channel::{meta::ChannelMetaRow, ChannelListRow},
            user::{
                normal::{NormalChannelUserModel, NormalChannelUserRow, NormalChannelUserUpdate},
                UserProfileModel, UserProfileRow, UserProfileUpdate,
            },
        },
        schema::{channel_list, channel_meta, normal_channel_user, user_profile},
        PoolTaskError,
    },
    updater::channel::ChannelUpdater,
    user::DisplayUser,
    ClientResult,
};

use self::user::NormalChannelUser;

use super::{leave_source, ChannelMetaMap, ListChannelProfile, UserList};

#[derive(Debug, Clone)]
pub struct NormalChannel {
    pub users: UserList<NormalChannelUser>,

    pub meta_map: ChannelMetaMap,
}

#[derive(Debug, Clone, Copy)]
pub struct NormalChannelOp<'a> {
    id: i64,
    conn: &'a Conn,
}

impl<'a> NormalChannelOp<'a> {
    pub(crate) const fn new(id: i64, conn: &'a Conn) -> Self {
        Self { id, conn }
    }

    pub const fn id(self) -> i64 {
        self.id
    }

    pub async fn read_chat(self, watermark: i64) -> ClientResult<()> {
        let id = self.id;

        self.conn
            .pool
            .spawn(move |conn| {
                diesel::update(channel_list::table)
                    .filter(channel_list::id.eq(id))
                    .set((
                        channel_list::last_seen_log_id.eq(watermark),
                        channel_list::unread_count.eq(0),
                    ))
                    .execute(conn)?;

                Ok(())
            })
            .await?;

        Ok(())
    }

    pub async fn leave(self, block: bool) -> ClientResult<()> {
        let id = self.id;
        let channel_type = self
            .conn
            .pool
            .spawn(move |conn| {
                Ok(channel_list::table
                    .filter(channel_list::id.eq(id))
                    .select(channel_list::type_)
                    .first::<String>(conn)?)
            })
            .await?;
        let from = leave_source(&talk_loco_client::talk::channel::ChannelType::from(
            channel_type.as_str(),
        ));

        TalkSession(&self.conn.session)
            .normal_channel(id)
            .leave(block, &from)
            .await?;

        self.conn
            .pool
            .spawn_transaction(move |conn| ChannelUpdater::new(id).remove(conn))
            .await?;

        Ok(())
    }
}

pub(super) fn load_list_profile(
    conn: &mut SqliteConnection,
    display_users: &[DisplayUser],
    row: &ChannelListRow,
) -> Result<ListChannelProfile, PoolTaskError> {
    let id = row.id;
    let name: Option<String> = channel_meta::table
        .filter(
            channel_meta::channel_id
                .eq(id)
                .and(channel_meta::type_.eq(ChannelMetaType::Title as i32)),
        )
        .select(channel_meta::content)
        .first(conn)
        .optional()?;

    let image_meta: Option<String> = channel_meta::table
        .filter(
            channel_meta::channel_id
                .eq(id)
                .and(channel_meta::type_.eq(ChannelMetaType::Profile as i32)),
        )
        .select(channel_meta::content)
        .first(conn)
        .optional()?;

    let name = name.unwrap_or_else(|| {
        display_users
            .iter()
            .map(|user| user.profile.nickname.as_str())
            .collect::<Vec<&str>>()
            .join(", ")
    });

    Ok(ListChannelProfile {
        name,
        image: image_meta.and_then(|meta| serde_json::from_str(&meta).ok()),
    })
}

pub(crate) async fn load_channel(
    id: i64,
    conn: &Conn,
    normal: NormalChatOnChannel,
) -> ClientResult<NormalChannel> {
    let (users, meta) = conn
        .pool
        .spawn_transaction(move |conn| {
            let mut user_list: UserList<NormalChannelUser> = UserList::new();

            match normal.users {
                ChatOnChannelUsers::Ids(ids) => {
                    for user_id in ids.iter().copied() {
                        if let Some(user) = get_channel_user(conn, id, user_id)? {
                            user_list.push((user_id, user));
                        }
                    }
                }

                ChatOnChannelUsers::Users(users) => {
                    update_channel_users(conn, id, &users)?;

                    for user_id in users.iter().map(|user| user.user_id) {
                        if let Some(user) = get_channel_user(conn, id, user_id)? {
                            user_list.push((user_id, user));
                        }
                    }
                }
            }

            let meta = channel_meta::table
                .filter(channel_meta::channel_id.eq(id))
                .load_iter::<ChannelMetaRow, _>(conn)?
                .map(|res| res.map(|row| (row.meta_type, row.into())))
                .collect::<Result<ChannelMetaMap, _>>()?;

            Ok((user_list, meta))
        })
        .await?;

    Ok(NormalChannel {
        users,
        meta_map: meta,
    })
}

fn get_channel_user(
    conn: &mut SqliteConnection,
    id: i64,
    user_id: i64,
) -> Result<Option<NormalChannelUser>, PoolTaskError> {
    let models = user_profile::table
        .filter(
            user_profile::channel_id
                .eq(id)
                .and(user_profile::id.eq(user_id)),
        )
        .inner_join(
            normal_channel_user::table.on(normal_channel_user::channel_id
                .eq(user_profile::channel_id)
                .and(normal_channel_user::id.eq(user_profile::id))),
        )
        .select((
            UserProfileModel::as_select(),
            NormalChannelUserModel::as_select(),
        ))
        .first::<(UserProfileModel, NormalChannelUserModel)>(conn)
        .optional()?;

    Ok(models.map(|(profile, normal)| NormalChannelUser::from_models(profile, normal)))
}

fn update_channel_users(
    conn: &mut SqliteConnection,
    id: i64,
    users: &[normal::user::User],
) -> Result<(), PoolTaskError> {
    for user in users {
        diesel::insert_into(user_profile::table)
            .values(UserProfileRow::from_normal_user(id, user))
            .on_conflict((user_profile::id, user_profile::channel_id))
            .do_update()
            .set(UserProfileUpdate::from(user))
            .execute(conn)?;

        let row = NormalChannelUserRow::from_user(id, user);
        let update = NormalChannelUserUpdate::from(user);

        if update.has_changes() {
            diesel::insert_into(normal_channel_user::table)
                .values(row)
                .on_conflict((normal_channel_user::id, normal_channel_user::channel_id))
                .do_update()
                .set(update)
                .execute(conn)?;
        } else {
            diesel::insert_into(normal_channel_user::table)
                .values(row)
                .on_conflict((normal_channel_user::id, normal_channel_user::channel_id))
                .do_nothing()
                .execute(conn)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use diesel::{connection::SimpleConnection, Connection};

    use super::*;

    fn test_connection() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.batch_execute(include_str!(
            "../../../migrations/2023-10-21-003644_v0.1/up.sql"
        ))
        .unwrap();
        conn
    }

    #[test]
    fn sparse_member_update_preserves_cached_fields() {
        let mut conn = test_connection();
        let channel_id = 7;
        let user_id = 11;

        diesel::insert_into(user_profile::table)
            .values(UserProfileRow {
                id: user_id,
                channel_id,
                nickname: "old nickname",
                profile_url: "profile",
                full_profile_url: "full profile",
                original_profile_url: "original profile",
            })
            .execute(&mut conn)
            .unwrap();
        diesel::insert_into(normal_channel_user::table)
            .values(NormalChannelUserRow {
                id: user_id,
                channel_id,
                country_iso: "KR",
                account_id: 42,
                status_message: "old status",
                linked_services: "old services",
                suspended: true,
            })
            .execute(&mut conn)
            .unwrap();

        let user: normal::user::User = bson::from_document(bson::doc! {
            "userId": user_id,
            "nickName": "new nickname",
            "countryIso": "US",
            "profileImageUrl": bson::Bson::Null,
            "accountId": bson::Bson::Null,
            "statusMessage": "",
            "suspended": false,
        })
        .unwrap();

        update_channel_users(&mut conn, channel_id, &[user]).unwrap();

        let profile = user_profile::table
            .filter(
                user_profile::channel_id
                    .eq(channel_id)
                    .and(user_profile::id.eq(user_id)),
            )
            .select(UserProfileModel::as_select())
            .first::<UserProfileModel>(&mut conn)
            .unwrap();
        let member = normal_channel_user::table
            .filter(
                normal_channel_user::channel_id
                    .eq(channel_id)
                    .and(normal_channel_user::id.eq(user_id)),
            )
            .select(NormalChannelUserModel::as_select())
            .first::<NormalChannelUserModel>(&mut conn)
            .unwrap();

        assert_eq!(profile.nickname, "new nickname");
        assert_eq!(profile.profile_url, "profile");
        assert_eq!(profile.full_profile_url, "full profile");
        assert_eq!(profile.original_profile_url, "original profile");
        assert_eq!(member.country_iso, "US");
        assert_eq!(member.account_id, 42);
        assert_eq!(member.status_message, "");
        assert_eq!(member.linked_services, "old services");
        assert!(!member.suspended);
    }
}
