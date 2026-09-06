use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use talk_loco_client::talk::session::{
    channel::{chat_on::OpenChatOnChannel, open::user::User},
    TalkSession,
};

use crate::{
    conn::Conn,
    database::{
        model::channel::meta::ChannelMetaRow,
        schema::{channel_list, channel_meta},
    },
    updater::channel::ChannelUpdater,
    user::UserProfile,
    ClientResult,
};

use super::{leave_source, ChannelMetaMap, UserList};

#[derive(Debug, Clone)]
pub struct OpenChannelUser {
    pub user_type: i32,
    pub account_id: i64,
    pub country_iso: Option<String>,
    pub service_user_type: Option<i32>,
    pub suspended: bool,
    pub suspicion: String,
    pub open_member_type: i32,
    pub profile_type: i32,
    pub profile_link_id: i64,
    pub open_token: i32,
    pub watermark: i64,
    pub profile: UserProfile,
}

impl From<User> for OpenChannelUser {
    fn from(user: User) -> Self {
        let profile = UserProfile::from(user.clone());

        Self {
            user_type: user.user_type,
            account_id: user.account_id,
            country_iso: user.country_iso,
            service_user_type: user.service_user_type,
            suspended: user.suspended,
            suspicion: user.suspicion,
            open_member_type: user.open_member_type,
            profile_type: user.profile_type,
            profile_link_id: user.profile_link_id,
            open_token: user.open_token,
            watermark: 0,
            profile,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpenChannel {
    pub users: UserList<OpenChannelUser>,
    pub meta_map: ChannelMetaMap,
    pub open_token: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct OpenChannelOp<'a> {
    id: i64,
    link_id: i64,
    conn: &'a Conn,
}

pub(crate) async fn load_channel(
    id: i64,
    conn: &Conn,
    open: OpenChatOnChannel,
) -> ClientResult<OpenChannel> {
    let users = match open.users {
        Some(users) => users,
        None => {
            TalkSession(&conn.session)
                .open_channel(id, 0)
                .list_users()
                .await?
        }
    }
    .into_iter()
    .map(|user| (user.user_id, OpenChannelUser::from(user)))
    .collect();

    let meta_map = conn
        .pool
        .spawn(move |conn| {
            Ok(channel_meta::table
                .filter(channel_meta::channel_id.eq(id))
                .load_iter::<ChannelMetaRow, _>(conn)?
                .map(|res| res.map(|row| (row.meta_type, row.into())))
                .collect::<Result<ChannelMetaMap, _>>()?)
        })
        .await?;

    Ok(OpenChannel {
        users,
        meta_map,
        open_token: open.open_token,
    })
}

impl<'a> OpenChannelOp<'a> {
    pub const fn new(id: i64, link_id: i64, conn: &'a Conn) -> Self {
        Self { id, link_id, conn }
    }

    pub const fn id(self) -> i64 {
        self.id
    }

    pub const fn link_id(self) -> i64 {
        self.link_id
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
            .open_channel(id, self.link_id)
            .leave(block, &from)
            .await?;

        self.conn
            .pool
            .spawn_transaction(move |conn| ChannelUpdater::new(id).remove(conn))
            .await?;

        Ok(())
    }
}
