# ADR-C-001: Independent-Agent Matrix Registration Model

**Status:** Accepted
**Date:** 2026-07-02
**Deciders:** Guilhem, Pierre-Luc
**Refs:** Occitan#42, Fondament#40 (Epic), Occitan#43 (Initiative), Occitan#21 (Workspace Provisioning), Fondament#21 (Charradissa provisioning openBao role)

## Context

Prior to 2026-07-02, the mental model of Charradissa's Matrix role conflated two distinct
responsibilities: **workspace provisioning** (creating rooms, setting power levels, inviting
participants) and **per-agent identity** (claiming agent usernames in the appservice user
namespace and relaying room events on behalf of agents). Under that model Charradissa's
lifecycle was coupled to agent identity — an agent's presence depended on Charradissa's
appservice claiming its username and relaying its events.

Five commits on 2026-07-02 in Charradissa dissolved that coupling:

- `fix(registration): stop claiming the 9 independent agents' usernames exclusively`
- `fix(appservice): never relay for the 9 independently-run agent rooms`
- `refactor: charradissa-daemon stops relaying for the 9 independent agents`
- `fix(appservice): update guilhem's room in the migration skip-list to #occitan`
- `fix(transport): match Amassada's Transport::consult contract change`

The 9 independently-run agents are the 8 Occitan-stack component agents (Gardian, Fondament,
Farga, Amassada, Charradissa, Cor, Caissa, Nervi) plus Guilhem, the org steward — whose room the
skip-list migration explicitly re-pointed to `#occitan`.

## Decision

**1. Per-agent Matrix independence.** Each of the 9 agents runs its own independent Matrix
session and handles its own room directly. Agents authenticate independently; their Matrix
identity is not mediated through Charradissa at runtime.

**2. Charradissa does not relay for independent agents.** The Charradissa daemon and appservice
no longer relay events for the 9 independently-run agent rooms. In `charradissa-matrix`
(`appservice.rs`) these rooms are on a skip-list: *"handled entirely by their own agent's
independent Matrix session now — never relay, regardless of what project_routes / … say"* —
preventing double-relay. The Charradissa appservice `registration` no longer claims those
agents' usernames exclusively.

**3. Charradissa AS scope = workspace provisioning only.** Charradissa retains its Application
Service status, narrowed to **workspace provisioning** for new Occitan projects: room creation,
space membership, power-level configuration, agent invitation, and archival — the room-lifecycle
topology it owns. It does NOT own agent accounts or provide runtime event relay for
independently-run agents.

**Concrete boundary**
- Charradissa AS authority: room creation, membership provisioning, power levels, topic/metadata
  for new workspaces, room archival.
- Agent-owned: username registration/authentication and room participation via the agent's own
  Matrix session.
- Out of scope for Charradissa: agent login, credential renewal, runtime Matrix session
  management, event relay for the 9 independent rooms.

## Consequences

**Positive**
- Agent Matrix identity is decoupled from Charradissa availability — an agent operates as long as
  its own credentials are valid, independent of Charradissa's health.
- No double-relay: each independent room has exactly one session handling it (the agent's own).
- Charradissa's scope is cleanly bounded — workspace setup at project creation, not ongoing
  session management. Simpler provisioning mental model: Charradissa provisions the *room*, the
  agent provisions its own *account*.

**Negative / constraints**
- Each agent component must manage its own Matrix credentials (secret storage, renewal) as a
  per-agent operational responsibility.
- The Workspace Provisioning flow (Occitan#21) must distinguish room provisioning (Charradissa)
  from agent onboarding (each agent) — two separate steps, not one.
- The Caissa bootstrap job (`caissa bootstrap-matrix-agents`) must align with this model; its
  behavior needs verification against the new definition (per Occitan#42, cross-component
  impact).
- Any tooling or documentation that assumed Charradissa owns/relays agent accounts must be
  updated (e.g. anything reading the old Charradissa-as-relay topology).

## Implementation notes

- The refactor landed across five Charradissa commits on 2026-07-02; identified for ADR during
  nightly-dream consolidation 2026-07-03.
- Skip-list / never-relay logic: `charradissa-matrix/src/appservice.rs` (9 independent rooms).
- Appservice user-namespace construction: `charradissa-core/src/registration.rs`
  (component-agent namespace entries); daemon relay removal: `charradissa-daemon/src/registry.rs`.
- Project-driven provisioning design (Charradissa's provisioning-only role) is specified in
  `charradissa/docs/superpowers/specs/2026-06-27-project-driven-room-provisioning-design.md` and
  the corresponding plan; provisioning-config coverage in
  `charradissa-core/tests/provisioning_config_tests.rs`.
- The provisioning agent's credential scope (openBao role) is tracked in Fondament#21
  ("openBao role definition for Charradissa provisioning agent"); the role grants
  workspace-provisioning scope, not agent-account management. NOTE: as of this ADR, Fondament#21
  is open and there is no separate `charradissa-provisioner.yaml` definition in
  `definitions/fondament/` — the Charradissa agent is defined by `charradissa-agent.yaml`.
- Occitan#42 tracks the refactor establishing this model; cross-reference Occitan#21 (Workspace
  Provisioning) for the full provisioning flow.
