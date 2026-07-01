use fondament_core::lint::fast::{LintResult, run_fast};
use fondament_core::tree::DefinitionTree;
use tempfile::TempDir;

fn write_def(dir: &TempDir, path: &str, content: &str) {
    let full = dir.path().join(path);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(full, content).unwrap();
}

#[test]
fn valid_definition_passes_lint() {
    let dir = TempDir::new().unwrap();
    write_def(&dir, "disciplines/valid.yaml", r#"
id: disciplines/valid
kind: discipline
default_model: claude-sonnet-4-6
context: "Valid."
tools:
  always_on: []
  jit: []
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let results = run_fast(&tree);
    assert!(results.iter().all(|r| matches!(r, LintResult::Pass(_))));
}

// ── project-composition lint ──────────────────────────────────────────────────

const VALID_COMPOSITION: &str = r#"
id: fondament/projects/alpha-agent
kind: project-composition
name: "alpha-agent"
description: "Agent de projet pour Alpha"
parts:
  - role: "assistant de développement"
    source: inline
    content: |
      Tu es l'agent de projet Alpha.
  - role: context
    source: farga
    project: "alpha"
model: claude-sonnet-4-6
"#;

#[test]
fn valid_project_composition_passes_lint() {
    let dir = TempDir::new().unwrap();
    write_def(&dir, "fondament/projects/alpha-agent.yaml", VALID_COMPOSITION);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let results = run_fast(&tree);
    assert!(
        results.iter().all(|r| matches!(r, LintResult::Pass(_))),
        "valid composition must produce only Pass results, got: {:?}", results
    );
}

#[test]
fn composition_missing_name_fails_lint() {
    let dir = TempDir::new().unwrap();
    write_def(&dir, "fondament/projects/no-name.yaml", r#"
id: fondament/projects/no-name
kind: project-composition
parts:
  - role: "dev"
    source: inline
    content: "Tu es l'agent."
model: claude-sonnet-4-6
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let results = run_fast(&tree);
    assert!(
        results.iter().any(|r| matches!(r, LintResult::Fail { rule, .. } if rule == "project-name-present")),
        "missing name must trigger project-name-present failure"
    );
}

#[test]
fn composition_missing_parts_fails_lint() {
    let dir = TempDir::new().unwrap();
    write_def(&dir, "fondament/projects/no-parts.yaml", r#"
id: fondament/projects/no-parts
kind: project-composition
name: "no-parts-agent"
model: claude-sonnet-4-6
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let results = run_fast(&tree);
    assert!(
        results.iter().any(|r| matches!(r, LintResult::Fail { rule, .. } if rule == "project-parts-present")),
        "empty parts must trigger project-parts-present failure"
    );
}

#[test]
fn composition_missing_model_fails_lint() {
    let dir = TempDir::new().unwrap();
    write_def(&dir, "fondament/projects/no-model.yaml", r#"
id: fondament/projects/no-model
kind: project-composition
name: "no-model-agent"
parts:
  - role: "dev"
    source: inline
    content: "Tu es l'agent."
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let results = run_fast(&tree);
    assert!(
        results.iter().any(|r| matches!(r, LintResult::Fail { rule, .. } if rule == "project-model-present")),
        "missing model must trigger project-model-present failure"
    );
}

#[test]
fn composition_farga_part_without_project_fails_lint() {
    let dir = TempDir::new().unwrap();
    write_def(&dir, "fondament/projects/bad-farga.yaml", r#"
id: fondament/projects/bad-farga
kind: project-composition
name: "bad-farga-agent"
parts:
  - role: context
    source: farga
model: claude-sonnet-4-6
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let results = run_fast(&tree);
    assert!(
        results.iter().any(|r| matches!(r, LintResult::Fail { rule, .. } if rule == "farga-project-set")),
        "farga part without project must trigger farga-project-set failure"
    );
}

#[test]
fn invalid_model_id_fails_lint() {
    let dir = TempDir::new().unwrap();
    write_def(&dir, "roles/bad.yaml", r#"
id: roles/bad
kind: role
default_model: totally-bogus-model
context: "Bad model."
tools:
  always_on: []
  jit: []
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let results = run_fast(&tree);
    assert!(results.iter().any(|r| matches!(r, LintResult::Fail { .. })));
}

#[test]
fn grok_model_passes_lint() {
    let dir = TempDir::new().unwrap();
    write_def(&dir, "roles/grok-dev.yaml", r#"
id: roles/grok-dev
kind: role
default_model: grok-3
context: "Grok dev role for complementary runs."
tools:
  always_on: []
  jit: []
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let results = run_fast(&tree);
    // No valid-model-id failure for grok*
    assert!(!results.iter().any(|r| matches!(r, LintResult::Fail { rule, .. } if rule == "valid-model-id")));
}

#[test]
fn openai_and_qwen_models_pass_lint() {
    let dir = TempDir::new().unwrap();
    write_def(&dir, "roles/gpt-reviewer.yaml", r#"
id: roles/gpt-reviewer
kind: role
default_model: gpt-4o
context: "OpenAI-backed reviewer role."
tools:
  always_on: []
  jit: []
"#);
    write_def(&dir, "roles/qwen-analyst.yaml", r#"
id: roles/qwen-analyst
kind: role
default_model: qwen-max
context: "Qwen-backed analyst role."
tools:
  always_on: []
  jit: []
"#);
    write_def(&dir, "roles/custom-provider.yaml", r#"
id: roles/custom-provider
kind: role
default_model: "mistral:mistral-large-latest"
context: "Generic provider:model format."
tools:
  always_on: []
  jit: []
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let results = run_fast(&tree);
    assert!(!results.iter().any(|r| matches!(r, LintResult::Fail { rule, .. } if rule == "valid-model-id")));
}

#[test]
fn gemini_model_passes_lint() {
    let dir = TempDir::new().unwrap();
    write_def(&dir, "roles/gemini-analyst.yaml", r#"
id: roles/gemini-analyst
kind: role
default_model: gemini-2.5-pro
context: "Gemini-backed analyst role."
tools:
  always_on: []
  jit: []
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let results = run_fast(&tree);
    assert!(!results.iter().any(|r| matches!(r, LintResult::Fail { rule, .. } if rule == "valid-model-id")));
}
