# ADR-O-002: Multi-Provider Endpoint-Field Canvas Participant Pattern

**Status:** Accepted
**Date:** 2026-07-01
**Deciders:** Guilhem, Pierre-Luc
**Refs:** Occitan#39, Occitan#40, Fondament#40 (Epic), Occitan#43 (Initiative), Amassada#27, Caissa#63

## Context

Amassada's canvas dispatch model historically routed every participant turn through its own
`dispatch()` path, which calls the Anthropic API gateway directly. The 2026-07-01 Grok/xAI
integration introduced a requirement to run heterogeneous-provider canvases — Anthropic and
Grok/xAI participants complementing each other in the same canvas — without hard-coding
provider topology into Amassada's dispatch logic.

The mechanism chosen was an **endpoint field** carried on the participant / canvas
configuration (`ModelSpec.endpoint`, already live in Fondament and Amassada), paired with a
`model` string. When a participant carries an `endpoint`, Amassada dispatches that turn to the
endpoint instead of calling Anthropic directly.

This is deliberate division of labour, visible in `amassada-core/src/dispatch.rs`:

- Direct `dispatch()` **only supports Anthropic/Claude models**. It explicitly rejects
  `grok*` / `xai*` models with the message *"direct dispatch only supports anthropic/claude
  models; for grok/xai use endpoint in the participant/canvas config for complementary
  multi-model runs."*
- `dispatch_to_endpoint(endpoint_url, req)` POSTs a `TurnHttpRequest` to `{endpoint}/turn`
  and expects a `TurnHttpResponse` back. This is a general external-turn transport, not a
  Grok-specific one.

The **current** Grok/xAI integration is a deliberately non-agentic *shim*: the Caissa
entrypoint shim executes a `curl` to the configured endpoint when `MODEL` starts with `grok`
(Caissa: *"Shim is intentionally non-agentic (no MCP); proper agentic Grok support goes via
Amassada endpoint to a dedicated service."*).

## Decision

**1. Endpoint-field canvas participants.** A canvas participant may carry an `endpoint` field
(`ModelSpec.endpoint`). When present, Amassada routes that participant's turns to the endpoint
via `dispatch_to_endpoint` rather than through direct Anthropic dispatch. This enables
complementary multi-provider canvases (Anthropic + Grok/xAI, and future providers) without a
central provider registry and without a Fondament definition change per new inference provider.

**2. `dispatch()` is Anthropic-only.** Direct dispatch rejects `grok*` / `xai*` models by
design. Non-Anthropic providers are reachable *only* via the endpoint mechanism. This keeps the
Anthropic-gateway code path free of provider-conditional branching.

**3. The current Grok/xAI shim is non-agentic.** The 2026-07-01 curl-shim form of the endpoint
mechanism is inference-only. In that form a participant MUST NOT:
- use MCP tool invocation;
- trigger sub-dispatches or agent spawning;
- access the Nervi fabric or the Farga context system.

The endpoint transport itself carries an `mcp_scopes` field (propagated verbatim to a receiving
*agent* endpoint), so the mechanism is not intrinsically non-agentic — but the current Grok/xAI
integration uses it in inference-only mode. Anything richer must go through the agentic upgrade
path below, not through the curl shim.

**4. Upgrade path.** "Proper agentic Grok support" is explicitly *not* the shim: it goes via an
Amassada endpoint pointing at a **dedicated agentic service** that receives `mcp_scopes` and
enforces its own tool surface. Until that service exists, endpoint participants remain
inference-only.

**5. SSRF validation is mandatory (see Consequences / Occitan#40).** Any `endpoint` value MUST
be validated against an allowlist before dispatch. This is the load-bearing security control of
the pattern and is currently a known gap under remediation.

## Consequences

**Positive**
- Complementary multi-provider inference without registry coupling; a new inference provider is
  a single endpoint URL, no Fondament definition change.
- One canvas execution model for homogeneous and heterogeneous-provider canvases.
- The Anthropic dispatch path stays provider-clean; provider variance lives entirely behind the
  endpoint transport.

**Negative / constraints**
- Current shim is inference-only: endpoint participants cannot use tools, MCP, Nervi, or Farga
  until the dedicated agentic service exists.
- **SSRF is the critical risk.** The `endpoint` field currently accepts any HTTP URL without
  validation (Occitan#40, Farga signal a2771fa5, nightly-dream adversarial challenge
  2026-07-01). If a participant definition derives from untrusted input (a Matrix message, an
  external signal, a tampered Farga-stored canvas), a malicious `endpoint` can point at an
  internal cluster URL (`*.svc.cluster.local`, RFC-1918, `localhost`/`127.*`) and the shim's
  curl / `dispatch_to_endpoint` becomes a confused deputy — a Server-Side Request Forgery that
  bypasses ingress-layer network policy. This is ELOPe-adjacent in capability (system-defence
  A-7) and the fix belongs at the substrate, before the HTTP call (A-2), not as an agent-level
  check.
- Endpoint URLs are not validated against any Fondament provider registry; a wrong URL fails at
  dispatch time rather than at definition time.

## SSRF validation requirement (Occitan#40)

Before any endpoint dispatch, the URL MUST be validated against an **allowlist of permitted
external model-provider domains**. The validation MUST reject:
- `*.svc.cluster.local`, `*.cluster.local` and any other in-cluster service DNS;
- RFC-1918 private ranges (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`);
- loopback (`127.0.0.0/8`, `localhost`) and link-local (`169.254.0.0/16`);
- IPv6 equivalents (`::1`, `fc00::/7`, `fe80::/10`);
- cloud metadata endpoints (`169.254.169.254`, `100.100.100.200`, …);
- any host that resolves to a private/internal address after DNS resolution.

Validation is enforced in **two** places, defense-in-depth:
- **Amassada** — validate `participant.endpoint` before `dispatch_to_endpoint` (tracked:
  Amassada#27, open Epic).
- **Caissa entrypoint shim** — validate before executing the `curl` (tracked: Caissa#63, open
  Epic).

As of this ADR both remediations are **open** — the allowlist is not yet enforced. Guilhem must
review the proposed allowlist before the component agents merge (Class 2).

## Implementation notes

- `ModelSpec.endpoint` is live in Fondament and Amassada as of 2026-07-01.
- Amassada dispatch split: `dispatch()` (Anthropic-only, rejects grok/xai) vs.
  `dispatch_to_endpoint(endpoint_url, req)` → `POST {endpoint}/turn`
  (`amassada-core/src/dispatch.rs`).
- The endpoint transport carries `mcp_scopes` (restricts the receiving agent pod's MCP tools;
  empty = no restriction) — this is the seam for the future agentic-service upgrade path.
- Caissa entrypoint shim curls the endpoint when `MODEL` starts with `grok`; intentionally
  non-agentic (no MCP).
- SSRF remediation: Amassada#27 and Caissa#63 (both open Epics under Occitan#43).
- This ADR supersedes the prior assumption that every Amassada canvas participant uses an
  Anthropic-hosted model dispatched through `dispatch()`.
