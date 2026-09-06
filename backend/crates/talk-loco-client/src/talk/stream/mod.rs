pub mod command;

use self::command::{
    ChgMeta, DecunRead, DelMem, Left, Msg, NewMem, SyncDlMsg, SyncJoin, SyncLinkCr, SyncLinkPf,
    SyncMemT, SyncRewr,
};
use command::Kickout;
use futures_loco_protocol::loco_protocol::command::BoxedCommand;

macro_rules! create_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $method:literal => $variant_name:ident$(($variant_ty:ty))?
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $(
                $(#[$variant_meta])*
                $variant_name$(($variant_ty))?,
            )*

            #[doc = "Unknown command"]
            Unknown(BoxedCommand),
        }

        impl $name {
            pub fn deserialize_from(command: BoxedCommand) -> ::bson::de::Result<Self> {
                Ok(match &*command.header.method {
                    $(
                        $method => StreamCommand::$variant_name$((::bson::from_slice::<$variant_ty>(&command.data)?))?,
                    )*

                    _ => StreamCommand::Unknown(command),
                })
            }
        }

        $(
            $(
                impl From<$variant_ty> for $name {
                    fn from(value: $variant_ty) -> Self {
                        Self::$variant_name(value)
                    }
                }
            )?
        )*

        impl From<BoxedCommand> for $name {
            fn from(value: BoxedCommand) -> Self {
                Self::Unknown(value)
            }
        }
    };
}

create_enum!(
    #[derive(Debug)]
    pub enum StreamCommand {
        "KICKOUT" => Kickout(Kickout),
        "CHANGESVR" => SwitchServer,

        "MSG" => Chat(Msg),
        "DECUNREAD" => ChatRead(DecunRead),
        "CHGMETA" => ChangeMeta(ChgMeta),

        "SYNCJOIN" => SyncChannelJoin(SyncJoin),
        "SYNCDLMSG" => SyncChatDeletion(SyncDlMsg),
        "SYNCREWR" => SyncRewrite(SyncRewr),

        "SYNCLINKCR" => SyncLinkCreation(SyncLinkCr),
        "SYNCMEMT" => SyncOpenUserType(SyncMemT),
        "SYNCLINKPF" => SyncLinkProfile(SyncLinkPf),

        "LEFT" => Left(Left),

        "NEWMEM" => NewUser(NewMem),
        "DELMEM" => DelUser(DelMem),
    }
);

#[cfg(test)]
mod tests {
    use bson::doc;
    use futures_loco_protocol::loco_protocol::command::{Command, Header, Method};

    use super::*;

    #[test]
    fn current_sync_link_profile_push_uses_synclinkpf_and_parses_olu() {
        let command = Command {
            header: Header {
                id: 1,
                status: 0,
                method: Method::new("SYNCLINKPF").unwrap(),
                data_type: 0,
            },
            data: bson::to_vec(&doc! {
                "olu": {
                    "userId": 42_i64,
                    "nn": "profile",
                    "pv": 4_i64,
                },
                "li": 7_i64,
            })
            .unwrap()
            .into_boxed_slice(),
        };

        let StreamCommand::SyncLinkProfile(profile) =
            StreamCommand::deserialize_from(command).unwrap()
        else {
            panic!("SYNCLINKPF was not recognized")
        };

        assert_eq!(profile.link_id, 7);
        assert_eq!(profile.chat_id, None);
        assert_eq!(profile.open_link_user.user_id, 42);
        assert_eq!(profile.open_link_user.open_token, -1);
        assert_eq!(profile.open_link_user.profile_link_id, 0);
        assert_eq!(profile.open_link_user.privilege.bits(), 4);
    }

    #[test]
    fn current_message_push_accepts_android_optional_defaults() {
        let command = Command {
            header: Header {
                id: 1,
                status: 0,
                method: Method::new("MSG").unwrap(),
                data_type: 0,
            },
            data: bson::to_vec(&doc! {
                "chatId": 7_i64,
                "chatLog": {
                    "logId": 9_i64,
                    "chatId": 7_i64,
                    "prevId": 8_i64,
                    "type": 1_i32,
                    "attachment": bson::Bson::Null,
                },
            })
            .unwrap()
            .into_boxed_slice(),
        };

        let StreamCommand::Chat(message) = StreamCommand::deserialize_from(command).unwrap() else {
            panic!("MSG was not recognized")
        };

        assert!(!message.no_seen);
        assert_eq!(message.chatlog.author_id, -1);
        assert_eq!(message.chatlog.send_at, 0);
        assert_eq!(message.chatlog.chat.message_id, 0);
        assert_eq!(message.chatlog.chat.content.attachment, None);
    }
}
