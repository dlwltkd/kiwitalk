pub mod archive;
pub mod channel;
mod conn;
mod constants;
mod database;
pub mod event;
pub mod handler;
pub mod init;
mod task;
mod updater;
pub mod user;

use channel::{
    load_list_item,
    normal::{self, NormalChannelOp},
    open::OpenChannelOp,
    ChannelListItem, ChannelOp, ClientChannel,
};
use conn::Conn;
use diesel::{BoolExpressionMethods, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl};

use database::{
    model::channel::ChannelListRow,
    schema::{channel_list, user_profile},
    PoolTaskError,
};
use talk_loco_client::{
    talk::{
        channel::ChannelType,
        session::{channel::chat_on::ChatOnChannelType, TalkSession},
    },
    RequestError,
};
use task::BackgroundTask;
use thiserror::Error;
use updater::chat::ChatUpdater;

pub use updater::chat::{HistorySyncResult, HistorySyncStop};

pub use talk_loco_client;

#[derive(Debug)]
pub struct HeadlessTalk {
    pub conn: Conn,

    pub _ping_task: BackgroundTask,
    pub _stream_task: BackgroundTask,
}

impl HeadlessTalk {
    pub async fn shutdown(&self) {
        futures::join!(self._ping_task.shutdown(), self._stream_task.shutdown());
    }

    pub fn user_id(&self) -> i64 {
        self.conn.user_id
    }

    pub fn set_channel_active(&self, channel_id: i64, active: bool) {
        self.conn.set_channel_active(channel_id, active);
    }

    pub async fn channel_list(&self) -> Result<Vec<(i64, ChannelListItem)>, PoolTaskError> {
        self.conn
            .pool
            .spawn(|conn| {
                let rows = channel_list::table
                    .select(channel_list::all_columns)
                    .order((channel_list::last_update.desc(), channel_list::id.desc()))
                    .load::<ChannelListRow>(conn)?;
                let mut list = Vec::with_capacity(rows.len());

                for row in rows {
                    if let Some(list_item) = load_list_item(conn, &row)? {
                        list.push((row.id, list_item));
                    }
                }

                Ok(list)
            })
            .await
    }

    pub async fn load_channel(&self, id: i64) -> ClientResult<Option<ClientChannel>> {
        let room_state_guard = self.conn.room_state_lock.lock().await;
        let room_token = self
            .conn
            .pool
            .spawn(move |conn| {
                Ok(channel_list::table
                    .filter(channel_list::id.eq(id))
                    .select(channel_list::room_token)
                    .first::<i64>(conn)
                    .optional()?
                    .unwrap_or(0))
            })
            .await?;

        let res = TalkSession(&self.conn.session)
            .channel(id)
            .chat_on(Some(room_token))
            .await?;
        let next_room_token = res.token;
        let server_last_log_id = res.last_log_id;
        let mark_read = res.noti_read;
        let active_user_count = res.active_user_ids.as_ref().map(|ids| ids.len() as i32);
        let watermark_pairs = res
            .active_user_ids
            .unwrap_or_default()
            .into_iter()
            .zip(res.watermarks.unwrap_or_default())
            .collect::<Vec<_>>();

        let mut channel = match res.channel_type {
            ChatOnChannelType::DirectChat(normal)
            | ChatOnChannelType::MultiChat(normal)
            | ChatOnChannelType::MemoChat(normal) => Some(ClientChannel::Normal(
                normal::load_channel(id, &self.conn, normal).await?,
            )),

            ChatOnChannelType::OpenDirect(open) | ChatOnChannelType::OpenMulti(open) => Some(
                ClientChannel::Open(channel::open::load_channel(id, &self.conn, open).await?),
            ),

            _ => None,
        };

        let persisted_watermarks = watermark_pairs.clone();
        self.conn
            .pool
            .spawn_transaction(move |conn| {
                diesel::update(channel_list::table)
                    .filter(channel_list::id.eq(id))
                    .set(channel_list::room_token.eq(next_room_token))
                    .execute(conn)?;

                if server_last_log_id > 0 {
                    diesel::update(
                        channel_list::table
                            .filter(channel_list::id.eq(id))
                            .filter(channel_list::last_log_id.lt(server_last_log_id)),
                    )
                    .set(channel_list::last_log_id.eq(server_last_log_id))
                    .execute(conn)?;
                }

                if mark_read {
                    diesel::update(channel_list::table)
                        .filter(channel_list::id.eq(id))
                        .set(channel_list::unread_count.eq(0))
                        .execute(conn)?;

                    if server_last_log_id > 0 {
                        diesel::update(channel_list::table)
                            .filter(channel_list::id.eq(id))
                            .set(channel_list::last_seen_log_id.eq(Some(server_last_log_id)))
                            .execute(conn)?;
                    }
                }

                if let Some(active_user_count) = active_user_count {
                    diesel::update(channel_list::table)
                        .filter(channel_list::id.eq(id))
                        .set(channel_list::active_user_count.eq(active_user_count))
                        .execute(conn)?;
                }

                for (user_id, watermark) in persisted_watermarks {
                    diesel::update(user_profile::table)
                        .filter(
                            user_profile::channel_id
                                .eq(id)
                                .and(user_profile::id.eq(user_id)),
                        )
                        .set(user_profile::watermark.eq(watermark))
                        .execute(conn)?;
                }

                Ok(())
            })
            .await?;

        drop(room_state_guard);

        if let Some(ClientChannel::Normal(normal)) = &mut channel {
            for (user_id, user) in &mut normal.users {
                if let Some((_, watermark)) = watermark_pairs
                    .iter()
                    .find(|(watermark_user_id, _)| watermark_user_id == user_id)
                {
                    user.watermark = *watermark;
                }
            }
        }

        if let Some(ClientChannel::Open(open)) = &mut channel {
            for (user_id, user) in &mut open.users {
                if let Some((_, watermark)) = watermark_pairs
                    .iter()
                    .find(|(watermark_user_id, _)| watermark_user_id == user_id)
                {
                    user.watermark = *watermark;
                }
            }
        }

        Ok(channel)
    }

