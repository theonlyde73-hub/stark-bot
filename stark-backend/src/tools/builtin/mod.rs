//! Built-in tools for the agent
//!
//! Tools are organized into submodules by category:
//! - `bash`: Shell operations and filesystem tools (grep, glob, exec, git, file ops)
//! - `code`: Development tools (committer, deploy, pr_quality)
//! - `core`: Essential agent tools (ask_user, subagent, task management)
//! - `cryptocurrency`: Web3, x402, and blockchain tools
//! - `social_media`: Platform integrations (Twitter, Discord, GitHub)

// Submodules
pub mod bash;
pub mod code;
pub mod core;
pub mod cryptocurrency;
pub mod social_media;

// Individual tools (remaining uncategorized)
mod memory_get;
mod memory_store;
mod multi_memory_search;
mod process_status;
mod web_fetch;

// Re-exports from submodules
pub use bash::{
    ApplyPatchTool, DeleteFileTool, EditFileTool, ExecTool, GitTool, GlobTool, GrepTool,
    ListFilesTool, ReadFileTool, RenameFileTool, WriteFileTool,
};
pub use code::{CommitterTool, DeployTool, PrQualityTool};
pub use core::{
    AgentSendTool, ApiKeysCheckTool, AskUserTool, ManageSkillsTool, ModifySoulTool, SayToUserTool,
    SetAgentSubtypeTool, SubagentStatusTool, SubagentTool, TaskFullyCompletedTool,
};
pub use cryptocurrency::{
    load_networks, load_tokens, BridgeUsdcTool, BroadcastWeb3TxTool, DecodeCalldataTool,
    DexScreenerTool, ListQueuedWeb3TxTool, PolymarketTradeTool, RegisterSetTool,
    SelectWeb3NetworkTool, SendEthTool, ToRawAmountTool, TokenLookupTool, Web3FunctionCallTool,
    X402AgentInvokeTool, X402FetchTool, X402PostTool, X402RpcTool,
};
pub use social_media::{DiscordLookupTool, GithubUserTool, TwitterPostTool};

// Re-exports from individual tools
pub use memory_get::MemoryGetTool;
pub use memory_store::MemoryStoreTool;
pub use multi_memory_search::MultiMemorySearchTool;
pub use process_status::ProcessStatusTool;
pub use web_fetch::WebFetchTool;
