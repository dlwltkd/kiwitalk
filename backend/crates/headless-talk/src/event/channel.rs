use talk_loco_client::talk::{channel::ChannelMeta, chat::Chatlog};

#[derive(Debug, Clone)]
pub enum ChannelEvent {
    Chat {
        link_id: Option<i64>,

        user_nickname: Option<String>,
        chat: Chatlog,

        /// Whether this client acknowledged the message as already read.
        read: bool,

        /// Whether the desktop shell should show a system notification.
        notify: bool,
    },

    ChatRead {
        /// Read user id
        user_id: i64,

        /// Read chat log id
        log_id: i64,
    },

    MetaChanged(ChannelMeta),

    ChatDeleted(Chatlog),

    Added {
        chatlog: Option<Chatlog>,
    },
    Left,

    UserJoin(Chatlog),
    UserLeft(Chatlog),
}
