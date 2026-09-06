-- Cached previews and live messages do not prove that older history was fetched.
UPDATE channel_history_sync SET cursor = 0;

CREATE INDEX IF NOT EXISTS chat_channel_log ON chat (channel_id, log_id);
