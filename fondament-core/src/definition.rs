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
}

impl DefinitionFile {
    pub fn effective_model(&self) -> ModelId {
        self.default_model.clone().unwrap_or_default()
    }
}
