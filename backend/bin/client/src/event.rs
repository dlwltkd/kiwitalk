use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use serde::Serialize;
use tokio::sync::mpsc;

use crate::channel::{ChannelMeta, Chatlog};

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "content")]
pub(crate) enum ClientEvent {
    Channel { id: String, event: ChannelEvent },

    SwitchServer,

    Kickout { reason: i32 },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "content")]
pub(crate) enum ChannelEvent {
    Chat {
        chat: Chatlog,
        read: bool,
    },

    #[serde(rename_all = "camelCase")]
    ChatRead {
        user_id: String,
        log_id: String,
    },

    #[serde(rename_all = "camelCase")]
    UnreadChanged {
        unread_count: i32,
    },

    ChatDeleted(Chatlog),

    MetaChanged(ChannelMeta),

    Added(Option<Chatlog>),

    Left,
}

#[derive(Debug, Clone)]
pub(crate) struct EventSender {
    tx: mpsc::Sender<anyhow::Result<ClientEvent>>,
    overflowed: Arc<AtomicBool>,
}

impl EventSender {
    pub(crate) fn new(
        tx: mpsc::Sender<anyhow::Result<ClientEvent>>,
        overflowed: Arc<AtomicBool>,
    ) -> Self {
        Self { tx, overflowed }
    }

    pub(crate) fn enqueue(&self, event: anyhow::Result<ClientEvent>) {
        match self.tx.try_send(event) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                if !self.overflowed.swap(true, Ordering::AcqRel) {
                    log::warn!("client event queue overflowed; reconnect is required");
                }
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                log::debug!("client event receiver is closed");
            }
        }
    }

    pub(crate) fn overflowed(&self) -> bool {
        self.overflowed.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_queue_is_nonblocking_and_marks_the_session_for_reconnect() {
        let (tx, mut rx) = mpsc::channel(1);
        let sender = EventSender::new(tx, Arc::new(AtomicBool::new(false)));

        sender.enqueue(Ok(ClientEvent::SwitchServer));
        sender.enqueue(Ok(ClientEvent::SwitchServer));

        assert!(sender.overflowed());
        assert!(matches!(
            rx.try_recv().unwrap().unwrap(),
            ClientEvent::SwitchServer
        ));
    }
}
