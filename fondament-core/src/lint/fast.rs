use crate::tree::DefinitionTree;

#[derive(Debug)]
pub enum LintResult {
    Pass(String),
    Fail { id: String, rule: String, message: String },
    Warn { id: String, rule: String, message: String },
}

pub fn run_fast(tree: &DefinitionTree) -> Vec<LintResult> {
    let mut results = Vec::new();

    for def in tree.all() {
        // Rule: model ID must be a known Claude model
        if let Some(model) = &def.default_model {
            if !model.is_valid() {
                results.push(LintResult::Fail {
                    id: def.id.clone(),
                    rule: "valid-model-id".into(),
                    message: format!("unknown model '{}'; expected a known claude-* or grok* model (for multi-provider support)", model.0),
                });
                continue;
            }
        }

        // Rule: extends must reference existing IDs
        for parent in &def.extends {
            if tree.get(parent).is_none() {
                results.push(LintResult::Fail {
                    id: def.id.clone(),
                    rule: "extends-exists".into(),
                    message: format!("extends '{}' not found in tree", parent),
                });
            }
        }

        // Rule: context should not be empty for discipline/practice/role
        if matches!(def.kind.as_str(), "discipline" | "practice" | "role") && !def.modifier {
            if def.context.as_deref().map_or(true, str::is_empty) {
                results.push(LintResult::Warn {
                    id: def.id.clone(),
                    rule: "non-empty-context".into(),
                    message: "context is empty — agent will have no domain expertise".into(),
                });
            }
        }

        // Rules for project-composition kind
        if def.kind == "project-composition" {
            if def.name.as_deref().map_or(true, str::is_empty) {
                results.push(LintResult::Fail {
                    id: def.id.clone(),
                    rule: "project-name-present".into(),
                    message: "project-composition must have a non-empty 'name' field".into(),
                });
                continue;
            }

            if def.parts.is_empty() {
                results.push(LintResult::Fail {
                    id: def.id.clone(),
                    rule: "project-parts-present".into(),
                    message: "project-composition must have at least one entry in 'parts'".into(),
                });
                continue;
            }

            if def.model.as_deref().map_or(true, str::is_empty) {
                results.push(LintResult::Fail {
                    id: def.id.clone(),
                    rule: "project-model-present".into(),
                    message: "project-composition must have a non-empty 'model' field".into(),
                });
                continue;
            }

            // Validate each farga-sourced part has a project field
            let mut farga_ok = true;
            for part in &def.parts {
                let source = part.get("source").and_then(|v| v.as_str());
                if source == Some("farga") {
                    let project = part.get("project").and_then(|v| v.as_str());
                    if project.map_or(true, str::is_empty) {
                        results.push(LintResult::Fail {
                            id: def.id.clone(),
                            rule: "farga-project-set".into(),
                            message: "part with source: farga must have a non-empty 'project' field".into(),
                        });
                        farga_ok = false;
                        break;
                    }
                }
            }
            if !farga_ok { continue; }
        }

        results.push(LintResult::Pass(def.id.clone()));
    }

    results
}
