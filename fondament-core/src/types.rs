use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelId(pub String);

impl ModelId {
    pub fn is_valid(&self) -> bool {
        matches!(self.0.as_str(),
            "claude-haiku-4-5-20251001" | "claude-sonnet-4-6" |
            "claude-opus-4-8" | "claude-fable-5"
        )
    }
}

impl Default for ModelId {
    fn default() -> Self { Self("claude-sonnet-4-6".into()) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedAgent {
    pub system_prompt: String,
    pub tools: Vec<crate::tools::ToolDefinition>,
    pub jit_tools: Vec<crate::tools::ToolDefinition>,
    pub default_model: ModelId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerKind {
    Org,
    Initiative,
    Project,
    Discipline,
    Practice,
    Role,
    Stance,
    Facet,
}
