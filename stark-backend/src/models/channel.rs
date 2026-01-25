use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelType {
    Telegram,
    Slack,
}

impl ChannelType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChannelType::Telegram => "telegram",
            ChannelType::Slack => "slack",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "telegram" => Some(ChannelType::Telegram),
            "slack" => Some(ChannelType::Slack),
            _ => None,
        }
    }
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: i64,
    pub channel_type: String,
    pub name: String,
    pub enabled: bool,
    #[serde(skip_serializing)]
    pub bot_token: String,
    #[serde(skip_serializing)]
    pub app_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Channel {
    pub fn channel_type_enum(&self) -> Option<ChannelType> {
        ChannelType::from_str(&self.channel_type)
    }
}

/// Response type for channel API endpoints (hides sensitive tokens)
#[derive(Debug, Clone, Serialize)]
pub struct ChannelResponse {
    pub id: i64,
    pub channel_type: String,
    pub name: String,
    pub enabled: bool,
    pub has_bot_token: bool,
    pub has_app_token: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running: Option<bool>,
}

impl From<Channel> for ChannelResponse {
    fn from(channel: Channel) -> Self {
        Self {
            id: channel.id,
            channel_type: channel.channel_type,
            name: channel.name,
            enabled: channel.enabled,
            has_bot_token: !channel.bot_token.is_empty(),
            has_app_token: channel.app_token.is_some(),
            created_at: channel.created_at,
            updated_at: channel.updated_at,
            running: None,
        }
    }
}

impl ChannelResponse {
    pub fn with_running(mut self, running: bool) -> Self {
        self.running = Some(running);
        self
    }
}

/// Request type for creating a channel
#[derive(Debug, Clone, Deserialize)]
pub struct CreateChannelRequest {
    pub channel_type: String,
    pub name: String,
    pub bot_token: String,
    pub app_token: Option<String>,
}

/// Request type for updating a channel
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub bot_token: Option<String>,
    pub app_token: Option<String>,
}
