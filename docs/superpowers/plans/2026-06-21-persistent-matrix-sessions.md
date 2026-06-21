# Persistent Per-Room Matrix Sessions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the one-shot `claude --print`-per-message model in Guilhem's Matrix presence with a persistent Node.js sidecar (Claude Agent SDK) per actively-chatting room, supervised from his existing pod, fixing both the continuity gap and the per-message cold-start latency.

**Architecture:** `listen.rs` becomes a process supervisor holding one long-lived child process per active room; each child runs a small Node.js script that opens a Claude Agent SDK session and answers messages fed to it over stdin/stdout, reusing the SDK's `resume` session-continuity mechanism instead of re-sending full room history as text on every turn.

**Tech Stack:** Rust (`tokio::process`, `tokio::sync::RwLock`), Node.js (`@anthropic-ai/claude-agent-sdk`), newline-delimited JSON over stdin/stdout for the Rust↔Node IPC.

## Global Constraints

- Per-room process isolation (not a shared multi-room process) — confirmed choice in the spec.
- Idle timeout default: 30 minutes, configurable via `caissa.toml`.
- `room_sessions` is in-memory only — no persistence across pod restarts (explicit spec choice).
- npm package: `@anthropic-ai/claude-agent-sdk`, imported as `import { query } from "@anthropic-ai/claude-agent-sdk"`.
- Session continuity via the SDK's stable `resume` option (a captured `session_id` string), not the `unstable_v2_*` preview API — the preview API is explicitly unstable and not a foundation to build on.
- Skills field on Fondament personas is a flat list (no always_on/jit split, unlike `tools`) — matches the SDK's `skills` option, which takes a plain array of names.

---

### Task 1: Add `skills` field to all 6 Fondament facet files

**Files:**
- Modify: `Fondament/definitions/fondament/developer.yaml`
- Modify: `Fondament/definitions/fondament/infra-engineer.yaml`
- Modify: `Fondament/definitions/fondament/qa-engineer.yaml`
- Modify: `Fondament/definitions/fondament/security-analyst.yaml`
- Modify: `Fondament/definitions/fondament/app-architect.yaml`
- Modify: `Fondament/definitions/fondament/data-architect.yaml`
- Modify: `Fondament/definitions/fondament/guilhem.yaml`

