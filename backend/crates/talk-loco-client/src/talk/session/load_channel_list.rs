use serde::{Deserialize, Deserializer, Serialize};
use serde_with::skip_serializing_none;

use crate::talk::{channel::ChannelType, chat::Chatlog};

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Request<'a> {
    /// Known chatroom id list
    #[serde(rename = "chatIds")]
    pub chat_ids: &'a [i64],

    /// last seen log id paired with chat_ids
    #[serde(rename = "maxIds")]
    pub max_ids: &'a [i64],

    /// Unknown
    #[serde(rename = "lastTokenId")]
    pub last_token_id: i64,

    /// Last chatroom id from list in last response
    #[serde(rename = "lastChatId")]
    pub last_chat_id: Option<i64>,
}

/// Android LCHATLIST request. Unlike the initial LOGINLIST request,
/// `lastChatId` is always present on Android.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PageRequest<'a> {
    #[serde(rename = "chatIds")]
    pub chat_ids: &'a [i64],

    #[serde(rename = "maxIds")]
    pub max_ids: &'a [i64],

    #[serde(rename = "lastTokenId")]
    pub last_token_id: i64,

    #[serde(rename = "lastChatId")]
    pub last_chat_id: i64,
}

/// Request every chatroom list
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Response {
    /// false if there is more channels to be requested with LCHATLIST
    pub eof: bool,

    /// Latest mcm(?) revision
    #[serde(rename = "mcmRevision")]
    pub mcm_revision: i64,

    /// Latest chatroom id
    #[serde(rename = "lastChatId")]
    pub last_chat_id: Option<i64>,

    /// Latest token(Unknown) id
    #[serde(rename = "lastTokenId")]
    pub last_token_id: Option<i64>,

    /// Latest token(Unknown)(?)
    #[serde(rename = "ltk")]
    pub last_token: Option<i64>,

    /// Latest block token(Unknown)(?)
    #[serde(rename = "lbk")]
    pub last_block_token: i64,

    // Unknown, Unknown item type
    //pub kc: Vec<()>
    /// Deleted chatroom ids(?)
    #[serde(rename = "delChatIds")]
    pub deleted_chat_ids: Vec<i64>,

    /// Chatrooms from which this account was removed.
    #[serde(default, rename = "kc")]
    pub kicked_chat_ids: Vec<i64>,

    #[serde(rename = "chatDatas")]
    pub chat_datas: Vec<ChannelListData>,
}

fn default_true() -> bool {
    true
}

fn default_android_last_token() -> i32 {
    -1
}

pub(super) fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

fn deserialize_default_true<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<bool>::deserialize(deserializer)?.unwrap_or(true))
}

fn deserialize_android_last_token<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<i32>::deserialize(deserializer)?.unwrap_or(-1))
}

fn deserialize_i64_list_lossy<'de, D>(deserializer: D) -> Result<Vec<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = Option::<Vec<bson::Bson>>::deserialize(deserializer)?.unwrap_or_default();

    Ok(values
        .into_iter()
        .filter_map(|value| match value {
            bson::Bson::Int64(value) => Some(value),
            _ => None,
        })
        .collect())
}

fn deserialize_string_list_lossy<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = Option::<Vec<bson::Bson>>::deserialize(deserializer)?.unwrap_or_default();

    Ok(values
        .into_iter()
        .filter_map(|value| match value {
            bson::Bson::String(value) => Some(value),
            _ => None,
        })
        .collect())
}

fn deserialize_string_default_lossy<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match Option::<bson::Bson>::deserialize(deserializer)? {
        Some(bson::Bson::String(value)) => value,
        _ => String::new(),
    })
}

fn deserialize_optional_i64_lossy<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match Option::<bson::Bson>::deserialize(deserializer)? {
        Some(bson::Bson::Int64(value)) => Some(value),
        Some(bson::Bson::Int32(value)) => Some(i64::from(value)),
        _ => None,
    })
}

fn deserialize_optional_i32_lossy<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match Option::<bson::Bson>::deserialize(deserializer)? {
        Some(bson::Bson::Int32(value)) => Some(value),
        Some(bson::Bson::Int64(value)) => i32::try_from(value).ok(),
        _ => None,
    })
}

