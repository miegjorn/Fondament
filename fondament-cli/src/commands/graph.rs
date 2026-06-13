use fondament_core::tree::DefinitionTree;
use std::path::Path;

pub async fn run(defs: &Path) -> anyhow::Result<()> {
    let tree = DefinitionTree::load(defs)?;
    println!("digraph fondament {{");
    for def in tree.all() {
        for parent in &def.extends {
            println!("  \"{}\" -> \"{}\";", def.id, parent);
        }
    }
    println!("}}");
    Ok(())
}
