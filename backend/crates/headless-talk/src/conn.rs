use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};

use futures_loco_protocol::session::LocoSession;
use tokio::sync::Mutex;

use crate::database::DatabasePool;

#[derive(Debug, Clone)]
pub struct Conn {
    pub user_id: i64,
    pub session: LocoSession,
    pub pool: DatabasePool,
    pub(crate) room_state_lock: Arc<Mutex<()>>,
    active_channel_id: Arc<AtomicI64>,
}

impl Conn {
    pub(crate) fn with_inactive_channel(
        user_id: i64,
        session: LocoSession,
        pool: DatabasePool,
    ) -> Self {
        Self {
            user_id,
            session,
            pool,
            room_state_lock: Arc::new(Mutex::new(())),
            active_channel_id: Arc::new(AtomicI64::new(0)),
        }
    }

    pub(crate) fn set_channel_active(&self, channel_id: i64, active: bool) {
        if active {
            self.active_channel_id.store(channel_id, Ordering::Release);
        } else {
            let _ = self.active_channel_id.compare_exchange(
                channel_id,
                0,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }

    pub(crate) fn is_channel_active(&self, channel_id: i64) -> bool {
        self.active_channel_id.load(Ordering::Acquire) == channel_id
    }
}
