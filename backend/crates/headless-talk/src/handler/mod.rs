pub mod error;

use diesel::{
    dsl::exists, BoolExpressionMethods, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl,
};
use futures_loco_protocol::loco_protocol::command::BoxedCommand;
use futures_loco_protocol::loco_protocol::command::Header;
use talk_loco_client::talk::session::TalkSession;
use talk_loco_client::talk::stream::{
    command::{ChgMeta, DecunRead, DelMem, Kickout, Left, Msg, NewMem, SyncDlMsg, SyncJoin},
    StreamCommand,
};

use crate::{
    conn::Conn,
    database::{
        model::{channel::meta::ChannelMetaRow, chat::ChatRow},
        schema::{self, channel_list, channel_meta, chat},
    },
    event::{channel::ChannelEvent, ClientEvent},
    updater::channel::ChannelUpdater,
};

use self::error::HandlerError;

type HandlerResult = Result<Option<ClientEvent>, HandlerError>;

#[derive(Debug, Clone)]
pub(crate) struct SessionHandler {
    conn: Conn,
}

impl SessionHandler {
    pub fn new(conn: Conn) -> Self {
        Self { conn }
    }

    pub async fn handle(&self, read: BoxedCommand) -> HandlerResult {
        let header = read.header.clone();
        let method = (*header.method).to_owned();

        match StreamCommand::deserialize_from(read)
            .map_err(|source| HandlerError::Deserialize { method, source })?
        {
            StreamCommand::Kickout(kickout) => self.on_kickout(kickout).await,
            StreamCommand::SwitchServer => self.on_switch_server().await,
            StreamCommand::Chat(msg) => self.on_chat(header, msg).await,
            StreamCommand::ChatRead(read) => self.on_chat_read(read).await,
            StreamCommand::ChangeMeta(meta) => self.on_meta_change(meta).await,
            StreamCommand::SyncChatDeletion(deletion) => self.on_chat_deleted(deletion).await,
            StreamCommand::SyncChannelJoin(sync_join) => self.on_channel_join(sync_join).await,
            StreamCommand::Left(left) => self.on_left(left).await,
            StreamCommand::NewUser(new_mem) => self.on_new_user(new_mem).await,
            StreamCommand::DelUser(del_mem) => self.on_del_user(del_mem).await,

            _ => Ok(None),
        }
    }

    async fn on_kickout(&self, kickout: Kickout) -> HandlerResult {
        Ok(Some(ClientEvent::Kickout(kickout.reason)))
    }

    async fn on_switch_server(&self) -> HandlerResult {
        Ok(Some(ClientEvent::SwitchServer))
    }

    async fn on_chat(&self, header: Header, msg: Msg) -> HandlerResult {
        let is_mine = msg.chatlog.author_id == self.conn.user_id;
        let noti_read = self.conn.is_channel_active(msg.chat_id) && !is_mine;
        let (exists, push_alert) = self
            .conn
            .pool
            .spawn({
                let row = ChatRow::from_chatlog(msg.chatlog.clone(), None);

                move |conn| {
                    let channel_state = channel_list::table
                        .filter(channel_list::id.eq(row.channel_id))
                        .select((
                            channel_list::push_alert,
                            channel_list::last_seen_log_id,
                            channel_list::last_update,
                            channel_list::last_log_id,
                        ))
                        .first::<(bool, Option<i64>, i64, i64)>(conn)
                        .optional()?;
                    let channel_exists = channel_state.is_some();
                    let chat_exists =
                        diesel::select(exists(chat::table.filter(chat::log_id.eq(row.log_id))))
                            .get_result::<bool>(conn)?;

                    diesel::replace_into(chat::table)
                        .values(&row)
                        .execute(conn)?;

                    if let Some((_, last_seen_log_id, last_update, last_log_id)) = channel_state {
                        if is_mine || noti_read {
                            diesel::update(
                                channel_list::table.filter(channel_list::id.eq(row.channel_id)),
                            )
                            .set((
                                channel_list::unread_count.eq(0),
                                channel_list::last_seen_log_id
                                    .eq(Some(last_seen_log_id.unwrap_or_default().max(row.log_id))),
                                channel_list::last_update.eq(last_update.max(row.send_at)),
                                channel_list::last_log_id.eq(last_log_id.max(row.log_id)),
                            ))
                            .execute(conn)?;
                        } else if !chat_exists {
                            diesel::update(
                                channel_list::table.filter(channel_list::id.eq(row.channel_id)),
                            )
                            .set((
                                channel_list::unread_count.eq(channel_list::unread_count + 1),
                                channel_list::last_update.eq(last_update.max(row.send_at)),
                                channel_list::last_log_id.eq(last_log_id.max(row.log_id)),
                            ))
                            .execute(conn)?;
                        } else {
                            diesel::update(
                                channel_list::table.filter(channel_list::id.eq(row.channel_id)),
                            )
                            .set((
                                channel_list::last_update.eq(last_update.max(row.send_at)),
                                channel_list::last_log_id.eq(last_log_id.max(row.log_id)),
                            ))
                            .execute(conn)?;
                        }
                    }

                    Ok((
                        channel_exists,
                        channel_state
                            .map(|(push_alert, _, _, _)| push_alert)
                            .unwrap_or(true),
                    ))
                }
            })
            .await?;

        if !exists {
            let _ = ChannelUpdater::new(msg.chat_id)
                .initialize(&self.conn.session, &self.conn.pool)
                .await;
        }

        TalkSession(&self.conn.session)
            .acknowledge_message(header, noti_read)
            .await?;

        Ok(Some(ClientEvent::Channel {
            id: msg.chat_id,

            event: ChannelEvent::Chat {
                link_id: msg.link_id,

                user_nickname: msg.author_nickname,
                chat: msg.chatlog,
                read: is_mine || noti_read,
                notify: !is_mine && !noti_read && push_alert,
            },
        }))
    }

