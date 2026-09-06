pub mod normal;

use std::{
    ops::Bound,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context};
use headless_talk::{channel::ClientChannel, ClientError};
use kiwi_talk_result::TauriResult;
use serde::Serialize;
use talk_loco_client::{
    talk::chat::{Chat, ChatContent, ChatType},
    RequestError,
};

use crate::event::{ChannelEvent, ClientEvent};
use crate::ClientState;

use self::normal::NormalChannel;

const ANDROID_MESSAGE_ID_MODULUS: i64 = 2_147_483_547;

#[derive(Debug, Default)]
struct OutboundMessageIdState {
    device_hash: i64,
    last_id: i64,
    last_generated_id: i64,
}

static OUTBOUND_MESSAGE_ID_STATE: Mutex<OutboundMessageIdState> =
    Mutex::new(OutboundMessageIdState {
        device_hash: 0,
        last_id: 0,
        last_generated_id: 0,
    });

pub(super) fn android_message_id_device_hash(device_uuid: &str) -> i64 {
    let hash = device_uuid.encode_utf16().fold(0_i32, |hash, character| {
        hash.wrapping_mul(31).wrapping_add(i32::from(character))
    });

    i64::from(hash % 100)
}

fn android_message_id_for_time(time_millis: i64, device_hash: i64) -> i64 {
    ((time_millis.rem_euclid(ANDROID_MESSAGE_ID_MODULUS) / 100) * 100) + device_hash
}

fn next_outbound_message_id(device_hash: i64) -> i64 {
    let time_millis = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(i64::MAX);
    let generated_id = android_message_id_for_time(time_millis, device_hash);
    let mut state = OUTBOUND_MESSAGE_ID_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if state.device_hash != device_hash {
        state.device_hash = device_hash;
        state.last_id = 0;
        state.last_generated_id = 0;
    }

    let mut next = if state.last_id == 0
        || generated_id > state.last_id
        || generated_id < state.last_generated_id
    {
        generated_id
    } else {
        state.last_id + 100
    };
    if next > i64::from(i32::MAX) {
        next = android_message_id_for_time(next, device_hash);
    }

    state.last_id = next;
    state.last_generated_id = generated_id;

    next
}

fn outbound_text_chat(text: String, message_id_device_hash: i64) -> Chat {
    Chat {
        chat_type: ChatType::TEXT,
        content: ChatContent {
            message: Some(text),
            attachment: Some("{}".to_owned()),
            ..Default::default()
        },
        message_id: next_outbound_message_id(message_id_device_hash),
    }
}

fn log_chat_send_error(error: &ClientError) {
    match error {
        ClientError::Request(RequestError::Status(status)) => {
            log::warn!("chat send failed; stage=write; kind=status; status={status}");
        }
        ClientError::Request(RequestError::Serialize(error)) => {
            log::warn!("chat send failed; stage=write; kind=serialize; detail={error}");
        }
        ClientError::Request(RequestError::Deserialize(error)) => {
            log::warn!("chat send failed; stage=write; kind=deserialize; detail={error}");
        }
        ClientError::Request(RequestError::Read(error)) => {
            log::warn!(
                "chat send failed; stage=write; kind=read; io_kind={:?}",
                error.kind()
            );
        }
        ClientError::Request(RequestError::Write(error)) => {
            log::warn!(
                "chat send failed; stage=write; kind=write; io_kind={:?}",
                error.kind()
            );
        }
        ClientError::Database(error) => {
            log::warn!("chat send failed; stage=database; kind=database; detail={error}");
        }
    }
}

