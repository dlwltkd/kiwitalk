use serde::{Deserialize, Deserializer, Serialize};

use crate::talk::{channel::ChannelType, openlink::OpenLinkUser};

use super::{normal, open};

/// Android CHATONROOM request fields after `chatId`.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct Request {
    pub token: i64,

    #[serde(rename = "st", skip_serializing_if = "Option::is_none")]
    pub s_key_token: Option<i64>,

    #[serde(rename = "sc", skip_serializing_if = "Option::is_none")]
    pub s_chat_token: Option<i64>,

    #[serde(rename = "opt", skip_serializing_if = "Option::is_none")]
    pub openlink_profile_token: Option<i32>,
}

impl Request {
    pub const fn new(token: i64) -> Self {
        Self {
            token,
            s_key_token: None,
            s_chat_token: None,
            openlink_profile_token: None,
        }
    }
}

/// Contains user info, watermark list.
/// Client can update chatroom information before opening chatroom window.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ChatOnChannel {
    #[serde(flatten)]
    pub channel_type: ChatOnChannelType,

    /// Also watermark user ids
    #[serde(rename = "a")]
    pub active_user_ids: Option<Vec<i64>>,

    #[serde(rename = "w")]
    pub watermarks: Option<Vec<i64>>,

    #[serde(default, rename = "l")]
    pub last_log_id: i64,

    #[serde(default, rename = "o")]
    pub token: i64,

    #[serde(default = "default_true", rename = "notiRead")]
    pub noti_read: bool,
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "t")]
pub enum ChatOnChannelType {
    DirectChat(NormalChatOnChannel),

    MultiChat(NormalChatOnChannel),

    MemoChat(NormalChatOnChannel),

    #[serde(rename = "OD")]
    OpenDirect(OpenChatOnChannel),

    #[serde(rename = "OM")]
    OpenMulti(OpenChatOnChannel),

    #[serde(other)]
    Other,
}

impl ChatOnChannelType {
    pub fn ty(&self) -> Option<ChannelType> {
        Some(match self {
            ChatOnChannelType::DirectChat(_) => ChannelType::DirectChat,
            ChatOnChannelType::MultiChat(_) => ChannelType::MultiChat,
            ChatOnChannelType::MemoChat(_) => ChannelType::MemoChat,
            ChatOnChannelType::OpenDirect(_) => ChannelType::OpenDirect,
            ChatOnChannelType::OpenMulti(_) => ChannelType::OpenMulti,
            ChatOnChannelType::Other => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalChatOnChannel {
    pub users: ChatOnChannelUsers<normal::user::User>,
}

impl<'de> Deserialize<'de> for NormalChatOnChannel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            #[serde(default, rename = "m")]
            users: Option<Vec<normal::user::User>>,

            #[serde(default, rename = "mi")]
            user_ids: Option<Vec<i64>>,
        }

        let wire = Wire::deserialize(deserializer)?;
        let users = match (wire.users, wire.user_ids) {
            (Some(users), _) => ChatOnChannelUsers::Users(users),
            (None, Some(ids)) => ChatOnChannelUsers::Ids(ids),
            (None, None) => ChatOnChannelUsers::Ids(Vec::new()),
        };

        Ok(Self { users })
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenChatOnChannel {
    #[serde(default, rename = "otk")]
    pub open_token: i32,

    #[serde(rename = "olu")]
    pub open_link_user: Option<OpenLinkUser>,

    #[serde(rename = "m")]
    pub users: Option<Vec<open::user::User>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum ChatOnChannelUsers<T> {
    #[serde(rename = "mi")]
    Ids(Vec<i64>),
    #[serde(rename = "m")]
    Users(Vec<T>),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn android_chat_on_serializes_optional_tokens_with_wire_names() {
        let request = Request {
            token: 10,
            s_key_token: Some(11),
            s_chat_token: Some(12),
            openlink_profile_token: Some(13),
        };

        let document = bson::to_document(&request).unwrap();
        let keys = document.keys().map(String::as_str).collect::<BTreeSet<_>>();

        assert_eq!(keys, BTreeSet::from(["opt", "sc", "st", "token"]));
        assert_eq!(document.get_i64("token").unwrap(), 10);
        assert_eq!(document.get_i64("st").unwrap(), 11);
        assert_eq!(document.get_i64("sc").unwrap(), 12);
        assert_eq!(document.get_i32("opt").unwrap(), 13);
    }

    #[test]
    fn android_chat_on_omits_absent_optional_tokens() {
        let document = bson::to_document(&Request::new(0)).unwrap();

        assert_eq!(document, bson::doc! { "token": 0_i64 });
    }

    #[test]
    fn accepts_sparse_android_room_members() {
        let room: ChatOnChannel = bson::from_document(bson::doc! {
            "t": "MultiChat",
            "m": [{
                "userId": 100_i64,
                "nickName": "peer",
                "profileImageUrl": bson::Bson::Null,
                "statusMessage": bson::Bson::Null,
            }],
            "l": 2_i64,
            "o": 0_i64,
        })
        .unwrap();

        let ChatOnChannelType::MultiChat(normal) = room.channel_type else {
            panic!("expected a normal multi-chat room");
        };
        let ChatOnChannelUsers::Users(users) = normal.users else {
            panic!("expected full member records");
        };

        assert_eq!(users.len(), 1);
        assert!(users[0].linked_services.is_empty());
        assert!(users[0].profile_image_url.is_empty());
        assert_eq!(users[0].account_id, 0);
        assert_eq!(users[0].profile_image_url_if_present(), None);
        assert_eq!(users[0].status_message_if_present(), None);
        assert_eq!(users[0].account_id_if_present(), None);
        assert_eq!(users[0].suspended_if_present(), None);
    }

    #[test]
    fn distinguishes_explicit_default_values_from_missing_fields() {
        let user: normal::user::User = bson::from_document(bson::doc! {
            "userId": 100_i64,
            "nickName": "peer",
            "profileImageUrl": "",
            "accountId": 0_i64,
            "statusMessage": "",
            "suspended": false,
        })
        .unwrap();

        assert_eq!(user.profile_image_url_if_present(), Some(""));
        assert_eq!(user.account_id_if_present(), Some(0));
        assert_eq!(user.status_message_if_present(), Some(""));
        assert_eq!(user.suspended_if_present(), Some(false));
        assert_eq!(user.linked_services_if_present(), None);
    }

    #[test]
    fn accepts_sparse_android_room_response() {
        let room: ChatOnChannel = bson::from_document(bson::doc! {
            "t": "MultiChat",
        })
        .unwrap();

        let ChatOnChannelType::MultiChat(normal) = room.channel_type else {
            panic!("expected a normal multi-chat room");
        };

        assert_eq!(normal.users, ChatOnChannelUsers::Ids(Vec::new()));
        assert_eq!(room.last_log_id, 0);
        assert_eq!(room.token, 0);
        assert!(room.noti_read);
    }
}
