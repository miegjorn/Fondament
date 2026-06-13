use fondament_core::{lint::fast::run_fast, tree::DefinitionTree};
use std::path::Path;

pub async fn run(defs: &Path, scoped: Option<&str>) -> anyhow::Result<()> {
    let root = scoped.map(|s| defs.join(s)).unwrap_or(defs.to_path_buf());
    let tree = DefinitionTree::load(&root)?;
    let results = run_fast(&tree);
    let mut failures = 0;
    for r in &results {
        match r {
            fondament_core::lint::fast::LintResult::Fail { id, rule, message } => {
                eprintln!("FAIL  {} [{}]: {}", id, rule, message);
                failures += 1;
            }
            fondament_core::lint::fast::LintResult::Warn { id, rule, message } => {
                eprintln!("WARN  {} [{}]: {}", id, rule, message);
            }
            fondament_core::lint::fast::LintResult::Pass(id) => {
                println!("OK    {}", id);
            }
        }
    }
    if failures > 0 {
        anyhow::bail!("{} lint failure(s)", failures);
    }
    Ok(())
}
