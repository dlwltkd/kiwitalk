use diesel::{Insertable, Queryable};

use crate::database::schema::channel_history_sync;

#[derive(Debug, Insertable, Queryable, Clone, Copy, PartialEq, Eq)]
#[diesel(table_name = channel_history_sync)]
pub struct ChannelHistorySyncRow {
    pub channel_id: i64,
    pub cursor: i64,
}
