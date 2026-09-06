use serde::Deserialize;

use crate::talk::{
    channel::{ChannelMeta, ChannelType},
    chat::Chatlog,
};

use super::{normal, open};

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ChannelInfo {
    #[serde(flatten)]
    pub channel_type: ChannelInfoType,

    #[serde(default, rename = "chatId")]
    pub chat_id: i64,

    #[serde(default, rename = "activeMembersCount")]
    pub active_member_count: i32,

    #[serde(default, rename = "newMessageCount")]
    pub new_chat_count: i32,

    #[serde(default, rename = "lastSeenLogId")]
    pub last_seen_log_id: i64,

    #[serde(default, rename = "lastChatLog")]
    pub last_chatlog: Option<Chatlog>,

    #[serde(default, rename = "pushAlert")]
    pub push_alert: bool,

    #[serde(default, rename = "chatMetas")]
    pub channel_metas: Vec<ChannelMeta>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ChannelInfoType {
    DirectChat(NormalChannelInfo),

    MultiChat(NormalChannelInfo),

    MemoChat(NormalChannelInfo),

    #[serde(rename = "OD")]
    OpenDirect(OpenChannelInfo),

    #[serde(rename = "OM")]
    OpenMulti(OpenChannelInfo),

    #[serde(other)]
    Other,
}

impl ChannelInfoType {
    pub fn ty(&self) -> Option<ChannelType> {
        Some(match self {
            ChannelInfoType::DirectChat(_) => ChannelType::DirectChat,
            ChannelInfoType::MultiChat(_) => ChannelType::MultiChat,
            ChannelInfoType::MemoChat(_) => ChannelType::MemoChat,
            ChannelInfoType::OpenDirect(_) => ChannelType::OpenDirect,
            ChannelInfoType::OpenMulti(_) => ChannelType::OpenMulti,
            ChannelInfoType::Other => return None,
        })
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct NormalChannelInfo {
    #[serde(default, rename = "inviterId")]
    pub inviter_id: Option<i64>,

    #[serde(default, rename = "displayMembers")]
    pub display_members: Vec<normal::user::DisplayUser>,

    #[serde(default, rename = "joinedAtForNewMem")]
    pub joined_at_for_new_mem: i64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OpenChannelInfo {
    #[serde(default, rename = "displayMembers")]
    pub display_members: Vec<open::user::DisplayUser>,

    #[serde(default, rename = "linkId", alias = "li")]
    pub link_id: i64,

    #[serde(default, rename = "openLinkToken", alias = "otk")]
    pub open_token: i32,

    #[serde(default, rename = "directChat")]
    pub direct_chat: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_current_android_normal_chat_info() {
        let info: ChannelInfo = bson::from_document(bson::doc! {
            "chatId": 10_i64,
            "type": "MultiChat",
            "activeMembersCount": 2_i32,
            "newMessageCount": 1_i32,
            "inviterId": 3_i64,
            "lastSeenLogId": 4_i64,
            "displayMembers": [{
                "userId": 5_i64,
                "nickName": "member",
                "profileImageUrl": bson::Bson::Null,
            }],
            "pushAlert": true,
            "joinedAtForNewMem": 6_i32,
            "chatMetas": [],
            "linkId": 0_i64,
            "openLinkToken": 0_i32,
            "token": 7_i64,
        })
        .unwrap();

        assert_eq!(info.chat_id, 10);
        assert_eq!(info.last_seen_log_id, 4);
        assert!(info.push_alert);
        let ChannelInfoType::MultiChat(normal) = info.channel_type else {
            panic!("expected a normal multi-chat room");
        };
        assert_eq!(normal.joined_at_for_new_mem, 6);
        assert_eq!(normal.display_members.len(), 1);
    }

    #[test]
    fn accepts_current_android_open_chat_info_keys() {
        let info: ChannelInfo = bson::from_document(bson::doc! {
            "chatId": 20_i64,
            "type": "OM",
            "displayMembers": [],
            "linkId": 21_i64,
            "openLinkToken": 22_i32,
        })
        .unwrap();

        let ChannelInfoType::OpenMulti(open) = info.channel_type else {
            panic!("expected an open multi-chat room");
        };
        assert_eq!(open.link_id, 21);
        assert_eq!(open.open_token, 22);
    }
}
