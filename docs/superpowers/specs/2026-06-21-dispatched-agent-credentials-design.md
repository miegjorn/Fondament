# Dispatched agent tool/credential provisioning, driven by Fondament personas

## Why

Guilhem's dispatcher (`caissa dispatch`, the MCP server that creates k8s Jobs for
domain/facet sub-agents) works mechanically — Jobs schedule, pull the right image,
and run. But every dispatched agent observed this session hit the same wall: no
git/gh credentials, and every tool call beyond a tiny auto-allowed set blocked
because the headless `claude --print` invocation never received `--allowed-tools`,
`--mcp-config`, or any credential injection. Each agent correctly refused to
fabricate a fix and reported the blocker honestly to Farga instead of hallucinating
a PR — the developer discipline worked exactly as designed even though the
underlying capability didn't exist yet.

Separately: Fondament already has a `tools: { always_on: [], jit: [] }` field on
every persona definition, and `fondament-core`'s resolver already merges these
across the domain×facet×persona composition into a final tool list
(`fondament-core/src/resolver.rs:83-84`). Nothing has ever populated it, and
nothing downstream consumes it. This spec wires that existing-but-inert contract
up to actually drive what a dispatched agent can do.

## Scope

Two repos. No new services, no new MCP servers, no RBAC changes (secret resolution
for the new init container happens via kubelet using `secretKeyRef`, not via the
dispatcher's own k8s API client identity — no new Role permissions needed).

1. **Fondament** — populate `tools.always_on` on the 6 documented facet
   definitions, with an explicit facet-keyword → filename mapping (the dispatcher's
   documented keywords don't match the actual filenames 1:1 — only `developer`
   does).
2. **Caissa** — `invoke_agent`'s MCP schema gains an `allowed_tools` parameter;
   `build_job` gains the same OpenBao credential-injection init container Guilhem's
   own pod chart already has; `entrypoint.sh`'s task-mode branch actually uses
   `--mcp-config` and `--allowed-tools` instead of invoking `claude --print` bare.

## 1. Facet-keyword → filename mapping

The dispatcher's `invoke_agent`/`list_agent_specs` tool descriptions currently say
"Role facet: architect | developer | qa | infra | db | security" and tell Guilhem
to read `/fondament/roles/<facet>.yaml` — implying a filename match that doesn't
exist for 5 of 6 facets:

| Facet keyword | Fondament file | Notes |
|---|---|---|
| `developer` | `developer.yaml` | Already matches |
| `infra` | `infra-engineer.yaml` | |
| `qa` | `qa-engineer.yaml` | |
| `security` | `security-analyst.yaml` | Not `security-sre.yaml` (lives under `definitions/roles/`, a different directory, unrelated to the dispatcher's facet model) |
| `architect` | `app-architect.yaml` | Three architect-flavored files exist (`app-architect`, `aws-architect`, `data-architect`); `app-architect` is the general-purpose pick for ad-hoc cross-component architecture tasks |
| `db` | `data-architect.yaml` | No dedicated `db` file exists; `data-architect.yaml`'s data-modeling/storage/migration focus is the closest existing fit — reusing it rather than creating a new persona file |

This table goes into the `invoke_agent` tool description in `dispatch.rs`, replacing
the current `/fondament/roles/<facet>.yaml` instruction with the explicit mapping,
so Guilhem reads the right file every time instead of guessing from a directory
listing.

## 2. Fondament: populate `tools.always_on`

All 6 facet files get the same baseline list — every real dispatch this session
was code/doc work regardless of facet, so differentiating access levels now would
be guessing at a need that hasn't shown up yet. The `ToolDefinition` schema
(`fondament-core/src/tools.rs`) already supports per-facet differences whenever a
real one arises (e.g. a stricter read-only reviewer facet later) — this spec just
stops leaving the field empty.

Baseline list, expressed as `ToolDefinition` entries:

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

Applied identically to: `developer.yaml`, `infra-engineer.yaml`, `qa-engineer.yaml`,
`security-analyst.yaml`, `app-architect.yaml`, `data-architect.yaml`.

## 3. Caissa: `invoke_agent` gains `allowed_tools`

New optional parameter on `invoke_agent`'s input schema:

```
"allowed_tools": {
    "type": "string",
    "description": "Comma-separated Claude tool names for the spawned agent,
    read from the facet's tools.always_on (see the facet mapping table). Native
    tools pass through as-is (Bash, Edit, Write); Mcp tools are formatted as
    mcp__<server>__<tool> (e.g. mcp__farga__search_signals). Defaults to a
    read-only Farga tool set if omitted."
}
```

Threaded through `create_agent_job` as a new `ALLOWED_TOOLS` env var on the spawned
Job, same pattern as the existing `DOMAIN`/`FACET`/`TASK`/`AGENT_CONTEXT`/
`SESSION_ID` env vars.

## 4. Caissa: `build_job` gains credential injection

`build_job` (in `caissa-cli/src/commands/dispatch.rs`, already refactored to a pure
function earlier this session for testability) gains an init container and a creds
volume, mirroring `deploy/charts/guilhem/templates/guilhem.yaml`'s existing
`fetch-tokens` init container exactly — same OpenBao KV reads, same
`/creds/tokens.env` + `/creds/.git-credentials` + `/creds/.gitconfig` output, same
`emptyDir: { medium: Memory }` volume. Unconditional, not gated on a heuristic
(e.g. "does allowed_tools contain Bash") — every facet's baseline list includes
Bash per section 2, so the condition would always be true today; keeping it
unconditional avoids a second code path that silently diverges later.

## 5. Caissa: `entrypoint.sh` task-mode fix

Current task-mode branch:
```sh
OUTPUT=$(claude --print "$(cat /tmp/agent-task.txt)" 2>&1) || true
```

New:
```sh
[ -f /creds/tokens.env ] && . /creds/tokens.env
export GIT_CONFIG_GLOBAL=/creds/.gitconfig
OUTPUT=$(claude --print \
  --mcp-config /root/.claude/claude_desktop_config.json \
  --allowed-tools "${ALLOWED_TOOLS:-mcp__farga__search_signals,mcp__farga__read_context}" \
  "$(cat /tmp/agent-task.txt)" 2>&1) || true
```

This is the actual fix for what every dispatched agent hit this session: no
`--mcp-config` (MCP tools never connected despite the config file being written),
no `--allowed-tools` (every tool call blocked, headless mode can't answer the
resulting approval prompt), and no credentials (git/gh auth never set up). The
`${ALLOWED_TOOLS:-...}` fallback keeps task mode safe (read-only) if `invoke_agent`
is ever called without the parameter — consistent with the persona's own
discipline of not silently defaulting to broader access than requested.

## Out of scope

- Differentiating tool access per facet beyond the uniform baseline — no evidence
  yet that any facet needs less (or more) than `Bash, Edit, Write` + the three
  Farga tools.
- Creating a dedicated `db` persona file — `data-architect.yaml` is reused.
- RBAC changes to the dispatcher's ServiceAccount — not needed (see Scope).
- Retrying the 6 jobs that already failed this session under the old behavior —
  operational follow-up, not new code.

## Testing

- Fondament: YAML syntax validation (`python3 -c "import yaml; yaml.safe_load(...)"`)
  on each of the 6 edited files, same as the developer-discipline persona changes
  earlier this session. `fondament sweep` lint if available, same as before.
- Caissa `dispatch.rs`: extend the existing `build_job` unit tests
  (`build_job_sets_ghcr_pull_secret`, `build_job_uses_the_given_image`, added
  earlier this session) with two more: confirm the init container + creds volume
  are present, and confirm `ALLOWED_TOOLS` becomes an env var on the container when
  passed.
- `entrypoint.sh`: shell script, no Rust test harness — verified by actually
  dispatching one real agent Job after the fix lands and confirming `git`, `gh`,
  and a Farga MCP tool call all work inside it (manual verification step, not an
  automated test).
