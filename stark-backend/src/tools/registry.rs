use crate::ai::multi_agent::types::AgentSubtype;
use crate::tools::types::{ToolConfig, ToolContext, ToolDefinition, ToolGroup, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait that all tools must implement
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the tool definition for the AI API
    fn definition(&self) -> ToolDefinition;

    /// Executes the tool with the given parameters
    async fn execute(&self, params: Value, context: &ToolContext) -> ToolResult;

    /// Returns the tool's name
    fn name(&self) -> String {
        self.definition().name.clone()
    }

    /// Returns the tool's group for access control
    fn group(&self) -> ToolGroup {
        self.definition().group
    }
}

/// Registry that holds all available tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    default_config: ToolConfig,
}

impl ToolRegistry {
    pub fn new() -> Self {
        ToolRegistry {
            tools: HashMap::new(),
            default_config: ToolConfig::default(),
        }
    }

    pub fn with_config(config: ToolConfig) -> Self {
        ToolRegistry {
            tools: HashMap::new(),
            default_config: config,
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.definition().name.clone();
        self.tools.insert(name, tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// List all registered tools
    pub fn list(&self) -> Vec<&Arc<dyn Tool>> {
        self.tools.values().collect()
    }

    /// Get tools that are allowed by a configuration
    pub fn get_allowed_tools(&self, config: &ToolConfig) -> Vec<Arc<dyn Tool>> {
        self.tools
            .values()
            .filter(|tool| config.is_tool_allowed(&tool.definition().name, tool.group()))
            .cloned()
            .collect()
    }

    /// Get tools that are allowed for a specific agent subtype
    /// System tools are always included regardless of subtype
    pub fn get_allowed_tools_for_subtype(
        &self,
        config: &ToolConfig,
        subtype: AgentSubtype,
    ) -> Vec<Arc<dyn Tool>> {
        let allowed_groups = subtype.allowed_tool_groups();
        self.tools
            .values()
            .filter(|tool| {
                let group = tool.group();
                // System tools are always available
                let group_allowed =
                    group == ToolGroup::System || allowed_groups.contains(&group);
                // Also check against the tool config
                group_allowed && config.is_tool_allowed(&tool.definition().name, group)
            })
            .cloned()
            .collect()
    }

    /// Get tool definitions for a specific agent subtype
    pub fn get_tool_definitions_for_subtype(
        &self,
        config: &ToolConfig,
        subtype: AgentSubtype,
    ) -> Vec<ToolDefinition> {
        self.get_allowed_tools_for_subtype(config, subtype)
            .iter()
            .map(|tool| tool.definition())
            .collect()
    }

    /// Get tool definitions for a specific agent subtype, with additional required tools
    /// that are force-included regardless of config/profile restrictions.
    /// Used when a skill is activated that requires specific tools.
    pub fn get_tool_definitions_for_subtype_with_required(
        &self,
        config: &ToolConfig,
        subtype: AgentSubtype,
        required_tools: &[String],
    ) -> Vec<ToolDefinition> {
        // Start with the normal subtype-allowed tools
        let mut tools = self.get_allowed_tools_for_subtype(config, subtype);
        let mut tool_names: std::collections::HashSet<String> =
            tools.iter().map(|t| t.definition().name.clone()).collect();

        // Force-include required tools even if they're not normally allowed
        for tool_name in required_tools {
            if !tool_names.contains(tool_name) {
                if let Some(tool) = self.get(tool_name) {
                    log::info!(
                        "[REGISTRY] Force-including required tool '{}' for active skill",
                        tool_name
                    );
                    tools.push(tool);
                    tool_names.insert(tool_name.clone());
                } else {
                    log::warn!(
                        "[REGISTRY] Required tool '{}' not found in registry",
                        tool_name
                    );
                }
            }
        }

        tools.iter().map(|tool| tool.definition()).collect()
    }

    /// Get tool definitions for allowed tools (for sending to AI)
    pub fn get_tool_definitions(&self, config: &ToolConfig) -> Vec<ToolDefinition> {
        self.get_allowed_tools(config)
            .iter()
            .map(|tool| tool.definition())
            .collect()
    }

    /// Get tool definitions using default config
    pub fn get_default_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.get_tool_definitions(&self.default_config)
    }

    /// Execute a tool by name
    pub async fn execute(
        &self,
        name: &str,
        params: Value,
        context: &ToolContext,
        config: Option<&ToolConfig>,
    ) -> ToolResult {
        let effective_config = config.unwrap_or(&self.default_config);

        // Get the tool
        let tool = match self.get(name) {
            Some(t) => t,
            None => return ToolResult::error(format!("Tool '{}' not found", name)),
        };

        // Check if tool is allowed
        if !effective_config.is_tool_allowed(name, tool.group()) {
            return ToolResult::error(format!("Tool '{}' is not allowed", name));
        }

        // Execute the tool
        tool.execute(params, context).await
    }

    /// Get default configuration
    pub fn default_config(&self) -> &ToolConfig {
        &self.default_config
    }

    /// Set default configuration
    pub fn set_default_config(&mut self, config: ToolConfig) {
        self.default_config = config;
    }

    /// Check if a tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get count of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::types::{PropertySchema, ToolInputSchema};

    struct MockTool {
        definition: ToolDefinition,
    }

    impl MockTool {
        fn new(name: &str, group: ToolGroup) -> Self {
            MockTool {
                definition: ToolDefinition {
                    name: name.to_string(),
                    description: format!("Mock {} tool", name),
                    input_schema: ToolInputSchema::default(),
                    group,
                },
            }
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn definition(&self) -> ToolDefinition {
            self.definition.clone()
        }

        async fn execute(&self, _params: Value, _context: &ToolContext) -> ToolResult {
            ToolResult::success("mock result")
        }
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(MockTool::new("test_tool", ToolGroup::Web));
        registry.register(tool);

        assert!(registry.has_tool("test_tool"));
        assert!(!registry.has_tool("nonexistent"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_tool_config_allows() {
        let config = ToolConfig {
            profile: crate::tools::types::ToolProfile::Standard,
            ..Default::default()
        };

        // Web, filesystem, and exec are allowed in Standard profile
        assert!(config.is_tool_allowed("web_fetch", ToolGroup::Web));
        assert!(config.is_tool_allowed("read_file", ToolGroup::Filesystem));
        assert!(config.is_tool_allowed("exec", ToolGroup::Exec));
        // Messaging is not allowed in Standard profile
        assert!(!config.is_tool_allowed("send_message", ToolGroup::Messaging));
    }

    #[test]
    fn test_tool_config_deny_list() {
        let config = ToolConfig {
            profile: crate::tools::types::ToolProfile::Full,
            deny_list: vec!["dangerous_tool".to_string()],
            ..Default::default()
        };

        // Denied tool should be blocked even with Full profile
        assert!(!config.is_tool_allowed("dangerous_tool", ToolGroup::System));
        // Other tools should be allowed
        assert!(config.is_tool_allowed("safe_tool", ToolGroup::System));
    }
}
