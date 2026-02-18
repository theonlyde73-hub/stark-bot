# Phase 2: Worker Delegation Upgrade

## New Files (2)

### 1. `stark-backend/src/channels/coalescing.rs`
- `MessageCoalescer` struct with debounce (1.5s) + max_wait (5s) timers
- Per-channel key grouping via `DashMap`
- `CoalescerConfig { debounce_ms, max_wait_ms, enabled }`
- `add_message()` returns a future that resolves when the coalesced batch is ready
- `flush()` to force-flush all pending messages

### 2. `stark-backend/src/tools/builtin/core/branch.rs`
- `BranchTool` — spawn context-inheriting branches
- Inherits parent messages (last 20) + compaction summary
- Restricted to Memory+System tool groups
- Result injected as system message back into parent

## Existing Files to Modify (10)

### 1. `ai/multi_agent/types.rs`
- Add `SubAgentMode` enum: `Standard`, `Branch`, `SilentBranch`
- Add `WorkerCheckpoint` struct: `iteration: u32, context_snapshot: String, timestamp: DateTime`
- Add to `SubAgentContext`: `mode: SubAgentMode`, `parent_context_snapshot: Option<String>`, `checkpoints: Vec<WorkerCheckpoint>`

### 2. `ai/multi_agent/subagent_manager.rs`
- Add `spawn_branch()` method for context-inheriting branches
- Modify tool restriction for branch mode (Memory+System only)
- Add checkpoint logic: save every 25 turns
- Add overflow recovery: retry from last checkpoint up to 3 times

### 3. `context/mod.rs`
- Add `CompactionLevel` enum: `Background`, `Aggressive`, `Emergency`
- Add `ThreeTierCompactionConfig` struct
- Add `check_compaction_level()` — determine which tier to trigger
- Add `compact_emergency()` — synchronous hard-drop 50%
- Add `compact_aggressive()` — compact 50%
- Add `compact_tiered()` — route to appropriate tier

### 4. `gateway/protocol.rs`
- Add 6 event types: `BranchStarted`, `BranchCompleted`, `BranchFailed`, `CoalesceBuffering`, `CoalesceFlushed`, `WorkerCheckpoint`

### 5. `channels/dispatcher/mod.rs`
- Add coalescer field to `MessageDispatcher`
- Add `dispatch_coalesced()` method
- Add `maybe_spawn_silent_branch()` — auto-trigger every 10 messages

### 6. `channels/mod.rs`
- Add `pub mod coalescing;`

### 7. `tools/builtin/core/mod.rs`
- Register `BranchTool`

### 8. `db/sqlite.rs`
- Add migration columns to `sub_agents` table: mode, parent_context_snapshot, checkpoints
- Add coalescing/compaction config to `bot_settings`

### 9. `models/mod.rs`
- Add coalescing config fields to BotSettings: `coalescing_enabled`, `coalescing_debounce_ms`, `coalescing_max_wait_ms`
- Add compaction config fields: `compaction_background_threshold`, `compaction_aggressive_threshold`, `compaction_emergency_threshold`

### 10. `db/tables/chat_sessions.rs`
- Add `get_session_message_count_since_last_branch()` method
