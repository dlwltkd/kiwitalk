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

    /// Whether this login is switching between accounts on the device.
    #[serde(rename = "isSw")]
    pub is_switching: Option<bool>,
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

    /// Whether the server asked the client to keep its last token frozen.
    #[serde(default, rename = "flti")]
    pub freeze_last_token_id: Option<bool>,
}

fn default_true() -> bool {
    true
}

fn deserialize_default_true<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<bool>::deserialize(deserializer)?.unwrap_or(true))
}

fn deserialize_android_last_token<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<i32>::deserialize(deserializer)?.unwrap_or(-1))
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

    #[serde(
        default,
        rename = "minLogId",
        deserialize_with = "load_channel_list::deserialize_null_default"
    )]
    pub min_log_id: i64,

    #[serde(
        default,
        rename = "mcmRevision",
        deserialize_with = "load_channel_list::deserialize_null_default"
    )]
    pub mcm_revision: i32,

    #[serde(
        default,
        rename = "delChatIds",
        deserialize_with = "load_channel_list::deserialize_null_default"
    )]
    pub deleted_chat_ids: Vec<i64>,

    #[serde(
        default,
        rename = "kc",
        deserialize_with = "load_channel_list::deserialize_null_default"
    )]
    pub kicked_chat_ids: Vec<i64>,

    #[serde(
        default = "default_true",
        deserialize_with = "deserialize_default_true"
    )]
    pub eof: bool,

    #[serde(
        default,
        rename = "lastTokenId",
        deserialize_with = "load_channel_list::deserialize_null_default"
    )]
    pub last_token_id: i64,

    #[serde(
        default = "default_android_last_token",
        rename = "ltk",
        deserialize_with = "deserialize_android_last_token"
    )]
    pub last_token: i32,

    #[serde(
        default,
        rename = "lbk",
        deserialize_with = "load_channel_list::deserialize_null_default"
    )]
    pub last_block_token: i32,

    #[serde(
        default,
        rename = "lastChatId",
        deserialize_with = "load_channel_list::deserialize_null_default"
    )]
    pub last_chat_id: i64,

    #[serde(
        default,
        rename = "pkUpdate",
        deserialize_with = "load_channel_list::deserialize_null_default"
    )]
    pub pk_update: bool,

    #[serde(default, rename = "pkToken")]
    pub pk_token: Option<i64>,

    #[serde(
        default,
        rename = "flti",
        deserialize_with = "load_channel_list::deserialize_null_default"
    )]
    pub freeze_last_token_id: bool,

    #[serde(
        default,
        deserialize_with = "load_channel_list::deserialize_null_default"
    )]
    pub sb: i32,

    #[serde(
        default,
        rename = "chatDatas",
        deserialize_with = "load_channel_list::deserialize_null_default"
    )]
    pub chat_datas: Vec<load_channel_list::AndroidChannelListData>,
}

fn default_android_last_token() -> i32 {
    -1
}

