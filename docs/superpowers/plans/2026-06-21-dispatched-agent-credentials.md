# Dispatched Agent Tool/Credential Provisioning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire Fondament's existing-but-inert `tools.always_on` persona field through Caissa's dispatcher so a spawned agent Job actually gets working git/gh credentials, MCP tool access, and `--allowed-tools` permissions instead of running `claude --print` bare and getting blocked on every tool call.

**Architecture:** Fondament personas declare their tool needs (already had the empty schema, now populated); Caissa's `invoke_agent` MCP tool accepts an `allowed_tools` string the caller derives from that declaration; `build_job` provisions the same OpenBao credential-injection init container Guilhem's own pod chart already has; `entrypoint.sh`'s task-mode branch actually uses both.

**Tech Stack:** YAML (Fondament persona definitions), Rust + `k8s_openapi`/`kube` (Caissa dispatcher), POSIX shell (`entrypoint.sh`), Cargo workspace tests.

## Global Constraints

- Facet-keyword → filename mapping (from the spec, exact): `developer`→`developer.yaml`, `infra`→`infra-engineer.yaml`, `qa`→`qa-engineer.yaml`, `security`→`security-analyst.yaml`, `architect`→`app-architect.yaml`, `db`→`data-architect.yaml`.
- All 6 facets get the identical baseline tool list — no per-facet differentiation in this plan (per spec "Out of scope").
- No RBAC changes to the dispatcher's ServiceAccount (secret resolution for the new init container happens via kubelet using `secretKeyRef`, not the dispatcher's own k8s API client identity).
- `build_job`'s existing signature (`job_name, domain, facet, session_id, namespace, image, env`) does not change — the init container and creds volume are unconditional additions inside the function body, not new parameters.

---

### Task 1: Populate `tools.always_on` on all 6 Fondament facet files

**Files:**
- Modify: `Fondament/definitions/fondament/developer.yaml`
- Modify: `Fondament/definitions/fondament/infra-engineer.yaml`
- Modify: `Fondament/definitions/fondament/qa-engineer.yaml`
- Modify: `Fondament/definitions/fondament/security-analyst.yaml`
- Modify: `Fondament/definitions/fondament/app-architect.yaml`
- Modify: `Fondament/definitions/fondament/data-architect.yaml`

**Interfaces:**
- Consumes: nothing (pure content change).
- Produces: nothing consumed by other tasks directly — Task 3's `allowed_tools` mapping table in `dispatch.rs` references this content by description, not by reading the files at build time. No code in any task parses these YAML files.

This task is content-only — no automated tests apply beyond YAML syntax validation.

- [ ] **Step 1: Confirm each file's current trailing block**

Run: `tail -5 /Users/bedardpl/project/Fondament/definitions/fondament/developer.yaml /Users/bedardpl/project/Fondament/definitions/fondament/infra-engineer.yaml /Users/bedardpl/project/Fondament/definitions/fondament/qa-engineer.yaml /Users/bedardpl/project/Fondament/definitions/fondament/security-analyst.yaml /Users/bedardpl/project/Fondament/definitions/fondament/app-architect.yaml /Users/bedardpl/project/Fondament/definitions/fondament/data-architect.yaml`

Expected: each file ends with exactly:
```yaml
tools:
  always_on: []
  jit: []
```
If any file has drifted from this, read it fully before editing — the replacement in Step 2 assumes this exact trailing text.

- [ ] **Step 2: Replace the trailing block in all 6 files with the populated tool list**

In each of the 6 files, replace:
```yaml
tools:
  always_on: []
  jit: []
```
with:
```yaml
tools:
  always_on:
    - id: bash
      kind: native
      tool: Bash
    - id: edit
      kind: native
      tool: Edit
    - id: write
      kind: native
      tool: Write
    - id: farga-search-signals
      kind: mcp
      server: farga
      tool: search_signals
    - id: farga-read-context
      kind: mcp
      server: farga
      tool: read_context
    - id: farga-update-component-todo
      kind: mcp
      server: farga
      tool: update_component_todo
  jit: []
```

This is the exact same block in all 6 files — apply it identically to `developer.yaml`, `infra-engineer.yaml`, `qa-engineer.yaml`, `security-analyst.yaml`, `app-architect.yaml`, `data-architect.yaml`.

- [ ] **Step 3: Validate YAML syntax on all 6 files**

