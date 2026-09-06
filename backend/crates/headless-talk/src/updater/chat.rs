use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use diesel::{
    dsl::sql, sql_types::BigInt, sql_types::Integer, sql_types::Nullable, upsert::excluded,
    ExpressionMethods, NullableExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl,
    SqliteConnection,
};
use futures_loco_protocol::session::LocoSession;
use talk_loco_client::talk::{
    chat::{ChatType, Chatlog},
    session::{channel::SyncChatResponse, TalkSession},
};
use tokio::time::{self, Instant};

use crate::{
    database::{
        model::{channel::history_sync::ChannelHistorySyncRow, chat::ChatRow},
        schema::{channel_history_sync, chat},
        DatabasePool,
    },
    ClientResult,
};

const GAP_PAGE_SIZE: i64 = 32;
const SYNC_PAGE_LIMIT: usize = 20;
const SYNC_TIME_LIMIT: Duration = Duration::from_secs(12);
const SYNC_PAGE_DELAY: Duration = Duration::from_millis(100);
const MISSING_PREDECESSOR: &str = "NOT EXISTS (SELECT 1 FROM chat AS previous \
     WHERE previous.channel_id = chat.channel_id AND previous.log_id = chat.prev_log_id)";

fn prepare_history_cursor(
    conn: &mut SqliteConnection,
    channel_id: i64,
    target: i64,
) -> diesel::QueryResult<i64> {
    let cursor = channel_history_sync::table
        .filter(channel_history_sync::channel_id.eq(channel_id))
        .select(channel_history_sync::cursor)
        .first::<i64>(conn)
        .optional()?
        .unwrap_or(0);
    if cursor < target || target <= 0 {
        return Ok(cursor);
    }

    let missing = chat::table
        .filter(chat::channel_id.eq(channel_id))
        .filter(chat::prev_log_id.gt(0))
        .filter(chat::log_id.le(target))
        .filter(sql::<diesel::sql_types::Bool>(MISSING_PREDECESSOR))
        .select(diesel::dsl::min(chat::prev_log_id))
        .first::<Option<i64>>(conn)?;
    let Some(missing) = missing else {
        return Ok(cursor);
    };
    let resume = chat::table
        .filter(chat::channel_id.eq(channel_id))
        .filter(chat::log_id.lt(missing))
        .select(diesel::dsl::max(chat::log_id))
        .first::<Option<i64>>(conn)?
        .unwrap_or(0);
    diesel::update(
        channel_history_sync::table.filter(channel_history_sync::channel_id.eq(channel_id)),
    )
    .set(channel_history_sync::cursor.eq(resume))
    .execute(conn)?;
    Ok(resume)
}

