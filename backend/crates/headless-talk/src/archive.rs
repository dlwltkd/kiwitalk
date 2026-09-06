use std::collections::{HashMap, HashSet};

use chrono::{FixedOffset, NaiveDate, TimeZone};
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection,
};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    database::{
        schema::{channel_chat_archive, channel_list, chat},
        PoolTaskError,
    },
    HeadlessTalk,
};

#[cfg(test)]
mod tests;

const MAX_EXPORT_BYTES: usize = 8 * 1024 * 1024;
static DATE_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^-{3,}\s*(\d{4})년\s*(\d{1,2})월\s*(\d{1,2})일\s+.+?\s*-{3,}$").unwrap()
});
static MESSAGE_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\[(.+?)\] \[(AM|PM|오전|오후) (\d{1,2}):(\d{2})\](?: (.*))?$").unwrap()
});

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntry {
    pub date: String,
    pub time: Option<String>,
    pub sender: Option<String>,
    pub sender_id: Option<String>,
    pub content: String,
    // Exported times have minute precision. This value is used only to match local rows.
    pub send_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatArchive {
    pub source_name: String,
    pub saved_at: String,
    pub utc_offset_minutes: i32,
    pub message_count: usize,
    pub matched_count: usize,
    pub entries: Vec<ArchiveEntry>,
}

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("{0}")]
    Format(&'static str),
    #[error(
        "This file could not be matched to the room. At least two matching messages are required."
    )]
    WrongRoom,
    #[error(transparent)]
    Database(#[from] diesel::result::Error),
    #[error(transparent)]
    Pool(#[from] PoolTaskError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub fn parse_export(
    source_name: &str,
    text: &str,
    utc_offset_minutes: i32,
) -> Result<ChatArchive, ArchiveError> {
    if text.len() > MAX_EXPORT_BYTES {
        return Err(ArchiveError::Format(
            "Chat exports must be 8 MiB or smaller.",
        ));
    }
    if !(-720..=840).contains(&utc_offset_minutes) {
        return Err(ArchiveError::Format("Invalid export time zone."));
    }
    let offset = FixedOffset::east_opt(utc_offset_minutes * 60).unwrap();
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    let title = lines.next().unwrap_or_default();
    let saved = lines.next().unwrap_or_default();
    if !(title.ends_with(" with KakaoTalk Chats") || title.ends_with("님과 카카오톡 대화"))
    {
        return Err(ArchiveError::Format(
            "Choose a UTF-8 KakaoTalk desktop chat export (.txt).",
        ));
    }
    let saved_at = saved
        .strip_prefix("Date Saved :")
        .or_else(|| saved.strip_prefix("저장한 날짜 :"))
        .ok_or(ArchiveError::Format("The export's saved date is missing."))?
        .trim()
        .to_owned();
    let mut date: Option<NaiveDate> = None;
    let mut entries: Vec<ArchiveEntry> = Vec::new();
    let mut continuation = false;
    for line in lines {
        if let Some(parts) = DATE_LINE.captures(line) {
            date = NaiveDate::from_ymd_opt(
                parts[1].parse().unwrap_or_default(),
                parts[2].parse().unwrap_or_default(),
                parts[3].parse().unwrap_or_default(),
            );
            if date.is_none() {
                return Err(ArchiveError::Format("The export contains an invalid date."));
            }
            continuation = false;
        } else if let Some(parts) = MESSAGE_LINE.captures(line) {
            let date = date.ok_or(ArchiveError::Format("A message is missing its date."))?;
            let hour: u32 = parts[3].parse().unwrap_or_default();
            let minute: u32 = parts[4].parse().unwrap_or(u32::MAX);
            if !(1..=12).contains(&hour) || minute > 59 {
                return Err(ArchiveError::Format("The export contains an invalid time."));
            }
            let hour = hour % 12
                + if matches!(&parts[2], "PM" | "오후") {
                    12
                } else {
                    0
                };
            let timestamp = offset
                .from_local_datetime(&date.and_hms_opt(hour, minute, 0).unwrap())
                .single()
                .unwrap()
                .timestamp();
            entries.push(ArchiveEntry {
                date: date.to_string(),
                time: Some(format!("{hour:02}:{minute:02}")),
                sender: Some(parts[1].to_owned()),
                sender_id: None,
                content: parts.get(5).map_or("", |part| part.as_str()).to_owned(),
                send_at: Some(timestamp),
            });
            continuation = true;
        } else if matches!(
            line.trim(),
            "The message has been deleted." | "삭제된 메시지입니다."
        ) {
            entries.push(ArchiveEntry {
                date: date
                    .ok_or(ArchiveError::Format("An event is missing its date."))?
                    .to_string(),
                time: None,
                sender: None,
                sender_id: None,
                content: line.to_owned(),
                send_at: None,
            });
            continuation = false;
        } else if continuation {
            let content = &mut entries.last_mut().unwrap().content;
            content.push('\n');
            content.push_str(line);
        } else if !line.trim().is_empty() {
            return Err(ArchiveError::Format(
                "The export contains an unsupported date or event format.",
            ));
        }
    }
    let message_count = entries
        .iter()
        .filter(|entry| entry.sender.is_some())
        .count();
    if message_count == 0 {
        return Err(ArchiveError::Format(
            "No messages were found in this export.",
        ));
    }
    Ok(ChatArchive {
        source_name: source_name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("chat.txt")
            .chars()
            .take(255)
            .collect(),
        saved_at,
        utc_offset_minutes,
        message_count,
        matched_count: 0,
        entries,
    })
}

/// Imports a local snapshot without changing server logs, read markers, or sync cursors.
pub fn import_export(
    conn: &mut SqliteConnection,
    channel_id: i64,
    source_name: &str,
    text: &str,
    utc_offset_minutes: i32,
) -> Result<ChatArchive, ArchiveError> {
    let mut archive = parse_export(source_name, text, utc_offset_minutes)?;
    conn.transaction(|conn| {
        channel_list::table
            .find(channel_id)
            .select(channel_list::id)
            .first::<i64>(conn)?;
        let first = archive
            .entries
            .iter()
            .filter_map(|entry| entry.send_at)
            .min()
            .unwrap();
        let last = archive
            .entries
            .iter()
            .filter_map(|entry| entry.send_at)
            .max()
            .unwrap()
            + 59;
        let rows = chat::table
            .filter(chat::channel_id.eq(channel_id))
            .filter(chat::send_at.between(first, last))
            .filter(chat::deleted_time.is_null())
            .filter(chat::type_.eq(1))
            .select((chat::send_at, chat::author_id, chat::message))
            .load::<(i64, i64, Option<String>)>(conn)?;
        let mut anchors: HashMap<(i64, String), HashSet<i64>> = HashMap::new();
        for (sent_at, author, content) in rows {
            if let Some(content) = content.filter(|value| !value.trim().is_empty()) {
                anchors
                    .entry((sent_at.div_euclid(60), content.trim().replace("\r\n", "\n")))
                    .or_default()
                    .insert(author);
            }
        }
        let mut matches = HashSet::new();
        let mut senders: HashMap<String, HashSet<i64>> = HashMap::new();
        for entry in &archive.entries {
            let (Some(timestamp), Some(sender)) = (entry.send_at, entry.sender.as_ref()) else {
                continue;
            };
            let key = (timestamp.div_euclid(60), entry.content.trim().to_owned());
            if let Some(authors) = anchors.get(&key).filter(|authors| authors.len() == 1) {
                matches.insert(key);
                senders.entry(sender.clone()).or_default().extend(authors);
            }
        }
        if matches.len() < 2 {
            return Err(ArchiveError::WrongRoom);
        }
        archive.matched_count = matches.len();
        for entry in &mut archive.entries {
            entry.sender_id = entry
                .sender
                .as_ref()
                .and_then(|name| senders.get(name))
                .filter(|ids| ids.len() == 1)
                .and_then(|ids| ids.iter().next())
                .map(ToString::to_string);
        }
        let payload = serde_json::to_string(&archive)?;
        diesel::insert_into(channel_chat_archive::table)
            .values((
                channel_chat_archive::channel_id.eq(channel_id),
                channel_chat_archive::payload.eq(&payload),
            ))
            .on_conflict(channel_chat_archive::channel_id)
            .do_update()
            .set(channel_chat_archive::payload.eq(&payload))
            .execute(conn)?;
        Ok(archive)
    })
}

pub fn load_archive(
    conn: &mut SqliteConnection,
    channel_id: i64,
) -> Result<Option<ChatArchive>, ArchiveError> {
    channel_chat_archive::table
        .find(channel_id)
        .select(channel_chat_archive::payload)
        .first::<String>(conn)
        .optional()?
        .map(|json| serde_json::from_str(&json))
        .transpose()
        .map_err(Into::into)
}

impl HeadlessTalk {
    pub async fn import_chat_archive(
        &self,
        id: i64,
        source_name: String,
        text: String,
        utc_offset_minutes: i32,
    ) -> Result<ChatArchive, ArchiveError> {
        self.conn
            .pool
            .spawn(move |conn| {
                Ok(import_export(
                    conn,
                    id,
                    &source_name,
                    &text,
                    utc_offset_minutes,
                ))
            })
            .await?
    }

    pub async fn load_chat_archive(&self, id: i64) -> Result<Option<ChatArchive>, ArchiveError> {
        self.conn
            .pool
            .spawn(move |conn| Ok(load_archive(conn, id)))
            .await?
    }
}
