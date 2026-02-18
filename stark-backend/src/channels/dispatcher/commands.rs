use crate::ai::{AiClient, AiResponse, Message, ThinkingLevel, ToolHistoryEntry};
use crate::tools::ToolDefinition;
use crate::channels::types::{DispatchResult, NormalizedMessage};
use crate::context;
use crate::gateway::protocol::GatewayEvent;
use crate::models::SessionScope;
use crate::telemetry;
use once_cell::sync::Lazy;
use regex::Regex;
use std::time::Duration;
use tokio::time::interval;

use super::MessageDispatcher;

/// How often to broadcast "still waiting" events during long AI calls
const AI_PROGRESS_INTERVAL_SECS: u64 = 30;

/// Compiled regex pattern for inline thinking (e.g., "/t:medium What is...")
pub(super) static INLINE_THINKING_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^/(?:t|think|thinking):(\w+)\s+(.+)$").unwrap()
});

/// Compiled regex pattern for standalone thinking directives (e.g., "/think:medium")
pub(super) static THINKING_DIRECTIVE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^/(?:t|think|thinking)(?::(\w+))?$").unwrap()
});

/// Parse inline thinking directive from message (e.g., "/think:high What is...")
/// Returns the thinking level and the clean message text
pub(super) fn parse_inline_thinking(text: &str) -> (Option<ThinkingLevel>, Option<String>) {
    let text = text.trim();

    // Use static pattern to avoid recompiling on every call
    if let Some(captures) = INLINE_THINKING_PATTERN.captures(text) {
        let level_str = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        let clean_text = captures.get(2).map(|m| m.as_str().to_string());

        if let Some(level) = ThinkingLevel::from_str(level_str) {
            return (Some(level), clean_text);
        }
    }

    // No inline thinking directive found
    (None, None)
}

impl MessageDispatcher {
    /// Handle thinking directive messages (e.g., "/think:medium" sets session default)
    pub(super) async fn handle_thinking_directive(&self, message: &NormalizedMessage) -> Option<DispatchResult> {
        let text = message.text.trim();

        // Check if this is a standalone thinking directive
        if let Some(captures) = THINKING_DIRECTIVE_PATTERN.captures(text) {
            let level_str = captures.get(1).map(|m| m.as_str()).unwrap_or("low");

            if let Some(level) = ThinkingLevel::from_str(level_str) {
                // Store the thinking level preference for this session
                // For now, we just acknowledge it (session storage could be added later)
                let response = format!(
                    "Thinking level set to **{}**. {}",
                    level,
                    match level {
                        ThinkingLevel::Off => "Extended thinking is now disabled.",
                        ThinkingLevel::Minimal => "Using minimal thinking (~1K tokens).",
                        ThinkingLevel::Low => "Using low thinking (~4K tokens).",
                        ThinkingLevel::Medium => "Using medium thinking (~10K tokens).",
                        ThinkingLevel::High => "Using high thinking (~32K tokens).",
                        ThinkingLevel::XHigh => "Using maximum thinking (~64K tokens).",
                    }
                );

                self.broadcaster.broadcast(GatewayEvent::agent_response(
                    message.channel_id,
                    &message.user_name,
                    &response,
                ));

                log::info!(
                    "Thinking level set to {} for user {} on channel {}",
                    level,
                    message.user_name,
                    message.channel_id
                );

                return Some(DispatchResult::success(response));
            } else {
                // Invalid level specified
                let response = format!(
                    "Invalid thinking level '{}'. Valid options: off, minimal, low, medium, high, xhigh",
                    level_str
                );
                self.broadcaster.broadcast(GatewayEvent::agent_response(
                    message.channel_id,
                    &message.user_name,
                    &response,
                ));
                return Some(DispatchResult::success(response));
            }
        }

        None
    }

