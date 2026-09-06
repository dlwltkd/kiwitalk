use std::{env, fs};

use diesel::{Connection, SqliteConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use headless_talk::archive::import_export;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 4 {
        return Err(
            "usage: import_chat_export DATABASE CHANNEL_ID EXPORT.txt UTC_OFFSET_MINUTES".into(),
        );
    }
    if !std::path::Path::new(&args[0]).is_file() {
        return Err("database does not exist".into());
    }
    let text = fs::read_to_string(&args[2])?;
    let mut conn = SqliteConnection::establish(&args[0])?;
    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");
    conn.run_pending_migrations(MIGRATIONS)?;
    let archive = import_export(
        &mut conn,
        args[1].parse()?,
        &args[2],
        &text,
        args[3].parse()?,
    )?;
    println!(
        "Imported {} messages and {} untimed events; {} local matches.",
        archive.message_count,
        archive.entries.len() - archive.message_count,
        archive.matched_count
    );
    Ok(())
}
