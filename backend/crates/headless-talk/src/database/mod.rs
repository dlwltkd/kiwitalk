mod constants;
pub mod model;
pub mod schema;

use diesel::{connection::SimpleConnection, Connection, SqliteConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use futures::Future;
use r2d2::Pool;
use std::sync::Mutex;
use thiserror::Error;

use self::constants::{CONNECTION_INIT_SQL, DATABASE_INIT_SQL};

type ConnectionManager = diesel::r2d2::ConnectionManager<SqliteConnection>;

#[derive(Debug, Default)]
struct ConnectionCustomizer {
    setup_lock: Mutex<()>,
}

impl r2d2::CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for ConnectionCustomizer {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        let _guard = self
            .setup_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        conn.batch_execute(CONNECTION_INIT_SQL)
            .map_err(diesel::r2d2::Error::QueryError)
    }
}

#[derive(Debug, Clone)]
pub struct DatabasePool(Pool<ConnectionManager>);

impl DatabasePool {
    pub async fn initialize(url: impl Into<String>) -> PoolTaskResult<Self> {
        let url = url.into();

        tokio::task::spawn_blocking(move || {
            let mut bootstrap = SqliteConnection::establish(&url)?;
            bootstrap.batch_execute(DATABASE_INIT_SQL)?;
            drop(bootstrap);

            let pool = Pool::builder()
                .connection_customizer(Box::new(ConnectionCustomizer::default()))
                .build(ConnectionManager::new(url))?;

            Ok(Self(pool))
        })
        .await?
    }

    pub fn get(&self) -> Result<PooledConnection, r2d2::Error> {
        self.0.get()
    }

    pub fn spawn<R, F>(&self, closure: F) -> impl Future<Output = PoolTaskResult<R>>
    where
        R: Send + 'static,
        F: FnOnce(&mut PooledConnection) -> PoolTaskResult<R> + Send + 'static,
    {
        let this = self.clone();

        async move { tokio::task::spawn_blocking(move || closure(&mut this.get()?)).await? }
    }

    pub fn spawn_transaction<R, F>(&self, closure: F) -> impl Future<Output = PoolTaskResult<R>>
    where
        R: Send + 'static,
        F: FnOnce(&mut PooledConnection) -> PoolTaskResult<R> + Send + 'static,
    {
        self.spawn(move |conn| conn.transaction(closure))
    }

    pub async fn migrate_to_latest(&self) -> Result<(), MigrationError> {
        const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

        let this = self.clone();

        tokio::task::spawn_blocking(move || {
            this.get()?.run_pending_migrations(MIGRATIONS)?;

            Ok(())
        })
        .await?
    }
}

pub type PooledConnection = r2d2::PooledConnection<ConnectionManager>;

#[derive(Debug, Error)]
#[error(transparent)]
pub enum PoolTaskError {
    Connection(#[from] diesel::result::ConnectionError),

    Database(#[from] diesel::result::Error),

    Pool(#[from] r2d2::Error),

    Task(#[from] tokio::task::JoinError),
}

#[derive(Debug, Error)]
#[error(transparent)]
pub enum MigrationError {
    Migration(#[from] Box<dyn std::error::Error + Send + Sync>),

    Pool(#[from] r2d2::Error),

    Task(#[from] tokio::task::JoinError),
}

pub type PoolTaskResult<T> = Result<T, PoolTaskError>;

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use diesel::{sql_query, Connection, QueryDsl, RunQueryDsl};

    use super::*;
    use crate::database::schema::{channel_history_sync, channel_list};

    struct TestDatabase(PathBuf);

    impl TestDatabase {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            Self(std::env::temp_dir().join(format!(
                "kiwitalk-history-migration-{}-{nonce}.sqlite3",
                std::process::id()
            )))
        }
    }

    impl Drop for TestDatabase {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn existing_database_gets_current_history_and_room_state() {
        const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

        let database = TestDatabase::new();
        let mut conn = SqliteConnection::establish(database.0.to_str().unwrap()).unwrap();
        conn.batch_execute(include_str!(
            "../../migrations/2023-10-21-003644_v0.1/up.sql"
        ))
        .unwrap();
        conn.batch_execute(
            "INSERT INTO channel_list \
             (id, type, display_users, active_user_count, unread_count, last_update) \
             VALUES (1, 'DirectChat', '[]', 1, 0, 0);",
        )
        .unwrap();

        conn.run_pending_migrations(MIGRATIONS).unwrap();

        let channel_count = channel_list::table
            .count()
            .get_result::<i64>(&mut conn)
            .unwrap();
        let checkpoint_count = channel_history_sync::table
            .count()
            .get_result::<i64>(&mut conn)
            .unwrap();
        let room_token = channel_list::table
            .select(channel_list::room_token)
            .first::<i64>(&mut conn)
            .unwrap();
        let last_log_id = channel_list::table
            .select(channel_list::last_log_id)
            .first::<i64>(&mut conn)
            .unwrap();

        assert_eq!(channel_count, 1);
        assert_eq!(checkpoint_count, 0);
        assert_eq!(room_token, 0);
        assert_eq!(last_log_id, 0);
    }

    #[test]
    fn connection_setup_does_not_change_journal_mode_while_database_is_busy() {
        let database = TestDatabase::new();
        let url = database.0.to_str().unwrap();
        let mut writer = SqliteConnection::establish(url).unwrap();
        writer
            .batch_execute("CREATE TABLE lock_test (id INTEGER); BEGIN IMMEDIATE;")
            .unwrap();
        let mut other = SqliteConnection::establish(url).unwrap();

        let result =
            r2d2::CustomizeConnection::on_acquire(&ConnectionCustomizer::default(), &mut other);
        writer.batch_execute("ROLLBACK;").unwrap();

        assert!(result.is_ok());
    }

    #[derive(diesel::QueryableByName)]
    struct JournalMode {
        #[diesel(sql_type = diesel::sql_types::Text)]
        journal_mode: String,
    }

    #[tokio::test]
    async fn pool_initialization_enables_wal_once() {
        let database = TestDatabase::new();
        let pool = DatabasePool::initialize(database.0.to_string_lossy())
            .await
            .unwrap();

        let journal_mode = pool
            .spawn(|conn| {
                Ok(sql_query("PRAGMA journal_mode;")
                    .get_result::<JournalMode>(conn)?
                    .journal_mode)
            })
            .await
            .unwrap();

        assert_eq!(journal_mode, "wal");
    }
}
