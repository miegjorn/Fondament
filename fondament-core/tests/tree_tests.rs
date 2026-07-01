use fondament_core::tree::DefinitionTree;
use tempfile::TempDir;

fn write_file(dir: &TempDir, path: &str, content: &str) {
    let full = dir.path().join(path);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(full, content).unwrap();
}

#[test]
fn loads_discipline_from_file() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "disciplines/data/db/mysql.yaml", r#"
id: data/db/mysql
kind: discipline
default_model: claude-haiku-4-5-20251001
context: "You are a MySQL expert."
tools:
  always_on: []
  jit: []
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let def = tree.get("data/db/mysql").unwrap();
    assert_eq!(def.id, "data/db/mysql");
    assert_eq!(def.kind.as_str(), "discipline");
}

#[test]
fn returns_none_for_unknown_id() {
    let dir = TempDir::new().unwrap();
    let tree = DefinitionTree::load(dir.path()).unwrap();
    assert!(tree.get("nonexistent").is_none());
}

#[test]
fn aporia_discipline_has_modifier_true() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "disciplines/aporia.yaml", r#"
id: disciplines/aporia
kind: discipline
modifier: true
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let def = tree.get("disciplines/aporia").unwrap();
    assert!(def.modifier, "aporia must have modifier: true");
}

#[test]
fn discipline_without_modifier_field_defaults_to_false() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "disciplines/rust-async.yaml", r#"
id: disciplines/rust-async
kind: discipline
context: "You are a Rust async expert."
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let def = tree.get("disciplines/rust-async").unwrap();
    assert!(!def.modifier, "discipline without modifier field must default to false");
}

// ── tool schema validation (Fondament#23) ─────────────────────────────────────

#[test]
fn flat_string_always_on_is_rejected_with_schema_error() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "disciplines/bad-schema.yaml", r#"
id: disciplines/bad-schema
kind: discipline
default_model: claude-sonnet-4-6
context: "Test."
tools:
  always_on:
    - mcp__farga__write_signal
  jit: []
"#);
    let result = DefinitionTree::load(dir.path());
    assert!(result.is_err(), "flat-string always_on must be rejected at load time");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("schema error") && err.contains("tools.always_on[0]"),
        "error must name the offending field and index — got: {}", err
    );
    assert!(
        err.contains("mcp__farga__write_signal"),
        "error must echo the offending string value — got: {}", err
    );
}

#[test]
fn flat_string_jit_is_rejected_with_schema_error() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "disciplines/bad-jit.yaml", r#"
id: disciplines/bad-jit
kind: discipline
default_model: claude-sonnet-4-6
context: "Test."
tools:
  always_on: []
  jit:
    - mcp__farga__list_projects
"#);
    let result = DefinitionTree::load(dir.path());
    assert!(result.is_err(), "flat-string jit must be rejected at load time");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("schema error") && err.contains("tools.jit[0]"),
        "error must name the offending jit field — got: {}", err
    );
}

#[test]
fn structured_always_on_loads_correctly() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "disciplines/good-tools.yaml", r#"
id: disciplines/good-tools
kind: discipline
default_model: claude-sonnet-4-6
context: "Test."
tools:
  always_on:
    - id: farga-write-signal
      kind: mcp
      server: farga
      tool: write_signal
  jit: []
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let def = tree.get("disciplines/good-tools").unwrap();
    assert_eq!(def.tools.always_on.len(), 1);
    assert_eq!(def.tools.always_on[0].id, "farga-write-signal");
}

#[test]
fn definition_without_tools_block_loads_correctly() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "stances/pragmatist.yaml", r#"
id: stances/pragmatist
kind: stance
context: "Balance idealism with delivery constraints."
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    assert!(tree.get("stances/pragmatist").is_some());
}
