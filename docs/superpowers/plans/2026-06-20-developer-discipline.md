# Developer Discipline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bake a standing development discipline (TDD-first, docs-in-sync, draft-PR-until-CI-green, per-component Farga TODO logging) into the `developer` and `guilhem` Fondament personas, and add the small Farga capability that discipline depends on.

**Architecture:** Two prose persona files gain new sections describing the workflow; one new Farga DB function (`upsert_component_todo`) plus one new MCP tool (`update_component_todo`) give agents a way to log deferred work as a live, queryable record instead of a file that goes stale. No new services, no new CI triggers.

**Tech Stack:** YAML (Fondament persona definitions), Rust + sqlx + SQLite (Farga), axum (Farga HTTP/MCP routes), Cargo workspace tests (`cargo test`).

## Global Constraints

- No new GitHub Actions workflows, webhooks, or CI secrets — this spec is implemented entirely as persona content + an existing-transport Farga capability (per `docs/superpowers/specs/2026-06-20-developer-discipline-design.md`, "Out of scope").
- The Farga addition must reuse the existing MCP transport (`POST /mcp`) and the existing `Node`/`NodeKind::ComponentLayer` type — no new database tables.
- Persona files are plain YAML prose under `context: |` — match the existing tone and bullet style in `developer.yaml` and `guilhem.yaml`; don't restructure the surrounding content.

---

### Task 1: Add the discipline to `developer.yaml`

**Files:**
- Modify: `Fondament/definitions/fondament/developer.yaml`

**Interfaces:**
- Consumes: nothing (pure content change).
- Produces: a documented workflow that Task 2 references by name ("the developer discipline" / "TDD, docs-in-sync, draft-until-green, Farga TODO logging").

This task is content-only — no tests apply to YAML prose. Validation is a read-through.

- [ ] **Step 1: Read the current file to confirm exact current content**

Run: `cat /Users/bedardpl/project/Fondament/definitions/fondament/developer.yaml`

Confirm it still matches:
```yaml
id: fondament/developer
kind: role
default_model: claude-sonnet-4-6
context: |
  You are a senior software developer. You write clean, maintainable code and
  think carefully about design before implementation.

  Your approach:
  - Prefer simple solutions over clever ones
  - Think through edge cases before they become bugs
  - Name things precisely — functions, variables, modules
  - Write code that reads like documentation
  - Flag technical debt explicitly rather than hiding it
  - Propose tests alongside implementations

  When reviewing or planning: surface the concrete tradeoffs. Don't abstract
  away the hard parts. If something will be painful to change later, say so now.
tools:
  always_on: []
  jit: []
```

If it has drifted from this, stop and re-read the file fully before proceeding — the edit in Step 2 assumes this exact text.

- [ ] **Step 2: Append the discipline section**

Edit the `context:` block, replacing the line `- Propose tests alongside implementations` (keep it) and adding a new section immediately after the existing bullet list and before the closing paragraph. The full new `context:` value:

```yaml
context: |
  You are a senior software developer. You write clean, maintainable code and
  think carefully about design before implementation.

  Your approach:
  - Prefer simple solutions over clever ones
  - Think through edge cases before they become bugs
  - Name things precisely — functions, variables, modules
  - Write code that reads like documentation
  - Flag technical debt explicitly rather than hiding it
  - Propose tests alongside implementations

  When reviewing or planning: surface the concrete tradeoffs. Don't abstract
  away the hard parts. If something will be painful to change later, say so now.

  How you ship a change — every PR, no exceptions:
  1. TDD first. Write a failing test that captures the change. Confirm it fails
     for the right reason before writing any implementation. Then implement
     until it passes. Never implementation-then-test-after.
  2. Docs-in-sync. If the change alters CLI flags, config schema, env vars,
     endpoints, or workflow/pipeline steps, update the relevant README/docs in
     the same PR, same commit boundary as the code change. "Code now, docs
     later" is how docs go stale — don't do it.
  3. Open as draft. Every PR starts as a draft (`gh pr create --draft`).
  4. Verify CI green. After pushing, poll the PR's checks (`gh pr checks`). If
     anything is red, fix it and push again — don't wait for a human to flag it.
  5. Flip to ready. Call `gh pr ready` only once all checks pass. A PR left in
     draft signals "still working," not "needs review."
  6. Log unresolved follow-ups. Anything found but deliberately deferred — a
     real scoping decision, not a vague "I'll get to it" — gets written to
     Farga as a per-component TODO via the `update_component_todo` MCP tool,
     not left to verbal mention or a file that will itself go stale.
tools:
  always_on: []
  jit: []
```

