use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use crate::error::{FondamentError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompositionAddress {
    Role {
        role: String,
        stance_override: Option<String>,
    },
    Composed {
        project: String,
        facet: Option<String>,
        stance: String,
    },
}

impl FromStr for CompositionAddress {
    type Err = FondamentError;

    fn from_str(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Err(FondamentError::AddressParse("empty".into()));
        }
        let (path, stance) = s.split_once('+').map(|(p, st)| (p, Some(st))).unwrap_or((s, None));

        if path.starts_with("fondament/") || stance.is_none() {
            Ok(CompositionAddress::Role {
                role: path.to_string(),
                stance_override: stance.map(str::to_string),
            })
        } else {
            let stance = stance.unwrap();
            let (project, facet) = path.split_once('/')
                .map(|(p, f)| (p.to_string(), Some(f.to_string())))
                .unwrap_or((path.to_string(), None));
            Ok(CompositionAddress::Composed { project, facet, stance: stance.to_string() })
        }
    }
}

impl fmt::Display for CompositionAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompositionAddress::Role { role, stance_override } => {
                write!(f, "{}", role)?;
                if let Some(s) = stance_override { write!(f, "+{}", s)?; }
                Ok(())
            }
            CompositionAddress::Composed { project, facet, stance } => {
                write!(f, "{}", project)?;
                if let Some(fa) = facet { write!(f, "/{}", fa)?; }
                write!(f, "+{}", stance)
            }
        }
    }
}
