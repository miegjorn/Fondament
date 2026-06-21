# Developer discipline: TDD, docs-in-sync, draft-PR-until-green, Farga TODO

## Why

This session surfaced two real cases of doc/code drift causing hours of debugging:
- `Caissa/docs/install.md` referencing the pre-migration `ghcr.io/occitan/...` registry
  names, long after the org moved to `miegjorn`.
- `Gardian`'s `OpenBaoBackend` module existing only in a local working tree —
  never committed — so every CI build since had been failing to compile, silently
  masked because Kubernetes kept the old pod alive.

Neither was caught by review because nothing enforces "the PR that changes behavior
also updates what describes that behavior, and ships only once CI proves it." This
spec bakes that enforcement into the developer persona itself, so it applies to every
PR Guilhem (or a future dispatched developer agent) ever opens — not as a one-off
scan, but as standing practice.

## Scope

Two persona files, one small Farga addition. No new GitHub Actions workflow, no new
webhook, no new CI secret plumbing.

1. `Fondament/definitions/fondament/developer.yaml` — gains the discipline.
2. `Fondament/definitions/fondament/guilhem.yaml` — defers to it for direct dev work,
   since the dispatcher/delegation framework (Guilhem → domain/facet developer agents
   via the dispatcher MCP) doesn't exist yet. Until it does, Guilhem does dev work
   himself (Bash + git/gh, as he already did for the Amassada WS-fanout PR) and
   follows the same discipline directly.
3. `Farga` — add an upsert-able per-component TODO record, replacing the original
   "TODO.md per repo" idea. Farga already has a `Node` type with a `ComponentLayer`
   kind and a `stale` flag clearly intended for this; nothing currently writes one.

A one-time retroactive sweep of the 7 existing repos is in scope as a follow-up
operational task (not new infra) — applying the new discipline backward once it
lands, to catch drift that already exists.

## 1. The discipline (developer.yaml)

Ordered workflow for any PR:

1. **TDD first.** Write a failing test that captures the change. Confirm it fails for
   the right reason. Implement until it passes. Never implementation-then-test-after.
2. **Docs-in-sync.** If the change alters CLI flags, config schema, env vars,
   endpoints, or workflow/pipeline steps, update the relevant README/docs in the
   *same PR*, same commit boundary as the code change. Never "code now, docs later."
3. **Open as draft.** Every PR starts as a draft (`gh pr create --draft`).
4. **Verify CI green.** After pushing, poll the PR's checks (`gh pr checks`). If
   anything is red, fix it and push again — don't wait for a human to flag it.
5. **Flip to ready.** Call `gh pr ready` only once all checks pass. A PR left in draft
   signals "still working," not "needs review."
6. **Log unresolved follow-ups.** Anything found but deliberately deferred — a real
   scoping decision, not a vague "I'll get to it" — gets written to Farga as a
   per-component TODO entry (see below), not left to verbal mention or a file that
   will itself go stale.

This is additive to the existing `developer.yaml` content (clean code, precise naming,
flagging tech debt) — it does not replace any of the current bullets.

## 2. Guilhem's deferral (guilhem.yaml)

Add a short note to Guilhem's context: when the dispatcher/delegation framework is not
yet available and he does dev work directly (as in the Amassada PR), he follows the
developer persona's discipline above — TDD, docs-in-sync, draft-until-green, Farga
TODO logging. Once dispatch exists, this becomes the default path for dispatched
developer agents instead, and Guilhem's role reverts to chronicling rather than
directly authoring code.

## 3. Farga per-component TODO

Current state (checked this session): `farga-core::types::Node` has a `ComponentLayer`
variant of `NodeKind` and a `stale: bool` field, but:
- only `insert_node` exists in `farga-server/src/db.rs` — no update/upsert path.
- nothing in the codebase ever constructs a `ComponentLayer` node.

Addition:
- `farga-server/src/db.rs`: `upsert_component_todo(pool, project, component, content)`
  — find a `ComponentLayer` node matching `(project, component)`; if found, overwrite
  `content` and bump `updated_at`; if not found, insert one via `Node::new`.
- Expose as a new Farga MCP tool (`update_component_todo`), alongside the existing
  `post_signal` / `search_signals` / `read_context` / `list_projects` tools already
  wired into Guilhem's `caissa listen` MCP config. No new transport — same MCP
  connection, one more tool in the allow-list.
- Read path: existing `search_signals`/`read_context`-style queries extend naturally
  to also surface `ComponentLayer` nodes; no new read endpoint required for v1.

This replaces the original "maintain a TODO.md per repo" idea: one live, queryable
record per component instead of a markdown file that would itself be exactly the
kind of doc that goes stale.

## 4. Retroactive sweep (follow-up, not new infra)

Once the persona change and Farga addition land: walk each of the 7 repos
(Fondament, Caissa, Amassada, Charradissa, Farga, Gardian, Cor) applying the
discipline backward — diff docs against actual code/config, fix what's safely
fixable in a draft PR per repo, log anything requiring judgment as a Farga
`ComponentLayer` TODO. This is an operational task using the now-existing discipline,
not a new mechanism.

## Out of scope

- No scheduled/webhook trigger ("scan on every push to main") — superseded by the
  persona-based framing. The discipline applies automatically whenever any PR is
  opened, with no separate trigger needed.
- No GitHub Actions changes in any of the 7 repos.
- No new Anthropic API key / secret plumbing.
- Dispatcher/delegation framework itself (Guilhem → domain/facet developer agents) —
  referenced as a future dependency, not built here.

## Testing

- `developer.yaml` / `guilhem.yaml` are prose context files, not code — no unit
  tests apply. Validation is behavioral: next time Guilhem opens a PR, confirm it
  follows the draft → CI-green → ready sequence and includes doc updates if behavior
  changed.
- The Farga addition (`upsert_component_todo` + MCP tool) gets normal Rust unit/
  integration test coverage in `farga-server`, following existing patterns in that
  crate (e.g. `tests/convention_tests.rs`-style coverage seen in `gardian-core`).