**Interfaces:**
- Consumes: nothing (content-only, mirrors Task 1 of the prior `2026-06-21-dispatched-agent-credentials.md` plan).
- Produces: nothing consumed by other tasks' code directly — Task 2's sidecar reads this list at runtime via the same manual-relay model as `tools.always_on` (the caller passes it through, not a Fondament-resolver integration; this is the spec's explicit scope boundary).

This task is content-only — no automated tests beyond YAML syntax validation.

- [ ] **Step 1: Confirm each file's current trailing block ends with `tools: { always_on: [...], jit: [] }`**

Run: `tail -10 /Users/bedardpl/project/Fondament/definitions/fondament/developer.yaml`
Expected: ends with the 6-entry `always_on` list added earlier this session, then `jit: []`. If any of the 7 files (6 facets + guilhem.yaml) has drifted, read it fully before editing.

- [ ] **Step 2: Append a `skills` field after `tools:` in each file**

For `developer.yaml` and `infra-engineer.yaml`, after the existing `tools:` block, add:
```yaml
skills:
  - superpowers:test-driven-development
  - superpowers:systematic-debugging
  - superpowers:requesting-code-review
  - superpowers:receiving-code-review
  - superpowers:finishing-a-development-branch
  - superpowers:verification-before-completion
  - superpowers:using-git-worktrees
```

For `qa-engineer.yaml`, add:
```yaml
skills:
  - superpowers:test-driven-development
  - superpowers:systematic-debugging
  - superpowers:verification-before-completion
```

For `security-analyst.yaml`, add:
```yaml
skills:
  - superpowers:systematic-debugging
  - superpowers:receiving-code-review
  - superpowers:verification-before-completion
```

For `app-architect.yaml` and `data-architect.yaml`, add:
```yaml
skills:
  - superpowers:brainstorming
  - superpowers:writing-plans
```

For `guilhem.yaml`, add (after its `tools:` block):
```yaml
skills:
  - superpowers:brainstorming
  - superpowers:writing-plans
  - superpowers:subagent-driven-development
  - superpowers:test-driven-development
  - superpowers:systematic-debugging
  - superpowers:requesting-code-review
  - superpowers:finishing-a-development-branch
  - superpowers:verification-before-completion
  - superpowers:using-git-worktrees
```

- [ ] **Step 3: Validate YAML syntax on all 7 files**

Run:
```bash
python3 -c "
import yaml
for f in ['developer','infra-engineer','qa-engineer','security-analyst','app-architect','data-architect','guilhem']:
    d = yaml.safe_load(open(f'/Users/bedardpl/project/Fondament/definitions/fondament/{f}.yaml'))
    assert isinstance(d['skills'], list) and len(d['skills']) > 0, f
print('OK')
"
```
Expected: `OK`

- [ ] **Step 4: Commit**

```bash
cd /Users/bedardpl/project/Fondament
git add definitions/fondament/developer.yaml definitions/fondament/infra-engineer.yaml definitions/fondament/qa-engineer.yaml definitions/fondament/security-analyst.yaml definitions/fondament/app-architect.yaml definitions/fondament/data-architect.yaml definitions/fondament/guilhem.yaml
git commit -m "feat(fondament): add skills field declaring per-persona Superpowers skills"
git push
```

---

### Task 2: `agent-sidecar.js` — the persistent SDK session process

**Files:**
- Create: `Caissa/sandbox/agent-sidecar.js`
- Create: `Caissa/sandbox/agent-sidecar.test.js` (Node's built-in `node:test` runner — no new dependency needed for this)

**Interfaces:**
- Consumes: `@anthropic-ai/claude-agent-sdk`'s `query()` function (external dependency, added in Task 3).
- Produces: a stdin/stdout protocol Task 4's Rust supervisor depends on exactly:
  - **First line on stdin** (init): `{"systemPrompt": string, "model": string, "allowedTools": string[], "skills": string[], "mcpServers": object}`.
  - **Subsequent lines on stdin** (messages): `{"sender": string, "content": string}`.
  - **One line on stdout per message processed**: `{"reply": string}` on success, or `{"error": string}` on failure. The process never writes anything else to stdout (logs, if any, must go to stderr) — Task 4's supervisor reads stdout line-by-line and treats each line as one JSON response.

- [ ] **Step 1: Write the failing test**

Create `Caissa/sandbox/agent-sidecar.test.js`:

```javascript
const test = require('node:test');
const assert = require('node:assert');
const { parseInitLine, parseMessageLine, formatReply, formatError } = require('./agent-sidecar.js');

test('parseInitLine extracts systemPrompt, model, allowedTools, skills, mcpServers', () => {
  const line = JSON.stringify({
    systemPrompt: 'You are Guilhem.',
    model: 'claude-sonnet-4-6',
    allowedTools: ['Bash', 'mcp__farga__search_signals'],
    skills: ['superpowers:systematic-debugging'],
    mcpServers: { farga: { type: 'http', url: 'http://farga:7500/mcp' } },
  });
  const init = parseInitLine(line);
  assert.strictEqual(init.systemPrompt, 'You are Guilhem.');
  assert.strictEqual(init.model, 'claude-sonnet-4-6');
  assert.deepStrictEqual(init.allowedTools, ['Bash', 'mcp__farga__search_signals']);
  assert.deepStrictEqual(init.skills, ['superpowers:systematic-debugging']);
  assert.deepStrictEqual(init.mcpServers, { farga: { type: 'http', url: 'http://farga:7500/mcp' } });
});

test('parseMessageLine extracts sender and content', () => {
  const line = JSON.stringify({ sender: '@pierre-luc:occitane.guilhem', content: 'hello' });
  const msg = parseMessageLine(line);
  assert.strictEqual(msg.sender, '@pierre-luc:occitane.guilhem');
  assert.strictEqual(msg.content, 'hello');
});

test('formatReply produces a single-line JSON object with a reply field', () => {
  const line = formatReply('the response text');
  assert.strictEqual(JSON.parse(line).reply, 'the response text');
  assert.ok(!line.includes('\n'), 'reply line must not contain embedded newlines');
});

test('formatError produces a single-line JSON object with an error field', () => {
  const line = formatError('boom');
  assert.strictEqual(JSON.parse(line).error, 'boom');
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd /Users/bedardpl/project/Caissa/sandbox && node --test agent-sidecar.test.js`
Expected: FAIL — `agent-sidecar.js` does not exist yet (`Cannot find module './agent-sidecar.js'`).

- [ ] **Step 3: Write `agent-sidecar.js`**

Create `Caissa/sandbox/agent-sidecar.js`:

```javascript
#!/usr/bin/env node
const readline = require('node:readline');

function parseInitLine(line) {
  const obj = JSON.parse(line);
  return {
    systemPrompt: obj.systemPrompt,
    model: obj.model,
    allowedTools: obj.allowedTools || [],
    skills: obj.skills || [],
    mcpServers: obj.mcpServers || {},
  };
}

function parseMessageLine(line) {
  const obj = JSON.parse(line);
  return { sender: obj.sender, content: obj.content };
}

function formatReply(text) {
  return JSON.stringify({ reply: text });
}

function formatError(message) {
  return JSON.stringify({ error: message });
}

async function main() {
  // Lazy require so the test file (which only needs the pure functions above)
  // doesn't need the SDK installed to run.
  const { query } = require('@anthropic-ai/claude-agent-sdk');

  const rl = readline.createInterface({ input: process.stdin, terminal: false });
  const lines = [];
  rl.on('line', (line) => lines.push(line));

  await new Promise((resolve) => rl.once('close', resolve));
  // NOTE: this buffers all input before processing, which only works for a
  // finite test harness. The real run loop (below) processes lines as they
  // arrive instead — this main() is replaced by runLoop() in production use,
  // kept separate so the pure parse/format functions stay testable without
  // a live stdin stream.
}

async function runLoop() {
  const { query } = require('@anthropic-ai/claude-agent-sdk');
  const rl = readline.createInterface({ input: process.stdin, terminal: false });

  let init = null;
  let sessionId = null;

  for await (const line of rl) {
    if (!init) {
      init = parseInitLine(line);
      continue;
    }

    const msg = parseMessageLine(line);
    const prompt = `${msg.sender}: ${msg.content}`;

    try {
      let replyText = '';
      const options = {
        model: init.model,
        systemPrompt: init.systemPrompt,
        allowedTools: init.allowedTools,
        skills: init.skills,
        mcpServers: init.mcpServers,
      };
      if (sessionId) {
        options.resume = sessionId;
      }

      for await (const message of query({ prompt, options })) {
        if (message.type === 'system' && message.session_id) {
          sessionId = message.session_id;
        }
        if (message.type === 'assistant') {
          for (const block of message.message.content) {
            if ('text' in block) {
              replyText += block.text;
            }
          }
        }
      }

      process.stdout.write(formatReply(replyText) + '\n');
    } catch (err) {
      process.stdout.write(formatError(err.message || String(err)) + '\n');
    }
  }
}

module.exports = { parseInitLine, parseMessageLine, formatReply, formatError };

if (require.main === module) {
  runLoop().catch((err) => {
    process.stderr.write(`fatal: ${err.stack || err}\n`);
    process.exit(1);
  });
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd /Users/bedardpl/project/Caissa/sandbox && node --test agent-sidecar.test.js`
Expected: all 4 tests pass. (The test file only exercises the pure `parseInitLine`/`parseMessageLine`/`formatReply`/`formatError` functions — `runLoop()` itself, which calls the live SDK, is verified manually in Task 5 against a real room, the same bar `entrypoint.sh`'s shell changes used earlier this session.)

- [ ] **Step 5: Commit**

```bash
cd /Users/bedardpl/project/Caissa
git add sandbox/agent-sidecar.js sandbox/agent-sidecar.test.js
git commit -m "feat(sandbox): agent-sidecar.js — persistent Claude Agent SDK session process"
git push
```

---

### Task 3: Bake the SDK and sidecar into the agent image

**Files:**
- Modify: `Caissa/sandbox/Dockerfile.agent`
- Modify: `Caissa/caissa-cli/src/commands/build.rs`

**Interfaces:**
- Consumes: `agent-sidecar.js` from Task 2.
- Produces: the built `caissa-sandbox:guilhem` image has `node /usr/local/bin/agent-sidecar.js` runnable, with `@anthropic-ai/claude-agent-sdk` resolvable from it, and the Superpowers plugin present on a filesystem path the sidecar can read (per the spec's confirmed constraint that the SDK discovers skills via the filesystem, not a programmatic registration API).

- [ ] **Step 1: Confirm the current Dockerfile.agent's Claude Code install step**

Run: `grep -n "npm install -g @anthropic-ai/claude-code" -B2 -A2 /Users/bedardpl/project/Caissa/sandbox/Dockerfile.agent`
Expected: one match, confirming the exact line to add the new dependency alongside.

- [ ] **Step 2: Add the Agent SDK npm package and the sidecar script to the image**

In `Caissa/sandbox/Dockerfile.agent`, immediately after the `RUN npm install -g @anthropic-ai/claude-code` line, add:
```dockerfile
# Agent SDK — used by agent-sidecar.js to hold persistent per-room Matrix
# sessions open, instead of spawning a fresh claude --print process per
# message. Installed globally so it's resolvable from /usr/local/bin/agent-sidecar.js
# regardless of cwd.
RUN npm install -g @anthropic-ai/claude-agent-sdk

# The persistent-session sidecar process. listen.rs spawns one instance of
# this per actively-chatting Matrix room.
COPY agent-sidecar.js /usr/local/bin/agent-sidecar.js
RUN chmod +x /usr/local/bin/agent-sidecar.js
```

- [ ] **Step 3: Confirm Superpowers is installed at a path the image will carry forward**

Run: `find ~/.claude/plugins -maxdepth 2 -iname "*superpowers*" 2>&1`
Expected: a path like `/Users/bedardpl/.claude/plugins/cache/claude-plugins-official/superpowers/<version>`. Note the exact path printed — Step 4 needs it.

If this differs from what's printed in the actual implementation environment, use the real printed path, not the example below — this is a discovery step, not a fixed value.

- [ ] **Step 4: Bake the Superpowers plugin into the image**

In `Caissa/sandbox/Dockerfile.agent`, after the sidecar `COPY` from Step 2, add (substituting the real path found in Step 3 for `<superpowers-plugin-path>`):
```dockerfile
# Bake Superpowers so agent-sidecar.js's SDK sessions can discover and use
# its skills (brainstorming, writing-plans, TDD, etc.) — the SDK discovers
# skills via the filesystem, not a programmatic registration API.
COPY superpowers-plugin/ /root/.claude/plugins/cache/claude-plugins-official/superpowers/
```

This requires the build context to actually contain a `superpowers-plugin/` directory. Add to `Caissa/caissa-cli/src/commands/build.rs`, in the `run()` function, after the existing `copy_dir_into` calls for `fondament_domains`/`fondament_roles`:

```rust
    // Bake the Superpowers plugin so dispatched/Matrix sessions can use its
    // skills (brainstorming, writing-plans, TDD, etc.) via the Agent SDK's
    // filesystem-based skill discovery.
    let superpowers_src = dirs::home_dir()
        .expect("no home directory")
        .join(".claude/plugins/cache/claude-plugins-official/superpowers");
    if superpowers_src.exists() {
        copy_dir_recursive(&superpowers_src, &build_dir.join("superpowers-plugin"))?;
    } else {
        eprintln!("[caissa] warning: superpowers plugin not found at {}, skipping", superpowers_src.display());
    }
```

This calls a `copy_dir_recursive` helper that doesn't exist yet — the existing `copy_dir_into` in this file is explicitly non-recursive (per its own doc comment: "Non-recursive: only copies top-level files"), but a plugin directory has subdirectories (skills, each with their own files). Add this new helper function to `build.rs`, after the existing `copy_dir_into`:

```rust
/// Recursively copy `src` into `dst`, creating directories as needed.
/// Unlike `copy_dir_into`, this handles nested subdirectories — needed for
/// the Superpowers plugin's skills/ directory structure.
fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}
```

Check whether the `dirs` crate is already a dependency:
Run: `grep -n "^dirs" /Users/bedardpl/project/Caissa/caissa-cli/Cargo.toml`

If it prints nothing, add it. In `Caissa/caissa-cli/Cargo.toml`, in the `[dependencies]` section, add a line:
```toml
dirs = "5"
```

- [ ] **Step 4: Confirm `caissa build guilhem` still runs without error**

Run: `cd /Users/bedardpl/project/Caissa && cargo build -p caissa-cli 2>&1 | tail -20`
Expected: `Finished` with no errors. (Running the actual `caissa build guilhem` Docker build is a heavier operation deferred to Task 5's end-to-end verification, since it requires a working Docker daemon and the full build context — this step only confirms the Rust code compiles.)

- [ ] **Step 5: Commit**

```bash
cd /Users/bedardpl/project/Caissa
git add sandbox/Dockerfile.agent caissa-cli/src/commands/build.rs caissa-cli/Cargo.toml Cargo.lock
git commit -m "feat(build): bake Agent SDK, sidecar, and Superpowers plugin into agent image"
git push
```

---

### Task 4: `listen.rs` becomes a per-room session supervisor

**Files:**
- Modify: `Caissa/caissa-cli/src/commands/listen.rs`

**Interfaces:**
- Consumes: `agent-sidecar.js`'s stdin/stdout protocol from Task 2 (init line, then one message-line-in/reply-line-out per turn). `CaissaConfig`'s existing `dispatcher_mcp_url`/`farga_mcp_url` fields (already present, per this session's earlier finding that `dispatcher_mcp_url` was defined but unused).
- Produces: `run_matrix_reply` keeps its existing signature and HTTP contract (`MatrixReplyReq` in, `MatrixReplyResp` out per the existing `/matrix/reply` route) — Charradissa's caller side needs no changes. This task is purely internal to `listen.rs`.

- [ ] **Step 1: Confirm the current `ListenState` and `run_matrix_reply`**

Run: `sed -n '16,23p;231,280p' /Users/bedardpl/project/Caissa/caissa-cli/src/commands/listen.rs`

Confirm `ListenState` currently has exactly these 6 fields (`farga_url`, `farga_project`, `farga_mcp_url`, `chronicle_model`, `matrix_model`, `amassada_url`) and `run_matrix_reply` spawns a fresh `claude --print` process per call with no session continuity. If this has drifted, read the full file before editing — the diffs below assume this exact starting shape.

- [ ] **Step 2: Write the failing test for the new `RoomSession` idle-reap logic**

Add a new `#[cfg(test)] mod tests` block at the end of `Caissa/caissa-cli/src/commands/listen.rs` (if one doesn't already exist — check first with `grep -n "mod tests" caissa-cli/src/commands/listen.rs`; if it exists, add these functions inside it instead of creating a new block):

```rust
#[cfg(test)]
mod session_supervisor_tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn room_session_is_not_idle_when_recently_active() {
        let session = RoomSession {
            session_id: None,
            last_activity: Instant::now(),
        };
        assert!(!session.is_idle(Duration::from_secs(1800)));
    }

    #[test]
    fn room_session_is_idle_after_timeout_elapsed() {
        let session = RoomSession {
            session_id: None,
            last_activity: Instant::now() - Duration::from_secs(1801),
        };
        assert!(session.is_idle(Duration::from_secs(1800)));
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cd /Users/bedardpl/project/Caissa && cargo test -p caissa-cli session_supervisor_tests`
Expected: FAIL to compile — `RoomSession` is not defined yet.

- [ ] **Step 4: Define `RoomSession` and add the supervisor map to `ListenState`**

In `Caissa/caissa-cli/src/commands/listen.rs`, change the `ListenState` struct from:
```rust
struct ListenState {
    farga_url: String,
    farga_project: String,
    farga_mcp_url: String,
    chronicle_model: String,
    matrix_model: String,
    amassada_url: String,
}
```
to:
```rust
struct ListenState {
    farga_url: String,
    farga_project: String,
    farga_mcp_url: String,
    chronicle_model: String,
    matrix_model: String,
    amassada_url: String,
    dispatcher_mcp_url: String,
    /// One persistent agent-sidecar.js child process per actively-chatting
    /// Matrix room. Reaped by an idle-timeout sweep (see spawn_idle_reaper).
    room_sessions: Arc<tokio::sync::RwLock<HashMap<String, RoomSession>>>,
}

/// One room's live session: the running sidecar child process, the Claude
/// Agent SDK session_id captured from its first reply (for --resume-style
/// continuity on later turns), and when it last handled a message.
struct RoomSession {
    session_id: Option<String>,
    last_activity: std::time::Instant,
}

impl RoomSession {
    fn is_idle(&self, timeout: std::time::Duration) -> bool {
        self.last_activity.elapsed() >= timeout
    }
}
```

Add these imports near the top of the file (check first which of these are already imported — only add what's missing):
```rust
use std::collections::HashMap;
use std::sync::Arc;
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cd /Users/bedardpl/project/Caissa && cargo test -p caissa-cli session_supervisor_tests`
Expected: `test result: ok. 2 passed; 0 failed`

- [ ] **Step 6: Update `ListenState` construction to populate the new fields**

Find the `ListenState` construction (currently around line 50):
```rust
    let state = Arc::new(ListenState {
        farga_url: config.farga_url,
        farga_project: config.project,
        farga_mcp_url: config.farga_mcp_url,
        chronicle_model: config.chronicle_model,
        matrix_model: config.matrix_model,
        amassada_url: config.amassada_url,
    });
```
Replace with:
```rust
    let state = Arc::new(ListenState {
        farga_url: config.farga_url,
        farga_project: config.project,
        farga_mcp_url: config.farga_mcp_url,
        chronicle_model: config.chronicle_model,
        matrix_model: config.matrix_model,
        amassada_url: config.amassada_url,
        dispatcher_mcp_url: config.dispatcher_mcp_url,
        room_sessions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    });
```

This task does NOT yet wire `room_sessions` into `run_matrix_reply`'s actual message-handling logic, nor add the idle-reaper background task, nor spawn `agent-sidecar.js` — those are real architectural changes with enough surface area (process lifecycle, stdin/stdout piping, error handling for a crashed child) that they need their own focused task, immediately following this one, rather than being folded in here. This task's deliverable is the data structure and its test — the next task wires it to actual behavior.

- [ ] **Step 7: Confirm the project builds**

Run: `cd /Users/bedardpl/project/Caissa && cargo build -p caissa-cli 2>&1 | tail -20`
Expected: `Finished` with no errors.

- [ ] **Step 8: Commit**

```bash
cd /Users/bedardpl/project/Caissa
git add caissa-cli/src/commands/listen.rs
git commit -m "feat(listen): add RoomSession + room_sessions map to ListenState"
git push
```

---

### Task 5: Wire the supervisor into `run_matrix_reply` and add the idle reaper

**Files:**
- Modify: `Caissa/caissa-cli/src/commands/listen.rs`

**Interfaces:**
- Consumes: `RoomSession`/`room_sessions` from Task 4; `agent-sidecar.js`'s protocol from Task 2.
- Produces: the actual behavior change — `run_matrix_reply` now spawns-or-reuses a per-room sidecar process instead of `claude --print`. Nothing further depends on this beyond Task 6's live verification.

- [ ] **Step 1: Add the sidecar-process wrapper type**

Add to `Caissa/caissa-cli/src/commands/listen.rs`, near the `RoomSession` definition from Task 4:

```rust
/// A running agent-sidecar.js child process for one room.
struct SidecarProcess {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::io::BufReader<tokio::process::ChildStdout>,
}

impl SidecarProcess {
    async fn spawn(init: &SidecarInit) -> anyhow::Result<Self> {
        use tokio::io::AsyncWriteExt;

        let mut child = tokio::process::Command::new("node")
            .arg("/usr/local/bin/agent-sidecar.js")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()?;

        let mut stdin = child.stdin.take().expect("stdin was piped");
        let stdout = child.stdout.take().expect("stdout was piped");

        let init_line = serde_json::to_string(init)?;
        stdin.write_all(init_line.as_bytes()).await?;
        stdin.write_all(b"\n").await?;

        Ok(Self {
            child,
            stdin,
            stdout: tokio::io::BufReader::new(stdout),
        })
    }

    async fn send(&mut self, sender: &str, content: &str) -> anyhow::Result<String> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

        let msg = serde_json::json!({ "sender": sender, "content": content });
        let line = serde_json::to_string(&msg)?;
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;

        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line).await?;

        let parsed: serde_json::Value = serde_json::from_str(response_line.trim())?;
        if let Some(err) = parsed.get("error").and_then(|v| v.as_str()) {
            anyhow::bail!("sidecar error: {}", err);
        }
        Ok(parsed.get("reply").and_then(|v| v.as_str()).unwrap_or("").to_string())
    }

    fn kill(&mut self) {
        let _ = self.child.start_kill();
    }
}

#[derive(serde::Serialize)]
struct SidecarInit {
    #[serde(rename = "systemPrompt")]
    system_prompt: String,
    model: String,
    #[serde(rename = "allowedTools")]
    allowed_tools: Vec<String>,
    skills: Vec<String>,
    #[serde(rename = "mcpServers")]
    mcp_servers: serde_json::Value,
}
```

Note: `SidecarProcess` holding a `tokio::process::Child` is not `Clone` and isn't safe to share via the same `RwLock<HashMap<String, RoomSession>>` pattern used for the lightweight `RoomSession` metadata — a running child process needs exclusive ownership for its stdin/stdout handles. Restructure `RoomSession` (from Task 4) to own the process directly:

Change `RoomSession` from:
```rust
struct RoomSession {
    session_id: Option<String>,
    last_activity: std::time::Instant,
}
```
to:
```rust
struct RoomSession {
    process: SidecarProcess,
    last_activity: std::time::Instant,
}
```
(The `session_id` field moves into `SidecarProcess`'s ownership conceptually — `agent-sidecar.js` tracks it internally per Task 2's design and never needs to round-trip it back to Rust; `listen.rs` only needs to keep the same process alive across turns for the SDK's own `resume` continuity to apply.)

Update Task 4's two tests (`room_session_is_not_idle_when_recently_active`, `room_session_is_idle_after_timeout_elapsed`) to construct a `RoomSession` with a real (or mock) `process` field — since `SidecarProcess::spawn` actually launches `node`, which isn't available/desired in a unit test, add a test-only constructor. Add to `RoomSession`:
```rust
impl RoomSession {
    #[cfg(test)]
    fn for_test(last_activity: std::time::Instant) -> Self {
        // A RoomSession built for idle-timeout testing only — never call
        // .send() on its process, since /bin/true exits immediately and
        // has no stdin/stdout protocol.
        let child = std::process::Command::new("true");
        let child = tokio::process::Command::from(child).spawn().expect("spawn /bin/true for test");
        Self {
            process: SidecarProcess {
                stdin: child.stdin.expect("no stdin"),
                stdout: tokio::io::BufReader::new(child.stdout.expect("no stdout")),
                child,
            },
            last_activity,
        }
    }

    fn is_idle(&self, timeout: std::time::Duration) -> bool {
        self.last_activity.elapsed() >= timeout
    }
}
```

This requires `/bin/true`'s `Command` to have `.stdin(Stdio::piped())`/`.stdout(Stdio::piped())` set, since by default `Child::stdin`/`Child::stdout` are `None`. Fix the test constructor:
```rust
    #[cfg(test)]
    fn for_test(last_activity: std::time::Instant) -> Self {
        let mut cmd = tokio::process::Command::new("true");
        cmd.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
        let mut child = cmd.spawn().expect("spawn /bin/true for test");
        let stdin = child.stdin.take().expect("stdin was piped");
        let stdout = child.stdout.take().expect("stdout was piped");
        Self {
            process: SidecarProcess { child, stdin, stdout: tokio::io::BufReader::new(stdout) },
            last_activity,
        }
    }
```

Update the two existing tests from Task 4 to use `RoomSession::for_test(...)` instead of the struct literal:
```rust
    #[test]
    fn room_session_is_not_idle_when_recently_active() {
        let session = RoomSession::for_test(Instant::now());
        assert!(!session.is_idle(Duration::from_secs(1800)));
    }

    #[test]
    fn room_session_is_idle_after_timeout_elapsed() {
        let session = RoomSession::for_test(Instant::now() - Duration::from_secs(1801));
        assert!(session.is_idle(Duration::from_secs(1800)));
    }
```

- [ ] **Step 2: Run the tests to verify they still pass after the restructure**

Run: `cd /Users/bedardpl/project/Caissa && cargo test -p caissa-cli session_supervisor_tests`
Expected: `test result: ok. 2 passed; 0 failed`

- [ ] **Step 3: Rewrite `run_matrix_reply` to use the supervisor**

Replace the entire current `run_matrix_reply` function body with:
```rust
async fn run_matrix_reply(state: &ListenState, req: &MatrixReplyReq) -> anyhow::Result<String> {
    let needs_tools = needs_farga_mcp(&req.content);

    let mut sessions = state.room_sessions.write().await;

    if !sessions.contains_key(&req.room_id) {
        let mcp_servers = if needs_tools {
            serde_json::json!({
                "farga": { "type": "http", "url": state.farga_mcp_url },
                "dispatcher": { "type": "http", "url": state.dispatcher_mcp_url },
            })
        } else {
            serde_json::json!({})
        };
        let allowed_tools = if needs_tools {
            vec![
                "Bash".to_string(), "Edit".to_string(), "Write".to_string(),
                "mcp__farga__search_signals".to_string(),
                "mcp__farga__read_context".to_string(),
                "mcp__farga__list_projects".to_string(),
                "mcp__farga__update_component_todo".to_string(),
                "mcp__dispatcher__invoke_agent".to_string(),
                "mcp__dispatcher__get_agent_result".to_string(),
                "mcp__dispatcher__list_agent_specs".to_string(),
            ]
        } else {
            vec!["Bash".to_string(), "Edit".to_string(), "Write".to_string()]
        };

        let init = SidecarInit {
            system_prompt: format!("You are Guilhem, replying in Matrix room {}.", req.room_id),
            model: state.matrix_model.clone(),
            allowed_tools,
            skills: vec![], // populated from the resolved facet's `skills` list by the caller; empty until Fondament-resolver wiring exists (out of scope, matches tools.always_on's existing manual-relay model)
            mcp_servers,
        };

        let process = SidecarProcess::spawn(&init).await?;
        sessions.insert(
            req.room_id.clone(),
            RoomSession { process, last_activity: std::time::Instant::now() },
        );
    }

    let session = sessions.get_mut(&req.room_id).expect("just inserted or already present");
    let reply = session.process.send(&req.sender, &req.content).await?;
    session.last_activity = std::time::Instant::now();

    Ok(reply)
}
```

- [ ] **Step 4: Add the idle-reaper background task**

Add this function to `Caissa/caissa-cli/src/commands/listen.rs`:
```rust
/// Periodically kills and removes any RoomSession that's been idle past
/// the timeout, releasing its sidecar process. Spawned once at startup
/// alongside the existing chronicle/archival background loops.
async fn spawn_idle_reaper(room_sessions: Arc<tokio::sync::RwLock<HashMap<String, RoomSession>>>) {
    const IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30 * 60);
    const SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

    loop {
        tokio::time::sleep(SWEEP_INTERVAL).await;
        let mut sessions = room_sessions.write().await;
        let idle_rooms: Vec<String> = sessions
            .iter()
            .filter(|(_, s)| s.is_idle(IDLE_TIMEOUT))
            .map(|(room, _)| room.clone())
            .collect();
        for room in idle_rooms {
            if let Some(mut session) = sessions.remove(&room) {
                tracing::info!("reaping idle session for room {}", room);
                session.process.kill();
            }
        }
    }
}
```

Find where the existing background loops are spawned (search for `tokio::spawn` in the file's `main`/`run` function — likely near the chronicle/archival loop setup) and add, in the same place:
```rust
    tokio::spawn(spawn_idle_reaper(Arc::clone(&state.room_sessions)));
```

- [ ] **Step 5: Confirm the project builds**

Run: `cd /Users/bedardpl/project/Caissa && cargo build -p caissa-cli 2>&1 | tail -20`
Expected: `Finished` with no errors.

- [ ] **Step 6: Run the full test suite**

Run: `cd /Users/bedardpl/project/Caissa && cargo test -p caissa-cli`
Expected: all tests pass, including the 2 `session_supervisor_tests`.

- [ ] **Step 7: Commit**

```bash
cd /Users/bedardpl/project/Caissa
git add caissa-cli/src/commands/listen.rs
git commit -m "feat(listen): wire room_sessions into run_matrix_reply, add idle reaper"
git push
```

---

### Task 6: Build, deploy, and verify with a real Matrix conversation

**Files:** none (operational verification).

**Interfaces:**
- Consumes: everything from Tasks 1–5, deployed.

- [ ] **Step 1: Trigger and confirm Caissa's CI build**

```bash
gh run list --repo miegjorn/Caissa --limit 1
```
Expected: `completed success` for the push from Task 5.

- [ ] **Step 2: Trigger and confirm `build-guilhem.yml`**

```bash
gh workflow run build-guilhem.yml --repo miegjorn/Fondament
gh run list --repo miegjorn/Fondament --workflow=build-guilhem.yml --limit 1
```
Poll until `completed success` — do not trigger this before confirming Step 1's Caissa build has finished (the `caissa` binary it extracts must reflect this branch's changes; this exact ordering mistake happened earlier this session and produced a stale image).

- [ ] **Step 3: Confirm ArgoCD synced and the pod is healthy**

```bash
kubectl get application guilhem -n argocd -o jsonpath='{.status.sync.status} {.status.health.status}{"\n"}'
kubectl get pods -n agents -l app.kubernetes.io/name=guilhem
```
Expected: `Synced Healthy`, pod `1/1 Running`. If the image is a floating tag with a node-level cache from a prior run (the exact failure mode found earlier this session), clear it:
```bash
docker exec occitan-control-plane crictl rmi ghcr.io/miegjorn/caissa-sandbox:guilhem 2>&1 || true
kubectl rollout restart deployment/guilhem -n agents
```

- [ ] **Step 4: Send two messages in the same Matrix room and confirm the same sidecar process handles both**

Send a first message ("what's 2+2") and a follow-up ("what did I just ask you?") a few seconds apart in the same room used throughout this session (`!zSeBDhFjVObXNpqAtF:occitane.guilhem`). Confirm the second reply correctly references the first question — this is the actual continuity bug this plan exists to fix.

While both messages are being processed, confirm only one sidecar process is running for that room (not two):
```bash
kubectl exec -n agents $(kubectl get pod -n agents -l app.kubernetes.io/name=guilhem -o jsonpath='{.items[0].metadata.name}') -- sh -c 'ps aux | grep agent-sidecar | grep -v grep'
```
Expected: exactly one `node /usr/local/bin/agent-sidecar.js` process.

- [ ] **Step 5: Confirm the idle reaper actually reaps**

After sending a message, wait past the 30-minute idle timeout (or temporarily lower `IDLE_TIMEOUT`/`SWEEP_INTERVAL` in a local test build to make this practical to verify quickly), then confirm the sidecar process is gone:
```bash
kubectl logs $(kubectl get pod -n agents -l app.kubernetes.io/name=guilhem -o jsonpath='{.items[0].metadata.name}') -n agents | grep "reaping idle session"
```
Expected: a log line confirming the reap happened for the test room.

## Follow-up (not a task in this plan)

The `skills: []` field passed in `run_matrix_reply`'s `SidecarInit` (Task 5, Step 3) is hardcoded empty — wiring it to actually read the resolved persona's `skills` list (Task 1) requires the same kind of manual-relay or Fondament-resolver work flagged as out of scope in both this spec and the `tools.always_on` design earlier this session. This is intentional: shipping the persistent-session infrastructure first, then closing the skills-wiring gap as a focused follow-up once the supervisor itself is proven live, keeps this plan's tasks independently testable rather than blocking the larger architecture change on a YAML-parsing detail.
