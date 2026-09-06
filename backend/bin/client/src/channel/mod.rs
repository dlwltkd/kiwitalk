pub mod normal;

use std::{
    ops::Bound,
    sync::atomic::{AtomicI64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context};
use headless_talk::channel::ClientChannel;
use kiwi_talk_result::TauriResult;
use serde::Serialize;
use talk_loco_client::talk::chat::{Chat, ChatContent, ChatType};

use crate::event::{ChannelEvent, ClientEvent};
use crate::ClientState;

use self::normal::NormalChannel;

static LAST_OUTBOUND_MESSAGE_ID: AtomicI64 = AtomicI64::new(0);

fn next_outbound_message_id() -> i64 {
    let wall_clock_id = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(i64::MAX)
    .max(1);
    let mut previous = LAST_OUTBOUND_MESSAGE_ID.load(Ordering::Relaxed);

    loop {
        let next = wall_clock_id.max(previous.saturating_add(1));

        match LAST_OUTBOUND_MESSAGE_ID.compare_exchange_weak(
            previous,
            next,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return next,
            Err(actual) => previous = actual,
        }
    }
}

fn outbound_text_chat(text: String) -> Chat {
    Chat {
        chat_type: ChatType::TEXT,
        content: ChatContent {
            message: Some(text),
            attachment: Some("{}".to_owned()),
            ..Default::default()
        },
        message_id: next_outbound_message_id(),
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "content")]
#[serde(rename_all = "camelCase")]
pub(crate) enum Channel {
    Normal(NormalChannel),
}

#[tauri::command(async)]
pub(crate) async fn load_channel(id: String, client: ClientState<'_>) -> TauriResult<Channel> {
    let talk = client.talk()?;

    let channel = talk
        .load_channel(id.parse().context("invalid id")?)
        .await?
        .ok_or_else(|| anyhow!("channel not found"))?;

    match channel {
        ClientChannel::Normal(normal) => Ok(Channel::Normal(NormalChannel::from(normal))),

        _ => Err(anyhow!("unsupported channel types").into()),
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Chatlog {
    log_id: String,
    prev_log_id: Option<String>,

    sender_id: String,
    send_at: i64,

    chat_type: i32,

    content: Option<String>,
    attachment: Option<String>,
    supplement: Option<String>,

    referer: Option<i32>,
}

impl From<talk_loco_client::talk::chat::Chatlog> for Chatlog {
    fn from(log: talk_loco_client::talk::chat::Chatlog) -> Self {
        Chatlog {
            log_id: log.log_id.to_string(),
            prev_log_id: log.prev_log_id.map(|id| id.to_string()),
            sender_id: log.author_id.to_string(),
            send_at: log.send_at,
            chat_type: log.chat.chat_type.0,
            content: log.chat.content.message,
            attachment: log.chat.content.attachment,
            supplement: log.chat.content.supplement,
            referer: log.referer,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistorySyncResult {
    fetched_count: usize,
    page_count: usize,
    complete: bool,
    stop_reason: &'static str,
}

impl From<headless_talk::HistorySyncResult> for HistorySyncResult {
    fn from(result: headless_talk::HistorySyncResult) -> Self {
        use headless_talk::HistorySyncStop;

        let stop_reason = match result.stop {
            HistorySyncStop::UpToDate => "upToDate",
            HistorySyncStop::ServerComplete => "serverComplete",
            HistorySyncStop::ReachedTarget => "reachedTarget",
            HistorySyncStop::EmptyBatch => "emptyBatch",
            HistorySyncStop::NoProgress => "noProgress",
            HistorySyncStop::PageLimit => "pageLimit",
            HistorySyncStop::TimeLimit => "timeLimit",
            HistorySyncStop::UnsupportedChannel => "unsupportedChannel",
        };

        Self {
            fetched_count: result.fetched_count,
            page_count: result.page_count,
            complete: result.complete(),
            stop_reason,
        }
    }
}

#[tauri::command(async)]
pub(crate) async fn channel_sync_history(
    id: String,
    client: ClientState<'_>,
) -> TauriResult<HistorySyncResult> {
    let talk = client.talk()?;
    let result = talk
        .sync_channel_history(id.parse().context("invalid channel id")?)
        .await
        .context("cannot synchronize channel history")?;

    Ok(result.into())
}

#[tauri::command(async)]
pub(crate) async fn channel_send_text(
    id: String,
    text: String,
    client: ClientState<'_>,
) -> TauriResult<Chatlog> {
    let talk = client.talk()?;
    let event_tx = client.event_sender()?;

    let log = talk
        .channel(id.parse().context("invalid channel id")?)
        .send_chat(outbound_text_chat(text), false)
        .await
        .context("cannot send chat")?;
    let log = Chatlog::from(log);

    event_tx.enqueue(Ok(ClientEvent::Channel {
        id,
        event: ChannelEvent::Chat(log.clone()),
    }));

    Ok(log)
}

#[tauri::command(async)]
pub(crate) async fn channel_load_chat(
    id: String,
    from_log_id: Option<String>,
    count: i64,
    exclusive: bool,
    client: ClientState<'_>,
) -> TauriResult<Vec<Chatlog>> {
    let talk = client.talk()?;

    let chats = talk
        .channel(id.parse().context("invalid channel id")?)
        .load_chat_from(
            match from_log_id {
                Some(from_log_id) => {
                    let log_id = from_log_id.parse().context("invalid logId")?;

                    if exclusive {
                        Bound::Excluded(log_id)
                    } else {
                        Bound::Included(log_id)
                    }
                }

                _ => Bound::Unbounded,
            },
            count,
        )
        .await
        .context("cannot load chats from database")?;

    Ok(chats.into_iter().map(Into::into).collect::<Vec<_>>())
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChannelMeta {
    #[serde(rename = "type")]
    meta_type: i32,

    revision: String,

    author_id: String,

    updated_at: f64,

    content: String,
}

impl From<talk_loco_client::talk::channel::ChannelMeta> for ChannelMeta {
    fn from(meta: talk_loco_client::talk::channel::ChannelMeta) -> Self {
        Self {
            meta_type: meta.meta_type,
            revision: meta.revision.to_string(),
            author_id: meta.author_id.to_string(),
            updated_at: meta.updated_at as f64 * 1000.0,
            content: meta.content,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::outbound_text_chat;
    use talk_loco_client::talk::chat::ChatType;

    #[test]
    fn outbound_text_chat_has_fresh_android_write_fields() {
        let first = outbound_text_chat("first".to_owned());
        let second = outbound_text_chat("second".to_owned());

        assert_eq!(first.chat_type, ChatType::TEXT);
        assert_eq!(first.content.message.as_deref(), Some("first"));
        assert_eq!(first.content.attachment.as_deref(), Some("{}"));
        assert!(first.message_id > 0);
        assert!(second.message_id > first.message_id);
        assert_eq!(second.content.attachment.as_deref(), Some("{}"));
    }
}
