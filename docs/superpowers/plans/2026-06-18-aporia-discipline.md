# Aporia Discipline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `aporia` modifier discipline to Fondament that injects a preamble into the agent's system prompt instructing internal multi-voice decomposition before collapse, enables extended thinking on the API call, and is packaged as a Cor installable plugin.

**Architecture:** Three layers of change — (1) YAML definition + `DefinitionFile.modifier` flag to mark it as non-corpus; (2) `CompositionAddress` gains `modifiers: Vec<String>` so `+aporia` stacks before a stance without being treated as one; (3) the resolver detects the modifier and prepends a generated preamble listing the agent's actual parts, then sets `thinking_budget` on `ResolvedAgent`. Nothing in Amassada, Charradissa, or Farga changes.

**Tech Stack:** Rust, serde/serde_yaml (already in workspace), Fondament workspace at `/Users/bedardpl/project/Fondament`.

---

## Context for the Implementer

### Baseline: 10 tests passing

```
test result: ok. 3 passed (resolver_tests)
test result: ok. 2 passed (tree_tests)
test result: ok. 3 passed (address_tests)
test result: ok. 2 passed (lint_tests)
```

### Existing `CompositionAddress` (address.rs)

```rust
pub enum CompositionAddress {
    Role { role: String, stance_override: Option<String> },
    Composed { project: String, facet: Option<String>, stance: String },
}
```

Parser: `split_once('+')` — only handles one `+`. Adding `modifiers` means splitting ALL `+` segments.

### Existing `DefinitionFile` (definition.rs)

```rust
pub struct DefinitionFile {
    pub id: String, pub kind: String,
    pub extends: Vec<String>,
    pub default_model: Option<ModelId>,
    pub context: Option<String>,
    pub tools: ToolSet,
    pub stance: Option<String>,
    pub cognitive_load: Option<String>,
}
```

### Existing `ResolvedAgent` (types.rs)

```rust
pub struct ResolvedAgent {
    pub system_prompt: String,
    pub tools: Vec<ToolDefinition>,
    pub jit_tools: Vec<ToolDefinition>,
    pub default_model: ModelId,
}
```

### Existing resolver (resolver.rs) — layer assembly order

1. Org context (Farga)
2. Initiative context (Farga)
3. Project context (Farga, for Composed only)
4. Fondament definition layers (extends chain walk)
5. Stance context (from tree)

→ Aporia preamble must be inserted BEFORE all of these.

### Cor plugin format (cor-core/src/manifest.rs)

```rust
pub struct Plugin {
    pub plugin: PluginMeta,       // id, version, kind, name, description, authors, license, repository
    pub compatibility: Compatibility, // stack, providers
    pub artifact: Option<ArtifactDef>,  // path
    pub install: Option<InstallDef>,    // target
}
```

Stored in `plugin.toml` (TOML format). Discipline plugins install to `Fondament/definitions/disciplines/installed/` by default.

---

## Task 1: `aporia.yaml` + `DefinitionFile.modifier` field

**Files:**
- Create: `Fondament/definitions/disciplines/aporia.yaml`
- Modify: `Fondament/fondament-core/src/definition.rs`
- Modify: `Fondament/fondament-core/tests/tree_tests.rs`

### Why `modifier: bool` matters

A modifier discipline does NOT contribute corpus content to the agent — its presence changes HOW the prompt is assembled. The field distinguishes `aporia` (modifier=true, no context) from `disciplines/rust-async` (modifier=false, has context). The resolver skips modifier disciplines in the extends chain walk and handles them separately.

- [ ] **Step 1: Write failing tree tests**

Append to `Fondament/fondament-core/tests/tree_tests.rs`:

```rust
#[test]
fn aporia_discipline_has_modifier_true() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "disciplines/aporia.yaml", r#"
id: disciplines/aporia
kind: discipline
modifier: true
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let def = tree.get("disciplines/aporia").unwrap();
    assert!(def.modifier, "aporia must have modifier: true");
}

#[test]
fn discipline_without_modifier_field_defaults_to_false() {
    let dir = TempDir::new().unwrap();
    write_file(&dir, "disciplines/rust-async.yaml", r#"
id: disciplines/rust-async
kind: discipline
context: "You are a Rust async expert."
"#);
    let tree = DefinitionTree::load(dir.path()).unwrap();
    let def = tree.get("disciplines/rust-async").unwrap();
    assert!(!def.modifier, "discipline without modifier field must default to false");
}
```

