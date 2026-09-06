use super::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

const EXPORT: &str = "Room with KakaoTalk Chats\nDate Saved : 2026-09-06 17:00:00\n\n--------------- 2026년 9월 6일 일요일 ---------------\n[Alice] [AM 9:04] first anchor\n[Bob] [AM 11:05] second anchor\n[Alice] [PM 12:16] absent message\nThe message has been deleted.\n";

fn database() -> SqliteConnection {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    conn.run_pending_migrations(MIGRATIONS).unwrap();
    diesel::sql_query("INSERT INTO channel_list (id, type, display_users, active_user_count, unread_count, last_update, last_seen_log_id, last_log_id) VALUES (1, 'DirectChat', '[]', 2, 3, 10, 900, 901), (2, 'DirectChat', '[]', 2, 0, 0, 0, 0)").execute(&mut conn).unwrap();
    diesel::sql_query("INSERT INTO channel_history_sync (channel_id, cursor) VALUES (1, 800)")
        .execute(&mut conn)
        .unwrap();
    let parsed = parse_export("room.txt", EXPORT, 540).unwrap();
    for (i, entry) in parsed.entries.iter().take(2).enumerate() {
        diesel::insert_into(chat::table)
            .values((
                chat::channel_id.eq(1_i64),
                chat::log_id.eq(800 + i as i64),
                chat::type_.eq(1),
                chat::message_id.eq(1_i64),
                chat::author_id.eq(10 + i as i64),
                chat::send_at.eq(entry.send_at.unwrap() + 37),
                chat::message.eq(&entry.content),
            ))
            .execute(&mut conn)
            .unwrap();
    }
    conn
}

#[test]
fn parses_bom_crlf_multiline_and_untimed_deletion_without_inventing_a_time() {
    let text = format!(
        "\u{feff}{}",
        EXPORT
            .replace("absent message", "first line\n\nsecond line")
            .replace('\n', "\r\n")
    );
    let archive = parse_export("/tmp/room.txt", &text, 540).unwrap();
    assert_eq!(archive.source_name, "room.txt");
    assert_eq!(archive.message_count, 3);
    assert_eq!(archive.entries.len(), 4);
    assert_eq!(archive.entries[2].content, "first line\n\nsecond line");
    assert_eq!(archive.entries[2].time.as_deref(), Some("12:16"));
    assert!(archive.entries[3].send_at.is_none());
    assert!(archive.entries[3].sender.is_none());
    assert_eq!(archive.entries[3].date, "2026-09-06");
}

#[test]
fn quoted_message_headers_in_the_body_do_not_change_the_sender_or_time() {
    let text = EXPORT.replace("absent message", "quoted [Bob] [PM 2:30] hello");
    let archive = parse_export("room.txt", &text, 540).unwrap();
    assert_eq!(archive.entries[2].sender.as_deref(), Some("Alice"));
    assert_eq!(archive.entries[2].time.as_deref(), Some("12:16"));
    assert_eq!(archive.entries[2].content, "quoted [Bob] [PM 2:30] hello");
}

#[test]
fn parses_korean_meridiem_and_midnight_correctly() {
    let text = EXPORT
        .replace("Room with KakaoTalk Chats", "친구 님과 카카오톡 대화")
        .replace("Date Saved :", "저장한 날짜 :")
        .replace("AM 9:04", "오전 12:04")
        .replace("PM 12:16", "오후 12:16");
    let archive = parse_export("room.txt", &text, 540).unwrap();
    assert_eq!(archive.entries[0].time.as_deref(), Some("00:04"));
    assert_eq!(archive.entries[2].time.as_deref(), Some("12:16"));
    assert_eq!(
        archive.entries[2].send_at.unwrap() - archive.entries[0].send_at.unwrap(),
        12 * 3600 + 12 * 60
    );
}

#[test]
fn rejects_invalid_exports_instead_of_silently_dropping_records() {
    for text in [
        "unrelated text".to_owned(),
        EXPORT.replace("9월 6일", "2월 31일"),
        EXPORT.replace("AM 9:04", "AM 13:04"),
    ] {
        assert!(parse_export("room.txt", &text, 540).is_err());
    }
    assert!(parse_export("room.txt", EXPORT, i32::MAX).is_err());
    assert!(parse_export("room.txt", &"x".repeat(MAX_EXPORT_BYTES + 1), 540).is_err());
}

#[test]
fn repeated_import_preserves_live_logs_read_markers_and_sync_progress() {
    let mut conn = database();
    // Compare the fields used by synchronization and reading independently of archive rows.
    let live = chat::table
        .order(chat::log_id)
        .load::<crate::database::model::chat::ChatRow>(&mut conn)
        .unwrap();
    for _ in 0..2 {
        let imported = import_export(&mut conn, 1, "room.txt", EXPORT, 540).unwrap();
        assert_eq!(imported.matched_count, 2);
        assert_eq!(imported.entries[0].sender_id.as_deref(), Some("10"));
        assert_eq!(imported.entries[1].sender_id.as_deref(), Some("11"));
        assert_eq!(imported.entries[2].sender_id.as_deref(), Some("10"));
    }
    assert_eq!(
        chat::table
            .order(chat::log_id)
            .load::<crate::database::model::chat::ChatRow>(&mut conn)
            .unwrap(),
        live
    );
    assert_eq!(
        channel_list::table
            .find(1_i64)
            .select((
                channel_list::last_seen_log_id,
                channel_list::last_log_id,
                channel_list::unread_count
            ))
            .first::<(Option<i64>, i64, i32)>(&mut conn)
            .unwrap(),
        (Some(900), 901, 3)
    );
    assert_eq!(
        crate::database::schema::channel_history_sync::table
            .select(crate::database::schema::channel_history_sync::cursor)
            .first::<i64>(&mut conn)
            .unwrap(),
        800
    );
    assert_eq!(
        channel_chat_archive::table
            .count()
            .get_result::<i64>(&mut conn)
            .unwrap(),
        1
    );
    assert_eq!(
        load_archive(&mut conn, 1).unwrap().unwrap().message_count,
        3
    );
}

#[test]
fn wrong_room_and_wrong_timezone_do_not_replace_a_saved_archive() {
    let mut conn = database();
    import_export(&mut conn, 1, "room.txt", EXPORT, 540).unwrap();
    assert!(matches!(
        import_export(&mut conn, 2, "wrong.txt", EXPORT, 540),
        Err(ArchiveError::WrongRoom)
    ));
    assert!(matches!(
        import_export(&mut conn, 1, "wrong.txt", EXPORT, 0),
        Err(ArchiveError::WrongRoom)
    ));
    assert_eq!(
        load_archive(&mut conn, 1).unwrap().unwrap().source_name,
        "room.txt"
    );
    assert!(load_archive(&mut conn, 2).unwrap().is_none());
}

#[test]
fn unknown_and_ambiguous_senders_keep_exported_names_without_false_identity() {
    let mut conn = database();
    let text = EXPORT
        .replace("[Bob]", "[Alice]")
        .replace("[Alice] [PM 12:16]", "[Visitor] [PM 12:16]");
    let imported = import_export(&mut conn, 1, "room.txt", &text, 540).unwrap();
    assert!(imported
        .entries
        .iter()
        .all(|entry| entry.sender_id.is_none()));
    assert_eq!(imported.entries[2].sender.as_deref(), Some("Visitor"));
}