    /// Call AI with progress notifications for long-running requests
    /// Broadcasts "still waiting" events every 30 seconds and handles timeout errors gracefully
    /// Also emits granular thinking phase tasks for better UI visibility
    pub(super) async fn generate_with_progress(
        &self,
        client: &AiClient,
        conversation: Vec<Message>,
        tool_history: Vec<ToolHistoryEntry>,
        tools: Vec<ToolDefinition>,
        channel_id: i64,
        session_id: i64,
    ) -> Result<AiResponse, crate::ai::AiError> {
        let broadcaster = self.broadcaster.clone();
        let mut elapsed_secs = 0u64;

        // Get execution ID for task tracking
        let execution_id = self.execution_tracker.get_execution_id(channel_id);

        // Emit granular thinking phase tasks
        let thinking_task_id = if let Some(ref exec_id) = execution_id {
            // Determine context for the thinking task
            let (phase_desc, phase_active) = if !tool_history.is_empty() {
                ("Processing tool results", "Analyzing results...")
            } else if !tools.is_empty() {
                ("Analyzing request", "Reasoning about approach...")
            } else {
                ("Generating response", "Composing response...")
            };

            let task_id = self.execution_tracker.start_task(
                channel_id,
                exec_id,
                Some(exec_id),
                crate::models::TaskType::Thinking,
                phase_desc,
                Some(phase_active),
            );
            Some(task_id)
        } else {
            None
        };

        // Get cancellation token for immediate interruption
        let cancel_token = self.execution_tracker.get_cancellation_token(channel_id);

        // Broadcast the full context being sent to the AI (for debug panel)
        broadcaster.broadcast(GatewayEvent::agent_context_update(
            channel_id,
            session_id,
            &conversation,
            &tools,
            &tool_history,
        ));

        // Spawn the actual AI request
        let ai_future = client.generate_with_tools(conversation, tool_history, tools.clone());
        tokio::pin!(ai_future);

        // Watchdog LLM timeout
        let llm_timeout = self.watchdog_config.timeout_for_llm();
        let llm_deadline = tokio::time::sleep(llm_timeout);
        tokio::pin!(llm_deadline);

        // Create a ticker for progress updates (shorter interval for more visibility)
        let mut progress_ticker = interval(Duration::from_secs(AI_PROGRESS_INTERVAL_SECS));
        progress_ticker.tick().await; // First tick is immediate, skip it

        // Thinking phase messages for variety
        let thinking_phases = [
            "Analyzing context...",
            "Evaluating options...",
            "Considering approach...",
            "Reviewing information...",
            "Formulating response...",
            "Deep thinking...",
        ];
        let mut phase_idx = 0;

        loop {
            tokio::select! {
                // Highest priority: check for cancellation via token (immediate)
                _ = cancel_token.cancelled() => {
                    log::info!("[AI_PROGRESS] Execution cancelled via token while waiting for AI response");

                    // Complete the thinking task
                    if let Some(ref task_id) = thinking_task_id {
                        self.execution_tracker.complete_task(task_id);
                    }

                    return Err(crate::ai::AiError::new("Execution cancelled by user"));
                }
                // Watchdog: enforce LLM call timeout
                _ = &mut llm_deadline => {
                    log::warn!(
                        "[WATCHDOG] LLM call timed out after {}s on channel {}",
                        llm_timeout.as_secs(),
                        channel_id
                    );

                    // Complete the thinking task
                    if let Some(ref task_id) = thinking_task_id {
                        self.execution_tracker.complete_task(task_id);
                    }

                    // Emit timeout telemetry
                    telemetry::emit_annotation("watchdog_llm_timeout", serde_json::json!({
                        "timeout_secs": llm_timeout.as_secs(),
                        "elapsed_secs": elapsed_secs,
                        "channel_id": channel_id,
                    }));

                    // Dispatch OnWatchdogTimeout hook
                    if let Some(hook_manager) = &self.hook_manager {
                        use crate::hooks::{HookContext, HookEvent};
                        let mut hook_ctx = HookContext::new(HookEvent::OnWatchdogTimeout)
                            .with_channel(channel_id, None)
                            .with_error(format!("LLM call timed out after {}s", llm_timeout.as_secs()));
                        hook_ctx.extra = serde_json::json!({
                            "operation": "llm_call",
                            "timeout_secs": llm_timeout.as_secs(),
                        });
                        let _ = hook_manager.execute(HookEvent::OnWatchdogTimeout, &mut hook_ctx).await;
                    }

                    return Err(crate::ai::AiError::new(
                        format!("LLM call timed out after {}s", llm_timeout.as_secs())
                    ));
                }
                result = &mut ai_future => {
                    // Complete the thinking task
                    if let Some(ref task_id) = thinking_task_id {
                        self.execution_tracker.complete_task(task_id);
                    }

                    match result {
                        Ok(response) => {
                            // If there are tool calls, emit a planning task
                            if !response.tool_calls.is_empty() {
                                if let Some(ref exec_id) = execution_id {
                                    let plan_desc = format!("Planning {} tool calls", response.tool_calls.len());
                                    let plan_task = self.execution_tracker.start_task(
                                        channel_id,
                                        exec_id,
                                        Some(exec_id),
                                        crate::models::TaskType::Planning,
                                        &plan_desc,
                                        Some("Preparing tool execution..."),
                                    );
                                    self.execution_tracker.complete_task(&plan_task);
                                }
                            }
                            return Ok(response);
                        }
                        Err(e) => {
                            let error_msg = e.to_string();
                            // Check if it's a timeout error
                            if error_msg.contains("timed out") || error_msg.contains("timeout") {
                                log::error!("[AI_PROGRESS] Request timed out after {}s: {}", elapsed_secs, error_msg);
                                broadcaster.broadcast(GatewayEvent::agent_error(
                                    channel_id,
                                    &format!("AI request timed out after {} seconds. The AI service may be overloaded. Please try again.", elapsed_secs + AI_PROGRESS_INTERVAL_SECS),
                                ));
                            }
                            return Err(e);
                        }
                    }
                }
                _ = progress_ticker.tick() => {
                    elapsed_secs += AI_PROGRESS_INTERVAL_SECS;
                    let phase_msg = thinking_phases[phase_idx % thinking_phases.len()];
                    phase_idx += 1;

                    log::info!("[AI_PROGRESS] Still waiting for AI response... ({}s elapsed)", elapsed_secs);
                    broadcaster.broadcast(GatewayEvent::agent_thinking(
                        channel_id,
                        Some(session_id),
                        &format!("{} ({}s)", phase_msg, elapsed_secs),
                    ));

                    // Update the thinking task's active form
                    if let Some(ref task_id) = thinking_task_id {
                        self.execution_tracker.update_task_active_form(
                            task_id,
                            &format!("{} ({}s)", phase_msg, elapsed_secs),
                        );
                    }
                }
            }
        }
    }

