//! Aporia contribution signal (Occitan ADR-N-005).
//!
//! A `Contribution` is one component agent's own resolved aporia output,
//! constructed only when the producing call actually ran with the aporia
//! modifier active. Publishing it to Nèrvi is the caller's responsibility
//! (this crate has no I/O); this module owns the gate, the shape, and the
//! subject/qualifier derivation so every publisher applies them identically.

use crate::address::CompositionAddress;
use crate::types::{ComposedPart, ResolvedAgent, StructuredReasoning};
use serde::{Deserialize, Serialize};

/// Fixed per ADR-N-005 — every `Contribution` declares it was resolved only
/// against the contributor's own composed parts, never reconciled against any
/// other agent. There is deliberately no other variant: a pre-resolved or
/// cross-reconciled contribution is not this signal kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionScope {
    #[serde(rename = "self")]
    SelfScope,
}

/// The `nervi.contribution.aporia` envelope, per Occitan ADR-N-005.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contribution {
    pub kind: String,
    pub contribution_id: String,
    /// `<fondament-ref>` — the composition address that produced this, e.g.
    /// `farga/project-x+aporia`.
    pub contributor: String,
    /// Names of the disciplines/stance the contributor reasoned over.
    /// Audit/transparency only — not required for consumption.
    pub composed_parts: Vec<String>,
    pub resolution_scope: ResolutionScope,
    pub content: String,
    pub produced_at: String,
    pub session_ref: Option<String>,
}

/// The signal kind string, per ADR-N-005.
pub const CONTRIBUTION_KIND: &str = "nervi.contribution.aporia";

/// The Nèrvi qualifier this signal is published under. Reuses the existing
/// `cross-project` qualifier (ADR-N-001) rather than minting a new one.
pub const CONTRIBUTION_QUALIFIER: &str = "cross-project";

impl Contribution {
    /// Construct a `Contribution` for a completed call, gated on that call
    /// having actually run the aporia pass.
    ///
    /// Returns `None` when `structured_reasoning` is `None` — this is the
    /// publish-side gate from ADR-N-005: a contribution may only be produced
    /// by a call that ran `+aporia`, never by an agent's mere existence.
    /// `project-agent.yaml`'s aporia-off-by-default posture is unaffected by
    /// this type; it only decides what a caller *may* publish once a call
    /// has already run aporia for its own reasons.
    ///
    /// Takes `structured_reasoning` directly (rather than a full
    /// `ResolvedAgent`) because dispatch call sites — e.g. Amassada's
    /// `TurnRequest`/`TurnResponse` — carry the field independently of any
    /// `ResolvedAgent` value by the time a response comes back.
    #[allow(clippy::too_many_arguments)]
    pub fn from_structured_reasoning(
        structured_reasoning: Option<&StructuredReasoning>,
        contributor: &CompositionAddress,
        composed_parts: &[ComposedPart],
        content: impl Into<String>,
        contribution_id: impl Into<String>,
        produced_at: impl Into<String>,
        session_ref: Option<String>,
    ) -> Option<Self> {
        structured_reasoning?;
        Some(Self {
            kind: CONTRIBUTION_KIND.to_string(),
            contribution_id: contribution_id.into(),
            contributor: contributor.to_string(),
            composed_parts: composed_parts.iter().map(|p| p.name.clone()).collect(),
            resolution_scope: ResolutionScope::SelfScope,
            content: content.into(),
            produced_at: produced_at.into(),
            session_ref,
        })
    }

    /// Convenience wrapper over [`Contribution::from_structured_reasoning`]
    /// for callers that already hold a full `ResolvedAgent`.
    #[allow(clippy::too_many_arguments)]
    pub fn from_resolved_agent(
        agent: &ResolvedAgent,
        contributor: &CompositionAddress,
        composed_parts: &[ComposedPart],
        content: impl Into<String>,
        contribution_id: impl Into<String>,
        produced_at: impl Into<String>,
        session_ref: Option<String>,
    ) -> Option<Self> {
        Self::from_structured_reasoning(
            agent.structured_reasoning.as_ref(),
            contributor,
            composed_parts,
            content,
            contribution_id,
            produced_at,
            session_ref,
        )
    }