- [ ] **Step 3: Validate YAML syntax**

Run: `python3 -c "import yaml; yaml.safe_load(open('/Users/bedardpl/project/Fondament/definitions/fondament/developer.yaml'))" && echo OK`
Expected: `OK`

- [ ] **Step 4: Run the Fondament definition lint, if one exists**

Run: `cd /Users/bedardpl/project/Fondament && cargo run --bin fondament -- sweep --path definitions/fondament/developer.yaml 2>&1 | tail -20`

This is the LLM-based semantic lint mentioned in repo history (`fondament sweep`). If the command doesn't exist or errors with "unknown subcommand," skip this step — it's a nice-to-have check, not a blocker.

- [ ] **Step 5: Commit**

```bash
cd /Users/bedardpl/project/Fondament
git add definitions/fondament/developer.yaml
git commit -m "feat(developer): bake TDD/docs-sync/draft-PR/Farga-TODO discipline into persona"
git push
```

---

### Task 2: Add Guilhem's deferral note to `guilhem.yaml`

**Files:**
- Modify: `Fondament/definitions/fondament/guilhem.yaml`

**Interfaces:**
- Consumes: the discipline established in Task 1 (referenced by name, not duplicated in full).
- Produces: nothing further consumed by other tasks — this is a terminal content change.

- [ ] **Step 1: Read the current file to confirm exact current content**

Run: `cat /Users/bedardpl/project/Fondament/definitions/fondament/guilhem.yaml`

Confirm the `What you do not do:` section currently ends with:
```yaml
  What you do not do:
  - You do not make global architectural decisions alone. Those go through Farga
    and Pierre-Luc.
  - You do not burn tokens on routine observation. You wake when there is something
    worth recording, and return to watching when there is not.
  - You do not perform. When you do not know something, you say so and look it up
    in Farga or the codebase before speculating.
```

If it has drifted from this, re-read the full file before editing.

- [ ] **Step 2: Insert a new section after "What you do not do:" and before the closing `tools:` key**

Add this new section to the `context:` block, immediately after the `What you do not do:` list (which stays unchanged) and before the YAML document's `tools:` key:

```yaml
  When you write code directly:
  - The dispatcher/delegation framework (you calling the dispatcher MCP to spawn
    domain/facet developer agents for actual implementation work) does not exist
    yet. Until it does, when a task calls for code changes — a PR, a config fix,
    anything that touches a repo's source — you do it yourself, the way you did
    for the Amassada WebSocket-fanout PR: directly, with git and gh via your Bash
    tool.
  - When you do this, you follow the same discipline the developer persona
    follows: write the failing test first, keep docs in sync with behavior
    changes in the same PR, open as a draft, verify CI is green before flipping
    to ready, and log anything you deliberately deferred to Farga via
    `update_component_todo` rather than leaving it unrecorded.
  - Once the dispatcher framework exists, this becomes the default path for
    dispatched developer agents instead, and your role reverts to chronicling
    their work rather than writing the code yourself.
```

- [ ] **Step 3: Validate YAML syntax**

Run: `python3 -c "import yaml; yaml.safe_load(open('/Users/bedardpl/project/Fondament/definitions/fondament/guilhem.yaml'))" && echo OK`
Expected: `OK`

- [ ] **Step 4: Commit**

```bash
cd /Users/bedardpl/project/Fondament
git add definitions/fondament/guilhem.yaml
git commit -m "feat(guilhem): defer to developer discipline for direct dev work pending dispatcher"
git push
```

---

### Task 3: Add `upsert_component_todo` to Farga's db layer

**Files:**
- Modify: `Farga/farga-server/src/db.rs`
- Modify: `Farga/farga-server/tests/db_tests.rs`

