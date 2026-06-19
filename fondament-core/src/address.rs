use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use crate::error::{FondamentError, Result};

/// Discipline names that act as reasoning modifiers rather than domain/corpus identifiers.
/// These are stripped out of stance position during parsing.
pub const KNOWN_MODIFIER_DISCIPLINES: &[&str] = &["deconstructive"];

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompositionAddress {
    Role {
        role: String,
        modifiers: Vec<String>,
        stance_override: Option<String>,
    },
    Composed {
        project: String,
        facet: Option<String>,
        modifiers: Vec<String>,
        stance: String,
    },
}

impl FromStr for CompositionAddress {
    type Err = FondamentError;

    fn from_str(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Err(FondamentError::AddressParse("empty".into()));
        }

        let parts: Vec<&str> = s.split('+').collect();
        let path = parts[0];
        let qualifiers = &parts[1..];

        if path.is_empty() {
            return Err(FondamentError::AddressParse("empty path in address".into()));
        }

        let mut modifiers: Vec<String> = Vec::new();
        let mut stance: Option<String> = None;

        for q in qualifiers {
            if q.is_empty() {
                return Err(FondamentError::AddressParse(
                    format!("empty segment in address: {}", s)
                ));
            }
            if KNOWN_MODIFIER_DISCIPLINES.contains(q) {
                modifiers.push(q.to_string());
            } else if stance.is_some() {
                return Err(FondamentError::AddressParse(
                    format!("multiple stances in address: {}", s)
                ));
            } else {
                stance = Some(q.to_string());
            }
        }

        if path.starts_with("fondament/") || stance.is_none() {
            Ok(CompositionAddress::Role {
                role: path.to_string(),
                modifiers,
                stance_override: stance,
            })
        } else {
            let stance = stance.unwrap();
            let (project, facet) = path.split_once('/')
                .map(|(p, f)| (p.to_string(), Some(f.to_string())))
                .unwrap_or((path.to_string(), None));
            Ok(CompositionAddress::Composed { project, facet, modifiers, stance })
        }
    }
}

impl fmt::Display for CompositionAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompositionAddress::Role { role, modifiers, stance_override } => {
                write!(f, "{}", role)?;
                for m in modifiers { write!(f, "+{}", m)?; }
                if let Some(s) = stance_override { write!(f, "+{}", s)?; }
                Ok(())
            }
            CompositionAddress::Composed { project, facet, modifiers, stance } => {
                write!(f, "{}", project)?;
                if let Some(fa) = facet { write!(f, "/{}", fa)?; }
                for m in modifiers { write!(f, "+{}", m)?; }
                write!(f, "+{}", stance)
            }
        }
    }
}