    /// Handle /new or /reset commands
    pub(super) async fn handle_reset_command(&self, message: &NormalizedMessage) -> DispatchResult {
        // Cancel any ongoing execution for this channel
        self.execution_tracker.cancel_execution(message.channel_id);

        // Cancel all subagents for this channel
        if let Some(ref manager) = self.subagent_manager {
            let cancelled = manager.cancel_all_for_channel(message.channel_id);
            if cancelled > 0 {
                log::info!(
                    "[RESET] Cancelled {} subagents for channel {}",
                    cancelled,
                    message.channel_id
                );
            }
        }

        // Brief delay to ensure in-flight operations acknowledge cancellation
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Determine session scope
        let scope = if message.chat_id != message.user_id {
            SessionScope::Group
        } else {
            SessionScope::Dm
        };

        // Get the current session
        match self.db.get_or_create_chat_session(
            &message.channel_type,
            message.channel_id,
            &message.chat_id,
            scope,
            None,
        ) {
            Ok(session) => {
                // Get identity for memory storage
                let identity_id = self.db.get_or_create_identity(
                    &message.channel_type,
                    &message.user_id,
                    Some(&message.user_name),
                ).ok().map(|id| id.identity_id);

                // Save session memory before reset (session memory hook)
                let message_count = self.db.count_session_messages(session.id).unwrap_or(0);
                if message_count >= 2 {
                    // Check if any memory-excluded tools were used in this session
                    let has_excluded_tools = self.db.get_recent_session_messages(session.id, 50)
                        .unwrap_or_default()
                        .iter()
                        .any(|m| {
                            m.role == crate::models::session_message::MessageRole::ToolCall
                                && crate::tools::types::MEMORY_EXCLUDE_TOOL_LIST.iter()
                                    .any(|t| m.content.contains(&format!("`{}`", t)))
                        });

                    if has_excluded_tools {
                        log::info!("[SESSION_MEMORY] Skipping session memory — memory-excluded tool was called");
                    } else if let Ok(Some(settings)) = self.db.get_active_agent_settings() {
                        if let Ok(client) = AiClient::from_settings(&settings) {
                            match context::save_session_memory(
                                &self.db,
                                &client,
                                session.id,
                                identity_id.as_deref(),
                                15, // Save last 15 messages
                            ).await {
                                Ok(()) => {
                                    log::info!("[SESSION_MEMORY] Saved session memory before reset");
                                }
                                Err(e) => {
                                    log::warn!("[SESSION_MEMORY] Failed to save session memory: {}", e);
                                }
                            }
                        }
                    }
                }

                // Reset the session
                match self.db.reset_chat_session(session.id) {
                    Ok(_) => {
                        let response = "Session reset. Let's start fresh!".to_string();
                        self.broadcaster.broadcast(GatewayEvent::agent_response(
                            message.channel_id,
                            &message.user_name,
                            &response,
                        ));
                        DispatchResult::success(response)
                    }
                    Err(e) => {
                        log::error!("Failed to reset session: {}", e);
                        DispatchResult::error(format!("Failed to reset session: {}", e))
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to get session for reset: {}", e);
                DispatchResult::error(format!("Session error: {}", e))
            }
        }
    }
}
