use std::time::Duration;

use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection};
use futures_loco_protocol::session::LocoSession;
use talk_loco_client::talk::session::TalkSession;
use tokio::time::{self, Instant};

use crate::{
    database::{
        model::{channel::history_sync::ChannelHistorySyncRow, chat::ChatRow},
        schema::{channel_history_sync, chat},
        DatabasePool,
    },
    ClientResult,
};

const SYNC_PAGE_SIZE: i32 = 50;
const SYNC_PAGE_LIMIT: usize = 20;
const SYNC_TIME_LIMIT: Duration = Duration::from_secs(12);
const SYNC_PAGE_DELAY: Duration = Duration::from_millis(100);

fn persist_page_rows(
    conn: &mut SqliteConnection,
    channel_id: i64,
    chat_rows: Vec<ChatRow>,
    cursor: i64,
) -> diesel::QueryResult<()> {
    if !chat_rows.is_empty() {
        diesel::replace_into(chat::table)
            .values(chat_rows)
            .execute(conn)?;
    }

    diesel::insert_or_ignore_into(channel_history_sync::table)
        .values(ChannelHistorySyncRow { channel_id, cursor })
        .execute(conn)?;

    diesel::update(
        channel_history_sync::table
            .filter(channel_history_sync::channel_id.eq(channel_id))
            .filter(channel_history_sync::cursor.lt(cursor)),
    )
    .set(channel_history_sync::cursor.eq(cursor))
    .execute(conn)?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistorySyncStop {
    UpToDate,
    ServerComplete,
    ReachedTarget,
    EmptyBatch,
    NoProgress,
    PageLimit,
    TimeLimit,
    UnsupportedChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistorySyncResult {
    pub fetched_count: usize,
    pub page_count: usize,
    pub stop: HistorySyncStop,
}

impl HistorySyncResult {
    const fn stopped(fetched_count: usize, page_count: usize, stop: HistorySyncStop) -> Self {
        Self {
            fetched_count,
            page_count,
            stop,
        }
    }

    pub const fn complete(self) -> bool {
        matches!(
            self.stop,
            HistorySyncStop::UpToDate
                | HistorySyncStop::ServerComplete
                | HistorySyncStop::ReachedTarget
        )
    }
}

#[derive(Debug)]
pub struct ChatUpdater<'a> {
    session: &'a LocoSession,
    pool: &'a DatabasePool,

    channel_id: i64,
}

impl<'a> ChatUpdater<'a> {
    pub fn new(session: &'a LocoSession, pool: &'a DatabasePool, channel_id: i64) -> Self {
        Self {
            session,
            pool,
            channel_id,
        }
    }

    async fn load_cursor(&self) -> ClientResult<i64> {
        let channel_id = self.channel_id;

        Ok(self
            .pool
            .spawn(move |conn| {
                Ok(channel_history_sync::table
                    .filter(channel_history_sync::channel_id.eq(channel_id))
                    .select(channel_history_sync::cursor)
                    .first::<i64>(conn)
                    .optional()?
                    .unwrap_or(0))
            })
            .await?)
    }

    async fn persist_page(&self, chat_rows: Vec<ChatRow>, cursor: i64) -> ClientResult<()> {
        let channel_id = self.channel_id;

        self.pool
            .spawn_transaction(move |conn| {
                persist_page_rows(conn, channel_id, chat_rows, cursor)?;

                Ok(())
            })
            .await?;

        Ok(())
    }

    pub async fn update_bounded(self, last_log_id: i64) -> ClientResult<HistorySyncResult> {
        let mut cursor = self.load_cursor().await?;

        if cursor >= last_log_id {
            return Ok(HistorySyncResult::stopped(0, 0, HistorySyncStop::UpToDate));
        }

        let deadline = Instant::now() + SYNC_TIME_LIMIT;
        let mut fetched_count = 0;

        for page_index in 0..SYNC_PAGE_LIMIT {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(HistorySyncResult::stopped(
                    fetched_count,
                    page_index,
                    HistorySyncStop::TimeLimit,
                ));
            }

            let response = match time::timeout(
                remaining,
                TalkSession(self.session)
                    .channel(self.channel_id)
                    .sync_chat_page(cursor, last_log_id, SYNC_PAGE_SIZE),
            )
            .await
            {
                Ok(response) => response?,
                Err(_) => {
                    return Ok(HistorySyncResult::stopped(
                        fetched_count,
                        page_index,
                        HistorySyncStop::TimeLimit,
                    ));
                }
            };

            let is_ok = response.is_ok;
            let chatlogs = response.chatlogs.unwrap_or_default();

            if chatlogs.is_empty() {
                if is_ok {
                    self.persist_page(Vec::new(), last_log_id).await?;
                }

                return Ok(HistorySyncResult::stopped(
                    fetched_count,
                    page_index + 1,
                    if is_ok {
                        HistorySyncStop::ServerComplete
                    } else {
                        HistorySyncStop::EmptyBatch
                    },
                ));
            }

            if chatlogs.iter().any(|chatlog| {
                chatlog.channel_id != self.channel_id
                    || chatlog.log_id <= cursor
                    || chatlog.log_id > last_log_id
            }) {
                return Ok(HistorySyncResult::stopped(
                    fetched_count,
                    page_index + 1,
                    HistorySyncStop::NoProgress,
                ));
            }

            let next_cursor = chatlogs.iter().map(|chatlog| chatlog.log_id).max().unwrap();

            if next_cursor <= cursor {
                return Ok(HistorySyncResult::stopped(
                    fetched_count,
                    page_index + 1,
                    HistorySyncStop::NoProgress,
                ));
            }

            let chat_rows = chatlogs
                .into_iter()
                .map(|chatlog| ChatRow::from_chatlog(chatlog, None))
                .collect::<Vec<_>>();
            let batch_count = chat_rows.len();
            let checkpoint = if is_ok { last_log_id } else { next_cursor };

            self.persist_page(chat_rows, checkpoint).await?;

            fetched_count += batch_count;
            cursor = checkpoint;
            let page_count = page_index + 1;

            if is_ok {
                return Ok(HistorySyncResult::stopped(
                    fetched_count,
                    page_count,
                    HistorySyncStop::ServerComplete,
                ));
            }

            if cursor >= last_log_id {
                return Ok(HistorySyncResult::stopped(
                    fetched_count,
                    page_count,
                    HistorySyncStop::ReachedTarget,
                ));
            }

            if page_count == SYNC_PAGE_LIMIT {
                return Ok(HistorySyncResult::stopped(
                    fetched_count,
                    page_count,
                    HistorySyncStop::PageLimit,
                ));
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining <= SYNC_PAGE_DELAY {
                return Ok(HistorySyncResult::stopped(
                    fetched_count,
                    page_count,
                    HistorySyncStop::TimeLimit,
                ));
            }

            time::sleep(SYNC_PAGE_DELAY).await;
        }

        Ok(HistorySyncResult::stopped(
            fetched_count,
            SYNC_PAGE_LIMIT,
            HistorySyncStop::PageLimit,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::{connection::SimpleConnection, Connection};

    fn test_connection() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.batch_execute(include_str!(
            "../../migrations/2023-10-21-003644_v0.1/up.sql"
        ))
        .unwrap();
        conn.batch_execute(include_str!(
            "../../migrations/2026-08-28-000000_history_sync_cursor/up.sql"
        ))
        .unwrap();
        conn
    }

    fn chat_row(log_id: i64, channel_id: i64) -> ChatRow {
        ChatRow {
            log_id,
            channel_id,
            prev_log_id: None,
            chat_type: 1,
            message_id: log_id,
            send_at: 0,
            author_id: 1,
            message: Some("test".to_owned()),
            attachment: None,
            supplement: None,
            referer: None,
            deleted_time: None,
        }
    }

    #[test]
    fn only_terminal_server_states_are_complete() {
        for stop in [
            HistorySyncStop::UpToDate,
            HistorySyncStop::ServerComplete,
            HistorySyncStop::ReachedTarget,
        ] {
            assert!(HistorySyncResult::stopped(0, 0, stop).complete());
        }

        for stop in [
            HistorySyncStop::EmptyBatch,
            HistorySyncStop::NoProgress,
            HistorySyncStop::PageLimit,
            HistorySyncStop::TimeLimit,
            HistorySyncStop::UnsupportedChannel,
        ] {
            assert!(!HistorySyncResult::stopped(0, 0, stop).complete());
        }
    }

    #[test]
    fn checkpoint_advances_with_persisted_page_and_never_regresses() {
        let mut conn = test_connection();

        persist_page_rows(&mut conn, 7, vec![chat_row(11, 7)], 11).unwrap();
        persist_page_rows(&mut conn, 7, Vec::new(), 5).unwrap();

        let cursor = channel_history_sync::table
            .select(channel_history_sync::cursor)
            .first::<i64>(&mut conn)
            .unwrap();
        let chat_count = chat::table.count().get_result::<i64>(&mut conn).unwrap();

        assert_eq!(cursor, 11);
        assert_eq!(chat_count, 1);
    }

    #[test]
    fn failed_page_transaction_does_not_advance_checkpoint() {
        let mut conn = test_connection();
        conn.batch_execute(
            "CREATE TRIGGER reject_test_chat BEFORE INSERT ON chat \
             BEGIN SELECT RAISE(FAIL, 'rejected test row'); END;",
        )
        .unwrap();

        let result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
            persist_page_rows(conn, 7, vec![chat_row(11, 7)], 11)
        });
        let cursor_count = channel_history_sync::table
            .count()
            .get_result::<i64>(&mut conn)
            .unwrap();

        assert!(result.is_err());
        assert_eq!(cursor_count, 0);
    }
}
