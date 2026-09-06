UPDATE channel_list
SET last_log_id = 0
WHERE last_log_id > 0
  AND last_log_id = COALESCE(last_seen_log_id, 0)
  AND NOT EXISTS (
    SELECT 1
    FROM chat
    WHERE chat.channel_id = channel_list.id
      AND chat.log_id = channel_list.last_log_id
  );
