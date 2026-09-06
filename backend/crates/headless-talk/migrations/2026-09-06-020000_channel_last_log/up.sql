ALTER TABLE channel_list
ADD COLUMN last_log_id BIGINT NOT NULL DEFAULT 0;

-- Older builds advanced this cursor even when SYNCMSG returned no chat logs.
-- Rebuild it from the local transcript after the next channel-list refresh.
DELETE FROM channel_history_sync;