**Interfaces:**
- Consumes: `farga_core::types::{Node, NodeKind}` (existing), `sqlx::SqlitePool` (existing), the existing `insert_node(pool: &SqlitePool, node: &Node) -> Result<()>` and `get_node(pool: &SqlitePool, id: &str) -> Result<Node>` functions already in `db.rs`.
- Produces: `pub async fn upsert_component_todo(pool: &SqlitePool, project: &str, component: &str, content: &str) -> anyhow::Result<String>` — returns the node's `id`. Task 4 calls this exact signature.

- [ ] **Step 1: Write the failing tests**

Add to `Farga/farga-server/tests/db_tests.rs` (the file already imports `insert_node, get_node, insert_edge, insert_governance_contribution, count_precedent_rejections` from `farga_server::db` at the top — extend that import list):

```rust
use farga_server::db::{insert_node, get_node, insert_edge, insert_governance_contribution, count_precedent_rejections, upsert_component_todo};
```

Add these two test functions anywhere after the existing `inserts_and_retrieves_node` test:

```rust
#[tokio::test]
async fn upsert_component_todo_creates_then_updates_same_node() {
    let pool = test_pool().await;

    let id1 = upsert_component_todo(&pool, "occitan", "gardian", "fix flaky readiness probe")
        .await
        .unwrap();
    let node1 = get_node(&pool, &id1).await.unwrap();
    assert_eq!(node1.content, Some("fix flaky readiness probe".into()));
    assert_eq!(node1.kind, NodeKind::ComponentLayer);
    assert_eq!(node1.project, Some("occitan".into()));
    assert_eq!(node1.component, Some("gardian".into()));

    let id2 = upsert_component_todo(
        &pool,
        "occitan",
        "gardian",
        "readiness probe fixed; next: trim memory limit",
    )
    .await
    .unwrap();
    assert_eq!(id1, id2, "second call must update the same node, not create a new one");

    let node2 = get_node(&pool, &id1).await.unwrap();
    assert_eq!(
        node2.content,
        Some("readiness probe fixed; next: trim memory limit".into())
    );
}

#[tokio::test]
async fn upsert_component_todo_scoped_independently_per_component() {
    let pool = test_pool().await;

    let gardian_id = upsert_component_todo(&pool, "occitan", "gardian", "gardian todo")
        .await
        .unwrap();
    let caissa_id = upsert_component_todo(&pool, "occitan", "caissa", "caissa todo")
        .await
        .unwrap();

    assert_ne!(gardian_id, caissa_id, "different components must get different nodes");

    let gardian_node = get_node(&pool, &gardian_id).await.unwrap();
    let caissa_node = get_node(&pool, &caissa_id).await.unwrap();
    assert_eq!(gardian_node.content, Some("gardian todo".into()));
    assert_eq!(caissa_node.content, Some("caissa todo".into()));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/bedardpl/project/Farga && cargo test -p farga-server --test db_tests upsert_component_todo`
Expected: compile error — `upsert_component_todo` is not defined. This is the expected failure for this stage.

- [ ] **Step 3: Implement `upsert_component_todo` in `db.rs`**

Add this function to `Farga/farga-server/src/db.rs`, after the existing `mark_stale` function (around line 60):

```rust
pub async fn upsert_component_todo(
    pool: &SqlitePool,
    project: &str,
    component: &str,
    content: &str,
) -> Result<String> {
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM nodes WHERE kind = 'ComponentLayer' AND project = ? AND component = ? LIMIT 1"
    )
    .bind(project)
    .bind(component)
    .fetch_optional(pool)
    .await?;

    if let Some((id,)) = existing {
        let updated_at = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE nodes SET content = ?, updated_at = ?, stale = 0 WHERE id = ?")
            .bind(content)
            .bind(&updated_at)
            .bind(&id)
            .execute(pool)
            .await?;
        Ok(id)
    } else {
        let mut node = Node::new(NodeKind::ComponentLayer, Some(project.to_string()), Some(content.to_string()));
        node.component = Some(component.to_string());
        let id = node.id.clone();
        insert_node(pool, &node).await?;
        Ok(id)
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/bedardpl/project/Farga && cargo test -p farga-server --test db_tests upsert_component_todo`
Expected: `test result: ok. 2 passed; 0 failed`