    async fn on_chat_read(&self, read: DecunRead) -> HandlerResult {
        self.conn
            .pool
            .spawn({
                let DecunRead {
                    chat_id: channel_id,
                    user_id,
                    watermark,
                } = read.clone();

                move |conn| {
                    use schema::user_profile;

                    diesel::update(user_profile::table)
                        .filter(
                            user_profile::channel_id
                                .eq(channel_id)
                                .and(user_profile::id.eq(user_id)),
                        )
                        .set(user_profile::watermark.eq(watermark))
                        .execute(conn)?;
                    Ok(())
                }
            })
            .await?;

        Ok(Some(ClientEvent::Channel {
            id: read.chat_id,

            event: ChannelEvent::ChatRead {
                user_id: read.user_id,
                log_id: read.watermark,
            },
        }))
    }

    async fn on_meta_change(&self, value: ChgMeta) -> HandlerResult {
        self.conn
            .pool
            .spawn({
                let value = value.clone();

                move |conn| {
                    diesel::replace_into(channel_meta::table)
                        .values(ChannelMetaRow::from(value))
                        .execute(conn)?;

                    Ok(())
                }
            })
            .await?;

        Ok(Some(ClientEvent::Channel {
            id: value.chat_id,
            event: ChannelEvent::MetaChanged(value.meta),
        }))
    }

    async fn on_chat_deleted(&self, value: SyncDlMsg) -> HandlerResult {
        self.conn
            .pool
            .spawn({
                let chatlog = value.chatlog.clone();

                move |conn| {
                    diesel::replace_into(chat::table)
                        .values(ChatRow::from_chatlog(chatlog, None))
                        .execute(conn)?;

                    Ok(())
                }
            })
            .await?;

        Ok(Some(ClientEvent::Channel {
            id: value.chatlog.channel_id,
            event: ChannelEvent::ChatDeleted(value.chatlog),
        }))
    }

    async fn on_channel_join(&self, sync_join: SyncJoin) -> HandlerResult {
        let _ = ChannelUpdater::new(sync_join.chat_id)
            .initialize(&self.conn.session, &self.conn.pool)
            .await;

        Ok(Some(ClientEvent::Channel {
            id: sync_join.chat_id,
            event: ChannelEvent::Added {
                chatlog: sync_join.chatlog,
            },
        }))
    }

    async fn on_left(&self, left: Left) -> HandlerResult {
        let channel_id = left.chat_id;

        self.conn
            .pool
            .spawn_transaction(move |conn| ChannelUpdater::new(channel_id).remove(conn))
            .await?;

        Ok(Some(ClientEvent::Channel {
            id: channel_id,
            event: ChannelEvent::Left,
        }))
    }

    async fn on_new_user(&self, new_mem: NewMem) -> Result<Option<ClientEvent>, HandlerError> {
        self.conn
            .pool
            .spawn({
                let chatlog = new_mem.chatlog.clone();

                move |conn| {
                    diesel::replace_into(chat::table)
                        .values(ChatRow::from_chatlog(chatlog, None))
                        .execute(conn)?;

                    Ok(())
                }
            })
            .await?;

        Ok(Some(ClientEvent::Channel {
            id: new_mem.chat_id,
            event: ChannelEvent::UserJoin(new_mem.chatlog),
        }))
    }

    async fn on_del_user(&self, del_mem: DelMem) -> Result<Option<ClientEvent>, HandlerError> {
        self.conn
            .pool
            .spawn({
                let chatlog = del_mem.chatlog.clone();

                move |conn| {
                    diesel::replace_into(chat::table)
                        .values(ChatRow::from_chatlog(chatlog, None))
                        .execute(conn)?;

                    Ok(())
                }
            })
            .await?;

        Ok(Some(ClientEvent::Channel {
            id: del_mem.chat_id,
            event: ChannelEvent::UserLeft(del_mem.chatlog),
        }))
    }
}
