mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "fondament", about = "Fondament agent primitive CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Check { path: Option<String> },
    Resolve { address: String, #[arg(long)] project: Option<String>, #[arg(long)] farga_url: Option<String> },
    Scaffold { kind: String, name: String },
    Graph,
    Sweep { path: Option<String> },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let defs = std::path::Path::new("definitions");

    match cli.command {
        Commands::Check { path } => commands::check::run(defs, path.as_deref()).await,
        Commands::Resolve { address, farga_url, .. } => commands::resolve::run(defs, &address, farga_url.as_deref()).await,
        Commands::Scaffold { kind, name } => commands::scaffold::run(&kind, &name).await,
        Commands::Graph => commands::graph::run(defs).await,
        Commands::Sweep { path } => commands::sweep::run(defs, path.as_deref()).await,
    }
}
