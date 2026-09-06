use serde::Deserialize;

use crate::talk::{channel::ChannelType, openlink::OpenLinkUser};

use super::{normal, open};

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

    #[serde(rename = "l")]
    pub last_log_id: i64,

    #[serde(rename = "o")]
    pub last_update: i64,
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

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct NormalChatOnChannel {
    #[serde(flatten)]
    pub users: ChatOnChannelUsers<normal::user::User>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenChatOnChannel {
    #[serde(rename = "otk")]
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
    use super::*;

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
}
