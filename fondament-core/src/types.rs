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
        } else if s.starts_with("gemini-") || s.starts_with("google:") {
            ("gemini".to_string(), s.trim_start_matches("google:").to_string())
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
          || s.starts_with("gemini-") || s.starts_with("google:")
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
    Gemini,
    Qwen,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSpec {
    pub provider: ModelProvider,
    pub model: String,
}

/// Provider-agnostic signal that an agent should engage extended reasoning.
/// Callers request a tier; dispatch translates to provider-specific params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReasoningIntensity { Low, Medium, High }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredReasoning {
    pub intensity: ReasoningIntensity,
}

impl StructuredReasoning {
    /// Derive intensity from number of composed parts (aporia path).
    pub fn from_parts_count(n: usize) -> Self {
        let intensity = match n {
            0..=1 => ReasoningIntensity::Low,
            2..=3 => ReasoningIntensity::Medium,
            _ => ReasoningIntensity::High,
        };
        Self { intensity }
    }

    /// Convert a raw token budget (from canvas YAML) to the nearest intensity tier.
    pub fn from_budget(budget: u32) -> Self {
        let intensity = if budget <= 3_000 {
            ReasoningIntensity::Low
        } else if budget <= 6_000 {
            ReasoningIntensity::Medium
        } else {
            ReasoningIntensity::High
        };
        Self { intensity }
    }

    /// Translate to Anthropic thinking_budget_tokens.
    pub fn anthropic_budget(&self) -> u32 {
        match self.intensity {
            ReasoningIntensity::Low    => 3_000,
            ReasoningIntensity::Medium => 6_000,
            ReasoningIntensity::High   => 10_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedAgent {
    pub system_prompt: String,
    pub tools: Vec<crate::tools::ToolDefinition>,
    pub jit_tools: Vec<crate::tools::ToolDefinition>,
    pub default_model: ModelId,
    /// Set when the aporia modifier is active. Dispatch translates
    /// this to provider-specific reasoning params (Anthropic: budget_tokens;
    /// Gemini/OpenAI-o: gracefully dropped — they reason natively).
    pub structured_reasoning: Option<StructuredReasoning>,
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

#[cfg(test)]
mod aporia_tax_tests {
    use super::*;

    #[test]
    fn parts_count_tiering_boundaries() {
        // 0-1 parts -> Low, 2-3 -> Medium, 4+ -> High.
        // This tiering is the entire input to the aporia cost model: it decides
        // how many extra thinking/reasoning tokens a call spends before the
        // Anthropic pricing multiplier is even applied.
        let cases = [
            (0usize, ReasoningIntensity::Low),
            (1, ReasoningIntensity::Low),
            (2, ReasoningIntensity::Medium),
            (3, ReasoningIntensity::Medium),
            (4, ReasoningIntensity::High),
            (5, ReasoningIntensity::High),
            (10, ReasoningIntensity::High),
        ];
        for (n, expected) in cases {
            assert_eq!(
                StructuredReasoning::from_parts_count(n).intensity,
                expected,
                "parts_count={n} should map to {expected:?}"
            );
        }
    }

    #[test]
    fn anthropic_budget_values_match_documented_tax() {
        assert_eq!(StructuredReasoning::from_parts_count(1).anthropic_budget(), 3_000);
        assert_eq!(StructuredReasoning::from_parts_count(3).anthropic_budget(), 6_000);
        assert_eq!(StructuredReasoning::from_parts_count(4).anthropic_budget(), 10_000);
    }

    #[test]
    fn from_budget_is_consistent_with_anthropic_budget_round_trip() {
        // Any budget derived from from_parts_count must round-trip through
        // from_budget to the same intensity tier, or the two constructors
        // have drifted apart.
        for n in 0..=6 {
            let via_parts = StructuredReasoning::from_parts_count(n);
            let via_budget = StructuredReasoning::from_budget(via_parts.anthropic_budget());
            assert_eq!(
                via_parts.intensity, via_budget.intensity,
                "from_parts_count({n}) and from_budget round-trip diverged"
            );
        }
    }
}
