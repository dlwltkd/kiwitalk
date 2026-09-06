use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::talk::chat::Chatlog;

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
    pub supplement: Option<&'a str>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Response {
    /// Chatroom id
    #[serde(rename = "chatId")]
    pub chat_id: i64,

    /// Previous chat log id
    #[serde(rename = "prevId")]
    pub prev_id: i64,

    /// Sent chat log id
    #[serde(rename = "logId")]
    pub log_id: i64,

    /// Send time in Unix time
    #[serde(rename = "sendAt")]
    pub send_at: i64,

    /// Sent chat message id
    #[serde(default, rename = "msgId")]
    pub msg_id: i64,

    /// Sent message
    #[serde(default)]
    #[serde(rename = "chatLog")]
    pub chatlog: Option<Chatlog>,
}

#[cfg(test)]
mod tests {
    use super::Request;

    #[test]
    fn android_text_write_serializes_message_id_and_empty_extra() {
        let request = Request {
            chat_type: 1,
            msg_id: 1_725_000_000_001,
            message: Some("hello"),
            no_seen: false,
            attachment: Some("{}"),
            supplement: None,
        };

        let document = bson::to_document(&request).unwrap();

        assert_eq!(document.get_i32("type").unwrap(), 1);
        assert_eq!(document.get_i64("msgId").unwrap(), 1_725_000_000_001);
        assert_eq!(document.get_str("msg").unwrap(), "hello");
        assert!(!document.get_bool("noSeen").unwrap());
        assert_eq!(document.get_str("extra").unwrap(), "{}");
        assert!(!document.contains_key("supplement"));
    }
}
