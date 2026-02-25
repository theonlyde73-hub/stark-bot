[Meta Marketer Pulse — {timestamp}]

Periodic performance check triggered.

**Your task:**

1. Check `kv_store(action="get", key="META_LAST_AUDIT_TS")` — if <6 hours ago, do a quick spend check only. Otherwise run a full audit.
2. For **quick check**: Pull `meta_insights(action="account_insights", date_preset="today")` and compare today's spend vs yesterday. Report only if anomalies detected (spend >50% higher or lower than yesterday).
3. For **full audit**: Run `meta_insights(action="audit")` with target CPA/ROAS from goals if available. Analyze all flagged issues.
4. Update kv_store tracking keys:
   - `META_LAST_AUDIT_TS` → current timestamp
   - `META_DAILY_SPEND_{date}` → today's spend
   - Any new alert keys for dedup
5. Cross-reference notable findings with `memory_search` for historical context.
6. Call `task_fully_completed` with your analysis summary.
