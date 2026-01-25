use serde::{Deserialize, Serialize};

/// Normalized message from any channel (Telegram, Slack, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedMessage {
    /// Channel database ID
    pub channel_id: i64,
    /// Channel type (telegram, slack)
    pub channel_type: String,
    /// Platform-specific chat/conversation ID
    pub chat_id: String,
    /// Platform-specific user ID
    pub user_id: String,
    /// Display name of the user
    pub user_name: String,
    /// Message text content
    pub text: String,
    /// Platform-specific message ID (for replies)
    pub message_id: Option<String>,
}

/// Handle to a running channel listener
pub struct ChannelHandle {
    pub channel_id: i64,
    pub channel_type: String,
    pub name: String,
    pub shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl ChannelHandle {
    pub fn new(
        channel_id: i64,
        channel_type: String,
        name: String,
        shutdown_tx: tokio::sync::oneshot::Sender<()>,
    ) -> Self {
        Self {
            channel_id,
            channel_type,
            name,
            shutdown_tx,
        }
    }
}

/// Result of dispatching a message to the AI
#[derive(Debug, Clone)]
pub struct DispatchResult {
    pub response: String,
    pub error: Option<String>,
}

impl DispatchResult {
    pub fn success(response: String) -> Self {
        Self {
            response,
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            response: String::new(),
            error: Some(error),
        }
    }
}
