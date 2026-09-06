pub const DATABASE_INIT_SQL: &str = "
PRAGMA busy_timeout = 5000;
PRAGMA journal_mode = WAL;
";

pub const CONNECTION_INIT_SQL: &str = "
PRAGMA busy_timeout = 5000;
PRAGMA synchronous = NORMAL;
PRAGMA wal_autocheckpoint = 8192;
";
