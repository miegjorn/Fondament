# Fondament Sweep Lint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `fondament sweep` — an LLM-powered lint pass that validates each definition's context matches its declared kind and id.

**Architecture:** New `sweep` command in fondament-cli; calls Claude API directly via reqwest; no persistent state.

**Tech Stack:** Rust, reqwest, serde_json, clap, fondament-core. CLI at `/Users/bedardpl/project/Fondament/fondament-cli`.

---

## Task 1: Add reqwest + serde_json + walkdir deps to fondament-cli/Cargo.toml

**File:** `/Users/bedardpl/project/Fondament/fondament-cli/Cargo.toml`

Current `[dependencies]` block:
```toml
[dependencies]
fondament-core = { path = "../fondament-core" }
tokio = { workspace = true }
clap = { workspace = true }
anyhow = { workspace = true }
async-trait = "0.1"
```

- [ ] Add `reqwest`, `serde_json`, and `serde` to `[dependencies]`:

```toml
[dependencies]
fondament-core = { path = "../fondament-core" }
tokio = { workspace = true }
clap = { workspace = true }
anyhow = { workspace = true }
async-trait = "0.1"
reqwest = { version = "0.12", features = ["json"] }
serde_json = "1"
serde = { version = "1", features = ["derive"] }
```

- [ ] Verify it compiles: `cargo build -p fondament-cli`

Expected output: no errors (new deps download and link cleanly).

**Commit:** `chore: add reqwest + serde_json to fondament-cli for sweep command`

---

## Task 2: Implement fondament-cli/src/commands/sweep.rs

**File:** `/Users/bedardpl/project/Fondament/fondament-cli/src/commands/sweep.rs` (new file)

- [ ] Create the file with the full implementation below:

```rust
use anyhow::anyhow;

// ── Public entry point ────────────────────────────────────────────────────────

pub async fn run(defs: &std::path::Path, path_filter: Option<&str>) -> anyhow::Result<()> {
    let tree = fondament_core::tree::DefinitionTree::load(defs)?;

    let entries: Vec<&fondament_core::definition::DefinitionFile> = tree
        .all()
        .filter(|def| {
            if let Some(filter) = path_filter {
                def.id.starts_with(filter)
            } else {
                true
            }
        })
        .collect();

    let mut invalid_count = 0usize;

    for def in &entries {
        let context = match &def.context {
            Some(ctx) if !ctx.trim().is_empty() => ctx.as_str(),
            _ => continue,
        };

        let result = assess_definition(&def.id, &def.kind, context).await?;
        let prefix = verdict_prefix(&result.verdict);
        if result.verdict == "invalid" {
            invalid_count += 1;
            eprintln!("{} {} — {}", prefix, def.id, result.reason);
        } else if result.verdict == "warning" {
            println!("{} {} — {}", prefix, def.id, result.reason);
        } else {
            println!("{} {}", prefix, def.id);
        }
    }

    if invalid_count > 0 {
        Err(anyhow!("{} definition(s) failed semantic lint", invalid_count))
    } else {
        Ok(())
    }
}

// ── Verdict helpers ───────────────────────────────────────────────────────────

pub fn verdict_prefix(verdict: &str) -> &'static str {
    match verdict {
        "valid"   => "✓",
        "warning" => "⚠",
        _         => "✗",
    }
}

pub fn count_invalids(results: &[AssessResult]) -> usize {
    results.iter().filter(|r| r.verdict == "invalid").count()
}

// ── API types & call ──────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct AssessResult {
    pub verdict: String, // "valid" | "warning" | "invalid"
    pub reason: String,
}

async fn assess_definition(id: &str, kind: &str, context: &str) -> anyhow::Result<AssessResult> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    let prompt = format!(
        "You are reviewing an agent definition file for semantic consistency.\n\n\
         Kind: {kind}\n\
         ID: {id}\n\
         Context:\n{context}\n\n\
         Does this context actually match what is claimed? A \"{kind}\" definition with id \"{id}\" should focus on that exact topic.\n\n\
         Respond ONLY with a JSON object, no markdown:\n\
         {{\"verdict\": \"valid\"|\"warning\"|\"invalid\", \"reason\": \"one sentence\"}}\n\
         - valid: context clearly matches the declared kind and id\n\
         - warning: context is related but has drift or gaps from the claimed focus\n\
         - invalid: context clearly doesn't match the kind/id claim"
    );

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-6",
            "max_tokens": 256,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await?
        .error_for_status()?;

    let json: serde_json::Value = resp.json().await?;
    let text = json["content"]
        .as_array()
        .and_then(|blocks| blocks.iter().find(|b| b["type"].as_str() == Some("text")))
        .and_then(|b| b["text"].as_str())
        .ok_or_else(|| anyhow::anyhow!("empty response from Claude"))?;

    let result: AssessResult = serde_json::from_str(text.trim())
        .map_err(|e| anyhow::anyhow!("could not parse assessment JSON '{}': {}", text, e))?;
    Ok(result)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_invalids_returns_correct_count() {
        let results = vec![
            AssessResult { verdict: "valid".into(),   reason: "ok".into() },
            AssessResult { verdict: "invalid".into(), reason: "drift".into() },
            AssessResult { verdict: "warning".into(), reason: "minor".into() },
            AssessResult { verdict: "invalid".into(), reason: "mismatch".into() },
        ];
        assert_eq!(count_invalids(&results), 2);
    }

    #[test]
    fn verdict_prefix_maps_correctly() {
        assert_eq!(verdict_prefix("valid"),   "✓");
        assert_eq!(verdict_prefix("warning"), "⚠");
        assert_eq!(verdict_prefix("invalid"), "✗");
        // anything unknown collapses to invalid marker
        assert_eq!(verdict_prefix("unknown"), "✗");
    }
}
```