Run:
```bash
python3 -c "
import yaml
for f in ['developer','infra-engineer','qa-engineer','security-analyst','app-architect','data-architect']:
    yaml.safe_load(open(f'/Users/bedardpl/project/Fondament/definitions/fondament/{f}.yaml'))
print('OK')
"
```
Expected: `OK`

- [ ] **Step 4: Commit**

```bash
cd /Users/bedardpl/project/Fondament
git add definitions/fondament/developer.yaml definitions/fondament/infra-engineer.yaml definitions/fondament/qa-engineer.yaml definitions/fondament/security-analyst.yaml definitions/fondament/app-architect.yaml definitions/fondament/data-architect.yaml
git commit -m "feat(fondament): populate tools.always_on for all 6 dispatched facets"
git push
```

---

### Task 2: `build_job` provisions OpenBao credential injection

**Files:**
- Modify: `Caissa/caissa-cli/src/commands/dispatch.rs`

**Interfaces:**
- Consumes: the existing `build_job(job_name: &str, domain: &str, facet: &str, session_id: &str, namespace: &str, image: &str, env: Vec<EnvVar>) -> Job` function — signature unchanged.
- Produces: `build_job`'s returned `Job` now always has `template.spec.init_containers` (one entry, name `fetch-tokens`) and `template.spec.volumes` (one entry, name `creds`), and the `agent` container has a `volume_mounts` entry for `creds`. Task 4 (entrypoint.sh) relies on `/creds/tokens.env` and `/creds/.gitconfig` existing at runtime because of this.

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` block in `Caissa/caissa-cli/src/commands/dispatch.rs`, after the existing `build_job_uses_the_given_image` test:

```rust
    #[test]
    fn build_job_includes_credential_init_container() {
        let job = build_job(
            "agent-amassada-developer-abc123",
            "amassada",
            "developer",
            "session-3",
            "agents",
            "ghcr.io/miegjorn/caissa-sandbox:guilhem",
            vec![],
        );

        let pod_spec = job.spec.unwrap().template.spec.unwrap();
        let init_containers = pod_spec.init_containers.expect("init_containers must be set");
        assert_eq!(init_containers.len(), 1);
        assert_eq!(init_containers[0].name, "fetch-tokens");
        assert_eq!(init_containers[0].image.as_deref(), Some("openbao/openbao:latest"));

        let volumes = pod_spec.volumes.expect("volumes must be set");
        assert!(volumes.iter().any(|v| v.name == "creds"), "expected a 'creds' volume");

        let agent_container = &pod_spec.containers[0];
        let mounts = agent_container.volume_mounts.as_ref().expect("agent container must mount creds");
        assert!(mounts.iter().any(|m| m.name == "creds" && m.mount_path == "/creds"));
    }

    #[test]
    fn build_job_init_container_reads_openbao_token_secret() {
        let job = build_job(
            "agent-farga-developer-def456",
            "farga",
            "developer",
            "session-4",
            "agents",
            "ghcr.io/miegjorn/caissa-sandbox:guilhem",
            vec![],
        );

        let pod_spec = job.spec.unwrap().template.spec.unwrap();
        let init_container = &pod_spec.init_containers.unwrap()[0];
        let env = init_container.env.as_ref().expect("init container must have env");
        let bao_token = env.iter().find(|e| e.name == "BAO_TOKEN").expect("BAO_TOKEN env var must be set");
        let secret_ref = bao_token.value_from.as_ref().unwrap().secret_key_ref.as_ref().unwrap();
        assert_eq!(secret_ref.name.as_deref(), Some("openbao"));
        assert_eq!(secret_ref.key, "token");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd /Users/bedardpl/project/Caissa && cargo test -p caissa-cli build_job_includes_credential_init_container build_job_init_container_reads_openbao_token_secret`
Expected: both FAIL — `init_containers` is `None` on the current `build_job` output.

- [ ] **Step 3: Add the new imports**

In `Caissa/caissa-cli/src/commands/dispatch.rs`, change the `k8s_openapi::api::core::v1` import line from:
```rust
use k8s_openapi::api::core::v1::{
    Container, EnvVar, EnvVarSource, LocalObjectReference, PodSpec, PodTemplateSpec,
    SecretKeySelector,
};
```
to:
```rust
use k8s_openapi::api::core::v1::{
    Container, EmptyDirVolumeSource, EnvVar, EnvVarSource, LocalObjectReference, PodSpec,
    PodTemplateSpec, SecretKeySelector, Volume, VolumeMount,
};
```

- [ ] **Step 4: Add the `fetch-tokens` init container constant and builder function**

Add this above the `build_job` function definition (after the `env_val` function):

```rust
/// Same OpenBao KV reads + git credential file layout as the `fetch-tokens` init
/// container in `deploy/charts/guilhem/templates/guilhem.yaml` — kept identical so
/// dispatched agents authenticate the same way Guilhem's own pod does.
const FETCH_TOKENS_SCRIPT: &str = r#"set -eu
export BAO_ADDR=http://openbao.occitan-system.svc.cluster.local:8200
GH=$(bao kv get -field=value secret/occitan/github)
GL=$(bao kv get -field=value secret/occitan/gitlab)
umask 077
cat > /creds/tokens.env <<EOF
export GH_TOKEN='$GH'
export GITHUB_TOKEN='$GH'
export GITLAB_TOKEN='$GL'
export GITLAB_PAT_TOKEN='$GL'
EOF
cat > /creds/.git-credentials <<EOF
https://x-access-token:$GH@github.com
https://oauth2:$GL@gitlab.com
EOF
cat > /creds/.gitconfig <<'EOF'
[credential]
    helper = store --file=/creds/.git-credentials
[user]
    name = Guilhem de Tudela
    email = guilhem@occitane.guilhem
[safe]
    directory = *
EOF
echo "tokens + git creds written to /creds"
"#;

fn fetch_tokens_init_container() -> Container {
    Container {
        name: "fetch-tokens".into(),
        image: Some("openbao/openbao:latest".into()),
        image_pull_policy: Some("IfNotPresent".into()),
        command: Some(vec!["/bin/sh".into(), "-c".into()]),
        args: Some(vec![FETCH_TOKENS_SCRIPT.into()]),
        env: Some(vec![EnvVar {
            name: "BAO_TOKEN".into(),
            value_from: Some(EnvVarSource {
                secret_key_ref: Some(SecretKeySelector {
                    name: Some("openbao".into()),
                    key: "token".into(),
                    optional: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        }]),
        volume_mounts: Some(vec![VolumeMount {
            name: "creds".into(),
            mount_path: "/creds".into(),
            ..Default::default()
        }]),
        ..Default::default()
    }
}
```

- [ ] **Step 5: Wire the init container, volume, and mount into `build_job`**

Change the `build_job` function body. Replace:
```rust
        spec: Some(JobSpec {
            ttl_seconds_after_finished: Some(600),
            backoff_limit: Some(1),
            template: PodTemplateSpec {
                metadata: None,
                spec: Some(PodSpec {
                    restart_policy: Some("Never".into()),
                    image_pull_secrets: Some(vec![LocalObjectReference {
                        name: Some("ghcr-creds".into()),
                    }]),
                    containers: vec![Container {
                        name: "agent".into(),
                        image: Some(image.into()),
                        env: Some(env),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
```
with:
```rust
        spec: Some(JobSpec {
            ttl_seconds_after_finished: Some(600),
            backoff_limit: Some(1),
            template: PodTemplateSpec {
                metadata: None,
                spec: Some(PodSpec {
                    restart_policy: Some("Never".into()),
                    image_pull_secrets: Some(vec![LocalObjectReference {
                        name: Some("ghcr-creds".into()),
                    }]),
                    init_containers: Some(vec![fetch_tokens_init_container()]),
                    containers: vec![Container {
                        name: "agent".into(),
                        image: Some(image.into()),
                        env: Some(env),
                        volume_mounts: Some(vec![VolumeMount {
                            name: "creds".into(),
                            mount_path: "/creds".into(),
                            ..Default::default()
                        }]),
                        ..Default::default()
                    }],
                    volumes: Some(vec![Volume {
                        name: "creds".into(),
                        empty_dir: Some(EmptyDirVolumeSource {
                            medium: Some("Memory".into()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }]),
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cd /Users/bedardpl/project/Caissa && cargo test -p caissa-cli build_job`
Expected: `test result: ok. 4 passed; 0 failed` (the 2 pre-existing `build_job_*` tests plus the 2 new ones).

- [ ] **Step 7: Run the full caissa-cli test suite to confirm no regression**

Run: `cd /Users/bedardpl/project/Caissa && cargo test -p caissa-cli`
Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
cd /Users/bedardpl/project/Caissa
git add caissa-cli/src/commands/dispatch.rs
git commit -m "feat(dispatch): provision OpenBao git/gh credentials for spawned agent Jobs"
git push
```

---

### Task 3: `invoke_agent` accepts and threads through `allowed_tools`

**Files:**
- Modify: `Caissa/caissa-cli/src/commands/dispatch.rs`

**Interfaces:**
- Consumes: `build_job`'s unchanged signature from Task 2; the existing `env_val(name: &str, value: &str) -> EnvVar` helper.
- Produces: `create_agent_job` gains a new `allowed_tools: &str` parameter (inserted after `context`, before `session_id`, in both the function signature and every call site). The spawned Job gains an `ALLOWED_TOOLS` env var. Task 4 (`entrypoint.sh`) reads this exact env var name.

- [ ] **Step 1: Update the `invoke_agent` tool schema and facet documentation**

In `Caissa/caissa-cli/src/commands/dispatch.rs`, in `tool_list()`, replace the `invoke_agent` tool's `inputSchema` block:
```rust
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "domain": {
                            "type": "string",
                            "description": "Component domain: farga | gardian | amassada | charradissa | cor | caissa | fondament | occitan"
                        },
                        "facet": {
                            "type": "string",
                            "description": "Role facet: architect | developer | qa | infra | db | security"
                        },
                        "task": {
                            "type": "string",
                            "description": "The task for the agent to perform. Be specific — this becomes the claude --print prompt."
                        },
                        "context": {
                            "type": "string",
                            "description": "Pre-assembled domain+facet context markdown. Written to /workspace/CLAUDE.md before the agent runs. Load from /fondament/domains/<domain>.yaml and /fondament/roles/<facet>.yaml in your session."
                        },
                        "session_id": {
                            "type": "string",
                            "description": "Session identifier. The agent writes its result as a Farga Signal under this project name. Use a unique ID per invocation so you can retrieve the result."
                        }
                    },
                    "required": ["domain", "facet", "task", "session_id"]
                }
```
with:
```rust
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "domain": {
                            "type": "string",
                            "description": "Component domain: farga | gardian | amassada | charradissa | cor | caissa | fondament | occitan"
                        },
                        "facet": {
                            "type": "string",
                            "description": "Role facet: architect | developer | qa | infra | db | security"
                        },
                        "task": {
                            "type": "string",
                            "description": "The task for the agent to perform. Be specific — this becomes the claude --print prompt."
                        },
                        "context": {
                            "type": "string",
                            "description": "Pre-assembled domain+facet context markdown. Written to /workspace/CLAUDE.md before the agent runs. Load from /fondament/domains/<domain>.yaml for domain context. For facet context, the filename does NOT match the facet keyword — use this mapping: developer->developer.yaml, infra->infra-engineer.yaml, qa->qa-engineer.yaml, security->security-analyst.yaml, architect->app-architect.yaml, db->data-architect.yaml. Read /fondament/roles/<mapped-filename> in your session."
                        },
                        "allowed_tools": {
                            "type": "string",
                            "description": "Comma-separated Claude tool names for the spawned agent, read from the facet file's tools.always_on list (same file as the context mapping above). Native tools pass through as-is (Bash, Edit, Write); Mcp tools are formatted as mcp__<server>__<tool> (e.g. mcp__farga__search_signals). Defaults to a read-only Farga tool set if omitted."
                        },
                        "session_id": {
                            "type": "string",
                            "description": "Session identifier. The agent writes its result as a Farga Signal under this project name. Use a unique ID per invocation so you can retrieve the result."
                        }
                    },
                    "required": ["domain", "facet", "task", "session_id"]
                }
```

Note `allowed_tools` is NOT added to `required` — it has a safe default (read-only Farga tools), per the spec's error-handling rule (no silent default to broader access than requested).

- [ ] **Step 2: Update `list_specs()`'s facet documentation**

Replace:
```rust
    lines.push("\nLoad context from /fondament/domains/<domain>.yaml and /fondament/roles/<facet>.yaml\nbefore calling invoke_agent.".into());
```
with:
```rust
    lines.push("\nLoad domain context from /fondament/domains/<domain>.yaml.\nFacet filenames under /fondament/roles/ do not match the facet keyword above —\nuse this mapping: developer->developer.yaml, infra->infra-engineer.yaml,\nqa->qa-engineer.yaml, security->security-analyst.yaml, architect->app-architect.yaml,\ndb->data-architect.yaml. Read the facet file's tools.always_on list and pass it\nas invoke_agent's allowed_tools (comma-separated tool names).".into());
```

- [ ] **Step 3: Extract `allowed_tools` in the `invoke_agent` call_tool arm**

In the `"invoke_agent" =>` match arm, replace:
```rust
            let domain = args["domain"].as_str().unwrap_or("").to_string();
            let facet = args["facet"].as_str().unwrap_or("").to_string();
            let task = args["task"].as_str().unwrap_or("").to_string();
            let context = args["context"].as_str().unwrap_or("").to_string();
            let session_id = args["session_id"].as_str().unwrap_or("").to_string();

            anyhow::ensure!(!domain.is_empty(), "domain is required");
            anyhow::ensure!(!facet.is_empty(), "facet is required");
            anyhow::ensure!(!task.is_empty(), "task is required");
            anyhow::ensure!(!session_id.is_empty(), "session_id is required");

            let job_id = create_agent_job(
                &state.k8s,
                &domain,
                &facet,
                &task,
                &context,
                &session_id,
                &state.agent_image,
                &state.agents_namespace,
                &state.farga_url,
                &state.farga_mcp_url,
            )
            .await?;
```
with:
```rust
            let domain = args["domain"].as_str().unwrap_or("").to_string();
            let facet = args["facet"].as_str().unwrap_or("").to_string();
            let task = args["task"].as_str().unwrap_or("").to_string();
            let context = args["context"].as_str().unwrap_or("").to_string();
            let allowed_tools = args["allowed_tools"].as_str()
                .unwrap_or("mcp__farga__search_signals,mcp__farga__read_context")
                .to_string();
            let session_id = args["session_id"].as_str().unwrap_or("").to_string();

            anyhow::ensure!(!domain.is_empty(), "domain is required");
            anyhow::ensure!(!facet.is_empty(), "facet is required");
            anyhow::ensure!(!task.is_empty(), "task is required");
            anyhow::ensure!(!session_id.is_empty(), "session_id is required");

            let job_id = create_agent_job(
                &state.k8s,
                &domain,
                &facet,
                &task,
                &context,
                &allowed_tools,
                &session_id,
                &state.agent_image,
                &state.agents_namespace,
                &state.farga_url,
                &state.farga_mcp_url,
            )
            .await?;
```

- [ ] **Step 4: Thread `allowed_tools` through `create_agent_job`**

Replace the `create_agent_job` function signature and env-vec construction:
```rust
async fn create_agent_job(
    client: &Client,
    domain: &str,
    facet: &str,
    task: &str,
    context: &str,
    session_id: &str,
    image: &str,
    namespace: &str,
    farga_url: &str,
    farga_mcp_url: &str,
) -> anyhow::Result<String> {
    let short_id = &uuid::Uuid::new_v4().to_string()[..8];
    let job_name = format!("agent-{}-{}-{}", domain, facet, short_id);

    let env = vec![
        env_val("DOMAIN", domain),
        env_val("FACET", facet),
        env_val("TASK", task),
        env_val("AGENT_CONTEXT", context),
        env_val("SESSION_ID", session_id),
        env_val("FARGA_URL", farga_url),
        env_val("FARGA_MCP_URL", farga_mcp_url),
```
with:
```rust
async fn create_agent_job(
    client: &Client,
    domain: &str,
    facet: &str,
    task: &str,
    context: &str,
    allowed_tools: &str,
    session_id: &str,
    image: &str,
    namespace: &str,
    farga_url: &str,
    farga_mcp_url: &str,
) -> anyhow::Result<String> {
    let short_id = &uuid::Uuid::new_v4().to_string()[..8];
    let job_name = format!("agent-{}-{}-{}", domain, facet, short_id);

    let env = vec![
        env_val("DOMAIN", domain),
        env_val("FACET", facet),
        env_val("TASK", task),
        env_val("AGENT_CONTEXT", context),
        env_val("ALLOWED_TOOLS", allowed_tools),
        env_val("SESSION_ID", session_id),
        env_val("FARGA_URL", farga_url),
        env_val("FARGA_MCP_URL", farga_mcp_url),
```

- [ ] **Step 5: Confirm the project builds**

Run: `cd /Users/bedardpl/project/Caissa && cargo build -p caissa-cli 2>&1 | tail -20`
Expected: `Finished` with no errors. (This is a parameter-threading change with no new logic branches — a build failure here means a call site or signature was missed; search for `create_agent_job(` to find any other call sites if so.)

- [ ] **Step 6: Run the full test suite**

Run: `cd /Users/bedardpl/project/Caissa && cargo test -p caissa-cli`
Expected: all tests pass (this task doesn't add new tests of its own — `create_agent_job` is an `async fn` wrapping the k8s API client and isn't covered by the existing unit-test pattern, which targets the pure `build_job` function instead).

- [ ] **Step 7: Commit**

```bash
cd /Users/bedardpl/project/Caissa
git add caissa-cli/src/commands/dispatch.rs
git commit -m "feat(dispatch): invoke_agent accepts allowed_tools, threads through to spawned Job"
git push
```

---

### Task 4: `entrypoint.sh` uses credentials, MCP config, and allowed-tools in task mode

**Files:**
- Modify: `Caissa/sandbox/entrypoint.sh`

**Interfaces:**
- Consumes: `/creds/tokens.env` and `/creds/.gitconfig` (written by Task 2's init container at `/creds`, mounted into this same container); the `ALLOWED_TOOLS` env var (set by Task 3); the existing `/root/.claude/claude_desktop_config.json` this same script already writes earlier in its own execution, in both modes.
- Produces: nothing consumed by other tasks — this is the terminal fix that makes the whole pipeline functional.

This is a shell script with no Rust test harness in this codebase — verified by Task 5's manual dispatch, not an automated test.

- [ ] **Step 1: Confirm the current task-mode block**

Run: `cat -n /Users/bedardpl/project/Caissa/sandbox/entrypoint.sh`

Confirm the task-mode branch (inside `if [ -n "${TASK:-}" ]; then`) currently reads:
```sh
  # Run the task non-interactively, capture output.
  printf '%s' "$TASK" > /tmp/agent-task.txt
  OUTPUT=$(claude --print "$(cat /tmp/agent-task.txt)" 2>&1) || true
```
If it has drifted from this, read the full file before editing.

- [ ] **Step 2: Add credential sourcing, MCP config, and allowed-tools to the task-mode invocation**

Replace:
```sh
  # Run the task non-interactively, capture output.
  printf '%s' "$TASK" > /tmp/agent-task.txt
  OUTPUT=$(claude --print "$(cat /tmp/agent-task.txt)" 2>&1) || true
```
with:
```sh
  # Source OpenBao-provided git/gh credentials if the fetch-tokens init
  # container ran (it always does for dispatched agent Jobs — see
  # build_job in caissa-cli/src/commands/dispatch.rs).
  [ -f /creds/tokens.env ] && . /creds/tokens.env
  export GIT_CONFIG_GLOBAL=/creds/.gitconfig

  # Run the task non-interactively, capture output. --mcp-config connects
  # the farga/dispatcher MCP servers configured above; --allowed-tools is
  # required for ANY tool call to succeed in headless mode (no interactive
  # approval is possible). ALLOWED_TOOLS is set by the dispatcher from the
  # facet's tools.always_on list (see Fondament definitions/fondament/*.yaml);
  # the fallback here is intentionally read-only.
  printf '%s' "$TASK" > /tmp/agent-task.txt
  OUTPUT=$(claude --print \
    --mcp-config /root/.claude/claude_desktop_config.json \
    --allowed-tools "${ALLOWED_TOOLS:-mcp__farga__search_signals,mcp__farga__read_context}" \
    "$(cat /tmp/agent-task.txt)" 2>&1) || true
```

- [ ] **Step 3: Validate shell syntax**

Run: `sh -n /Users/bedardpl/project/Caissa/sandbox/entrypoint.sh`
Expected: no output, exit code 0 (this only checks syntax, not behavior — Task 5 verifies behavior against a real cluster).

- [ ] **Step 4: Commit**

```bash
cd /Users/bedardpl/project/Caissa
git add sandbox/entrypoint.sh
git commit -m "fix(entrypoint): task mode now uses credentials, mcp-config, and allowed-tools"
git push
```

---

### Task 5: Build, deploy, and verify with a real dispatched agent

**Files:** none (operational verification, no code changes).

**Interfaces:**
- Consumes: everything from Tasks 1–4, deployed.

- [ ] **Step 1: Trigger Caissa's CI to build and push the new `caissa` and `caissa-sandbox:guilhem` images**

The push in Task 4's commit triggers `Caissa/.github/workflows/build.yml` automatically (same pipeline verified working earlier this session). Confirm it succeeds:
```bash
gh run list --repo miegjorn/Caissa --limit 1
```
Expected: `completed  success` for the `Build & push` workflow on the commit from Task 4.

Separately, the dispatched-agent image (`caissa-sandbox:guilhem`) is built by `Fondament/.github/workflows/build-guilhem.yml` (it extracts the `caissa` binary and runs `caissa build guilhem`, which bakes in `sandbox/entrypoint.sh` via `include_str!` at compile time — see `caissa-cli/src/commands/build.rs`). Trigger it after Caissa's build completes:
```bash
gh workflow run build-guilhem.yml --repo miegjorn/Fondament
```
Then poll:
```bash
gh run list --repo miegjorn/Fondament --workflow=build-guilhem.yml --limit 1
```
Expected: `completed  success`.

- [ ] **Step 2: Confirm ArgoCD synced the new `dispatcher` and `guilhem` images**

```bash
kubectl get application dispatcher guilhem -n argocd -o jsonpath='{range .items[*]}{.metadata.name}{": "}{.status.sync.status}{" "}{.status.health.status}{"\n"}{end}'
```
Expected: both `Synced Healthy`. If not synced within a minute or two (automated sync policy should pick it up — confirmed present on both apps earlier this session), force it:
```bash
kubectl patch application dispatcher -n argocd --type merge -p '{"operation":{"sync":{"revision":"HEAD"}}}'
kubectl patch application guilhem -n argocd --type merge -p '{"operation":{"sync":{"revision":"HEAD"}}}'
```

- [ ] **Step 3: Dispatch one real test agent via the dispatcher's MCP endpoint**

Port-forward the dispatcher service if not already reachable, then call `invoke_agent` directly (bypassing Guilhem, to isolate testing this pipeline alone):
```bash
kubectl port-forward -n agents svc/dispatcher 19090:9090 &
sleep 2
curl -s -X POST -H 'Content-Type: application/json' -d '{
  "jsonrpc":"2.0","id":1,"method":"tools/call",
  "params":{"name":"invoke_agent","arguments":{
    "domain":"caissa",
    "facet":"developer",
    "task":"Run git --version and gh --version, then call the farga MCP tool search_signals with project=occitan, then report what you found. Do not modify any files.",
    "context":"You are a test dispatch verifying credential and tool wiring.",
    "allowed_tools":"Bash,mcp__farga__search_signals",
    "session_id":"verify-credentials-task5"
  }}
}' http://localhost:19090/mcp
```
Expected: a JSON response containing a `job_id`.

- [ ] **Step 4: Poll for the job's completion and inspect its result**

```bash
kubectl get jobs -n agents -l caissa.io/session=verify-credentials-task5 -w
```
Wait for `COMPLETIONS` to show `1/1`, then Ctrl-C and check the result:
```bash
curl -s -X POST -H 'Content-Type: application/json' -d '{
  "jsonrpc":"2.0","id":1,"method":"tools/call",
  "params":{"name":"search_signals","arguments":{"project":"verify-credentials-task5"}}
}' http://localhost:7500/mcp
```
(Requires the Farga port-forward from earlier in this session, `kubectl port-forward -n occitan-system svc/farga 7500:7500`, to be active.)

Expected: the signal content shows `git version ...` and `gh version ...` output (proving Bash + credentials work — `gh --version` doesn't require auth, but its presence confirms the binary ran without an approval block) and real Farga signal content from the `search_signals` call (proving MCP wiring works) — not "command requires approval" or "No Farga MCP is connected" as seen in every pre-fix dispatch this session.

- [ ] **Step 5: Clean up the test job**

```bash
kubectl delete job -n agents -l caissa.io/session=verify-credentials-task5
```

## Follow-up (not a task in this plan)

The 6 jobs that failed under the old behavior earlier this session (`agent-amassada-developer-249ab399` and the other 5 — all already `Completed`/cleaned up, not stuck) do not need re-dispatching by this plan. Whoever resumes that backlog (Guilhem, per his own stated "I will poll for results and report back" model) re-dispatches them naturally once this fix is live; the 7 underlying blockers were already fixed via direct PRs earlier this session, so most of them are likely no-ops on re-dispatch.
