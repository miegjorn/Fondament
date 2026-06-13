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
