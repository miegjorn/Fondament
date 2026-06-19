use fondament_core::{
    address::CompositionAddress,
    farga::{FargaReader, OrgContext, InitiativeContext, ProjectContext},
    resolver::resolve,
    tree::DefinitionTree,
};
use async_trait::async_trait;
use tempfile::TempDir;

struct MockFarga;

#[async_trait]
impl FargaReader for MockFarga {
    async fn org_layer(&self, _org: &str) -> fondament_core::error::Result<OrgContext> {
        Ok(OrgContext { content: "We are a trustworthy org.".into() })
    }
    async fn initiative_layer(&self, _org: &str) -> fondament_core::error::Result<Vec<InitiativeContext>> {
        Ok(vec![InitiativeContext { content: "Goal: grow 20% QoQ.".into() }])
    }
    async fn project_layer(&self, _project: &str) -> fondament_core::error::Result<ProjectContext> {
        Ok(ProjectContext { content: "Project: rewrite auth service.".into() })
    }
    async fn component_layer(&self, _project: &str, _path: &str) -> fondament_core::error::Result<ProjectContext> {
        Ok(ProjectContext { content: "".into() })
    }
}

fn make_tree() -> DefinitionTree {
    let dir = TempDir::new().unwrap();
    let role = r#"
id: fondament/app-architect
kind: role
default_model: claude-sonnet-4-6
context: "You design software systems."
tools:
  always_on: []
  jit: []
"#;
    let path = dir.path().join("roles/app-architect.yaml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, role).unwrap();
    DefinitionTree::load(dir.path()).unwrap()
}

#[tokio::test]
async fn resolves_role_address_to_agent() {
    let tree = make_tree();
    let farga = MockFarga;
    let address: CompositionAddress = "fondament/app-architect".parse().unwrap();
    let agent = resolve(&address, &tree, &farga, "acme").await.unwrap();
    assert!(agent.system_prompt.contains("You design software systems."));
    assert!(agent.system_prompt.contains("We are a trustworthy org."));
    assert_eq!(agent.default_model.0, "claude-sonnet-4-6");
}

fn make_tree_with_stances() -> DefinitionTree {
    let dir = TempDir::new().unwrap();
    let stances: &[(&str, &str)] = &[
        ("stances/builder.yaml", "id: stances/builder\nkind: stance\ncontext: |\n  Construct solutions.\n"),
        ("stances/realist.yaml", "id: stances/realist\nkind: stance\ncontext: |\n  Assess feasibility.\n"),
        ("stances/dreamer.yaml", "id: stances/dreamer\nkind: stance\ncontext: |\n  Explore without constraint.\n"),
        ("stances/moderator.yaml", "id: stances/moderator\nkind: stance\ncontext: |\n  Hold the process.\n"),
    ];
    for (path, content) in stances {
        let full = dir.path().join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, content).unwrap();
    }
    DefinitionTree::load(dir.path()).unwrap()
}

#[tokio::test]
async fn resolves_builder_stance_context() {
    let tree = make_tree_with_stances();
    let farga = MockFarga;
    let address: CompositionAddress = "stances/builder".parse().unwrap();
    let agent = resolve(&address, &tree, &farga, "acme").await.unwrap();
    assert!(agent.system_prompt.contains("Construct solutions"));
}

#[tokio::test]
async fn all_four_stances_resolve_without_error() {
    let tree = make_tree_with_stances();
    let farga = MockFarga;
    for stance in &["builder", "realist", "dreamer", "moderator"] {
        let addr: CompositionAddress = format!("stances/{}", stance).parse().unwrap();
        let agent = resolve(&addr, &tree, &farga, "acme").await.unwrap();
        assert!(!agent.system_prompt.is_empty(), "stance {} produced empty prompt", stance);
    }
}

fn make_tree_with_deconstructive() -> (DefinitionTree, TempDir) {
    let dir = TempDir::new().unwrap();
    let files: &[(&str, &str)] = &[
        ("disciplines/system-design.yaml",
         "id: disciplines/system-design\nkind: discipline\ncontext: \"You architect systems.\"\n"),
        ("disciplines/deconstructive.yaml",
         "id: disciplines/deconstructive\nkind: discipline\nmodifier: true\n"),
        ("roles/platform-architect.yaml",
         "id: fondament/platform-architect\nkind: role\nextends: [disciplines/system-design]\ndefault_model: claude-sonnet-4-6\ncontext: \"You are a platform architect.\"\n"),
    ];
    for (path, content) in files {
        let full = dir.path().join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, content).unwrap();
    }
    (DefinitionTree::load(dir.path()).unwrap(), dir)
}

#[tokio::test]
async fn deconstructive_modifier_injects_preamble_before_domain_content() {
    let (tree, _dir) = make_tree_with_deconstructive();
    let address: CompositionAddress = "fondament/platform-architect+deconstructive".parse().unwrap();
    let agent = resolve(&address, &tree, &MockFarga, "acme").await.unwrap();
    assert!(
        agent.system_prompt.contains("deconstructive discipline"),
        "preamble header must appear in system_prompt"
    );
    assert!(
        agent.system_prompt.contains("Before producing any response"),
        "preamble instructions must appear in system_prompt"
    );
    let preamble_pos = agent.system_prompt.find("deconstructive discipline").unwrap();
    let domain_pos = agent.system_prompt.find("platform architect").unwrap();
    assert!(preamble_pos < domain_pos, "preamble must precede domain content");
}

#[tokio::test]
async fn deconstructive_modifier_sets_thinking_budget() {
    let (tree, _dir) = make_tree_with_deconstructive();
    let address: CompositionAddress = "fondament/platform-architect+deconstructive".parse().unwrap();
    let agent = resolve(&address, &tree, &MockFarga, "acme").await.unwrap();
    assert!(agent.thinking_budget.is_some(), "thinking_budget must be set with deconstructive modifier");
    let budget = agent.thinking_budget.unwrap();
    assert!(budget >= 3_000, "minimum budget is 3000 tokens");
    assert!(budget <= 10_000, "budget is capped at 10000 tokens");
}

#[tokio::test]
async fn without_deconstructive_no_preamble_no_budget() {
    let (tree, _dir) = make_tree_with_deconstructive();
    let address: CompositionAddress = "fondament/platform-architect".parse().unwrap();
    let agent = resolve(&address, &tree, &MockFarga, "acme").await.unwrap();
    assert!(
        !agent.system_prompt.contains("deconstructive discipline"),
        "preamble must not appear without deconstructive modifier"
    );
    assert!(
        agent.thinking_budget.is_none(),
        "thinking_budget must be None without deconstructive modifier"
    );
}
