pub async fn run(kind: &str, name: &str) -> anyhow::Result<()> {
    let template = match kind {
        "discipline" => format!("id: disciplines/{}\nkind: discipline\ndefault_model: claude-sonnet-4-6\ncontext: |\n  You are an expert in {}.\ntools:\n  always_on: []\n  jit: []\n", name, name),
        "role" => format!("id: roles/{}\nkind: role\nextends: []\nstance: builder\ncognitive_load: medium\ndefault_model: claude-sonnet-4-6\ncontext: |\n  You are a {}.\ntools:\n  always_on: []\n  jit: []\n", name, name),
        "stance" => format!("id: stances/{}\nkind: stance\ncontext: |\n  Stance: {}.\n", name, name),
        _ => anyhow::bail!("unknown kind '{}'; use: discipline, role, stance", kind),
    };
    let dir = std::path::Path::new("definitions").join(format!("{}s", kind));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.yaml", name));
    std::fs::write(&path, template)?;
    println!("Created {}", path.display());
    Ok(())
}