fn deserialize_optional_string_lossy<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match Option::<bson::Bson>::deserialize(deserializer)? {
        Some(bson::Bson::String(value)) => Some(value),
        _ => None,
    })
}

/// Android 26.7.2 LCHATLIST response before normalization into the legacy
/// cross-platform representation.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AndroidResponse {
    #[serde(
        default,
        rename = "mcmRevision",
        deserialize_with = "deserialize_null_default"
    )]
    pub mcm_revision: i32,

    #[serde(
        default,
        rename = "chatDatas",
        deserialize_with = "deserialize_null_default"
    )]
    pub chat_datas: Vec<AndroidChannelListData>,

    #[serde(
        default,
        rename = "delChatIds",
        deserialize_with = "deserialize_null_default"
    )]
    pub deleted_chat_ids: Vec<i64>,

    #[serde(default, rename = "kc", deserialize_with = "deserialize_null_default")]
    pub kicked_chat_ids: Vec<i64>,

    #[serde(
        default = "default_true",
        deserialize_with = "deserialize_default_true"
    )]
    pub eof: bool,

    #[serde(
        default,
        rename = "lastTokenId",
        deserialize_with = "deserialize_null_default"
    )]
    pub last_token_id: i64,

    #[serde(default, rename = "lbk", deserialize_with = "deserialize_null_default")]
    pub last_block_token: i32,

    #[serde(
        default,
        rename = "lastChatId",
        deserialize_with = "deserialize_null_default"
    )]
    pub last_chat_id: i64,

    #[serde(
        default = "default_android_last_token",
        rename = "ltk",
        deserialize_with = "deserialize_android_last_token"
    )]
    pub last_token: i32,
}

impl From<AndroidResponse> for Response {
    fn from(response: AndroidResponse) -> Self {
        Self {
            eof: response.eof,
            mcm_revision: i64::from(response.mcm_revision),
            last_chat_id: Some(response.last_chat_id),
            last_token_id: Some(response.last_token_id),
            last_token: Some(i64::from(response.last_token)),
            last_block_token: i64::from(response.last_block_token),
            deleted_chat_ids: response.deleted_chat_ids,
            kicked_chat_ids: response.kicked_chat_ids,
            chat_datas: response
                .chat_datas
                .into_iter()
                .map(normalize_android_channel)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AndroidChannelListData {
    #[serde(rename = "c")]
    pub id: i64,

    #[serde(flatten)]
    pub channel_type: ChannelListType,

    #[serde(rename = "s")]
    pub last_seen_log_id: i64,

    #[serde(default, rename = "ll", deserialize_with = "deserialize_null_default")]
    pub last_log_id: i64,

    #[serde(default, rename = "l")]
    pub preview: Option<AndroidChatPreview>,

    #[serde(rename = "a")]
    pub active_member_count: i32,

    #[serde(rename = "n")]
    pub unread_count: i32,

    #[serde(default, rename = "ii", deserialize_with = "deserialize_null_default")]
    pub inviter_id: i64,

    #[serde(default, rename = "o", deserialize_with = "deserialize_null_default")]
    pub last_log_send_at: i32,

    #[serde(rename = "p")]
    pub push_alert: bool,

    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub mmr: i64,

    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub jn: i32,

    #[serde(
        default,
        rename = "m",
        deserialize_with = "deserialize_string_default_lossy"
    )]
    pub metadata: String,

    #[serde(default, rename = "ml", deserialize_with = "deserialize_null_default")]
    pub min_log_id: i64,

    #[serde(default, rename = "bmids")]
    pub blind_member_ids: Option<Vec<i64>>,

    #[serde(default, rename = "i", deserialize_with = "deserialize_i64_list_lossy")]
    pub icon_user_ids: Vec<i64>,

    #[serde(
        default,
        rename = "k",
        deserialize_with = "deserialize_string_list_lossy"
    )]
    pub icon_user_nicknames: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AndroidChatPreview {
    #[serde(
        default,
        rename = "logId",
        deserialize_with = "deserialize_optional_i64_lossy"
    )]
    pub log_id: Option<i64>,

    #[serde(
        default,
        rename = "prevId",
        deserialize_with = "deserialize_optional_i64_lossy"
    )]
    pub prev_log_id: Option<i64>,

    #[serde(
        default,
        rename = "chatId",
        deserialize_with = "deserialize_optional_i64_lossy"
    )]
    pub channel_id: Option<i64>,

    #[serde(
        default,
        rename = "authorId",
        deserialize_with = "deserialize_optional_i64_lossy"
    )]
    pub author_id: Option<i64>,

    #[serde(
        default,
        rename = "sendAt",
        deserialize_with = "deserialize_optional_i64_lossy"
    )]
    pub send_at: Option<i64>,

    #[serde(
        default,
        rename = "type",
        deserialize_with = "deserialize_optional_i32_lossy"
    )]
    pub chat_type: Option<i32>,

    #[serde(
        default,
        rename = "msgId",
        deserialize_with = "deserialize_optional_i64_lossy"
    )]
    pub message_id: Option<i64>,

    #[serde(default, deserialize_with = "deserialize_optional_string_lossy")]
    pub message: Option<String>,

    #[serde(default, deserialize_with = "deserialize_optional_string_lossy")]
    pub attachment: Option<String>,

    #[serde(default, deserialize_with = "deserialize_optional_string_lossy")]
    pub supplement: Option<String>,

    #[serde(default, deserialize_with = "deserialize_optional_i32_lossy")]
    pub referer: Option<i32>,
}

