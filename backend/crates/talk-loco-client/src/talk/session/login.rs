use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::talk::session::load_channel_list;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    Desktop,
    AndroidSubdevice,
}

/// Login to loco server
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Request<'a> {
    /// Current OS (win32, android, mac, etc.)
    pub os: &'a str,

    /// Network type (0 for wired)
    #[serde(rename = "ntype")]
    pub net_type: i16,

    /// Official app version
    #[serde(rename = "appVer")]
    pub app_version: &'a str,

    /// Network MCCMNC ("999" on pc)
    #[serde(rename = "MCCMNC")]
    pub mccmnc: &'a str,

    /// Protocol version. "1" on mobile and "1.0" on PC.
    #[serde(rename = "prtVer")]
    pub protocol_version: &'a str,

    /// Device uuid String. Usually hashed unique id.
    #[serde(rename = "duuid")]
    pub device_uuid: &'a str,

    /// OAuth access token
    #[serde(rename = "oauthToken")]
    pub oauth_token: &'a str,

    #[serde(rename = "lang")]
    pub language: &'a str,

    /// Device type (PC Only(?)) (2 for pc)
    #[serde(rename = "dtype")]
    pub device_type: Option<i8>,

    /// Unknown (Mobile only)
    pub revision: Option<i32>,

    /// 6 bytes binary (0x?? 0x?? 0xff 0xff 0x?? 0x??)
    #[serde(with = "serde_byte_array")]
    pub rp: [u8; 6],

    /// PC status (PC only) (Same with SETST status?)
    #[serde(rename = "pcst")]
    pub pc_status: Option<i32>,

    #[serde(flatten)]
    pub chat_list: load_channel_list::Request<'a>,

    /// Unknown
    #[serde(rename = "lbk")]
    pub last_block_token: i32,

    /// background checking(?) (Mobile only)
    #[serde(rename = "bg")]
    pub background: Option<bool>,
}

/// Contains userId, tokens, chatroom list.
/// The purposes of tokens, revisions are unknown yet.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Response {
    /// Logon user id
    #[serde(rename = "userId")]
    pub user_id: i64,

    #[serde(flatten)]
    pub chat_list: load_channel_list::Response,

    /// Oldest chat id (?)
    #[serde(rename = "minLogId")]
    pub min_log_id: Option<i64>,

    /// Unknown (Mobile only)
    pub revision: Option<i32>,

    /// Revision(?) Info (Json) (Mobile only)
    #[serde(rename = "revisionInfo")]
    pub revision_info: Option<String>,

    /// Unknown
    pub sb: i32,

    /// 6 bytes binary
    pub rp: Option<bson::Binary>,

    /// Unknown
    #[serde(rename = "pkUpdate")]
    pub pk_update: Option<bool>,

    /// Unknown
    #[serde(rename = "pkToken")]
    pub pk_token: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AndroidResponse {
    #[serde(rename = "userId")]
    pub user_id: i64,

    #[serde(default)]
    pub revision: Option<i32>,

    #[serde(default, rename = "revisionInfo")]
    pub revision_info: Option<String>,

    #[serde(default)]
    pub rp: Option<bson::Binary>,

    #[serde(default, rename = "minLogId")]
    pub min_log_id: Option<i64>,

    #[serde(default)]
    pub sb: i32,

    #[serde(default, rename = "chatDatas")]
    pub chat_datas: Vec<AndroidChannelListData>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AndroidChannelListData {
    #[serde(rename = "c")]
    pub id: i64,

    #[serde(flatten)]
    pub channel_type: load_channel_list::ChannelListType,

    #[serde(default, rename = "s")]
    pub last_seen_log_id: Option<i64>,

    #[serde(default, rename = "l")]
    pub preview: Option<AndroidChatPreview>,

    #[serde(default, rename = "a")]
    pub active_member_count: i32,

    #[serde(default, rename = "n")]
    pub unread_count: i32,

    #[serde(default, rename = "i")]
    pub icon_user_ids: Option<Vec<i64>>,

    #[serde(default, rename = "k")]
    pub icon_user_nicknames: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AndroidChatPreview {
    #[serde(rename = "logId")]
    pub log_id: i64,
}

impl From<AndroidResponse> for Response {
    fn from(response: AndroidResponse) -> Self {
        let chat_datas = response
            .chat_datas
            .into_iter()
            .map(|channel| {
                let last_log_id = channel
                    .preview
                    .as_ref()
                    .map(|preview| preview.log_id)
                    .or(channel.last_seen_log_id)
                    .unwrap_or(0);

                load_channel_list::ChannelListData {
                    id: channel.id,
                    channel_type: channel.channel_type,
                    last_log_id,
                    last_seen_log_id: channel.last_seen_log_id,
                    chatlog: None,
                    active_member_count: channel.active_member_count,
                    unread_count: channel.unread_count,
                    last_update: 0,
                    push_alert: false,
                    icon_user_ids: channel.icon_user_ids,
                    icon_user_nicknames: channel.icon_user_nicknames,
                    mmr: 0,
                    jn: None,
                }
            })
            .collect();

        Self {
            user_id: response.user_id,
            chat_list: load_channel_list::Response {
                eof: true,
                mcm_revision: 0,
                last_chat_id: None,
                last_token_id: None,
                last_token: None,
                last_block_token: 0,
                deleted_chat_ids: Vec::new(),
                chat_datas,
            },
            min_log_id: response.min_log_id,
            revision: response.revision,
            revision_info: response.revision_info,
            sb: response.sb,
            rp: response.rp,
            pk_update: None,
            pk_token: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_sparse_android_loginlist_response() {
        let response: AndroidResponse = bson::from_document(bson::doc! {
            "userId": 100000001_i64,
            "revision": 1_i32,
            "revisionInfo": "[]",
            "rp": bson::Binary {
                subtype: bson::spec::BinarySubtype::Generic,
                bytes: vec![0],
            },
            "minLogId": 0_i64,
            "sb": 1_i32,
            "chatDatas": [{
                "c": 900000000000001_i64,
                "t": "MultiChat",
                "a": 3_i32,
                "n": 0_i32,
                "ii": 0_i32,
                "s": 1_i64,
                "l": { "logId": 2_i64, "message": "synthetic" },
                "k": ["Alice", "Bob"],
            }],
        })
        .unwrap();

        let response = Response::from(response);
        assert!(response.chat_list.eof);
        assert_eq!(response.chat_list.chat_datas.len(), 1);
        assert_eq!(response.chat_list.chat_datas[0].last_log_id, 2);
        assert_eq!(response.chat_list.chat_datas[0].last_seen_log_id, Some(1));
        assert!(response.chat_list.chat_datas[0].chatlog.is_none());
    }
}
