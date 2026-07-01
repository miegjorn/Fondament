use fondament_core::{
    address::CompositionAddress,
    farga::{FargaReader, OrgContext, InitiativeContext, ProjectContext},
    resolver::{build_aporia_preamble, resolve},
    tree::DefinitionTree,
    ComposedPart, PartKind,
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

fn make_tree_with_aporia() -> (DefinitionTree, TempDir) {
    let dir = TempDir::new().unwrap();
    let files: &[(&str, &str)] = &[
        ("disciplines/system-design.yaml",
         "id: disciplines/system-design\nkind: discipline\ncontext: \"You architect systems.\"\n"),
        ("disciplines/aporia.yaml",
         "id: disciplines/aporia\nkind: discipline\nmodifier: true\n"),
        ("stances/adversarial.yaml",
         "id: stances/adversarial\nkind: stance\ncontext: \"Challenge every assumption.\"\n"),
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
async fn aporia_modifier_injects_preamble_before_domain_content() {
    let (tree, _dir) = make_tree_with_aporia();
    let address: CompositionAddress = "fondament/platform-architect+aporia".parse().unwrap();
    let agent = resolve(&address, &tree, &MockFarga, "acme").await.unwrap();
    assert!(
        agent.system_prompt.contains("aporia discipline"),
        "preamble header must appear in system_prompt"
    );
    assert!(
        agent.system_prompt.contains("Before producing any response"),
        "preamble instructions must appear in system_prompt"
    );
    let preamble_pos = agent.system_prompt.find("aporia discipline").unwrap();
    let domain_pos = agent.system_prompt.find("platform architect").unwrap();
    assert!(preamble_pos < domain_pos, "preamble must precede domain content");
}

#[tokio::test]
async fn aporia_modifier_sets_structured_reasoning() {
    let (tree, _dir) = make_tree_with_aporia();
    let address: CompositionAddress = "fondament/platform-architect+aporia".parse().unwrap();
    let agent = resolve(&address, &tree, &MockFarga, "acme").await.unwrap();
    assert!(agent.structured_reasoning.is_some(), "structured_reasoning must be set with aporia modifier");
    let budget = agent.structured_reasoning.unwrap().anthropic_budget();
    assert!(budget >= 3_000, "minimum anthropic budget is 3000 tokens");
    assert!(budget <= 10_000, "anthropic budget is capped at 10000 tokens");
}

#[tokio::test]
async fn without_aporia_no_preamble_no_reasoning() {
    let (tree, _dir) = make_tree_with_aporia();
    let address: CompositionAddress = "fondament/platform-architect".parse().unwrap();
    let agent = resolve(&address, &tree, &MockFarga, "acme").await.unwrap();
    assert!(
        !agent.system_prompt.contains("aporia discipline"),
        "preamble must not appear without aporia modifier"
    );
    assert!(
        agent.structured_reasoning.is_none(),
        "structured_reasoning must be None without aporia modifier"
    );
}

#[tokio::test]
async fn aporia_with_stance_collects_stance_as_part() {
    let (tree, _dir) = make_tree_with_aporia();
    // "fondament/platform-architect+aporia" has no stance_override,
    // but stances/adversarial is in the tree. Test a role that directly references
    // the stance via a Role with stance_override.
    let address: CompositionAddress = "fondament/platform-architect+aporia+adversarial".parse().unwrap();
    // This parses as Role { role: "fondament/platform-architect", modifiers: ["aporia"], stance_override: Some("adversarial") }
    let agent = resolve(&address, &tree, &MockFarga, "acme").await.unwrap();
    // Stance context must appear in the system prompt
    assert!(agent.system_prompt.contains("Challenge every assumption"),
        "adversarial stance context must appear");
    // Preamble must be present (aporia active)
    assert!(agent.system_prompt.contains("aporia discipline"),
        "preamble must be injected");
    // 1 discipline + 1 stance = 2 parts → Medium → 6000 tokens at Anthropic
    let budget = agent.structured_reasoning.unwrap().anthropic_budget();
    assert!(budget >= 6_000, "2 parts should yield at least 6000 budget tokens, got {}", budget);
}

#[test]
fn definition_file_deserializes_component_field() {
    let yaml = r#"
id: fondament/amassada-agent
kind: component-agent
component: amassada
default_model: claude-sonnet-4-6
context: "You are the Amassada agent."
"#;
    let def: fondament_core::definition::DefinitionFile = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(def.component.as_deref(), Some("amassada"));
}

#[test]
fn definition_file_component_defaults_to_none() {
    let yaml = r#"
id: fondament/guilhem
kind: role
context: "You are Guilhem."
"#;
    let def: fondament_core::definition::DefinitionFile = serde_yaml::from_str(yaml).unwrap();
    assert!(def.component.is_none());
}

#[test]
fn composed_part_session_node_renders_with_weight() {
    let parts = vec![ComposedPart {
        kind: PartKind::SessionNode,
        name: "N8".into(),
        weight: 1.0,
        corpus_ref: None,
    }];
    let preamble = build_aporia_preamble(&parts);
    assert!(
        preamble.contains("session-node:"),
        "preamble must contain 'session-node:'"
    );
    assert!(
        preamble.contains("weight:"),
        "preamble must contain 'weight:'"
    );
    assert!(
        preamble.contains("N8"),
        "preamble must contain the node name"
    );
    assert!(
        preamble.contains("1.00"),
        "preamble must render weight as two decimal places"
    );
}

#[test]
fn composed_part_domain_renders_unchanged() {
    let parts = vec![ComposedPart {
        kind: PartKind::Domain,
        name: "auth-service".into(),
        weight: 0.0,
        corpus_ref: None,
    }];
    let preamble = build_aporia_preamble(&parts);
    assert!(
        preamble.contains("[domain: auth-service]"),
        "domain part must render as '[domain: <name>]', got:\n{}", preamble
    );
}