impl AndroidChatPreview {
    fn into_chatlog(self, parent_channel_id: i64, fallback_send_at: i64) -> Option<Chatlog> {
        let log_id = self.log_id?;
        let chat_type = self.chat_type.unwrap_or(1);

        Some(Chatlog {
            log_id,
            prev_log_id: self.prev_log_id,
            channel_id: self
                .channel_id
                .filter(|channel_id| *channel_id != 0)
                .unwrap_or(parent_channel_id),
            author_id: self.author_id.unwrap_or(-1),
            send_at: self.send_at.unwrap_or(fallback_send_at),
            chat: crate::talk::chat::Chat {
                chat_type: crate::talk::chat::ChatType(chat_type),
                content: crate::talk::chat::ChatContent {
                    message: self.message,
                    attachment: self.attachment,
                    supplement: self.supplement,
                },
                message_id: self.message_id.unwrap_or_default(),
            },
            referer: self.referer,
        })
    }
}

pub(super) fn normalize_android_channel(channel: AndroidChannelListData) -> ChannelListData {
    let preview_log_id = channel.preview.as_ref().and_then(|preview| preview.log_id);
    let last_log_id = channel
        .last_log_id
        .ne(&0)
        .then_some(channel.last_log_id)
        .or(preview_log_id)
        .unwrap_or(0);
    let chatlog = channel
        .preview
        .and_then(|preview| preview.into_chatlog(channel.id, i64::from(channel.last_log_send_at)));

    ChannelListData {
        id: channel.id,
        channel_type: channel.channel_type,
        last_log_id,
        last_seen_log_id: Some(channel.last_seen_log_id),
        chatlog,
        active_member_count: channel.active_member_count,
        unread_count: channel.unread_count,
        last_log_send_at: i64::from(channel.last_log_send_at),
        push_alert: channel.push_alert,
        icon_user_ids: Some(channel.icon_user_ids),
        icon_user_nicknames: Some(channel.icon_user_nicknames),
        mmr: channel.mmr,
        jn: Some(channel.jn),
        inviter_id: Some(channel.inviter_id),
        metadata: Some(channel.metadata),
        min_log_id: Some(channel.min_log_id),
        blind_member_ids: channel.blind_member_ids,
    }
}

