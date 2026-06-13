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
                    message: format!("unknown model '{}'; expected claude-haiku-4-5-20251001, claude-sonnet-4-6, claude-opus-4-8, or claude-fable-5", model.0),
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
        if matches!(def.kind.as_str(), "discipline" | "practice" | "role") {
            if def.context.as_deref().map_or(true, str::is_empty) {
                results.push(LintResult::Warn {
                    id: def.id.clone(),
                    rule: "non-empty-context".into(),
                    message: "context is empty — agent will have no domain expertise".into(),
                });
            }
        }

        results.push(LintResult::Pass(def.id.clone()));
    }

    results
}