impl From<AndroidResponse> for Response {
    fn from(response: AndroidResponse) -> Self {
        let chat_datas = response
            .chat_datas
            .into_iter()
            .map(load_channel_list::normalize_android_channel)
            .collect();

        Self {
            user_id: response.user_id,
            chat_list: load_channel_list::Response {
                eof: response.eof,
                mcm_revision: i64::from(response.mcm_revision),
                last_chat_id: Some(response.last_chat_id),
                last_token_id: Some(response.last_token_id),
                last_token: Some(i64::from(response.last_token)),
                last_block_token: i64::from(response.last_block_token),
                deleted_chat_ids: response.deleted_chat_ids,
                kicked_chat_ids: response.kicked_chat_ids,
                chat_datas,
            },
            min_log_id: Some(response.min_log_id),
            revision: response.revision,
            revision_info: response.revision_info,
            sb: response.sb,
            rp: response.rp,
            pk_update: Some(response.pk_update),
            pk_token: response.pk_token,
            freeze_last_token_id: Some(response.freeze_last_token_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn android_loginlist_serializes_the_26_7_2_field_set() {
        let request = Request {
            os: "android",
            net_type: 0,
            app_version: "26.7.2",
            mccmnc: "999",
            protocol_version: "1",
            device_uuid: "synthetic-device",
            oauth_token: "synthetic-token",
            language: "ko",
            device_type: None,
            revision: Some(0),
            rp: [0, 0, 0xff, 0xff, 0, 0],
            pc_status: None,
            chat_list: load_channel_list::Request {
                chat_ids: &[],
                max_ids: &[],
                last_token_id: 0,
                last_chat_id: None,
            },
            last_block_token: 0,
            background: Some(false),
            is_switching: Some(false),
        };

        let document = bson::to_document(&request).unwrap();
        let keys = document.keys().map(String::as_str).collect::<BTreeSet<_>>();

        assert_eq!(
            keys,
            BTreeSet::from([
                "MCCMNC",
                "appVer",
                "bg",
                "chatIds",
                "duuid",
                "isSw",
                "lang",
                "lastTokenId",
                "lbk",
                "maxIds",
                "ntype",
                "oauthToken",
                "os",
                "prtVer",
                "revision",
                "rp",
            ])
        );
    }

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
                "ii": 0_i64,
                "s": 1_i64,
                "l": { "logId": 2_i64, "message": "synthetic" },
                "k": ["Alice", "Bob"],
                "p": false,
            }],
        })
        .unwrap();

        let response = Response::from(response);
        assert!(response.chat_list.eof);
        assert_eq!(response.chat_list.chat_datas.len(), 1);
        let channel = &response.chat_list.chat_datas[0];
        assert_eq!(channel.last_log_id, 2);
        assert_eq!(channel.last_seen_log_id, Some(1));

        let preview = channel.chatlog.as_ref().unwrap();
        assert_eq!(preview.log_id, 2);
        assert_eq!(preview.channel_id, channel.id);
        assert_eq!(preview.chat.content.message.as_deref(), Some("synthetic"));
    }

    #[test]
    fn accepts_android_null_defaults_and_filters_display_lists() {
        let response: AndroidResponse = bson::from_document(bson::doc! {
            "userId": 100000001_i64,
            "minLogId": bson::Bson::Null,
            "mcmRevision": bson::Bson::Null,
            "delChatIds": bson::Bson::Null,
            "kc": bson::Bson::Null,
            "eof": bson::Bson::Null,
            "lastTokenId": bson::Bson::Null,
            "ltk": bson::Bson::Null,
            "lbk": bson::Bson::Null,
            "lastChatId": bson::Bson::Null,
            "pkUpdate": bson::Bson::Null,
            "flti": bson::Bson::Null,
            "sb": bson::Bson::Null,
            "chatDatas": [{
                "c": 900000000000001_i64,
                "t": "MultiChat",
                "a": 3_i32,
                "n": 0_i32,
                "s": 1_i64,
                "p": true,
                "ii": bson::Bson::Null,
                "ll": bson::Bson::Null,
                "o": bson::Bson::Null,
                "mmr": bson::Bson::Null,
                "jn": bson::Bson::Null,
                "m": bson::Bson::Null,
                "ml": bson::Bson::Null,
                "i": [11_i64, bson::Bson::Null, 12_i64],
                "k": ["Alice", bson::Bson::Null, 7_i32, "Bob"],
                "bmids": bson::Bson::Null,
            }],
        })
        .unwrap();

        assert!(response.eof);
        assert_eq!(response.last_token, -1);
        assert_eq!(response.mcm_revision, 0);

        let response = Response::from(response);
        let channel = &response.chat_list.chat_datas[0];
        assert_eq!(channel.metadata.as_deref(), Some(""));
        assert_eq!(channel.icon_user_ids.as_deref(), Some(&[11, 12][..]));
        assert_eq!(
            channel.icon_user_nicknames.as_deref(),
            Some(&["Alice".to_owned(), "Bob".to_owned()][..])
        );
    }

    #[test]
    fn preserves_android_loginlist_pagination_and_removal_state() {
        let response: AndroidResponse = bson::from_document(bson::doc! {
            "userId": 100000001_i64,
            "mcmRevision": 9_i32,
            "delChatIds": [11_i64],
            "kc": [12_i64],
            "eof": false,
            "lastTokenId": 101_i64,
            "ltk": 7_i32,
            "lbk": 8_i32,
            "lastChatId": 99_i64,
            "pkUpdate": true,
            "pkToken": 55_i64,
            "flti": true,
            "chatDatas": [{
                "c": 900000000000001_i64,
                "t": "MultiChat",
                "a": 3_i32,
                "n": 2_i32,
                "s": 1_i64,
                "ll": 4_i64,
                "o": 123_i32,
                "p": true,
                "mmr": 6_i64,
                "jn": 7_i32,
                "ii": 21_i64,
                "m": "{\"title\":\"synthetic\"}",
                "ml": 3_i64,
                "bmids": [31_i64],
            }],
        })
        .unwrap();

        let response = Response::from(response);
        assert!(!response.chat_list.eof);
        assert_eq!(response.chat_list.mcm_revision, 9);
        assert_eq!(response.chat_list.deleted_chat_ids, [11]);
        assert_eq!(response.chat_list.kicked_chat_ids, [12]);
        assert_eq!(response.chat_list.last_token_id, Some(101));
        assert_eq!(response.chat_list.last_token, Some(7));
        assert_eq!(response.chat_list.last_block_token, 8);
        assert_eq!(response.chat_list.last_chat_id, Some(99));
        assert_eq!(response.pk_update, Some(true));
        assert_eq!(response.pk_token, Some(55));
        assert_eq!(response.freeze_last_token_id, Some(true));

        let channel = &response.chat_list.chat_datas[0];
        assert_eq!(channel.last_log_id, 4);
        assert_eq!(channel.last_log_send_at, 123);
        assert!(channel.push_alert);
        assert_eq!(channel.mmr, 6);
        assert_eq!(channel.jn, Some(7));
        assert_eq!(channel.inviter_id, Some(21));
        assert_eq!(
            channel.metadata.as_deref(),
            Some("{\"title\":\"synthetic\"}")
        );
        assert_eq!(channel.min_log_id, Some(3));
        assert_eq!(channel.blind_member_ids.as_deref(), Some(&[31][..]));
    }
}
