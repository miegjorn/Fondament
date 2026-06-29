use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub id: String,
    pub kind: ToolKind,
    pub server: Option<String>,
    pub tool: Option<String>,
    pub handler: Option<String>,
    #[serde(default)]
    pub constraints: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolKind {
    Mcp,
    Api,
    Native,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolSet {
    #[serde(default)]
    pub always_on: Vec<ToolDefinition>,
    #[serde(default)]
    pub jit: Vec<ToolDefinition>,
}

#[derive(Debug, Default)]
pub struct ToolRegistry {
    tools: std::collections::HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    pub fn register(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.id.clone(), tool);
    }

    pub fn get(&self, id: &str) -> Option<&ToolDefinition> {
        self.tools.get(id)
    }
}