- [ ] **Step 2: Run to verify failure**

```bash
cd /Users/bedardpl/project/Fondament && cargo test -p fondament-core tree 2>&1 | grep -E "^error|FAILED" | head -5
```

Expected: compile error — `modifier` field doesn't exist on `DefinitionFile`.

- [ ] **Step 3: Add `modifier` field to DefinitionFile**

Replace the entire content of `Fondament/fondament-core/src/definition.rs`:

```rust
use serde::{Deserialize, Serialize};
use crate::tools::ToolSet;
use crate::types::ModelId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionFile {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub extends: Vec<String>,
    pub default_model: Option<ModelId>,
    pub context: Option<String>,
    #[serde(default)]
    pub tools: ToolSet,
    pub stance: Option<String>,
    pub cognitive_load: Option<String>,
    /// True for disciplines that act as reasoning modifiers rather than corpus contributors.
    /// Modifier disciplines inject behaviour into prompt assembly but add no context layer.
    #[serde(default)]
    pub modifier: bool,
}

impl DefinitionFile {
    pub fn effective_model(&self) -> ModelId {
        self.default_model.clone().unwrap_or_default()
    }
}
```

- [ ] **Step 4: Create aporia.yaml**

Create `Fondament/definitions/disciplines/aporia.yaml`:

```yaml
id: disciplines/aporia
kind: discipline
modifier: true
```