    pub async fn sync_channel_history(&self, id: i64) -> ClientResult<HistorySyncResult> {
        let room_state_guard = self.conn.room_state_lock.lock().await;
        let room_state = self
            .conn
            .pool
            .spawn(move |conn| {
                Ok(channel_list::table
                    .filter(channel_list::id.eq(id))
                    .select((
                        channel_list::last_log_id,
                        channel_list::type_,
                        channel_list::room_token,
                    ))
                    .first::<(i64, String, i64)>(conn)
                    .optional()?)
            })
            .await?;

        let Some((mut last_log_id, channel_type, room_token)) = room_state else {
            return Ok(HistorySyncResult::stopped(
                0,
                0,
                HistorySyncStop::UnsupportedChannel,
            ));
        };

        if matches!(
            ChannelType::from(channel_type.as_str()),
            ChannelType::Other(_)
        ) {
            return Ok(HistorySyncResult::stopped(
                0,
                0,
                HistorySyncStop::UnsupportedChannel,
            ));
        }

        if last_log_id <= 0 {
            let room = TalkSession(&self.conn.session)
                .channel(id)
                .chat_on(Some(room_token))
                .await?;

            if room.channel_type.ty().is_none() {
                return Ok(HistorySyncResult::stopped(
                    0,
                    0,
                    HistorySyncStop::UnsupportedChannel,
                ));
            }

            last_log_id = room.last_log_id;
            let next_room_token = room.token;
            let mark_read = room.noti_read;
            let active_user_count = room.active_user_ids.map(|ids| ids.len() as i32);

            self.conn
                .pool
                .spawn_transaction(move |conn| {
                    diesel::update(channel_list::table)
                        .filter(channel_list::id.eq(id))
                        .set(channel_list::room_token.eq(next_room_token))
                        .execute(conn)?;

                    if last_log_id > 0 {
                        diesel::update(
                            channel_list::table
                                .filter(channel_list::id.eq(id))
                                .filter(channel_list::last_log_id.lt(last_log_id)),
                        )
                        .set(channel_list::last_log_id.eq(last_log_id))
                        .execute(conn)?;
                    }

                    if mark_read {
                        diesel::update(channel_list::table)
                            .filter(channel_list::id.eq(id))
                            .set(channel_list::unread_count.eq(0))
                            .execute(conn)?;

                        if last_log_id > 0 {
                            diesel::update(channel_list::table)
                                .filter(channel_list::id.eq(id))
                                .set(channel_list::last_seen_log_id.eq(Some(last_log_id)))
                                .execute(conn)?;
                        }
                    }

                    if let Some(active_user_count) = active_user_count {
                        diesel::update(channel_list::table)
                            .filter(channel_list::id.eq(id))
                            .set(channel_list::active_user_count.eq(active_user_count))
                            .execute(conn)?;
                    }

                    Ok(())
                })
                .await?;
        }

        drop(room_state_guard);

        ChatUpdater::new(&self.conn.session, &self.conn.pool, id)
            .update_bounded(last_log_id)
            .await
    }

    pub fn channel(&self, id: i64) -> ChannelOp<'_> {
        ChannelOp::new(id, &self.conn)
    }

    pub fn normal_channel(&self, id: i64) -> NormalChannelOp<'_> {
        NormalChannelOp::new(id, &self.conn)
    }

    pub fn open_channel(&self, id: i64, link_id: i64) -> OpenChannelOp<'_> {
        OpenChannelOp::new(id, link_id, &self.conn)
    }

    pub async fn set_status(&self, client_status: ClientStatus) -> ClientResult<()> {
        TalkSession(&self.conn.session)
            .set_status(client_status as _)
            .await?;

        Ok(())
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientStatus {
    Unlocked = 1,
    Locked = 2,
}

pub type ClientResult<T> = Result<T, ClientError>;

#[derive(Debug, Error)]
#[error(transparent)]
pub enum ClientError {
    Request(#[from] RequestError),
    Database(#[from] PoolTaskError),
}
