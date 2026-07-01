use std::collections::HashMap;
use std::path::Path;
use crate::definition::DefinitionFile;
use crate::error::{FondamentError, Result};

#[derive(Debug, Default, Clone)]
pub struct DefinitionTree {
    definitions: HashMap<String, DefinitionFile>,
}

impl DefinitionTree {
    pub fn load(root: &Path) -> Result<Self> {
        let mut tree = Self::default();
        tree.load_dir(root)?;
        Ok(tree)
    }

    fn load_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() { return Ok(()); }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.load_dir(&path)?;
            } else if path.extension().map_or(false, |e| e == "yaml") {
                let content = std::fs::read_to_string(&path)?;
                validate_tool_schema(&content, &path)?;
                let def: DefinitionFile = serde_yaml::from_str(&content)?;
                self.definitions.insert(def.id.clone(), def);
            }
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&DefinitionFile> {
        self.definitions.get(id)
    }

    pub fn all(&self) -> impl Iterator<Item = &DefinitionFile> {
        self.definitions.values()
    }

    pub fn reload_file(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        validate_tool_schema(&content, path)?;
        let def: DefinitionFile = serde_yaml::from_str(&content)?;
        self.definitions.insert(def.id.clone(), def);
        Ok(())
    }
}

/// Pre-parse schema validator for definition files.
///
/// Runs against the raw YAML before serde deserialisation. Checks that all entries
/// in `tools.always_on` and `tools.jit` are YAML mappings (the canonical Form B),
/// not plain strings (the deprecated Form A).
///
/// ## Why pre-parse?
///
/// The `DefinitionTree` only holds successfully-parsed `DefinitionFile` values — by
/// the time the fast lint runs, any Form-A entry would already have caused a cryptic
/// serde type error. This validator intercepts that case first and emits a
/// `FondamentError::Schema` with a clear, actionable message pointing at the exact
/// field and index that violated the schema.
///
/// ## Canonical form (Form B)
///
/// ```yaml
/// tools:
///   always_on:
///     - id: farga-write-signal
///       kind: mcp
///       server: farga
///       tool: write_signal
/// ```
///
/// Form A (flat string list) is **not supported**:
///
/// ```yaml
/// tools:
///   always_on:
///     - mcp__farga__write_signal   # ← rejected
/// ```
fn validate_tool_schema(content: &str, path: &Path) -> Result<()> {
    let raw: serde_yaml::Value = serde_yaml::from_str(content)?;

    let Some(tools) = raw.get("tools") else { return Ok(()); };

    for field in &["always_on", "jit"] {
        let Some(entries) = tools.get(*field) else { continue; };
        let Some(list) = entries.as_sequence() else { continue; };

        for (i, entry) in list.iter().enumerate() {
            if entry.is_string() {
                return Err(FondamentError::Schema(
                    path.display().to_string(),
                    format!(
                        "tools.{}[{}] is a plain string {:?} — use the structured object form: \
                         `- id: <id>\\n  kind: mcp|api|native\\n  server: <server>\\n  tool: <tool>`. \
                         See definitions/fondament/guilhem.yaml for the canonical schema.",
                        field,
                        i,
                        entry.as_str().unwrap_or("?"),
                    ),
                ));
            }
        }
    }

    Ok(())
}