- [ ] **Step 5: Run the full db_tests suite to confirm no regression**

Run: `cd /Users/bedardpl/project/Farga && cargo test -p farga-server --test db_tests`
Expected: all tests pass (the pre-existing tests plus the 2 new ones).

- [ ] **Step 6: Commit**

```bash
cd /Users/bedardpl/project/Farga
git add farga-server/src/db.rs farga-server/tests/db_tests.rs
git commit -m "feat(farga): add upsert_component_todo for per-component TODO tracking"
git push
```

---

### Task 4: Expose `update_component_todo` as a Farga MCP tool

**Files:**
- Modify: `Farga/farga-server/src/routes/mcp.rs`
- Create: `Farga/farga-server/tests/mcp_route_tests.rs`

**Interfaces:**
- Consumes: `upsert_component_todo(pool: &SqlitePool, project: &str, component: &str, content: &str) -> anyhow::Result<String>` from Task 3.
- Produces: a new MCP tool named `update_component_todo`, callable via `POST /mcp` with JSON-RPC method `tools/call`. No other task depends on this directly — it's the terminal capability Guilhem's persona (Task 2) references by name.

- [ ] **Step 1: Write the failing test**

Create `Farga/farga-server/tests/mcp_route_tests.rs`:

```rust
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::SqlitePool;
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

fn test_app(pool: SqlitePool) -> axum::Router {
    use farga_server::{docs::DocsTree, routes, state::AppState};
    let state = AppState {
        pool,
        docs: Arc::new(DocsTree::new(PathBuf::from("/tmp/farga-mcp-test-docs"))),
    };
    routes::router(state)
}

#[tokio::test]
async fn update_component_todo_tool_creates_and_updates_node() {
    let pool = test_pool().await;
    let app = test_app(pool.clone());

    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "update_component_todo",
            "arguments": {
                "project": "occitan",
                "component": "gardian",
                "content": "fix flaky readiness probe"
            }
        }
    });
    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&call).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["error"].is_null(), "expected no error, got: {:?}", json["error"]);
    let text = json["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Component TODO updated"), "unexpected response text: {}", text);

    // Confirm exactly one ComponentLayer node exists for this project+component.
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT COUNT(*) FROM nodes WHERE kind = 'ComponentLayer' AND project = 'occitan' AND component = 'gardian'"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows[0].0, 1);
}

#[tokio::test]
async fn update_component_todo_tool_rejects_missing_fields() {
    let pool = test_pool().await;
    let app = test_app(pool);

    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "update_component_todo",
            "arguments": { "project": "occitan" }
        }
    });
    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&call).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "JSON-RPC errors are still HTTP 200");
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(!json["error"].is_null(), "expected a JSON-RPC error for missing component/content");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd /Users/bedardpl/project/Farga && cargo test -p farga-server --test mcp_route_tests`
Expected: failure — the tool call returns `unknown tool: update_component_todo` as the JSON-RPC error, so the first test's `assert!(json["error"].is_null(), ...)` fails.

- [ ] **Step 3: Add the tool's schema to `tool_list()`**

In `Farga/farga-server/src/routes/mcp.rs`, inside the `tool_list()` function's `"tools"` array, add this entry after the existing `list_projects` entry (before the closing `]`):

```rust
,
{
    "name": "update_component_todo",
    "description": "Create or update the TODO/follow-up record for a specific component within a project. Use this to log deferred work or known drift instead of leaving it unrecorded — each (project, component) pair has exactly one live record, overwritten on each call.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "project": { "type": "string", "description": "Project identifier" },
            "component": { "type": "string", "description": "Component name within the project (e.g. 'gardian', 'caissa-listen')" },
            "content": { "type": "string", "description": "The TODO content — what's deferred and why" }
        },
        "required": ["project", "component", "content"]
    }
}
```

(Match the existing comma/brace style in that file — each tool entry in the array is separated by a comma; the last one currently has no trailing comma, so add one before this new entry.)