fn persist_page_rows(
    conn: &mut SqliteConnection,
    channel_id: i64,
    chat_rows: Vec<ChatRow>,
    cursor: i64,
) -> diesel::QueryResult<()> {
    upsert_chat_rows(conn, chat_rows)?;

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

fn upsert_chat_rows(
    conn: &mut SqliteConnection,
    chat_rows: Vec<ChatRow>,
) -> diesel::QueryResult<()> {
    for row in chat_rows {
        diesel::insert_into(chat::table)
            .values(&row)
            .on_conflict(chat::log_id)
            .do_update()
            .set((
                chat::prev_log_id.eq(sql::<Nullable<BigInt>>(
                    "COALESCE(excluded.prev_log_id, chat.prev_log_id)",
                )),
                chat::type_.eq(sql::<Integer>("(chat.type & ")
                    .bind::<Integer, _>(ChatType::DELETED_MASK)
                    .sql(") | excluded.type")),
                chat::message_id.eq(excluded(chat::message_id)),
                chat::send_at.eq(excluded(chat::send_at)),
                chat::author_id.eq(excluded(chat::author_id)),
                chat::message.eq(excluded(chat::message)),
                chat::attachment.eq(excluded(chat::attachment)),
                chat::supplement.eq(excluded(chat::supplement)),
                chat::referer.eq(excluded(chat::referer)),
            ))
            .execute(conn)?;
    }

    Ok(())
}

fn missing_predecessors(
    conn: &mut SqliteConnection,
    channel_id: i64,
    target: i64,
    attempted: &BTreeSet<i64>,
) -> diesel::QueryResult<Vec<i64>> {
    Ok(chat::table
        .filter(chat::channel_id.eq(channel_id))
        .filter(chat::log_id.le(target))
        .filter(chat::prev_log_id.gt(0))
        .filter(chat::prev_log_id.lt(chat::log_id.nullable()))
        .filter(chat::prev_log_id.ne_all(attempted.iter().copied().map(Some)))
        .filter(sql::<diesel::sql_types::Bool>(MISSING_PREDECESSOR))
        .select(chat::prev_log_id)
        .distinct()
        .order(chat::prev_log_id.desc())
        .limit(GAP_PAGE_SIZE)
        .load::<Option<i64>>(conn)?
        .into_iter()
        .flatten()
        .collect())
}

fn persist_missing_chats(
    conn: &mut SqliteConnection,
    channel_id: i64,
    requested: &[i64],
    logs: Vec<Chatlog>,
) -> diesel::QueryResult<Option<usize>> {
    if logs
        .iter()
        .any(|log| log.channel_id != channel_id || !requested.contains(&log.log_id))
    {
        return Ok(None);
    }
    let rows: BTreeMap<_, _> = logs
        .into_iter()
        .map(|log| (log.log_id, ChatRow::from_chatlog(log, None)))
        .collect();
    let existing = chat::table
        .filter(chat::log_id.eq_any(rows.keys()))
        .select(chat::log_id)
        .load::<i64>(conn)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let inserted = rows.keys().filter(|id| !existing.contains(id)).count();
    upsert_chat_rows(conn, rows.into_values().collect())?;
    Ok(Some(inserted))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistorySyncStop {
    UpToDate,
    ServerComplete,
    ReachedTarget,
    EmptyBatch,
    NoProgress,
    HistoryGap,
    PageLimit,
    TimeLimit,
    UnsupportedChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistorySyncResult {
    pub fetched_count: usize,
    pub page_count: usize,
    pub cached_count: usize,
    pub gap_count: usize,
    pub stop: HistorySyncStop,
}

impl HistorySyncResult {
    pub(crate) const fn stopped(
        fetched_count: usize,
        page_count: usize,
        stop: HistorySyncStop,
    ) -> Self {
        Self {
            fetched_count,
            page_count,
            cached_count: 0,
            gap_count: 0,
            stop,
        }
    }

    pub const fn complete(self) -> bool {
        self.gap_count == 0
            && matches!(
                self.stop,
                HistorySyncStop::UpToDate
                    | HistorySyncStop::ServerComplete
                    | HistorySyncStop::ReachedTarget
            )
    }
}

fn audit_history(
    conn: &mut SqliteConnection,
    channel_id: i64,
    mut result: HistorySyncResult,
) -> diesel::QueryResult<HistorySyncResult> {
    result.cached_count = chat::table
        .filter(chat::channel_id.eq(channel_id))
        .count()
        .get_result::<i64>(conn)? as usize;
    result.gap_count = chat::table
        .filter(chat::channel_id.eq(channel_id))
        .filter(chat::prev_log_id.gt(0))
        .filter(sql::<diesel::sql_types::Bool>(MISSING_PREDECESSOR))
        .count()
        .get_result::<i64>(conn)? as usize;
    if result.gap_count > 0
        && matches!(
            result.stop,
            HistorySyncStop::UpToDate
                | HistorySyncStop::ServerComplete
                | HistorySyncStop::ReachedTarget
        )
    {
        result.stop = HistorySyncStop::HistoryGap;
    }
    Ok(result)
}

#[derive(Debug, PartialEq, Eq)]
struct PageProgress {
    cursor: i64,
    fetched_count: usize,
    stop: Option<HistorySyncStop>,
}

fn apply_sync_page(
    conn: &mut SqliteConnection,
    channel_id: i64,
    cursor: i64,
    target: i64,
    response: SyncChatResponse,
) -> diesel::QueryResult<PageProgress> {
    let stopped = |stop| PageProgress {
        cursor,
        fetched_count: 0,
        stop: Some(stop),
    };
    if response.chatlogs.is_empty() {
        return Ok(stopped(HistorySyncStop::EmptyBatch));
    }
    if response
        .chatlogs
        .iter()
        .any(|log| log.channel_id != channel_id || log.log_id <= 0 || log.log_id > target)
    {
        return Ok(stopped(HistorySyncStop::NoProgress));
    }

    let rows: BTreeMap<_, _> = response
        .chatlogs
        .into_iter()
        .map(|log| (log.log_id, ChatRow::from_chatlog(log, None)))
        .collect();
    let next_cursor = cursor.max(*rows.last_key_value().unwrap().0);
    let fetched_count = rows.keys().filter(|&&log_id| log_id > cursor).count();
    persist_page_rows(conn, channel_id, rows.into_values().collect(), next_cursor)?;

    let stop = if next_cursor <= cursor {
        Some(HistorySyncStop::NoProgress)
    } else if next_cursor >= target {
        Some(if response.is_ok {
            HistorySyncStop::ServerComplete
        } else {
            HistorySyncStop::ReachedTarget
        })
    } else if response.is_ok {
        Some(HistorySyncStop::HistoryGap)
    } else {
        None
    };
    Ok(PageProgress {
        cursor: next_cursor,
        fetched_count,
        stop,
    })
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

    async fn load_cursor(&self, target: i64) -> ClientResult<i64> {
        let channel_id = self.channel_id;

        Ok(self
            .pool
            .spawn_transaction(move |conn| Ok(prepare_history_cursor(conn, channel_id, target)?))
            .await?)
    }

    async fn finish(
        &self,
        fetched_count: usize,
        page_count: usize,
        stop: HistorySyncStop,
    ) -> ClientResult<HistorySyncResult> {
        let channel_id = self.channel_id;
        Ok(self
            .pool
            .spawn(move |conn| {
                Ok(audit_history(
                    conn,
                    channel_id,
                    HistorySyncResult::stopped(fetched_count, page_count, stop),
                )?)
            })
            .await?)
    }

    pub async fn update_bounded(self, last_log_id: i64) -> ClientResult<HistorySyncResult> {
        let deadline = Instant::now() + SYNC_TIME_LIMIT;
        let mut result = self.update_forward(last_log_id, deadline).await?;
        if matches!(
            result.stop,
            HistorySyncStop::PageLimit | HistorySyncStop::TimeLimit | HistorySyncStop::NoProgress
        ) {
            return Ok(result);
        }

        let mut attempted = BTreeSet::new();
        loop {
            let channel_id = self.channel_id;
            let excluded = attempted.clone();
            let missing = self
                .pool
                .spawn(move |conn| {
                    Ok(missing_predecessors(
                        conn,
                        channel_id,
                        last_log_id,
                        &excluded,
                    )?)
                })
                .await?;
            if missing.is_empty() {
                return self
                    .finish(result.fetched_count, result.page_count, result.stop)
                    .await;
            }
            if result.page_count >= SYNC_PAGE_LIMIT {
                return self
                    .finish(
                        result.fetched_count,
                        result.page_count,
                        HistorySyncStop::PageLimit,
                    )
                    .await;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return self
                    .finish(
                        result.fetched_count,
                        result.page_count,
                        HistorySyncStop::TimeLimit,
                    )
                    .await;
            }
            let logs = match time::timeout(
                remaining,
                TalkSession(self.session)
                    .channel(channel_id)
                    .get_chat_logs(&missing),
            )
            .await
            {
                Ok(response) => response?,
                Err(_) => {
                    return self
                        .finish(
                            result.fetched_count,
                            result.page_count,
                            HistorySyncStop::TimeLimit,
                        )
                        .await
                }
            };
            result.page_count += 1;
            attempted.extend(missing.iter().copied());
            let inserted = self
                .pool
                .spawn_transaction(move |conn| {
                    Ok(persist_missing_chats(conn, channel_id, &missing, logs)?)
                })
                .await?;
            let Some(inserted) = inserted else {
                return self
                    .finish(
                        result.fetched_count,
                        result.page_count,
                        HistorySyncStop::NoProgress,
                    )
                    .await;
            };
            result.fetched_count += inserted;
            let channel_id = self.channel_id;
            let finished = self
                .pool
                .spawn(move |conn| {
                    let last = chat::table
                        .filter(chat::channel_id.eq(channel_id))
                        .select(diesel::dsl::max(chat::log_id))
                        .first::<Option<i64>>(conn)?;
                    let stop = if last.is_some_and(|id| id >= last_log_id) {
                        HistorySyncStop::ReachedTarget
                    } else {
                        HistorySyncStop::HistoryGap
                    };
                    Ok(audit_history(
                        conn,
                        channel_id,
                        HistorySyncResult::stopped(result.fetched_count, result.page_count, stop),
                    )?)
                })
                .await?;
            if finished.complete() {
                return Ok(finished);
            }
            result = finished;
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining <= SYNC_PAGE_DELAY {
                return self
                    .finish(
                        result.fetched_count,
                        result.page_count,
                        HistorySyncStop::TimeLimit,
                    )
                    .await;
            }
            time::sleep(SYNC_PAGE_DELAY).await;
        }
    }

    async fn update_forward(
        &self,
        last_log_id: i64,
        deadline: Instant,
    ) -> ClientResult<HistorySyncResult> {
        let mut cursor = self.load_cursor(last_log_id).await?;

        if cursor >= last_log_id {
            return self.finish(0, 0, HistorySyncStop::UpToDate).await;
        }

        let mut fetched_count = 0;

        for page_index in 0..SYNC_PAGE_LIMIT {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return self
                    .finish(fetched_count, page_index, HistorySyncStop::TimeLimit)
                    .await;
            }

            let response = match time::timeout(
                remaining,
                TalkSession(self.session)
                    .channel(self.channel_id)
                    .chat_logs_since(cursor),
            )
            .await
            {
                Ok(response) => response?,
                Err(_) => {
                    return self
                        .finish(fetched_count, page_index, HistorySyncStop::TimeLimit)
                        .await;
                }
            };

            let channel_id = self.channel_id;
            let page_target = response
                .chatlogs
                .iter()
                .filter(|log| log.channel_id == channel_id)
                .map(|log| log.log_id)
                .max()
                .unwrap_or(last_log_id)
                .max(last_log_id);
            let progress = self
                .pool
                .spawn_transaction(move |conn| {
                    Ok(apply_sync_page(
                        conn,
                        channel_id,
                        cursor,
                        page_target,
                        SyncChatResponse {
                            is_ok: response.eof,
                            chatlogs: response.chatlogs,
                            jsi_log_id: None,
                            last_token_id: None,
                        },
                    )?)
                })
                .await?;
            fetched_count += progress.fetched_count;
            cursor = progress.cursor;
            let page_count = page_index + 1;

            if let Some(stop) = progress.stop {
                return self.finish(fetched_count, page_count, stop).await;
            }

            if page_count == SYNC_PAGE_LIMIT {
                return self
                    .finish(fetched_count, page_count, HistorySyncStop::PageLimit)
                    .await;
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining <= SYNC_PAGE_DELAY {
                return self
                    .finish(fetched_count, page_count, HistorySyncStop::TimeLimit)
                    .await;
            }

            time::sleep(SYNC_PAGE_DELAY).await;
        }

        self.finish(fetched_count, SYNC_PAGE_LIMIT, HistorySyncStop::PageLimit)
            .await
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
            HistorySyncStop::HistoryGap,
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

    fn response(rows: Vec<ChatRow>, is_ok: bool) -> SyncChatResponse {
        SyncChatResponse {
            is_ok,
            chatlogs: rows.into_iter().map(Into::into).collect(),
            jsi_log_id: None,
            last_token_id: None,
        }
    }

    #[test]
    fn empty_page_does_not_complete_or_skip_to_cached_preview() {
        let mut conn = test_connection();
        persist_page_rows(&mut conn, 7, vec![chat_row(100, 7)], 0).unwrap();
        let progress = apply_sync_page(&mut conn, 7, 0, 100, response(vec![], true)).unwrap();
        assert_eq!(progress.stop, Some(HistorySyncStop::EmptyBatch));
        assert_eq!(prepare_history_cursor(&mut conn, 7, 100).unwrap(), 0);
    }

    #[test]
    fn overlapping_page_keeps_new_records_and_deduplicates() {
        let mut conn = test_connection();
        persist_page_rows(&mut conn, 7, vec![chat_row(10, 7)], 10).unwrap();
        let progress = apply_sync_page(
            &mut conn,
            7,
            10,
            100,
            response(
                vec![
                    chat_row(12, 7),
                    chat_row(10, 7),
                    chat_row(11, 7),
                    chat_row(12, 7),
                ],
                false,
            ),
        )
        .unwrap();
        assert_eq!(
            progress,
            PageProgress {
                cursor: 12,
                fetched_count: 2,
                stop: None
            }
        );
        assert_eq!(chat::table.count().get_result::<i64>(&mut conn).unwrap(), 3);
    }

    #[test]
    fn terminal_page_does_not_skip_a_gap_before_cached_target() {
        let mut conn = test_connection();
        persist_page_rows(&mut conn, 7, vec![chat_row(100, 7)], 0).unwrap();
        let progress =
            apply_sync_page(&mut conn, 7, 0, 100, response(vec![chat_row(10, 7)], true)).unwrap();
        assert_eq!(progress.cursor, 10);
        assert_eq!(progress.stop, Some(HistorySyncStop::HistoryGap));
        assert_eq!(prepare_history_cursor(&mut conn, 7, 100).unwrap(), 10);
    }

    #[test]
    fn replay_preserves_deletion_and_known_predecessor() {
        let mut conn = test_connection();
        let mut deleted = chat_row(10, 7);
        deleted.chat_type |= ChatType::DELETED_MASK;
        deleted.deleted_time = Some(99);
        deleted.prev_log_id = Some(9);
        persist_page_rows(&mut conn, 7, vec![deleted.clone()], 10).unwrap();
        apply_sync_page(&mut conn, 7, 0, 10, response(vec![chat_row(10, 7)], true)).unwrap();
        assert_eq!(chat::table.first::<ChatRow>(&mut conn).unwrap(), deleted);
    }

    #[test]
    fn missing_predecessor_is_incomplete_and_retry_resumes_before_gap() {
        let mut conn = test_connection();
        let mut latest = chat_row(30, 7);
        latest.prev_log_id = Some(20);
        persist_page_rows(&mut conn, 7, vec![chat_row(10, 7), latest], 30).unwrap();
        let result = audit_history(
            &mut conn,
            7,
            HistorySyncResult::stopped(0, 0, HistorySyncStop::UpToDate),
        )
        .unwrap();
        assert!(!result.complete());
        assert_eq!((result.cached_count, result.gap_count), (2, 1));
        assert_eq!(prepare_history_cursor(&mut conn, 7, 30).unwrap(), 10);

        let mut missing = chat_row(20, 7);
        missing.prev_log_id = Some(10);
        let progress =
            apply_sync_page(&mut conn, 7, 10, 30, response(vec![missing], false)).unwrap();
        assert_eq!(progress.cursor, 20);
        assert_eq!(prepare_history_cursor(&mut conn, 7, 30).unwrap(), 20);
        let result = audit_history(
            &mut conn,
            7,
            HistorySyncResult::stopped(1, 1, HistorySyncStop::ReachedTarget),
        )
        .unwrap();
        assert_eq!(result.gap_count, 0);
        assert!(result.complete());
    }

    #[test]
    fn page_for_another_room_does_not_change_history() {
        let mut conn = test_connection();
        let progress =
            apply_sync_page(&mut conn, 7, 0, 30, response(vec![chat_row(10, 8)], false)).unwrap();
        assert_eq!(progress.stop, Some(HistorySyncStop::NoProgress));
        assert_eq!(chat::table.count().get_result::<i64>(&mut conn).unwrap(), 0);
    }

    #[test]
    fn checkpoint_repair_keeps_existing_messages() {
        let mut conn = test_connection();
        persist_page_rows(&mut conn, 7, vec![chat_row(100, 7)], 100).unwrap();
        conn.batch_execute(include_str!(
            "../../migrations/2026-09-06-040000_history_checkpoint_repair/up.sql"
        ))
        .unwrap();
        assert_eq!(prepare_history_cursor(&mut conn, 7, 100).unwrap(), 0);
        assert_eq!(
            chat::table.first::<ChatRow>(&mut conn).unwrap(),
            chat_row(100, 7)
        );
    }

    #[test]
    fn missing_lookup_only_follows_known_links_in_the_selected_room() {
        let mut conn = test_connection();
        let linked = |id, room, previous| {
            let mut row = chat_row(id, room);
            row.prev_log_id = Some(previous);
            row
        };
        upsert_chat_rows(
            &mut conn,
            vec![
                chat_row(10, 7),
                linked(20, 7, 10),
                linked(30, 7, 25),
                linked(35, 7, 25),
                linked(40, 7, 39),
                linked(50, 7, 50),
                linked(60, 7, 61),
                linked(70, 7, 0),
                linked(80, 8, 79),
                linked(100, 7, 99),
            ],
        )
        .unwrap();
        assert_eq!(
            missing_predecessors(&mut conn, 7, 70, &BTreeSet::new()).unwrap(),
            vec![39, 25]
        );
        assert_eq!(
            missing_predecessors(&mut conn, 7, 70, &BTreeSet::from([39])).unwrap(),
            vec![25]
        );
    }

    #[test]
    fn missing_lookup_batch_is_bounded() {
        let mut conn = test_connection();
        let rows = (1..=40)
            .map(|id| {
                let mut row = chat_row(id * 2, 7);
                row.prev_log_id = Some(id * 2 - 1);
                row
            })
            .collect();
        upsert_chat_rows(&mut conn, rows).unwrap();
        let missing = missing_predecessors(&mut conn, 7, 80, &BTreeSet::new()).unwrap();
        assert_eq!(missing.len(), GAP_PAGE_SIZE as usize);
        assert_eq!((missing[0], *missing.last().unwrap()), (79, 17));
    }

    #[test]
    fn recovered_messages_fill_chain_without_advancing_forward_checkpoint() {
        let mut conn = test_connection();
        let mut latest = chat_row(40, 7);
        latest.prev_log_id = Some(30);
        persist_page_rows(&mut conn, 7, vec![chat_row(10, 7), latest], 10).unwrap();
        let mut attempted = BTreeSet::new();
        for (id, previous) in [(30, 20), (20, 10)] {
            let requested = missing_predecessors(&mut conn, 7, 40, &attempted).unwrap();
            assert_eq!(requested, vec![id]);
            let mut row = chat_row(id, 7);
            row.prev_log_id = Some(previous);
            assert_eq!(
                persist_missing_chats(
                    &mut conn,
                    7,
                    &requested,
                    vec![row.clone().into(), row.into()]
                )
                .unwrap(),
                Some(1)
            );
            attempted.extend(requested);
        }
        assert!(missing_predecessors(&mut conn, 7, 40, &attempted)
            .unwrap()
            .is_empty());
        let result = audit_history(
            &mut conn,
            7,
            HistorySyncResult::stopped(2, 2, HistorySyncStop::ReachedTarget),
        )
        .unwrap();
        assert_eq!((result.cached_count, result.gap_count), (4, 0));
        assert!(result.complete());
        assert_eq!(prepare_history_cursor(&mut conn, 7, 40).unwrap(), 10);
    }

    #[test]
    fn missing_lookup_rejects_unrequested_or_wrong_room_records_before_writing() {
        let mut conn = test_connection();
        for unexpected in [chat_row(21, 7), chat_row(20, 8)] {
            assert_eq!(
                persist_missing_chats(
                    &mut conn,
                    7,
                    &[10, 20],
                    vec![chat_row(10, 7).into(), unexpected.into()]
                )
                .unwrap(),
                None
            );
            assert_eq!(chat::table.count().get_result::<i64>(&mut conn).unwrap(), 0);
        }
    }

    #[test]
    fn unavailable_lookup_is_not_selected_again_in_the_same_attempt() {
        let mut conn = test_connection();
        let mut latest = chat_row(20, 7);
        latest.prev_log_id = Some(10);
        persist_page_rows(&mut conn, 7, vec![latest], 0).unwrap();
        let missing = missing_predecessors(&mut conn, 7, 20, &BTreeSet::new()).unwrap();
        assert_eq!(
            persist_missing_chats(&mut conn, 7, &missing, Vec::new()).unwrap(),
            Some(0)
        );
        assert!(
            missing_predecessors(&mut conn, 7, 20, &missing.into_iter().collect())
                .unwrap()
                .is_empty()
        );
        let result = audit_history(
            &mut conn,
            7,
            HistorySyncResult::stopped(0, 1, HistorySyncStop::ReachedTarget),
        )
        .unwrap();
        assert!(!result.complete());
        assert_eq!(result.gap_count, 1);
    }
}
