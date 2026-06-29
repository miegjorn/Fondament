# Intake Agent — Canonical Output Format

This document defines the file structure and content contracts for the output
produced by the `intake-architect` agent when onboarding a new project into
the Occitan stack.

Every field marked **required** must be present and non-empty. Fields marked
**optional** may be omitted; their absence is meaningful (not an oversight).
Use `# TODO:` for fields you cannot fill from the brief. Use
`# DECISION NEEDED:` for fields where the brief is ambiguous on a design choice
that belongs to Pierre-Luc.

---

## Directory structure

```
<project-slug>/
  domains/<project-slug>.yaml   # Domain definition — the primary context document
  roles/
    architect.yaml              # Architect role stub
    developer.yaml              # Developer role stub
  capabilities.yaml             # Capability contracts (exposes + consumes)
  README.md                     # Generated project description
```

All files are placed under `Fondament/definitions/` in the Fondament repo.
Domain files go under `definitions/domains/`, role stubs under
`definitions/fondament/` (named `<project-slug>-architect.yaml` etc.).

---

## `domains/<project-slug>.yaml`

```yaml
# required
id: domain/<project-slug>
kind: domain
repo: <RepoName>                  # PascalCase, matches GitHub repo name

# optional: identifies the default facet for `caissa spawn <slug>`
default_facet: architect

context: |
  # required: 2–5 paragraphs. Answer: what is this project, what does it do,
  # what are its hard constraints, what does it NOT do.

  ## What <ProjectName> is
  <one paragraph: purpose and responsibility>

  ## Design constraints
  <bullet list of hard constraints — technical or organizational>

  ## Dependencies
  <bullet list of other Occitan components this project depends on>
```

**Content rules for `context:`**
- Lead with purpose, not implementation. The first sentence should answer
  "what does this project exist to do?"
- Name every hard constraint explicitly. Constraints that are implicit become
  misunderstandings later.
- Do not describe future aspirations as current capabilities.

---

## `roles/<project-slug>-architect.yaml`

```yaml
# required
id: fondament/<project-slug>-architect
kind: role
default_model: claude-sonnet-4-6   # or claude-opus-4-8 for reasoning-heavy roles

context: |
  # required: who is this agent, what does it own, how does it act.
  # Keep to 3–6 paragraphs. Avoid policy lists — prefer behavioral description.

  You are the architect agent for <ProjectName>.

  <What the architect owns and decides>

  <How the architect approaches design tradeoffs in this domain>

  <What the architect does not do — scope boundaries>

tools:
  always_on:
    # required: at minimum, Farga read/write for context continuity
    - id: farga-read-context
      kind: mcp
      server: farga
      tool: read_context
    - id: farga-write-signal
      kind: mcp
      server: farga
      tool: write_signal
    - id: farga-search-signals
      kind: mcp
      server: farga
      tool: search_signals
    # add project-specific MCP tools here
  jit: []                           # tools loaded on demand (expensive or rare)

# optional: list superpowers skill slugs relevant to this role
skills:
  - superpowers:systematic-debugging
  - superpowers:test-driven-development
```

---

## `roles/<project-slug>-developer.yaml`

```yaml
# required
id: fondament/<project-slug>-developer
kind: role
default_model: claude-sonnet-4-6

context: |
  You are a developer agent for <ProjectName>.

  <What the developer implements, the language/toolchain, the style expectations>

  <How the developer handles PRs, tests, CI — mirror the stack-wide developer
   discipline from fondament/developer unless project-specific rules differ>

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
    - id: farga-read-context
      kind: mcp
      server: farga
      tool: read_context
    - id: farga-write-signal
      kind: mcp
      server: farga
      tool: write_signal
    - id: farga-update-component-todo
      kind: mcp
      server: farga
      tool: update_component_todo
  jit: []

skills:
  - superpowers:test-driven-development
  - superpowers:systematic-debugging
  - superpowers:finishing-a-development-branch
```

---

## `capabilities.yaml`

```yaml
# required
project: <project-slug>

# What this project exposes to the rest of the Occitan stack.
# List only confirmed, implemented capabilities — not aspirational ones.
exposes:
  # - id: <capability-id>
  #   kind: http-api | mcp-tool | matrix-identity | helm-chart | library
  #   description: <one line>
  #   endpoint: <url-pattern or tool-name>   # optional

# What this project depends on from the rest of the stack.
# Match against other projects' exposes entries.
consumes:
  # - project: <other-project-slug>
  #   capability: <capability-id>
  #   required: true | false    # false = optional / graceful degradation
```