No `context` field — the aporia discipline injects a dynamically generated preamble
(computed from the agent's actual composition) rather than static corpus content.

- [ ] **Step 5: Run tree tests**

```bash
cd /Users/bedardpl/project/Fondament && cargo test -p fondament-core 2>&1 | grep "test result"
```

Expected: 4 tree tests pass (2 original + 2 new). All 10 baseline tests still pass.

- [ ] **Step 6: Commit**

```bash
cd /Users/bedardpl/project/Fondament && git add definitions/disciplines/aporia.yaml fondament-core/src/definition.rs fondament-core/tests/tree_tests.rs && git commit -m "feat: add aporia modifier discipline and DefinitionFile.modifier field"
```

---

## Task 2: Multi-modifier `CompositionAddress` + `ResolvedAgent.thinking_budget`

**Files:**
- Modify: `Fondament/fondament-core/src/address.rs`
- Modify: `Fondament/fondament-core/src/types.rs`
- Modify: `Fondament/fondament-core/tests/address_tests.rs`

### Address format changes

New addresses the parser must handle:

| Address | Variant | Result |
|---|---|---|
| `fondament/app-architect` | Role | role="fondament/app-architect", modifiers=[], stance_override=None |
| `fondament/roles/security-sre+aporia` | Role | role="fondament/roles/security-sre", modifiers=["aporia"], stance_override=None |
| `acme-auth/auth+adversarial` | Composed | project="acme-auth", facet="auth", modifiers=[], stance="adversarial" |
| `acme-auth/auth+aporia+adversarial` | Composed | project="acme-auth", facet="auth", modifiers=["aporia"], stance="adversarial" |
| `acme-auth/auth+aporia` | Role | role="acme-auth/auth", modifiers=["aporia"], stance_override=None |

The last case (modifier-only, non-fondament path) becomes `Role` because there is no stance — it still resolves against any tree entry named `acme-auth/auth` and Farga project layers are available separately.

### Known modifier list

```rust
pub const KNOWN_MODIFIER_DISCIPLINES: &[&str] = &["aporia"];
```

Parsing rule: split on ALL `+`; segments in `KNOWN_MODIFIER_DISCIPLINES` go to `modifiers`, others go to `stance` (error if two non-modifier `+` segments).

- [ ] **Step 1: Update address_tests.rs to match new struct shape**

The existing test `parses_composed_address_with_facet` destructures `Composed { project, facet, stance }` WITHOUT `..` — adding `modifiers` will cause a compile error. Fix the existing tests AND add new ones.

Replace the entire content of `Fondament/fondament-core/tests/address_tests.rs`:

```rust
use fondament_core::address::CompositionAddress;

#[test]
fn parses_role_address() {
    let a: CompositionAddress = "fondament/app-architect".parse().unwrap();
    match &a {
        CompositionAddress::Role { role, modifiers, stance_override } => {
            assert_eq!(role, "fondament/app-architect");
            assert!(modifiers.is_empty());
            assert!(stance_override.is_none());
        }
        _ => panic!("expected Role"),
    }
}

#[test]
fn parses_composed_address_with_facet() {
    let a: CompositionAddress = "acme-auth/auth+adversarial".parse().unwrap();
    match &a {
        CompositionAddress::Composed { project, facet, modifiers, stance } => {
            assert_eq!(project, "acme-auth");
            assert_eq!(facet.as_deref(), Some("auth"));
            assert!(modifiers.is_empty());
            assert_eq!(stance, "adversarial");
        }
        _ => panic!("expected Composed"),
    }
}

#[test]
fn display_roundtrips() {
    for s in ["fondament/app-architect", "proj/facet+builder"] {
        let a: CompositionAddress = s.parse().unwrap();
        assert_eq!(a.to_string(), s);
    }
}

#[test]
fn parses_role_with_aporia_modifier() {
    let a: CompositionAddress = "fondament/roles/security-sre+aporia".parse().unwrap();
    match &a {
        CompositionAddress::Role { role, modifiers, stance_override } => {
            assert_eq!(role, "fondament/roles/security-sre");
            assert_eq!(modifiers, &["aporia"]);
            assert!(stance_override.is_none());
        }
        _ => panic!("expected Role"),
    }
}

#[test]
fn parses_composed_with_modifier_and_stance() {
    let a: CompositionAddress = "acme-auth/auth+aporia+adversarial".parse().unwrap();
    match &a {
        CompositionAddress::Composed { project, facet, modifiers, stance } => {
            assert_eq!(project, "acme-auth");
            assert_eq!(facet.as_deref(), Some("auth"));
            assert_eq!(modifiers, &["aporia"]);
            assert_eq!(stance, "adversarial");
        }
        _ => panic!("expected Composed"),
    }
}

#[test]
fn parses_modifier_only_non_fondament_as_role() {
    // No stance means no Composed — falls through to Role
    let a: CompositionAddress = "acme-auth/auth+aporia".parse().unwrap();
    match &a {
        CompositionAddress::Role { role, modifiers, stance_override } => {
            assert_eq!(role, "acme-auth/auth");
            assert_eq!(modifiers, &["aporia"]);
            assert!(stance_override.is_none());
        }
        _ => panic!("expected Role (modifier-only, no stance)"),
    }
}

#[test]
fn display_roundtrips_with_modifier() {
    for s in [
        "fondament/roles/security-sre+aporia",
        "acme-auth/auth+aporia+adversarial",
    ] {
        let a: CompositionAddress = s.parse().unwrap();
        assert_eq!(a.to_string(), s, "display must roundtrip for {}", s);
    }
}
```

- [ ] **Step 2: Run to verify failure**

```bash
cd /Users/bedardpl/project/Fondament && cargo test -p fondament-core address 2>&1 | grep -E "^error|FAILED" | head -10
```

Expected: compile errors — `modifiers` field doesn't exist on `CompositionAddress` yet.

- [ ] **Step 3: Replace address.rs**

Replace the entire content of `Fondament/fondament-core/src/address.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use crate::error::{FondamentError, Result};

/// Discipline names that act as reasoning modifiers rather than domain/corpus identifiers.
/// These are stripped out of stance position during parsing.
pub const KNOWN_MODIFIER_DISCIPLINES: &[&str] = &["aporia"];

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompositionAddress {
    Role {
        role: String,
        modifiers: Vec<String>,
        stance_override: Option<String>,
    },
    Composed {
        project: String,
        facet: Option<String>,
        modifiers: Vec<String>,
        stance: String,
    },
}

impl FromStr for CompositionAddress {
    type Err = FondamentError;

    fn from_str(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Err(FondamentError::AddressParse("empty".into()));
        }

        let parts: Vec<&str> = s.split('+').collect();
        let path = parts[0];
        let qualifiers = &parts[1..];

        let mut modifiers: Vec<String> = Vec::new();
        let mut stance: Option<String> = None;

        for q in qualifiers {
            if KNOWN_MODIFIER_DISCIPLINES.contains(q) {
                modifiers.push(q.to_string());
            } else if stance.is_some() {
                return Err(FondamentError::AddressParse(
                    format!("multiple stances in address: {}", s)
                ));
            } else {
                stance = Some(q.to_string());
            }
        }

        if path.starts_with("fondament/") || stance.is_none() {
            Ok(CompositionAddress::Role {
                role: path.to_string(),
                modifiers,
                stance_override: stance,
            })
        } else {
            let stance = stance.unwrap();
            let (project, facet) = path.split_once('/')
                .map(|(p, f)| (p.to_string(), Some(f.to_string())))
                .unwrap_or((path.to_string(), None));
            Ok(CompositionAddress::Composed { project, facet, modifiers, stance })
        }
    }
}

impl fmt::Display for CompositionAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompositionAddress::Role { role, modifiers, stance_override } => {
                write!(f, "{}", role)?;
                for m in modifiers { write!(f, "+{}", m)?; }
                if let Some(s) = stance_override { write!(f, "+{}", s)?; }
                Ok(())
            }
            CompositionAddress::Composed { project, facet, modifiers, stance } => {
                write!(f, "{}", project)?;
                if let Some(fa) = facet { write!(f, "/{}", fa)?; }
                for m in modifiers { write!(f, "+{}", m)?; }
                write!(f, "+{}", stance)
            }
        }
    }
}
```

- [ ] **Step 4: Add `thinking_budget` to ResolvedAgent in types.rs**

Read `Fondament/fondament-core/src/types.rs`. Replace `ResolvedAgent`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedAgent {
    pub system_prompt: String,
    pub tools: Vec<crate::tools::ToolDefinition>,
    pub jit_tools: Vec<crate::tools::ToolDefinition>,
    pub default_model: ModelId,
    /// Set when the aporia modifier is active.
    /// Callers must pass this to the Anthropic API as thinking.budget_tokens.
    pub thinking_budget: Option<u32>,
}
```

- [ ] **Step 5: Fix compile errors in resolver.rs**

The resolver builds and returns `ResolvedAgent` — it will fail to compile because the struct now has a new required field. Open `Fondament/fondament-core/src/resolver.rs` and add `thinking_budget: None` to the `Ok(ResolvedAgent { ... })` at the end. (Task 3 will replace this with real logic — for now just set None.)

Look for the final `Ok(ResolvedAgent { ... })` block and add the field:
```rust
Ok(ResolvedAgent {
    system_prompt: layers.join("\n\n"),
    tools: always_on,
    jit_tools,
    default_model,
    thinking_budget: None,  // ← add this line
})
```

Also fix the `CompositionAddress::Composed { project, facet, stance }` match in resolver.rs — add `modifiers: _` or use `..` to avoid compile error from new field:

In the resolver, find:
```rust
if let CompositionAddress::Composed { project, .. } = address {
```
That should already have `..` — but double-check. Also find:
```rust
CompositionAddress::Role { stance_override: Some(stance), .. }
```
— that should be fine. But:
```rust
CompositionAddress::Composed { project, facet, stance }
```
needs to become:
```rust
CompositionAddress::Composed { project, facet, stance, .. }
```

Read resolver.rs and fix any exhaustive pattern matches on `CompositionAddress`.

- [ ] **Step 6: Run address tests**

```bash
cd /Users/bedardpl/project/Fondament && cargo test -p fondament-core 2>&1 | grep "test result"
```

Expected: 7 address tests pass (3 updated + 4 new). All other existing tests still pass. Total ≥ 14.

- [ ] **Step 7: Commit**

```bash
cd /Users/bedardpl/project/Fondament && git add fondament-core/src/address.rs fondament-core/src/types.rs fondament-core/src/resolver.rs fondament-core/tests/address_tests.rs && git commit -m "feat: add modifiers to CompositionAddress and thinking_budget to ResolvedAgent"
```

---

## Task 3: Resolver preamble injection + thinking budget + Cor plugin.toml

**Files:**
- Modify: `Fondament/fondament-core/src/resolver.rs`
- Modify: `Fondament/fondament-core/tests/resolver_tests.rs`
- Create: `Fondament/packages/aporia/plugin.toml`

### What the resolver adds

When `is_aporia` is true:
1. While walking the extends chain, collect `(kind, name)` pairs for non-modifier disciplines
2. Collect the stance (if any)
3. After assembly: build a preamble from the collected parts list
4. Insert preamble as the FIRST layer (index 0 in `layers` vec)
5. Set `thinking_budget = (parts.len() as u32 * 3_000).min(10_000).max(3_000)`

### Preamble format

```
--- injected by aporia discipline ---
You are composed of the following parts:
  - [discipline: system-design]
  - [stance: adversarial]

Before producing any response:
1. Become each part sequentially. Reason from its corpus alone.
2. Name the tensions between parts explicitly.
3. If a gap surfaces that no part of you owns, output it typed:
   GAP { domain: "...", question: "...", blocking: true/false }
4. Recompose. Collapse to your public response from that synthesis.

Your public response reflects the recomposed whole.
The internal debate is yours alone — it does not appear in output.
--- end injection ---
```

- [ ] **Step 1: Write failing resolver tests**

Append to `Fondament/fondament-core/tests/resolver_tests.rs`:

```rust
fn make_tree_with_aporia() -> (DefinitionTree, TempDir) {
    let dir = TempDir::new().unwrap();
    let files: &[(&str, &str)] = &[
        ("disciplines/system-design.yaml",
         "id: disciplines/system-design\nkind: discipline\ncontext: \"You architect systems.\"\n"),
        ("disciplines/aporia.yaml",
         "id: disciplines/aporia\nkind: discipline\nmodifier: true\n"),
        ("roles/platform-architect.yaml",
         "id: fondament/platform-architect\nkind: role\nextends: [disciplines/system-design]\ndefault_model: claude-sonnet-4-6\ncontext: \"You are a platform architect.\"\n"),
        ("stances/adversarial.yaml",
         "id: stances/adversarial\nkind: stance\ncontext: \"Challenge every assumption.\"\n"),
    ];
    for (path, content) in files {
        let full = dir.path().join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, content).unwrap();
    }
    let tree = DefinitionTree::load(dir.path()).unwrap();
    (tree, dir)
}

#[tokio::test]
async fn aporia_modifier_injects_preamble() {
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
    // Preamble must come BEFORE domain content
    let preamble_pos = agent.system_prompt.find("aporia discipline").unwrap();
    let domain_pos = agent.system_prompt.find("platform architect").unwrap();
    assert!(preamble_pos < domain_pos, "preamble must precede domain content");
}

#[tokio::test]
async fn aporia_modifier_sets_thinking_budget() {
    let (tree, _dir) = make_tree_with_aporia();
    let address: CompositionAddress = "fondament/platform-architect+aporia".parse().unwrap();
    let agent = resolve(&address, &tree, &MockFarga, "acme").await.unwrap();
    assert!(agent.thinking_budget.is_some(), "thinking_budget must be set with aporia modifier");
    let budget = agent.thinking_budget.unwrap();
    assert!(budget >= 3_000, "minimum budget is 3000 tokens");
    assert!(budget <= 10_000, "budget is capped at 10000 tokens");
}

#[tokio::test]
async fn without_aporia_no_preamble_no_budget() {
    let (tree, _dir) = make_tree_with_aporia();
    let address: CompositionAddress = "fondament/platform-architect".parse().unwrap();
    let agent = resolve(&address, &tree, &MockFarga, "acme").await.unwrap();
    assert!(
        !agent.system_prompt.contains("aporia discipline"),
        "preamble must not appear without aporia modifier"
    );
    assert!(
        agent.thinking_budget.is_none(),
        "thinking_budget must be None without aporia modifier"
    );
}
```

- [ ] **Step 2: Run to verify failure**

```bash
cd /Users/bedardpl/project/Fondament && cargo test -p fondament-core aporia 2>&1 | grep -E "FAILED|^error" | head -5
```

Expected: FAIL — preamble not injected, thinking_budget always None.

- [ ] **Step 3: Implement preamble injection in resolver.rs**

Read `Fondament/fondament-core/src/resolver.rs` in full. Then replace with:

```rust
use crate::address::CompositionAddress;
use crate::error::{FondamentError, Result};
use crate::farga::FargaReader;
use crate::tools::ToolDefinition;
use crate::tree::DefinitionTree;
use crate::types::{ModelId, ResolvedAgent};

pub async fn resolve(
    address: &CompositionAddress,
    tree: &DefinitionTree,
    farga: &dyn FargaReader,
    org: &str,
) -> Result<ResolvedAgent> {
    let mut layers: Vec<String> = Vec::new();
    let mut default_model = ModelId::default();
    let mut always_on: Vec<ToolDefinition> = Vec::new();
    let mut jit_tools: Vec<ToolDefinition> = Vec::new();
    // Collected for the aporia preamble: (kind, display_name)
    let mut collected_parts: Vec<(String, String)> = Vec::new();

    // Detect aporia modifier upfront (avoids borrow conflict later)
    let is_aporia = match address {
        CompositionAddress::Role { modifiers, .. } => modifiers.iter().any(|m| m == "aporia"),
        CompositionAddress::Composed { modifiers, .. } => modifiers.iter().any(|m| m == "aporia"),
    };

    // Layer 1: org context from Farga
    if let Ok(org_ctx) = farga.org_layer(org).await {
        if !org_ctx.content.is_empty() {
            layers.push(format!("## Organization Context\n{}", org_ctx.content));
        }
    }

    // Layer 2: initiative context from Farga
    if let Ok(initiatives) = farga.initiative_layer(org).await {
        for init in initiatives {
            if !init.content.is_empty() {
                layers.push(format!("## Strategic Initiative\n{}", init.content));
            }
        }
    }

    // Layer 3: project context (for Composed addresses)
    if let CompositionAddress::Composed { project, facet, .. } = address {
        if let Ok(proj_ctx) = farga.project_layer(project).await {
            if !proj_ctx.content.is_empty() {
                layers.push(format!("## Project Context\n{}", proj_ctx.content));
                let domain_name = facet.as_deref().unwrap_or(project.as_str()).to_string();
                collected_parts.push(("domain".into(), domain_name));
            }
        }
    }

    // Layer 4+: Fondament definition layers — walk extends chain
    let role_id = match address {
        CompositionAddress::Role { role, .. } => role.clone(),
        CompositionAddress::Composed { project, facet, stance, .. } => {
            format!("roles/{}-{}", facet.as_deref().unwrap_or(project), stance)
        }
    };

    let mut to_visit = vec![role_id];
    let mut visited = std::collections::HashSet::new();

    while let Some(id) = to_visit.pop() {
        if visited.contains(&id) {
            return Err(FondamentError::CircularExtends(id));
        }
        visited.insert(id.clone());

        if let Some(def) = tree.get(&id) {
            // Collect non-modifier disciplines as parts for the aporia preamble
            if def.kind == "discipline" && !def.modifier {
                let part_name = id.strip_prefix("disciplines/").unwrap_or(&id).to_string();
                collected_parts.push(("discipline".into(), part_name));
            }

            if let Some(ctx) = &def.context {
                if !ctx.is_empty() {
                    layers.push(ctx.clone());
                }
            }
            if let Some(model) = &def.default_model {
                default_model = model.clone();
            }
            always_on.extend(def.tools.always_on.clone());
            jit_tools.extend(def.tools.jit.clone());

            for parent in def.extends.iter().rev() {
                to_visit.push(parent.clone());
            }
        }
    }

    // Layer: stance
    let stance = match address {
        CompositionAddress::Role { stance_override: Some(s), .. } => Some(s.clone()),
        CompositionAddress::Composed { stance, .. } => Some(stance.clone()),
        _ => None,
    };
    if let Some(ref s) = stance {
        if let Some(stance_def) = tree.get(&format!("stances/{}", s)) {
            if let Some(ctx) = &stance_def.context {
                if !ctx.is_empty() {
                    layers.push(ctx.clone());
                    collected_parts.push(("stance".into(), s.clone()));
                }
            }
        }
    }

    // Aporia preamble: inject FIRST if modifier is active
    let thinking_budget = if is_aporia {
        let preamble = build_aporia_preamble(&collected_parts);
        layers.insert(0, preamble);
        let budget = (collected_parts.len() as u32 * 3_000).min(10_000).max(3_000);
        Some(budget)
    } else {
        None
    };

    Ok(ResolvedAgent {
        system_prompt: layers.join("\n\n"),
        tools: always_on,
        jit_tools,
        default_model,
        thinking_budget,
    })
}

fn build_aporia_preamble(parts: &[(String, String)]) -> String {
    let mut preamble = String::from(
        "--- injected by aporia discipline ---\nYou are composed of the following parts:\n"
    );
    if parts.is_empty() {
        preamble.push_str("  - [role: this agent] — reason from your full context\n");
    } else {
        for (kind, name) in parts {
            preamble.push_str(&format!("  - [{}: {}]\n", kind, name));
        }
    }
    preamble.push_str(
        "\nBefore producing any response:\n\
         1. Become each part sequentially. Reason from its corpus alone.\n\
         2. Name the tensions between parts explicitly.\n\
         3. If a gap surfaces that no part of you owns, output it typed:\n\
            GAP { domain: \"...\", question: \"...\", blocking: true/false }\n\
         4. Recompose. Collapse to your public response from that synthesis.\n\
         \n\
         Your public response reflects the recomposed whole.\n\
         The internal debate is yours alone — it does not appear in output.\n\
         --- end injection ---"
    );
    preamble
}
```

- [ ] **Step 4: Run all tests**

```bash
cd /Users/bedardpl/project/Fondament && cargo test 2>&1 | grep "test result"
```

Expected: ≥ 17 tests pass across all test files (10 original + 2 tree + 4 address + 3 resolver).
All 0 failures.

- [ ] **Step 5: Create Cor plugin.toml**

```bash
mkdir -p /Users/bedardpl/project/Fondament/packages/aporia
```

Create `Fondament/packages/aporia/plugin.toml`:

```toml
[plugin]
id = "aporia"
version = "0.1.0"
kind = "discipline"
name = "Aporia"
description = "A reasoning modifier that instructs agents to decompose into constituent parts before collapse. Enables extended thinking. Empirically outperforms crystallization in all tested comparisons."
authors = ["Pierre-Luc Bedard <bedardpl@gmail.com>"]
license = "MIT"
repository = "gitlab.com/cor912026/fondament"

[compatibility]
stack = "occitan"
providers = ["anthropic"]

[artifact]
path = "aporia.yaml"

[install]
target = "Fondament/definitions/disciplines/"
```

Create `Fondament/packages/aporia/aporia.yaml` (copy of the discipline file — this is what `cor install` would unpack):

```yaml
id: disciplines/aporia
kind: discipline
modifier: true
```

- [ ] **Step 6: Commit**

```bash
cd /Users/bedardpl/project/Fondament && git add fondament-core/src/resolver.rs fondament-core/tests/resolver_tests.rs packages/aporia/ && git commit -m "feat: inject aporia preamble and thinking budget in resolver; add Cor plugin package"
```

---

## Self-Review

**Spec coverage:**

| Spec requirement | Covered by |
|---|---|
| `fondament/disciplines/aporia.yaml` | Task 1 |
| Detect aporia in address, inject preamble | Task 3 resolver |
| Preamble lists agent's actual parts (disciplines + stance) | Task 3 `collected_parts` |
| Extended thinking budget — 3000 per part, capped at 10000 | Task 3 thinking_budget calc |
| Modifier not treated as corpus/domain | Task 3 `if !def.modifier` guard |
| `CompositionAddress` handles `+aporia+stance` | Task 2 parser |
| Cor plugin package (`plugin.toml`) | Task 3 step 5 |

**Placeholder scan:** None found.

**Type consistency check:**
- `CompositionAddress::Composed { modifiers, .. }` added in Task 2 → used in Task 3 resolver — consistent
- `ResolvedAgent.thinking_budget: Option<u32>` added in Task 2 → set in Task 3 → tested in resolver_tests — consistent
- `DefinitionFile.modifier: bool` added in Task 1 → checked as `!def.modifier` in Task 3 resolver → consistent
- `build_aporia_preamble(parts: &[(String, String)])` defined and called in Task 3 within same file — consistent
- `collected_parts.push(("discipline".into(), part_name))` in resolver → displayed as `[discipline: X]` in preamble — consistent

**Note on extended thinking and the API call:** `ResolvedAgent.thinking_budget` is a *signal* to the dispatch layer (Charradissa or whatever calls Fondament) to enable `thinking: { type: "enabled", budget_tokens: N }` on the Anthropic API request. This plan stops at making the budget available in `ResolvedAgent` — actually threading it into the API call is Charradissa's concern (dispatch.rs or tool_loop.rs). The plan does NOT implement that wiring to stay within scope.
