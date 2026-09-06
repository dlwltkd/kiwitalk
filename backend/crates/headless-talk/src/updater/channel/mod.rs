mod normal;

use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection};
use futures_loco_protocol::session::LocoSession;
use talk_loco_client::talk::{
    channel::{ChannelMetaType, ChannelType},
    session::{channel::info::ChannelInfoType, TalkSession},
};

use crate::{
    database::{
        model::{
            channel::{meta::ChannelMetaRow, ChannelListRow},
            chat::ChatRow,
            user::UserProfileRow,
        },
        schema::{channel_history_sync, channel_list, channel_meta, chat, user_profile},
        DatabasePool, PoolTaskError,
    },
    ClientResult,
};

use self::normal::NormalChannelUpdater;

#[derive(Debug)]
pub struct ChannelUpdater {
    id: i64,
}

impl ChannelUpdater {
    pub fn new(id: i64) -> Self {
        Self { id }
    }

    pub async fn initialize(
        self,
        session: &LocoSession,
        pool: &DatabasePool,
    ) -> ClientResult<Option<()>> {
        let res = TalkSession(session).channel(self.id).info().await?;

        let meta_rows = res
            .channel_metas
            .into_iter()
            .map(|meta| ChannelMetaRow::from_meta(self.id, meta))
            .collect::<Vec<_>>();

        let (channel_type, display_members, normal) = match res.channel_type {
            ChannelInfoType::DirectChat(normal) => (
                ChannelType::DirectChat,
                normal
                    .display_members
                    .iter()
                    .map(|user| {
                        (
                            user.user_id,
                            user.nickname.clone(),
                            user.profile_image_url.clone().unwrap_or_default(),
                        )
                    })
                    .collect::<Vec<_>>(),
                Some(normal),
            ),
            ChannelInfoType::MultiChat(normal) => (
                ChannelType::MultiChat,
                normal
                    .display_members
                    .iter()
                    .map(|user| {
                        (
                            user.user_id,
                            user.nickname.clone(),
                            user.profile_image_url.clone().unwrap_or_default(),
                        )
                    })
                    .collect::<Vec<_>>(),
                Some(normal),
            ),
            ChannelInfoType::MemoChat(normal) => (
                ChannelType::MemoChat,
                normal
                    .display_members
                    .iter()
                    .map(|user| {
                        (
                            user.user_id,
                            user.nickname.clone(),
                            user.profile_image_url.clone().unwrap_or_default(),
                        )
                    })
                    .collect::<Vec<_>>(),
                Some(normal),
            ),
            ChannelInfoType::OpenDirect(open) => (
                ChannelType::OpenDirect,
                open.display_members
                    .into_iter()
                    .map(|user| {
                        (
                            user.user_id,
                            user.nickname,
                            user.profile_image_url.unwrap_or_default(),
                        )
                    })
                    .collect::<Vec<_>>(),
                None,
            ),
            ChannelInfoType::OpenMulti(open) => (
                ChannelType::OpenMulti,
                open.display_members
                    .into_iter()
                    .map(|user| {
                        (
                            user.user_id,
                            user.nickname,
                            user.profile_image_url.unwrap_or_default(),
                        )
                    })
                    .collect::<Vec<_>>(),
                None,
            ),
            ChannelInfoType::Other => return Ok(None),
        };

        let display_user_ids = display_members
            .iter()
            .map(|(user_id, _, _)| *user_id)
            .collect::<Vec<_>>();

        let fallback_title = if !meta_rows
            .iter()
            .any(|meta| meta.meta_type == ChannelMetaType::Title as i32)
        {
            let title = display_members
                .iter()
                .map(|(_, nickname, _)| nickname.trim())
                .filter(|nickname| !nickname.is_empty())
                .collect::<Vec<_>>()
                .join(", ");

            (!title.is_empty()).then_some(ChannelMetaRow {
                channel_id: self.id,
                meta_type: ChannelMetaType::Title as i32,
                author_id: 0,
                updated_at: 0,
                revision: 0,
                content: title,
            })
        } else {
            None
        };

        let last_update = res
            .last_chatlog
            .as_ref()
            .map(|chatlog| chatlog.send_at)
            .unwrap_or_default();
        let last_log_id = res
            .last_chatlog
            .as_ref()
            .map(|chatlog| chatlog.log_id)
            .unwrap_or_default();
        let list_row = ChannelListRow {
            id: self.id,
            channel_type: channel_type.as_str().to_owned(),
            display_users: serde_json::to_string(&display_user_ids)
                .expect("integer IDs serialize to JSON"),
            active_user_count: res.active_member_count,
            unread_count: res.new_chat_count,
            last_seen_log_id: Some(res.last_seen_log_id),
            last_update,
            room_token: 0,
            push_alert: res.push_alert,
            last_log_id,
        };
        let last_chat = res
            .last_chatlog
            .map(|chatlog| ChatRow::from_chatlog(chatlog, None));

        pool.spawn_transaction(move |conn| {
            diesel::insert_or_ignore_into(channel_list::table)
                .values(&list_row)
                .execute(conn)?;

            if !meta_rows.is_empty() {
                diesel::replace_into(channel_meta::table)
                    .values(meta_rows)
                    .execute(conn)?;
            }

            if let Some(fallback_title) = fallback_title {
                diesel::insert_or_ignore_into(channel_meta::table)
                    .values(fallback_title)
                    .execute(conn)?;
            }

            if let Some(last_chat) = last_chat {
                diesel::insert_or_ignore_into(chat::table)
                    .values(last_chat)
                    .execute(conn)?;
            }

            for (user_id, nickname, profile_url) in &display_members {
                diesel::insert_or_ignore_into(user_profile::table)
                    .values(UserProfileRow {
                        id: *user_id,
                        channel_id: list_row.id,
                        nickname,
                        profile_url,
                        full_profile_url: profile_url,
                        original_profile_url: profile_url,
                    })
                    .execute(conn)?;
            }

            Ok(())
        })
        .await?;

        if let Some(normal) = normal {
            NormalChannelUpdater::new(self.id)
                .initialize(session, pool, normal, |_| Ok(()))
                .await?;
        }

        Ok(Some(()))
    }

    pub fn remove(self, conn: &mut SqliteConnection) -> Result<Option<()>, PoolTaskError> {
        diesel::delete(
            channel_history_sync::table.filter(channel_history_sync::channel_id.eq(self.id)),
        )
        .execute(conn)?;

        let row: ChannelListRow = if let Some(row) = {
            channel_list::table
                .select(channel_list::all_columns)
                .filter(channel_list::id.eq(self.id))
                .first::<ChannelListRow>(conn)
                .optional()?
        } {
            row
        } else {
            return Ok(None);
        };

        let ty = ChannelType::from(row.channel_type.as_str());

        match ty {
            ChannelType::DirectChat | ChannelType::MultiChat | ChannelType::MemoChat => {
                NormalChannelUpdater::new(self.id).remove(conn)?;
            }

            ChannelType::OpenDirect | ChannelType::OpenMulti => {}

            _ => return Ok(None),
        }

        diesel::delete(chat::table.filter(chat::channel_id.eq(self.id))).execute(conn)?;

        diesel::delete(channel_meta::table.filter(channel_meta::channel_id.eq(self.id)))
            .execute(conn)?;

        diesel::delete(user_profile::table.filter(user_profile::channel_id.eq(self.id)))
            .execute(conn)?;

        diesel::delete(channel_list::table.filter(channel_list::id.eq(self.id))).execute(conn)?;

        Ok(Some(()))
    }
}
