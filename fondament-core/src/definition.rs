use serde::{Deserialize, Serialize};
use crate::tools::ToolSet;
use crate::types::ModelId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionFile {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub extends: Vec<String>,
    pub default_model: Option<ModelId>,
    pub context: Option<String>,
    #[serde(default)]
    pub tools: ToolSet,
    pub stance: Option<String>,
    pub cognitive_load: Option<String>,
    #[serde(default)]
    pub modifier: bool,
    #[serde(default)]
    pub component: Option<String>,
    // project-composition fields — only present when kind == "project-composition"
    pub name: Option<String>,
    pub description: Option<String>,
    /// Composition model (distinct from default_model). Validated for project-composition kind.
    pub model: Option<String>,
    #[serde(default)]
    pub parts: Vec<serde_yaml::Value>,
}

impl DefinitionFile {
    pub fn effective_model(&self) -> ModelId {
        self.default_model.clone().unwrap_or_default()
    }
}