/// LOGINLIST chatroom list item.
/// Including essential chatroom info.
#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ChannelListData {
    /// Chatroom id
    #[serde(rename = "c")]
    pub id: i64,

    #[serde(flatten)]
    pub channel_type: ChannelListType,

    /// Last chat log id
    #[serde(rename = "ll")]
    pub last_log_id: i64,

    /// Last seen chat log id
    #[serde(rename = "s")]
    pub last_seen_log_id: Option<i64>,

    /// Last Chatlog
    #[serde(rename = "l")]
    pub chatlog: Option<Chatlog>,

    /// Active member count
    #[serde(rename = "a")]
    pub active_member_count: i32,

    /// Unread message count
    #[serde(rename = "n")]
    pub unread_count: i32,

    #[serde(rename = "o")]
    pub last_log_send_at: i64,

    // /// Chatroom metadata(?)
    // #[serde(rename = "m")]
    // pub metadata: ()
    /// Push alert setting
    #[serde(rename = "p")]
    pub push_alert: bool,

    /// Chatroom preview icon target user id list
    #[serde(rename = "i")]
    pub icon_user_ids: Option<Vec<i64>>,

    /// Chatroom preview icon target user name list
    #[serde(rename = "k")]
    pub icon_user_nicknames: Option<Vec<String>>,

    /// Unknown. Always 0 on openchat rooms.
    pub mmr: i64,

    /// Unknown. Only appears on non openchat rooms.
    pub jn: Option<i32>,

    /// User who invited this account to the room.
    #[serde(default, rename = "ii")]
    pub inviter_id: Option<i64>,

    /// Serialized room metadata supplied with the list item.
    #[serde(default, rename = "m")]
    pub metadata: Option<String>,

    /// Oldest log id retained for the room.
    #[serde(default, rename = "ml")]
    pub min_log_id: Option<i64>,

    /// Members hidden from the room preview.
    #[serde(default, rename = "bmids")]
    pub blind_member_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(tag = "t")]
pub enum ChannelListType {
    DirectChat,
    MultiChat,

    MemoChat,

    #[serde(rename = "OD")]
    OpenDirect(OpenChannelList),

    #[serde(rename = "OM")]
    OpenMulti(OpenChannelList),

    #[serde(other)]
    Other,
}

impl ChannelListType {
    pub fn ty(&self) -> Option<ChannelType> {
        Some(match self {
            ChannelListType::DirectChat => ChannelType::DirectChat,
            ChannelListType::MultiChat => ChannelType::MultiChat,
            ChannelListType::MemoChat => ChannelType::MemoChat,
            ChannelListType::OpenDirect(_) => ChannelType::OpenDirect,
            ChannelListType::OpenMulti(_) => ChannelType::OpenMulti,
            ChannelListType::Other => return None,
        })
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct OpenChannelList {
    #[serde(
        default = "default_missing_open_id",
        rename = "li",
        deserialize_with = "deserialize_missing_open_id"
    )]
    pub link_id: i64,

    #[serde(
        default = "default_missing_open_token",
        rename = "otk",
        deserialize_with = "deserialize_missing_open_token"
    )]
    pub open_token: i32,
}

const fn default_missing_open_id() -> i64 {
    -1
}

const fn default_missing_open_token() -> i32 {
    -1
}

fn deserialize_missing_open_id<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<i64>::deserialize(deserializer)?.unwrap_or(-1))
}

