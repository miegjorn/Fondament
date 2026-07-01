use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum PartKind {
    Domain,
    Discipline,
    Stance,
    SessionNode,
}

#[derive(Debug, Clone)]
pub struct ComposedPart {
    pub kind:       PartKind,
    pub name:       String,
    pub weight:     f32,
    pub corpus_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelId(pub String);

impl ModelId {
    /// Parse a model string into a provider + model for future multi-backend use.
    /// Currently keeps full backward compat for all existing Claude strings.
    pub fn as_spec(&self) -> (String, String) {
        let s = &self.0;
        if s.starts_with("grok") || s.starts_with("xai:") {
            ("grok".to_string(), s.trim_start_matches("xai:").to_string())
        } else if s.starts_with("gpt-") || s.starts_with("o1") || s.starts_with("o3") || s.starts_with("o4") || s.starts_with("openai:") {
            ("openai".to_string(), s.trim_start_matches("openai:").to_string())
        } else if s.starts_with("qwen-") || s.starts_with("qwq-") {
            ("qwen".to_string(), s.to_string())
        } else if s.contains("claude") || s.starts_with("anthropic:") {
            ("anthropic".to_string(), s.trim_start_matches("anthropic:").to_string())
        } else if let Some(idx) = s.find(':') {
            (s[..idx].to_string(), s[idx+1..].to_string())
        } else {
            ("unknown".to_string(), s.to_string())
        }
    }

    /// Accepts all known model prefixes across supported providers.
    /// Generic <provider>:<model> format accepted for any future provider.
    pub fn is_valid(&self) -> bool {
        let s = self.0.as_str();
        matches!(s,
            "claude-haiku-4-5-20251001" | "claude-sonnet-4-6" |
            "claude-opus-4-8" | "claude-fable-5"
        ) || s.starts_with("grok") || s.starts_with("xai:")
          || s.starts_with("gpt-") || s.starts_with("o1") || s.starts_with("o3") || s.starts_with("o4") || s.starts_with("openai:")
          || s.starts_with("qwen-") || s.starts_with("qwq-")
          || s.contains(':')
    }
}

impl Default for ModelId {
    fn default() -> Self { Self("claude-sonnet-4-6".into()) }
}

/// Forward scaffolding for multi-provider routing.
///
/// Currently routing decisions are made via the raw model string (e.g. "grok-3",
/// "claude-sonnet-4-6") plus the optional `endpoint` field on participants
/// (see Amassada's round.rs and dispatch.rs). ModelProvider / ModelSpec are
/// intentionally unwired for now — they exist so that richer provider-specific
/// logic (auth, thinking budget handling, native tool calling, etc.) can be
/// added later without changing the public ModelId shape.
///
/// as_spec() on ModelId already does a minimal grok vs anthropic split for
/// future use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ModelProvider {
    #[default]
    Anthropic,
    Grok,
    OpenAI,
    Qwen,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSpec {
    pub provider: ModelProvider,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedAgent {
    pub system_prompt: String,
    pub tools: Vec<crate::tools::ToolDefinition>,
    pub jit_tools: Vec<crate::tools::ToolDefinition>,
    pub default_model: ModelId,
    /// Set when the deconstructive modifier is active.
    /// Callers must pass this to the Anthropic API as thinking.budget_tokens.
    pub thinking_budget: Option<u32>,
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
