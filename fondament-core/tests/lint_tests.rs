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

#[test]
fn invalid_model_id_fails_lint() {
    let dir = TempDir::new().unwrap();
    write_def(&dir, "roles/bad.yaml", r#"
id: roles/bad
kind: role
default_model: gpt-4-turbo
context: "Bad model."
tools:
  always_on: []
  jit: []
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let results = run_fast(&tree);
    assert!(results.iter().any(|r| matches!(r, LintResult::Fail { .. })));
}
