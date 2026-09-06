use diesel::{QueryDsl, RunQueryDsl};
use futures_loco_protocol::session::LocoSession;
use nohash_hasher::IntMap;
use talk_loco_client::talk::{
    channel::ChannelMetaType, session::load_channel_list::ChannelListData,
};

use crate::{
    database::{
        model::{
            channel::{history_sync::ChannelHistorySyncRow, meta::ChannelMetaRow, ChannelListRow},
            chat::ChatRow,
        },
        schema::{channel_history_sync, channel_list, channel_meta, chat},
        DatabasePool,
    },
    ClientResult,
};

use super::channel::ChannelUpdater;

#[derive(Debug, Clone, Copy)]
pub struct ChannelListUpdater<'a> {
    session: &'a LocoSession,
    pool: &'a DatabasePool,
}

impl<'a> ChannelListUpdater<'a> {
    pub fn new(session: &'a LocoSession, pool: &'a DatabasePool) -> Self {
        Self { session, pool }
    }

    pub async fn update(
        self,
        iter: impl IntoIterator<Item = ChannelListData>,
        deleted_ids: impl IntoIterator<Item = i64> + Send + 'static,
        initialize_channels: bool,
    ) -> ClientResult<()> {
        let list_data = iter.into_iter().collect::<Vec<_>>();
        let deleted_ids = deleted_ids.into_iter().collect::<Vec<_>>();
        let update_map = self
            .pool
            .spawn(|conn| {
                Ok(IntMap::from_iter(
                    channel_list::table
                        .select((
                            channel_list::id,
                            (
                                channel_list::last_update,
                                channel_list::last_seen_log_id,
                                channel_list::display_users,
                                channel_list::room_token,
                                channel_list::last_log_id,
                            ),
                        ))
                        .load::<(i64, (i64, Option<i64>, String, i64, i64))>(conn)?,
                ))
            })
            .await?;

        let mut updates = Vec::with_capacity(list_data.len());
        let mut initialize_ids = Vec::new();

        for list_data in list_data {
            let channel_type = if let Some(ty) = list_data.channel_type.ty() {
                ty
            } else {
                continue;
            };

            let existing = update_map.get(&list_data.id);
            let should_initialize = initialize_channels
                && existing
                    .map(|(last_update, _, _, _, _)| *last_update < list_data.last_log_send_at)
                    .unwrap_or(true);
            let display_users = list_data
                .icon_user_ids
                .as_ref()
                .map(|ids| serde_json::to_string(ids).expect("integer IDs serialize to JSON"))
                .or_else(|| existing.map(|(_, _, users, _, _)| users.clone()))
                .unwrap_or_else(|| "[]".to_owned());
            let fallback_title = list_data.icon_user_nicknames.as_ref().and_then(|names| {
                let title = names
                    .iter()
                    .map(|name| name.trim())
                    .filter(|name| !name.is_empty())
                    .collect::<Vec<_>>()
                    .join(", ");

                (!title.is_empty()).then_some(ChannelMetaRow {
                    channel_id: list_data.id,
                    meta_type: ChannelMetaType::Title as i32,
                    author_id: 0,
                    updated_at: 0,
                    revision: 0,
                    content: title,
                })
            });

            let list_row = ChannelListRow {
                id: list_data.id,
                channel_type: channel_type.as_str().to_string(),
                display_users,
                unread_count: list_data.unread_count,
                active_user_count: list_data.active_member_count,
                last_seen_log_id: list_data
                    .last_seen_log_id
                    .or_else(|| existing.and_then(|(_, last_seen, _, _, _)| *last_seen)),
                last_update: existing
                    .map(|(last_update, _, _, _, _)| (*last_update).max(list_data.last_log_send_at))
                    .unwrap_or(list_data.last_log_send_at),
                room_token: existing.map(|(_, _, _, token, _)| *token).unwrap_or(0),
                push_alert: list_data.push_alert,
                last_log_id: if list_data.last_log_id > 0 {
                    list_data.last_log_id
                } else {
                    existing
                        .map(|(_, _, _, _, last_log_id)| *last_log_id)
                        .unwrap_or(0)
                },
            };
            let preview = list_data
                .chatlog
                .map(|chatlog| ChatRow::from_chatlog(chatlog, None));

            if should_initialize {
                initialize_ids.push(list_row.id);
            }

            updates.push((list_row, preview, fallback_title));
        }

        self.pool
            .spawn_transaction(move |conn| {
                for (list_row, preview, fallback_title) in updates {
                    diesel::insert_or_ignore_into(channel_history_sync::table)
                        .values(ChannelHistorySyncRow {
                            channel_id: list_row.id,
                            cursor: 0,
                        })
                        .execute(conn)?;

                    diesel::replace_into(channel_list::table)
                        .values(list_row)
                        .execute(conn)?;

                    if let Some(preview) = preview {
                        diesel::insert_or_ignore_into(chat::table)
                            .values(preview)
                            .execute(conn)?;
                    }

                    if let Some(fallback_title) = fallback_title {
                        diesel::insert_or_ignore_into(channel_meta::table)
                            .values(fallback_title)
                            .execute(conn)?;
                    }
                }

                for channel_id in deleted_ids {
                    ChannelUpdater::new(channel_id).remove(conn)?;
                }

                Ok(())
            })
            .await?;

        for channel_id in initialize_ids {
            // A room that cannot be enriched must not hide every other room.
            // Opening it later retries through CHATONROOM.
            let _ = ChannelUpdater::new(channel_id)
                .initialize(self.session, self.pool)
                .await;
        }

        Ok(())
    }
}