fn log_history_sync_error(error: &ClientError) {
    match error {
        ClientError::Request(RequestError::Status(status)) => {
            log::warn!("chat history sync failed; stage=sync; kind=status; status={status}");
        }
        ClientError::Request(RequestError::Serialize(error)) => {
            log::warn!("chat history sync failed; stage=sync; kind=serialize; detail={error}");
        }
        ClientError::Request(RequestError::Deserialize(error)) => {
            log::warn!("chat history sync failed; stage=sync; kind=deserialize; detail={error}");
        }
        ClientError::Request(RequestError::Read(error)) => {
            log::warn!(
                "chat history sync failed; stage=sync; kind=read; io_kind={:?}",
                error.kind()
            );
        }
        ClientError::Request(RequestError::Write(error)) => {
            log::warn!(
                "chat history sync failed; stage=sync; kind=write; io_kind={:?}",
                error.kind()
            );
        }
        ClientError::Database(error) => {
            log::warn!("chat history sync failed; stage=database; kind=database; detail={error}");
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "content")]
#[serde(rename_all = "camelCase")]
pub(crate) enum Channel {
    Normal(NormalChannel),
    Open(OpenChannel),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenChannel {
    users: Vec<(String, OpenChannelUser)>,
    metas: Vec<ChannelMeta>,
    open_token: i32,
}

impl From<headless_talk::channel::open::OpenChannel> for OpenChannel {
    fn from(open: headless_talk::channel::open::OpenChannel) -> Self {
        Self {
            users: open
                .users
                .into_iter()
                .map(|(id, user)| (id.to_string(), OpenChannelUser::from(user)))
                .collect(),
            metas: open.meta_map.into_values().map(ChannelMeta::from).collect(),
            open_token: open.open_token,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenChannelUser {
    nickname: String,
    profile_url: String,
    full_profile_url: String,
    original_profile_url: String,
    watermark: String,
    user_type: i32,
    account_id: String,
    country_iso: Option<String>,
    service_user_type: Option<i32>,
    suspended: bool,
    suspicion: String,
    open_member_type: i32,
    profile_type: i32,
    profile_link_id: String,
    open_token: i32,
}

impl From<headless_talk::channel::open::OpenChannelUser> for OpenChannelUser {
    fn from(user: headless_talk::channel::open::OpenChannelUser) -> Self {
        Self {
            nickname: user.profile.nickname,
            profile_url: user.profile.image_url,
            full_profile_url: user.profile.full_image_url,
            original_profile_url: user.profile.original_image_url,
            watermark: user.watermark.to_string(),
            user_type: user.user_type,
            account_id: user.account_id.to_string(),
            country_iso: user.country_iso,
            service_user_type: user.service_user_type,
            suspended: user.suspended,
            suspicion: user.suspicion,
            open_member_type: user.open_member_type,
            profile_type: user.profile_type,
            profile_link_id: user.profile_link_id.to_string(),
            open_token: user.open_token,
        }
    }
}

#[tauri::command]
pub(crate) fn channel_set_active(
    id: String,
    active: bool,
    client: ClientState<'_>,
) -> TauriResult<()> {
    let talk = client.talk()?;
    talk.set_channel_active(id.parse().context("invalid channel id")?, active);

    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn load_channel(id: String, client: ClientState<'_>) -> TauriResult<Channel> {
    let talk = client.talk()?;
    let event_tx = client.event_sender()?;
    let channel_id = id.parse().context("invalid id")?;

    let channel = talk
        .load_channel(channel_id)
        .await?
        .ok_or_else(|| anyhow!("channel not found"))?;

    event_tx.enqueue(Ok(ClientEvent::Channel {
        id,
        event: ChannelEvent::UnreadChanged { unread_count: 0 },
    }));

    match channel {
        ClientChannel::Normal(normal) => Ok(Channel::Normal(NormalChannel::from(normal))),
        ClientChannel::Open(open) => Ok(Channel::Open(OpenChannel::from(open))),
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
    cached_count: usize,
    gap_count: usize,
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
            HistorySyncStop::HistoryGap => "historyGap",
            HistorySyncStop::PageLimit => "pageLimit",
            HistorySyncStop::TimeLimit => "timeLimit",
            HistorySyncStop::UnsupportedChannel => "unsupportedChannel",
        };

        Self {
            fetched_count: result.fetched_count,
            page_count: result.page_count,
            cached_count: result.cached_count,
            gap_count: result.gap_count,
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
    let channel_id = id.parse().context("invalid channel id")?;

    log::info!("chat history sync started; stage=sync");

    let result = match talk.sync_channel_history(channel_id).await {
        Ok(result) => result,
        Err(error) => {
            log_history_sync_error(&error);
            return Err(anyhow::Error::new(error)
                .context("cannot synchronize channel history")
                .into());
        }
    };

    log::info!(
        "chat history sync finished; stage=complete; fetched={}; pages={}; cached={}; gaps={}; stop={:?}",
        result.fetched_count,
        result.page_count,
        result.cached_count,
        result.gap_count,
        result.stop
    );

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
    let message_id_device_hash = client.message_id_device_hash()?;
    let channel_id = id.parse().context("invalid channel id")?;

    log::info!("chat send started; stage=write");

    let log = match talk
        .channel(channel_id)
        .send_chat(outbound_text_chat(text, message_id_device_hash), false)
        .await
    {
        Ok(log) => log,
        Err(err) => {
            log_chat_send_error(&err);
            return Err(anyhow::Error::new(err).context("cannot send chat").into());
        }
    };
    let log = Chatlog::from(log);

    log::info!("chat send succeeded; stage=complete");

    event_tx.enqueue(Ok(ClientEvent::Channel {
        id,
        event: ChannelEvent::Chat {
            chat: log.clone(),
            read: true,
        },
    }));

    Ok(log)
}

#[tauri::command(async)]
pub(crate) async fn channel_load_archive(
    id: String,
    client: ClientState<'_>,
) -> TauriResult<Option<headless_talk::archive::ChatArchive>> {
    Ok(client
        .talk()?
        .load_chat_archive(id.parse().context("invalid channel id")?)
        .await?)
}

#[tauri::command(async)]
pub(crate) async fn channel_import_archive(
    id: String,
    source_name: String,
    text: String,
    utc_offset_minutes: i32,
    client: ClientState<'_>,
) -> TauriResult<headless_talk::archive::ChatArchive> {
    Ok(client
        .talk()?
        .import_chat_archive(
            id.parse().context("invalid channel id")?,
            source_name,
            text,
            utc_offset_minutes,
        )
        .await?)
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
    use super::{
        android_message_id_device_hash, android_message_id_for_time, outbound_text_chat,
        ANDROID_MESSAGE_ID_MODULUS,
    };
    use talk_loco_client::talk::chat::ChatType;

    #[test]
    fn outbound_text_chat_has_fresh_android_write_fields() {
        let first = outbound_text_chat("first".to_owned(), 17);
        let second = outbound_text_chat("second".to_owned(), 17);

        assert_eq!(first.chat_type, ChatType::TEXT);
        assert_eq!(first.content.message.as_deref(), Some("first"));
        assert_eq!(first.content.attachment.as_deref(), Some("{}"));
        assert!(first.message_id > 0);
        assert!(second.message_id > first.message_id);
        assert!(second.message_id <= i64::from(i32::MAX));
        assert_eq!(first.message_id % 100, 17);
        assert_eq!(second.content.attachment.as_deref(), Some("{}"));
    }

    #[test]
    fn android_message_id_matches_current_app_algorithm() {
        assert_eq!(
            android_message_id_for_time(1_788_674_000_123, -23),
            ((1_788_674_000_123_i64 % ANDROID_MESSAGE_ID_MODULUS) / 100) * 100 - 23
        );
        assert_eq!(android_message_id_device_hash("abc"), 54);
    }
}
