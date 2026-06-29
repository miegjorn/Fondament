use serde::{Deserialize, Serialize};
use crate::tools::ToolSet;
use crate::types::ModelId;

/// A skill reference — either a plain string id or a versioned object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SkillRef {
    Simple(String),
    Versioned { id: String, version: String },
}

impl SkillRef {
    pub fn id(&self) -> &str {
        match self {
            Self::Simple(s) => s,
            Self::Versioned { id, .. } => id,
        }
    }

    pub fn version(&self) -> &str {
        match self {
            Self::Simple(_) => "latest",
            Self::Versioned { version, .. } => version,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionFile {
    pub id: String,
    #[serde(default)]
    pub version: Option<String>,
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
    #[serde(default)]
    pub skills: Vec<SkillRef>,
}

impl DefinitionFile {
    pub fn effective_model(&self) -> ModelId {
        self.default_model.clone().unwrap_or_default()
    }
}