- [ ] Run unit tests: `cargo test -p fondament-cli sweep`

Expected output:
```
running 2 tests
test commands::sweep::tests::count_invalids_returns_correct_count ... ok
test commands::sweep::tests::verdict_prefix_maps_correctly ... ok
test result: ok. 2 passed; 0 failed; 0 ignored
```

---

## Task 3: Register sweep in main.rs and commands/mod.rs

**Files:**
- `/Users/bedardpl/project/Fondament/fondament-cli/src/commands/mod.rs`
- `/Users/bedardpl/project/Fondament/fondament-cli/src/main.rs`

### 3a — commands/mod.rs

Current content:
```rust
pub mod check;
pub mod graph;
pub mod resolve;
pub mod scaffold;
```

- [ ] Add `pub mod sweep;`:

```rust
pub mod check;
pub mod graph;
pub mod resolve;
pub mod scaffold;
pub mod sweep;
```

### 3b — main.rs

Current `Commands` enum:
```rust
#[derive(Subcommand)]
enum Commands {
    Check { path: Option<String> },
    Resolve { address: String, #[arg(long)] project: Option<String> },
    Scaffold { kind: String, name: String },
    Graph,
}
```

- [ ] Add `Sweep` variant:

```rust
#[derive(Subcommand)]
enum Commands {
    Check { path: Option<String> },
    Resolve { address: String, #[arg(long)] project: Option<String> },
    Scaffold { kind: String, name: String },
    Graph,
    Sweep { path: Option<String> },
}
```

Current `match` block:
```rust
    match cli.command {
        Commands::Check { path } => commands::check::run(defs, path.as_deref()).await,
        Commands::Resolve { address, .. } => commands::resolve::run(defs, &address).await,
        Commands::Scaffold { kind, name } => commands::scaffold::run(&kind, &name).await,
        Commands::Graph => commands::graph::run(defs).await,
    }
```

- [ ] Add `Sweep` arm:

```rust
    match cli.command {
        Commands::Check { path } => commands::check::run(defs, path.as_deref()).await,
        Commands::Resolve { address, .. } => commands::resolve::run(defs, &address).await,
        Commands::Scaffold { kind, name } => commands::scaffold::run(&kind, &name).await,
        Commands::Graph => commands::graph::run(defs).await,
        Commands::Sweep { path } => commands::sweep::run(defs, path.as_deref()).await,
    }
```

- [ ] Build and smoke-test: `cargo build -p fondament-cli && ./target/debug/fondament sweep --help`

Expected output:
```
Usage: fondament sweep [PATH]

Arguments:
  [PATH]

Options:
  -h, --help  Print help
```

- [ ] Run full test suite: `cargo test -p fondament-cli`

Expected: all tests pass, including the 2 new sweep unit tests.

**Commit:** `feat: fondament sweep command — LLM-based semantic lint for definition contexts`

---

## Notes for the executing agent

- `DefinitionTree::load` is the correct constructor (not `from_dir`). Signature: `pub fn load(root: &Path) -> Result<Self>` in `fondament-core/src/tree.rs`.
- `DefinitionFile.context` is `Option<String>` — skip where `None` or empty string.
- `DefinitionFile.id` and `DefinitionFile.kind` are plain `String`.
- The `path_filter` in `run` matches against `def.id.starts_with(filter)` — IDs are slash-delimited paths like `disciplines/rust-async`.
- `ANTHROPIC_API_KEY` must be set in the environment; the command errors immediately if absent.
- Do not add `walkdir` — `DefinitionTree::load` already walks recursively via `std::fs::read_dir`.
- Model used in prompt: `claude-sonnet-4-6` (matches the current production model).