**Rules:**
- An `exposes` entry that has no corresponding `consumes` entry in another
  project is orphaned — flag it with `# ORPHANED:` if it seems intentional,
  or remove it if it's aspirational.
- `required: false` means the project degrades gracefully when the dependency
  is unavailable. Only mark `false` if the code actually handles the absence.

---

## `README.md`

```markdown
# <ProjectName>

<One paragraph: what the project is and what it does. Machine-readable enough
that an agent can parse it, human-readable enough that Pierre-Luc can skim it.>

## What it is not

<One paragraph: explicit scope exclusions. What problems this project does NOT
solve, what it delegates to other components.>

## Occitan stack position

| Depends on | <list of components> |
|---|---|
| Exposes to | <list of components> |
| Namespace | <k8s namespace if applicable> |
| Port | <port if applicable, else —> |
```

---

## Worked example — project "Nèrvi"

The following is a complete, annotated example using the Nèrvi async
subscription fabric as the intake subject.

### `domains/nervi.yaml`

```yaml
id: domain/nervi
kind: domain
repo: nervi
default_facet: architect
context: |
  Nèrvi is the async subscription fabric of the Occitan stack — the nervous
  system layer that makes the stack event-driven by machine signals, not only
  by human conversation.

  In Occitan, "nèrvi" means nerve — sinew, impulse, the thread that carries
  sensation. Nèrvi carries machine-readable signals between components without
  requiring a synchronous session or a human participant.

  ## What Nèrvi is

  Nèrvi is a NATS JetStream deployment plus an MCP server that exposes
  pub/sub primitives to agents. Any agent with the Nèrvi MCP tools can
  publish a signal to a topic and any subscribed agent can consume it,
  asynchronously, without coordination.

  First scope: SRE log monitor → developer consumer on `ops.sre.alerts`.

  ## What Nèrvi is not

  Nèrvi is not Charradissa. Charradissa is the human-readable chat layer.
  Nèrvi is machine-readable signal routing — no Matrix rooms, no human UX.

  Nèrvi is not a general message queue. It is purpose-built for intra-stack
  agent signalling. Do not use it as an application data bus.

  ## Design constraints

  - Substrat: NATS JetStream (not Redis Streams, not Kafka).
  - Single-node to start; cluster expansion is a later decision.
  - Intra-cluster only: ClusterIP service, no external exposure.
  - Streams are durable (file storage) — messages survive pod restarts.

  ## Dependencies

  - Farga: Nèrvi writes anomaly signals to Farga when the SRE sensor fires.
  - Caissa deploy repo: Helm chart lives under deploy/charts/nervi alongside
    other Occitan charts.
```

### `capabilities.yaml`

```yaml
project: nervi

exposes:
  - id: nats-jetstream
    kind: helm-chart
    description: NATS JetStream cluster, stream "ops" (subject ops.>)
    endpoint: nats://nats.occitan-system.svc.cluster.local:4222
  - id: nervi-mcp
    kind: mcp-tool
    description: MCP server exposing nervi_publish and nervi_subscribe
    endpoint: http://nervi.occitan-system.svc.cluster.local:8080/mcp

consumes:
  - project: farga
    capability: write-signal
    required: false   # Nèrvi degrades gracefully if Farga is unreachable
```

### `README.md`

```markdown
# Nèrvi

Nèrvi is the async subscription fabric for the Occitan stack. It deploys NATS
JetStream on the cluster and exposes `nervi_publish` and `nervi_subscribe` MCP
tools so agents can exchange machine-readable signals without synchronous
coordination or a Matrix room.

## What it is not

Nèrvi is not Charradissa (the human-chat layer) and is not a general
application message queue. It carries intra-stack agent signals only. The first
sensor is an SRE log monitor that publishes anomalies to `ops.sre.alerts`.

## Occitan stack position

| Depends on | Farga (optional — signal writes) |
|---|---|
| Exposes to | Any agent with the Nèrvi MCP tools |
| Namespace | occitan-system |
| Port | 8080 (MCP), 4222 (NATS) |
```
