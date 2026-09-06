use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::talk::chat::Chatlog;

fn is_none_or_blank(value: &Option<&str>) -> bool {
    value.map(str::trim).unwrap_or_default().is_empty()
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Request<'a> {
    /// Chat type
    #[serde(rename = "type")]
    pub chat_type: i32,

    /// Message id
    ///
    /// Client send count??
    #[serde(rename = "msgId")]
    pub msg_id: i64,

    /// Message content
    ///
    /// Usually String, but can be json String according to chat type.
    #[serde(rename = "msg")]
    pub message: Option<&'a str>,

    /// If true, server will assume the client read last message.
    #[serde(rename = "noSeen")]
    pub no_seen: bool,

    /// Attachment content
    ///
    /// Json data. Have contents and extra data according to chat type.
    /// Also known as `extra`.
    #[serde(rename = "extra")]
    pub attachment: Option<&'a str>,

    /// Used on pluschat.
    ///
    /// Cannot be used to send by normal user
    #[serde(skip_serializing_if = "is_none_or_blank")]
    pub supplement: Option<&'a str>,

    /// Sending source used by notification replies and similar flows.
    #[serde(rename = "f", skip_serializing_if = "is_none_or_blank")]
    pub from: Option<&'a str>,

    /// Thread scope. Ordinary room messages use `ONLY_CHAT_ROOM` (`1`).
    pub scope: i32,

    #[serde(rename = "threadId")]
    pub thread_id: Option<i64>,

    #[serde(rename = "featureStat", skip_serializing_if = "is_none_or_blank")]
    pub feature_stat: Option<&'a str>,

    /// Whether the message should be delivered silently.
    pub silence: bool,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Response {
    /// Chatroom id
    #[serde(rename = "chatId")]
    pub chat_id: i64,

    /// Sent chat log id
    #[serde(rename = "logId")]
    pub log_id: i64,

    #[serde(default, rename = "authorNickname")]
    pub author_nickname: Option<String>,

    /// Send time in Unix time
    #[serde(default, rename = "sendAt")]
    pub send_at: Option<i32>,

    /// Sent chat message id
    #[serde(default, rename = "msgId")]
    pub msg_id: Option<i64>,

    /// Previous chat log id
    #[serde(default, rename = "prevId")]
    pub prev_id: Option<i64>,

    /// Sent message
    #[serde(default, rename = "chatLog")]
    pub chatlog: Option<Chatlog>,
}

/// Request body for forwarding an existing message.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ForwardRequest<'a> {
    #[serde(rename = "type")]
    pub chat_type: i32,

    #[serde(rename = "msgId")]
    pub msg_id: i64,

    #[serde(rename = "noSeen")]
    pub no_seen: Option<bool>,

    #[serde(rename = "extra")]
    pub attachment: &'a str,

    #[serde(rename = "msg")]
    pub message: Option<&'a str>,

    #[serde(rename = "fromChatId")]
    pub from_chat_id: Option<i64>,

    #[serde(rename = "fromLogId")]
    pub from_log_id: Option<i64>,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{ForwardRequest, Request};

    #[test]
    fn android_text_write_serializes_message_id_and_empty_extra() {
        let request = Request {
            chat_type: 1,
            msg_id: 1_725_000_000_001,
            message: Some("hello"),
            no_seen: false,
            attachment: Some("{}"),
            supplement: None,
            from: None,
            scope: 1,
            thread_id: None,
            feature_stat: None,
            silence: false,
        };

        let document = bson::to_document(&request).unwrap();
        let keys = document.keys().map(String::as_str).collect::<BTreeSet<_>>();

        assert_eq!(
            keys,
            BTreeSet::from(["extra", "msg", "msgId", "noSeen", "scope", "silence", "type"])
        );
        assert_eq!(document.get_i32("type").unwrap(), 1);
        assert_eq!(document.get_i64("msgId").unwrap(), 1_725_000_000_001);
        assert_eq!(document.get_str("msg").unwrap(), "hello");
        assert!(!document.get_bool("noSeen").unwrap());
        assert_eq!(document.get_str("extra").unwrap(), "{}");
        assert_eq!(document.get_i32("scope").unwrap(), 1);
        assert!(!document.get_bool("silence").unwrap());
    }

    #[test]
    fn android_write_omits_blank_optional_fields() {
        let request = Request {
            chat_type: 1,
            msg_id: 7,
            message: Some("hello"),
            no_seen: false,
            attachment: Some("{}"),
            supplement: Some(" "),
            from: Some(""),
            scope: 1,
            thread_id: None,
            feature_stat: Some("\t"),
            silence: false,
        };

        let document = bson::to_document(&request).unwrap();

        assert!(!document.contains_key("supplement"));
        assert!(!document.contains_key("f"));
        assert!(!document.contains_key("threadId"));
        assert!(!document.contains_key("featureStat"));
    }

    #[test]
    fn forward_uses_its_current_android_field_set() {
        let request = ForwardRequest {
            chat_type: 1,
            msg_id: 8,
            no_seen: Some(true),
            attachment: "{}",
            message: Some("forwarded"),
            from_chat_id: Some(10),
            from_log_id: Some(11),
        };

        let document = bson::to_document(&request).unwrap();
        let keys = document.keys().map(String::as_str).collect::<BTreeSet<_>>();

        assert_eq!(
            keys,
            BTreeSet::from([
                "extra",
                "fromChatId",
                "fromLogId",
                "msg",
                "msgId",
                "noSeen",
                "type",
            ])
        );
    }
}
