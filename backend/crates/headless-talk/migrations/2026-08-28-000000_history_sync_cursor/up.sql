CREATE TABLE channel_history_sync (
    channel_id BIGINT PRIMARY KEY NOT NULL,
    cursor BIGINT NOT NULL DEFAULT 0
);
