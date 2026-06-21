# Persistent per-room Claude sessions for Guilhem's Matrix presence

## Why

Two compounding problems with the current architecture (`Caissa/caissa-cli/src/commands/listen.rs`'s `run_matrix_reply`, one fresh `claude --print` process per Matrix message):

1. **No continuity.** Each reply is a stateless process that exits after one response. Guilhem cannot "wait and check back" on a dispatched job (confirmed live this session: he promised to poll in ~4 minutes, but the process that made that promise had already exited by the time anyone could ask about it). Follow-up questions like "did you get your answer?" fail to resolve correctly because there's no real conversation thread — only a flattened text re-statement of room history, re-sent in full on every message.
2. **Latency.** Every message pays full process cold-start, and (when MCP tools attach) a fresh MCP handshake, on top of model inference time. This is the "painfully long" chat experience the user is trying to fix.

A smaller, already-scoped fix (`--resume`/`--session-id` continuity within the existing one-shot-process model) addresses problem 1 but not problem 2 — it still spawns a new process per message. This spec covers the larger, user-chosen path: a genuinely persistent process per actively-chatting room, which fixes both, and additionally unlocks running the same skills/plugins (including Superpowers — verified against current docs, not assumed) that an interactive Claude Code session has, which the one-shot `--print` model cannot use at all.

## Scope

Two repos.

1. **Caissa** — `listen.rs` becomes a process supervisor: one persistent Node.js sidecar (running the Claude Agent SDK) per actively-chatting Matrix room, spawned on first message, idle-timeout torn down, communicating over stdin/stdout.
2. **Fondament** — persona definitions gain a `skills` field declaring which Superpowers skills should be available to that persona's SDK session, alongside the existing `tools` field.

## 1. Mechanism: Claude Agent SDK, not the bare CLI

`claude` without `--print` is an interactive TUI built for an attached terminal. Driving it via raw stdin/stdout from Rust would mean parsing ANSI/UI output — a fragile PTY hack, not a real integration point. The **Claude Agent SDK** (Anthropic's TypeScript/Python library, built on the same underlying agent harness as Claude Code) is the documented, supported mechanism for holding a persistent, multi-turn, tool/MCP/skill-enabled session open programmatically via `query()`.

Confirmed against current docs (not assumed):
- The SDK's `query()` has a `skills` option. Omitted, all filesystem-discovered Skills are enabled with the `Skill` tool available; `"all"` enables every discovered skill; a list of names enables only those; `[]` disables all.
- **Plugins are fully programmatically loadable** — pass their local install path in the SDK config. Superpowers is explicitly named as a real, working plugin under this model.
- Caveat that shapes the sidecar's deployment: *"The SDK does not provide a programmatic API for registering Skills — Skills are discovered through the filesystem."* The sidecar process needs filesystem access to wherever Superpowers and other plugins are installed (the same `.claude/plugins/` structure used elsewhere), not just network/API access.

This means the agent image (`caissa-sandbox:guilhem`, already built with `@anthropic-ai/claude-code` per `sandbox/Dockerfile.agent`) needs the Agent SDK package added, and the Superpowers plugin installed at a filesystem path the sidecar can see — both image-build-time additions, not runtime.

## 2. Process model: one persistent process per active room

Confirmed choice over a shared multi-room process: per-room isolation means a crash/hang in one room's session can't affect any other room, and idle-timeout naturally caps total resource use to however many rooms are actually active at once. This also matches the per-room mental model Charradissa already uses (`agent_routes`, `component_agents`).

## 3. Lifecycle, living in Guilhem's existing pod

Extend `listen.rs`'s `run_matrix_reply` path from "spawn `claude --print`, wait for exit, return" into a supervisor:

- `ListenState` gains `room_sessions: Arc<RwLock<HashMap<String, SessionHandle>>>`.
- `SessionHandle` wraps a long-running Node.js child process (the SDK sidecar) plus a `last_activity: Instant`.
- On a new message: look up the room's `SessionHandle`. If present and the child process is still alive, write the message as one line of newline-delimited JSON to its stdin, read the response line from its stdout. If absent (first message, or the prior session was reaped), spawn a new sidecar process, write an init message (system prompt, MCP server URLs, allowed tools, skills list — see section 5), then the actual message.
- A background sweep (the same `tokio::spawn` loop pattern already used elsewhere in `listen.rs` for the archival/tick loops) walks `room_sessions` every minute; any handle with `last_activity` older than the idle timeout (default 30 minutes, configurable via `caissa.toml`) gets its child process killed and its entry removed.
- Pod restart: `room_sessions` is in-memory only — a pod restart loses all live sessions, which is acceptable (matches the existing precedent that Guilhem's chronicle/memory lives in Farga, not in himself; conversational session state resetting on restart is a reasonable trade, not data loss).

## 4. The sidecar process (new component)

A small Node.js script (new file, `Caissa/sandbox/agent-sidecar.js`, baked into the agent image alongside `entrypoint.sh`) that:
- On startup, reads one line of JSON from stdin: the init payload (system prompt, MCP server configs, allowed tools, skill names, model).
- Opens a Claude Agent SDK session with that configuration.
- Loops: read one line of JSON from stdin (`{"message": "...", "sender": "..."}`), call the SDK's `query()` with the message, write one line of JSON to stdout (`{"reply": "..."}`) when the response completes.
- Exits cleanly on stdin EOF (the parent process closing the pipe is how `listen.rs` signals teardown, alongside an OS-level kill as a backstop).

## 5. Fondament: `skills` field on persona definitions

Parallel to the existing `tools: { always_on, jit }` field (populated for all 6 facets earlier this session), add a `skills` field — a flat list, since skills don't have the same permission-gating distinction tools do; the SDK's `skills` option takes a plain list of names.

Mapping, by role:

| Persona file | Skills | Why |
|---|---|---|
| `guilhem.yaml` | `superpowers:brainstorming`, `superpowers:writing-plans`, `superpowers:subagent-driven-development`, `superpowers:test-driven-development`, `superpowers:systematic-debugging`, `superpowers:requesting-code-review`, `superpowers:finishing-a-development-branch`, `superpowers:verification-before-completion`, `superpowers:using-git-worktrees` | Guilhem currently does direct dev work pending the dispatcher framework (per his own persona note added earlier this session) — needs the full build→verify→ship loop, plus design skills for when asked to plan something new. |
| `developer.yaml` | `superpowers:test-driven-development`, `superpowers:systematic-debugging`, `superpowers:requesting-code-review`, `superpowers:receiving-code-review`, `superpowers:finishing-a-development-branch`, `superpowers:verification-before-completion`, `superpowers:using-git-worktrees` | Matches the discipline already written as prose into this file's `context:` — these skills are the same loop, just as real invocable Superpowers skills instead of restated instructions. |
| `infra-engineer.yaml` | Same as `developer.yaml` | Infra work (Helm/chart/deploy fixes observed this session) follows the identical ship loop. |
| `qa-engineer.yaml` | `superpowers:test-driven-development`, `superpowers:systematic-debugging`, `superpowers:verification-before-completion` | Narrower — test-focused, no PR-shipping skills needed for a facet whose job is verifying others' work. |
| `security-analyst.yaml` | `superpowers:systematic-debugging`, `superpowers:receiving-code-review`, `superpowers:verification-before-completion` | Adversarial-review focused (matches this file's existing adversarial-stance context); not a primary code-shipping role. |
| `app-architect.yaml` | `superpowers:brainstorming`, `superpowers:writing-plans` | Design-focused role — these two skills are literally what this session used to get from idea to spec. |
| `data-architect.yaml` | `superpowers:brainstorming`, `superpowers:writing-plans` | Same reasoning as `app-architect.yaml`. |

This is a judgment call based on each role's documented function, same as the `tools.always_on` baseline decision earlier — not derived from any code, since nothing yet consumes this field (same as `tools.always_on` before this session's earlier work). The sidecar's init payload reads the resolved facet's `skills` list the same way it reads `tools.always_on` — via the caller (Guilhem) reading the facet file and passing the list through, not via a Fondament-resolver integration (out of scope, matches the existing `tools.always_on` consumption model).

## Out of scope

- A Fondament-side resolver that automatically reads `tools`/`skills` from the YAML and pushes them through the dispatch chain without manual relay — this spec keeps the same manual-relay model already in place for `tools.always_on` (flagged as a known soft spot in that design, not fixed here).
- Persisting `room_sessions` across pod restarts (e.g. via Farga) — in-memory is the explicit choice for v1.
- The shared-process (one process, many room sessions) alternative — explicitly rejected in favor of per-room isolation.
- Verifying full feature parity between the SDK and the interactive CLI beyond what's confirmed in section 1 — the skills/plugins question was explicitly de-risked by checking docs; other parity questions (if any surface during implementation) are implementation-time discoveries, not pre-verified here.

## Testing

- `listen.rs`'s session-supervisor logic (spawn-if-absent, route-if-present, idle-reap) is unit-testable with a fake/mock child-process abstraction — exact test design deferred to the implementation plan, following this session's established pattern of pure-function extraction for testability (e.g. `build_job` in `dispatch.rs`).
- `agent-sidecar.js` has no existing Node test harness in this repo — manual verification against a real room, same bar as `entrypoint.sh`'s shell changes earlier this session.
- Fondament's `skills` field: YAML syntax validation only, same as `tools.always_on` (content-only change, no code consumes it yet).
