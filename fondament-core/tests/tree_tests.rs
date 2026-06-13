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