fn deserialize_missing_open_token<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<i32>::deserialize(deserializer)?.unwrap_or(-1))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn android_lchatlist_always_serializes_last_chat_id() {
        let request = PageRequest {
            chat_ids: &[11],
            max_ids: &[22],
            last_token_id: 0,
            last_chat_id: 0,
        };

        let document = bson::to_document(&request).unwrap();
        let keys = document.keys().map(String::as_str).collect::<BTreeSet<_>>();

        assert_eq!(
            keys,
            BTreeSet::from(["chatIds", "lastChatId", "lastTokenId", "maxIds"])
        );
        assert_eq!(document.get_i64("lastChatId"), Ok(0));
    }

    #[test]
    fn normalizes_android_lchatlist_integer_widths_and_state() {
        let response: AndroidResponse = bson::from_document(bson::doc! {
            "mcmRevision": 9_i32,
            "chatDatas": [{
                "c": 100_i64,
                "t": "MultiChat",
                "a": 3_i32,
                "n": 2_i32,
                "s": 4_i64,
                "ll": 5_i64,
                "o": 6_i32,
                "p": true,
            }],
            "delChatIds": [11_i64],
            "kc": [12_i64],
            "eof": false,
            "lastTokenId": 13_i64,
            "lbk": 14_i32,
            "lastChatId": 15_i64,
            "ltk": 16_i32,
        })
        .unwrap();

        let response = Response::from(response);
        assert_eq!(response.mcm_revision, 9);
        assert_eq!(response.last_token_id, Some(13));
        assert_eq!(response.last_block_token, 14);
        assert_eq!(response.last_chat_id, Some(15));
        assert_eq!(response.last_token, Some(16));
        assert_eq!(response.deleted_chat_ids, [11]);
        assert_eq!(response.kicked_chat_ids, [12]);
        assert_eq!(response.chat_datas[0].last_log_id, 5);
        assert_eq!(response.chat_datas[0].last_log_send_at, 6);
    }

    #[test]
    fn preserves_full_android_list_preview() {
        let response = Response::from(
            bson::from_document::<AndroidResponse>(bson::doc! {
                "chatDatas": [{
                    "c": 100_i64,
                    "t": "MultiChat",
                    "a": 3_i32,
                    "n": 1_i32,
                    "s": 4_i64,
                    "ll": 9_i64,
                    "o": 123_i32,
                    "p": true,
                    "l": {
                        "logId": 9_i64,
                        "prevId": 8_i64,
                        "chatId": 100_i64,
                        "authorId": 200_i64,
                        "sendAt": 123_i32,
                        "type": 1_i32,
                        "msgId": 77_i64,
                        "message": "latest",
                    },
                }],
            })
            .unwrap(),
        );

        let preview = response.chat_datas[0].chatlog.as_ref().unwrap();
        assert_eq!(preview.log_id, 9);
        assert_eq!(preview.prev_log_id, Some(8));
        assert_eq!(preview.channel_id, 100);
        assert_eq!(preview.author_id, 200);
        assert_eq!(preview.send_at, 123);
        assert_eq!(preview.chat.chat_type.0, 1);
        assert_eq!(preview.chat.message_id, 77);
        assert_eq!(preview.chat.content.message.as_deref(), Some("latest"));
    }

    #[test]
    fn fills_sparse_android_list_preview_from_parent_room() {
        let response = Response::from(
            bson::from_document::<AndroidResponse>(bson::doc! {
                "chatDatas": [{
                    "c": 100_i64,
                    "t": "MultiChat",
                    "a": 3_i32,
                    "n": 0_i32,
                    "s": 4_i64,
                    "o": 123_i32,
                    "p": true,
                    "l": {
                        "logId": 9_i64,
                        "message": "latest",
                    },
                }],
            })
            .unwrap(),
        );

        let data = &response.chat_datas[0];
        let preview = data.chatlog.as_ref().unwrap();
        assert_eq!(data.last_log_id, 9);
        assert_eq!(preview.channel_id, 100);
        assert_eq!(preview.author_id, -1);
        assert_eq!(preview.send_at, 123);
        assert_eq!(preview.chat.chat_type.0, 1);
        assert_eq!(preview.chat.content.message.as_deref(), Some("latest"));
    }

    #[test]
    fn does_not_treat_android_read_watermark_as_latest_log() {
        let response = Response::from(
            bson::from_document::<AndroidResponse>(bson::doc! {
                "chatDatas": [{
                    "c": 100_i64,
                    "t": "MultiChat",
                    "a": 3_i32,
                    "n": 0_i32,
                    "s": 42_i64,
                    "o": 123_i32,
                    "p": true,
                }],
            })
            .unwrap(),
        );

        let data = &response.chat_datas[0];
        assert_eq!(data.last_seen_log_id, Some(42));
        assert_eq!(data.last_log_id, 0);
    }

    #[test]
    fn applies_android_lchatlist_defaults() {
        let response = Response::from(
            bson::from_document::<AndroidResponse>(bson::doc! {
                "mcmRevision": bson::Bson::Null,
                "chatDatas": bson::Bson::Null,
                "delChatIds": bson::Bson::Null,
                "kc": bson::Bson::Null,
                "eof": bson::Bson::Null,
                "lastTokenId": bson::Bson::Null,
                "lbk": bson::Bson::Null,
                "lastChatId": bson::Bson::Null,
                "ltk": bson::Bson::Null,
            })
            .unwrap(),
        );

        assert!(response.eof);
        assert_eq!(response.mcm_revision, 0);
        assert_eq!(response.last_token_id, Some(0));
        assert_eq!(response.last_block_token, 0);
        assert_eq!(response.last_chat_id, Some(0));
        assert_eq!(response.last_token, Some(-1));
        assert!(response.chat_datas.is_empty());
    }
}
