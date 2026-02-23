[DISCORD HOOK — New message detected]
Guild: {guildId} | Channel: #{channelName} ({channelId})
Author: {authorName} (ID: {authorId})
Message ID: {messageId}
Content:
{content}

Evaluate this message for violations per your rules. Then:
- If CLEAN: call task_fully_completed with summary "HEARTBEAT_OK". Do nothing else.
- If VIOLATION: immediately take action using your tools (discord_write deleteMessage, kv_store for strikes, discord_write sendMessage for warnings/bans). After completing all actions, call task_fully_completed with a brief summary of what you did.
