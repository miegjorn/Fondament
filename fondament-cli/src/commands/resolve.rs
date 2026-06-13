use fondament_core::{address::CompositionAddress, tree::DefinitionTree, resolver::resolve};
use std::path::Path;

struct NoopFarga;
#[async_trait::async_trait]
impl fondament_core::farga::FargaReader for NoopFarga {
    async fn org_layer(&self, _: &str) -> fondament_core::error::Result<fondament_core::farga::OrgContext> {
        Ok(fondament_core::farga::OrgContext { content: String::new() })
    }
    async fn initiative_layer(&self, _: &str) -> fondament_core::error::Result<Vec<fondament_core::farga::InitiativeContext>> { Ok(vec![]) }
    async fn project_layer(&self, _: &str) -> fondament_core::error::Result<fondament_core::farga::ProjectContext> {
        Ok(fondament_core::farga::ProjectContext { content: String::new() })
    }
    async fn component_layer(&self, _: &str, _: &str) -> fondament_core::error::Result<fondament_core::farga::ProjectContext> {
        Ok(fondament_core::farga::ProjectContext { content: String::new() })
    }
}

pub async fn run(defs: &Path, address: &str) -> anyhow::Result<()> {
    let tree = DefinitionTree::load(defs)?;
    let addr: CompositionAddress = address.parse()?;
    let agent = resolve(&addr, &tree, &NoopFarga, "local").await?;
    println!("=== System Prompt ===\n{}", agent.system_prompt);
    println!("\n=== Default Model ===\n{}", agent.default_model.0);
    Ok(())
}
