use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// Rules that govern dispatcher and prompt constraints for a skill.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillRules {
    #[serde(default)]
    pub dispatcher: Option<DispatcherSkillRules>,
    pub prompt_constraint: Option<String>,
}

/// Dispatcher-level rules embedded in a skill definition.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DispatcherSkillRules {
    pub invoke_agent: Option<InvokeAgentSkillRules>,
}

/// Fine-grained constraints on `invoke_agent` calls.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InvokeAgentSkillRules {
    pub caller_identity: Option<String>,
    pub allowed_facets: Option<Vec<String>>,
    #[serde(default)]
    pub domain_must_match_caller: bool,
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
    /// Per-trigger model assignments (e.g. chronicle, matrix, dream, dispatch).
    #[serde(default)]
    pub models: HashMap<String, String>,
    /// Declarative rules injected from skill definitions (dispatcher scope, prompt constraints).
    #[serde(default)]
    pub rules: Option<SkillRules>,
}

impl DefinitionFile {
    pub fn effective_model(&self) -> ModelId {
        self.default_model.clone().unwrap_or_default()
    }
}