    /// The Nèrvi subject this contribution publishes to, per ADR-N-005:
    /// `occitan.contribution.<component>`. `<component>` is the contributor's
    /// leading path segment — `role` for a `Role` address, `project` for a
    /// `Composed` address — with no facet, modifiers, or stance.
    pub fn subject(contributor: &CompositionAddress) -> String {
        format!("occitan.contribution.{}", contributor.component_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ReasoningIntensity, StructuredReasoning};

    fn aporia_agent() -> ResolvedAgent {
        ResolvedAgent {
            system_prompt: "irrelevant".into(),
            tools: vec![],
            jit_tools: vec![],
            default_model: Default::default(),
            structured_reasoning: Some(StructuredReasoning {
                intensity: ReasoningIntensity::High,
            }),
        }
    }

    fn non_aporia_agent() -> ResolvedAgent {
        ResolvedAgent {
            system_prompt: "irrelevant".into(),
            tools: vec![],
            jit_tools: vec![],
            default_model: Default::default(),
            structured_reasoning: None,
        }
    }

    fn addr() -> CompositionAddress {
        "farga/project-x+aporia".parse().unwrap()
    }

    #[test]
    fn gate_blocks_contribution_without_aporia() {
        let agent = non_aporia_agent();
        let result = Contribution::from_resolved_agent(
            &agent,
            &addr(),
            &[],
            "some resolved position",
            "contrib-1",
            "2026-07-01T00:00:00Z",
            None,
        );
        assert!(
            result.is_none(),
            "a call that did not run aporia must not produce a contribution"
        );
    }

    #[test]
    fn gate_allows_contribution_with_aporia() {
        let agent = aporia_agent();
        let result = Contribution::from_resolved_agent(
            &agent,
            &addr(),
            &[],
            "some resolved position",
            "contrib-1",
            "2026-07-01T00:00:00Z",
            None,
        );
        assert!(result.is_some());
    }

    #[test]
    fn resolution_scope_is_always_self() {
        let agent = aporia_agent();
        let c = Contribution::from_resolved_agent(
            &agent, &addr(), &[], "x", "id", "ts", None,
        )
        .unwrap();
        assert_eq!(c.resolution_scope, ResolutionScope::SelfScope);
    }

    #[test]
    fn kind_and_qualifier_match_adr_n_005() {
        let agent = aporia_agent();
        let c = Contribution::from_resolved_agent(
            &agent, &addr(), &[], "x", "id", "ts", None,
        )
        .unwrap();
        assert_eq!(c.kind, "nervi.contribution.aporia");
        assert_eq!(CONTRIBUTION_QUALIFIER, "cross-project");
    }

    #[test]
    fn composed_parts_are_carried_by_name_only() {
        let agent = aporia_agent();
        let parts = vec![
            ComposedPart {
                kind: crate::types::PartKind::Discipline,
                name: "security-sre".into(),
                weight: 0.0,
                corpus_ref: None,
            },
            ComposedPart {
                kind: crate::types::PartKind::Stance,
                name: "adversarial".into(),
                weight: 0.0,
                corpus_ref: None,
            },
        ];
        let c = Contribution::from_resolved_agent(
            &agent, &addr(), &parts, "x", "id", "ts", None,
        )
        .unwrap();
        assert_eq!(c.composed_parts, vec!["security-sre", "adversarial"]);
    }

    #[test]
    fn subject_uses_leading_component_only() {
        let composed: CompositionAddress = "farga/project-x+aporia".parse().unwrap();
        assert_eq!(Contribution::subject(&composed), "occitan.contribution.farga");

        let role: CompositionAddress = "guilhem+aporia".parse().unwrap();
        assert_eq!(Contribution::subject(&role), "occitan.contribution.guilhem");
    }

    #[test]
    fn serializes_resolution_scope_as_self_string() {
        let agent = aporia_agent();
        let c = Contribution::from_resolved_agent(
            &agent, &addr(), &[], "x", "id", "ts", None,
        )
        .unwrap();
        let json = serde_json::to_value(&c).unwrap();
        assert_eq!(json["resolution_scope"], "self");
    }
}
