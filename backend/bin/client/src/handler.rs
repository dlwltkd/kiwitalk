use headless_talk::event::{
    channel::ChannelEvent as TalkChannelEvent, ClientEvent as TalkClientEvent,
};
use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

use crate::event::{ChannelEvent, EventSender};

use super::event::ClientEvent;

pub(crate) async fn handle_event<R: Runtime>(
    app: &AppHandle<R>,
    event: TalkClientEvent,
    tx: EventSender,
) -> anyhow::Result<()> {
    match event {
        TalkClientEvent::Channel { id, event } => {
            handle_channel_event(app, id, event, tx).await?;
        }

        TalkClientEvent::SwitchServer => {
            tx.enqueue(Ok(ClientEvent::SwitchServer));
        }

        TalkClientEvent::Kickout(reason) => {
            tx.enqueue(Ok(ClientEvent::Kickout { reason }));
        }

        _ => {}
    }

    Ok(())
}

async fn handle_channel_event<R: Runtime>(
    app: &AppHandle<R>,
    id: i64,
    event: TalkChannelEvent,
    tx: EventSender,
) -> anyhow::Result<()> {
    match event {
        TalkChannelEvent::Chat {
            chat,
            user_nickname,
            read,
            notify,
            ..
        } => {
            tx.enqueue(Ok(ClientEvent::Channel {
                id: id.to_string(),
                event: ChannelEvent::Chat {
                    chat: chat.clone().into(),
                    read,
                },
            }));

            if notify {
                let message = chat
                    .chat
                    .content
                    .message
                    .as_deref()
                    .unwrap_or("Unknown message");

                let _ = app
                    .notification()
                    .builder()
                    .title(user_nickname.as_deref().unwrap_or("KiwiTalk"))
                    .body(message)
                    .show();
            }
        }

        TalkChannelEvent::ChatRead { user_id, log_id } => {
            tx.enqueue(Ok(ClientEvent::Channel {
                id: id.to_string(),
                event: ChannelEvent::ChatRead {
                    user_id: user_id.to_string(),
                    log_id: log_id.to_string(),
                },
            }));
        }

        TalkChannelEvent::MetaChanged(meta) => {
            tx.enqueue(Ok(ClientEvent::Channel {
                id: id.to_string(),
                event: ChannelEvent::MetaChanged(meta.into()),
            }));
        }

        TalkChannelEvent::ChatDeleted(chatlog) => {
            tx.enqueue(Ok(ClientEvent::Channel {
                id: id.to_string(),
                event: ChannelEvent::ChatDeleted(chatlog.into()),
            }));
        }

        TalkChannelEvent::Added { chatlog } => {
            tx.enqueue(Ok(ClientEvent::Channel {
                id: id.to_string(),
                event: ChannelEvent::Added(chatlog.map(|log| log.into())),
            }));
        }

        TalkChannelEvent::Left => {
            tx.enqueue(Ok(ClientEvent::Channel {
                id: id.to_string(),
                event: ChannelEvent::Left,
            }));
        }

        _ => {}
    }

    Ok(())
}