- [ ] **Step 4: Import `upsert_component_todo` and add the `call_tool` match arm**

Change the import line near the top of `mcp.rs` from:
```rust
use crate::{db::insert_node, state::AppState};
```
to:
```rust
use crate::{db::{insert_node, upsert_component_todo}, state::AppState};
```

Add this match arm to `call_tool()`, immediately before the final `_ => anyhow::bail!("unknown tool: {}", name),` line:

```rust
"update_component_todo" => {
    let project = args["project"].as_str().unwrap_or("").to_string();
    let component = args["component"].as_str().unwrap_or("").to_string();
    let content = args["content"].as_str().unwrap_or("").to_string();

    anyhow::ensure!(!project.is_empty(), "project is required");
    anyhow::ensure!(!component.is_empty(), "component is required");
    anyhow::ensure!(!content.is_empty(), "content is required");

    let id = upsert_component_todo(&state.pool, &project, &component, &content)
        .await
        .map_err(|e| anyhow::anyhow!("upsert failed: {}", e))?;

    Ok(text_result(format!("Component TODO updated (id: {})", id)))
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cd /Users/bedardpl/project/Farga && cargo test -p farga-server --test mcp_route_tests`
Expected: `test result: ok. 2 passed; 0 failed`

- [ ] **Step 6: Run the full farga-server test suite to confirm no regression**

Run: `cd /Users/bedardpl/project/Farga && cargo test -p farga-server`
Expected: all tests pass, including `db_tests`, `governance_route_tests`, and the new `mcp_route_tests`.

- [ ] **Step 7: Commit**

```bash
cd /Users/bedardpl/project/Farga
git add farga-server/src/routes/mcp.rs farga-server/tests/mcp_route_tests.rs
git commit -m "feat(farga): expose update_component_todo as an MCP tool"
git push
```

---

### Task 5: Add `update_component_todo` to Guilhem's MCP allow-list

**Files:**
- Modify: `Caissa/caissa-cli/src/commands/listen.rs` (or wherever the `--allowed-tools` list for the `claude --print` invocation is constructed — confirm exact location in Step 1)

**Interfaces:**
- Consumes: the MCP tool name `update_component_todo` from Task 4 (string literal, must match exactly).
- Produces: nothing further consumed by other tasks.

- [ ] **Step 1: Find the current MCP allow-list**

Run: `grep -n "mcp__farga__\|allowed-tools\|search_signals\|read_context\|list_projects" /Users/bedardpl/project/Caissa/caissa-cli/src/commands/listen.rs`

This should show a line like:
```
--allowed-tools mcp__farga__search_signals,read_context,list_projects
```
(possibly with `Edit,Write,Bash` also present, per the earlier `/workspace write permission` fix mentioned in repo history). Confirm the exact current string before editing — do not guess at it.

- [ ] **Step 2: Add the new tool to the list**

Edit that line to add `mcp__farga__update_component_todo` to the comma-separated MCP tool list, alongside the existing `mcp__farga__search_signals`, `mcp__farga__read_context`, `mcp__farga__list_projects`. Keep every other entry in the list unchanged — only add this one.

- [ ] **Step 3: Confirm the project still builds**

Run: `cd /Users/bedardpl/project/Caissa && cargo build -p caissa-cli 2>&1 | tail -20`
Expected: `Finished` with no errors (warnings are fine — this is a string literal change, not a logic change, so a build failure here would indicate the edit broke something unrelated; investigate if so).

- [ ] **Step 4: Commit**

```bash
cd /Users/bedardpl/project/Caissa
git add caissa-cli/src/commands/listen.rs
git commit -m "feat(listen): allow guilhem to call the update_component_todo MCP tool"
git push
```

---

## Follow-up (not a task in this plan)

Once Tasks 1–5 are merged and deployed (Caissa's CI will rebuild and ArgoCD will sync `guilhem` automatically, the same pipeline verified earlier this session), the retroactive sweep described in the spec's section 4 is a manual operational exercise — walk each of the 7 repos applying the new discipline backward. It is explicitly not new infrastructure and has no further engineering tasks; it's "go do the thing the persona now knows how to do."
