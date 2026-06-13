# Fondament Design Spec
_2026-06-12_

## 1. Purpose

Fondament (Occitan: *foundation*) is the single source of truth for all agent primitives in the Occitan stack. It owns discipline definitions, practice compositions, role declarations, stance postures, and tool registrations. It is a Rust library (`fondament-core`) plus a CLI (`fondament-cli`).

**Fondament defines the shape. Farga provides the clothes.**

An agent is a dynamically assembled context. Fondament defines the static primitives (disciplines, practices, roles, stances, tools). Farga holds the living content (org layer, initiative layer, project layer) that dresses those primitives into a fully resolved agent at dispatch time.

---

## 2. Crate Structure

```
fondament/
├── Cargo.toml
├── fondament-core/         # resolver library — consumed by Amassada and Charradissa
├── fondament-cli/          # validate, lint, sweep, scaffold
└── definitions/            # the primitive tree
    ├── disciplines/        # atomic horizontal knowledge domains
    │   ├── compute/
    │   ├── data/
    │   │   └── db/
    │   │       ├── mysql.yaml
    │   │       └── postgres.yaml
    │   ├── application/
    │   ├── network/
    │   ├── security/
    │   ├── delivery/
    │   └── process/        # business process design, BPMN, workflow engines
    ├── practices/          # vertical compositions of disciplines
    │   ├── bpa/            # extends: [process, application, data]
    │   ├── rpa/            # extends: [process, application/automation]
    │   ├── devops/         # extends: [compute, delivery, security]
    │   └── data-engineering/
    ├── stances/            # cognitive postures
    │   ├── builder.yaml
    │   ├── breaker.yaml
    │   ├── adversarial.yaml
    │   ├── moderator.yaml
    │   ├── realist.yaml
    │   └── neutral.yaml
    ├── roles/              # named compositions: discipline/practice + stance + cognitive_load
    │   ├── system-architect.yaml
    │   ├── bpa-analyst.yaml
    │   └── security-sre.yaml
    └── tools/              # tool connection specs (what + how)
        ├── mysql-mcp.yaml
        ├── jira-api.yaml
        └── github-mcp.yaml
```

---

## 3. Definition File Format

All definition files share a consistent shape. The `kind` field determines which fields are required.

### Discipline

```yaml
id: data/db/mysql
kind: discipline
extends: [data/db]              # optional parent discipline
default_model: claude-haiku-4-5 # cognitive load baseline for this domain

context: |
  You are an expert in MySQL. You understand schema design, query optimization,
  indexing strategies, and replication topology.

tools:
  always_on:
    - id: schema_reader
      kind: mcp
      server: mysql-mcp
      tool: read_schema
    - id: query_explainer
      kind: mcp
      server: mysql-mcp
      tool: explain_query
  jit:
    - id: query_optimizer
      kind: mcp
      server: mysql-mcp
      tool: optimize_query
    - id: slow_log_analyzer
      kind: api
      server: slow-log-api
      tool: analyze

lint_rules:
  - id: mysql-write-requires-approval
    check: any write tool requires approval flag
```

### Practice

```yaml
id: practices/bpa
kind: practice
extends: [disciplines/process, disciplines/application, disciplines/data]
default_model: claude-sonnet-4-6

context: |
  You specialize in Business Process Automation — modeling, optimizing, and
  automating business workflows using BPMN, workflow engines, and integration patterns.
```

### Role

```yaml
id: roles/security-sre
kind: role
extends: [disciplines/security, practices/devops]
stance: adversarial
cognitive_load: high
default_model: claude-opus-4-8  # overrides discipline baseline

context: |
  You operate across security and reliability. You challenge assumptions,
  probe failure modes, and treat every system boundary as an attack surface.
```

### Stance

```yaml
id: stances/adversarial
kind: stance

context: |
  Challenge every assumption. Seek failure modes. Your role is to stress-test
  proposals, not to build consensus. Disagreement is contribution.
```

### Tool registry entry

```yaml
id: mysql-mcp
kind: mcp
description: MySQL schema and query tools
credentials: gardian://mysql/read     # Gardian resolves the actual secret at runtime
```

```yaml
id: jira-api
kind: api
description: Jira issue and project management
base_url: "https://acme.atlassian.net/rest/api/3"
credentials: gardian://jira/token
```

Tool kinds: `mcp` | `api` | `native` (built-in Rust handler in the daemon).

---

## 4. Typed Composition Address

String form is valid in canvas YAML (`project/auth+adversarial`). Rust code uses the typed struct — malformed addresses fail at parse time, not at dispatch.

Two address forms are valid:

```
fondament/<role-id>              # references a Fondament role directly
<project-id>/<facet>+<stance>   # Farga project context + facet narrowing + stance
```

`fondament/app-architect` resolves fully from the Fondament tree.
`project/auth+adversarial` resolves the project domain from Farga, narrows to the `auth` facet, applies the `adversarial` stance — no explicit role required.

```rust
pub enum CompositionAddress {
    Role {
        role: RoleId,                        // "fondament/app-architect"
        stance_override: Option<StanceId>,
    },
    Composed {
        project: ProjectId,                  // Farga project context key
        facet: Option<FacetId>,              // "auth", "db", "infra"
        stance: StanceId,                    // always explicit in this form
    },
}

impl FromStr for CompositionAddress { ... }  // parses both forms
impl Display for CompositionAddress { ... }  // round-trips to string for YAML canvases
```

---

## 5. Layer Model & Resolution Order

A resolved agent is an additive stack. Lower layers narrow upper ones. The stack is assembled in this order:

```
1. org layer          ← Farga: culture, standing rules, org-wide constraints
2. initiative layer   ← Farga: strategic goals, active initiatives
3. project layer      ← Farga: local goals, success criteria, evolving project state
4. discipline/practice← Fondament: domain expertise, tool defaults
5. role               ← Fondament: composition + cognitive_load + default_model
6. stance             ← Fondament: cognitive posture appended last
7. facet              ← optional narrowing (scopes tools and context to a subdomain)
```

**Bootstrap loading**: OrgAgent and ProjectAgent load layers 1–3 from Farga at daemon startup — not JIT — because they need standing context before any message arrives. All other agents load project layer on first project touch.

**Model resolution** (L1 in the Amassada 3-layer chain):
- Discipline sets a cognitive load baseline → default model
- Role overrides the discipline baseline when the composition demands more
- Canvas (L2) and Moderator `[MODEL]` block (L3) can push further

---

## 6. Resolver & Hot-Reload

`fondament-core` exposes a single entry point:

```rust
pub struct Fondament {
    definitions: Arc<RwLock<DefinitionTree>>,
    farga: Arc<dyn FargaReader>,
}

impl Fondament {
    pub async fn load(definitions_path: &Path, farga: Arc<dyn FargaReader>) -> Result<Self>;
    pub fn watch(self) -> (Self, WatchHandle);   // hot-reload via notify crate

    pub async fn resolve(&self, address: &CompositionAddress) -> Result<ResolvedAgent>;
    pub fn tool_registry(&self) -> &ToolRegistry;
}

pub struct ResolvedAgent {
    pub system_prompt: String,          // fully assembled from all layers
    pub tools: Vec<ToolDefinition>,     // always_on tools, resolved
    pub jit_tools: Vec<ToolDefinition>, // available on demand
    pub default_model: ModelId,         // L1 in the 3-layer resolution chain
}
```

**Hot-reload**: file watcher via `notify` crate. On any change under `definitions/`, the fast lint runs immediately. If lint passes, the tree reloads. If lint fails, the error is logged and the previous valid tree remains active — no daemon restart needed, no downtime.

---

## 7. Farga Boundary

Fondament never knows how Farga stores or indexes context. The interface is a trait:

```rust
#[async_trait]
pub trait FargaReader: Send + Sync {
    async fn org_layer(&self, org: &OrgId) -> Result<OrgContext>;
    async fn initiative_layer(&self, org: &OrgId) -> Result<Vec<InitiativeContext>>;
    async fn project_layer(&self, project: &ProjectId) -> Result<ProjectContext>;
}
```

The concrete implementation lives in Farga. Fondament calls this trait at resolve time. The org and initiative layers are populated by sessions (intake canvases, Matrix sessions with authorized humans) and inferred from tooling (Jira initiatives → epics → stories).

---

## 8. Conflict & Convergence Analyzer

Two modes: fast lint on every file change, deep semantic sweep on schedule or CLI trigger.

### Fast lint — structural checks

Runs on every save via the file watcher. Does not use LLM.

```
- extends graph is acyclic (no circular inheritance)
- all referenced tool IDs exist in definitions/tools/
- all referenced disciplines/practices exist in the tree
- model IDs are valid known Claude model strings
- write tools carry requires_approval flag
- tool parameter shapes are compatible when a role extends multiple disciplines
- access control declarations don't directly contradict parent layer
```

Fast lint failures are logged with the offending file and rule. They do not crash the daemon. The previous valid tree remains active.

### Deep sweep — semantic checks

LLM-assisted. Runs on `fondament sweep` or on configurable schedule. Reads Farga project layers for cross-project checks.

```
- conflicting goals between roles at the same layer
- strategy conflicts (two roles pursuing incompatible approaches to the same objective)
- REST/API endpoint overlap across projects
- access control drift (permissions granted at role level contradicting org-layer policy)
- goal definition conflicts (initiative goals working against each other)
- convergence opportunities (disciplines with significant overlap → suggest shared primitive)
```

Sweep output:

```yaml
# fondament-sweep-2026-06-12.yaml
conflicts:
  - id: conflict-001
    severity: high
    kind: access_control
    description: "roles/legacy-admin grants db write without approval; org layer requires all writes approved"
    layers: [org/acme/standards.yaml, roles/legacy-admin.yaml]
    resolution: requires_human

  - id: conflict-002
    severity: low
    kind: goal_overlap
    description: "practices/bpa and practices/rpa both define process-automation ownership"
    layers: [practices/bpa/index.yaml, practices/rpa/index.yaml]
    resolution: suggested_merge

convergence:
  - id: conv-001
    description: "disciplines/data/db/mysql and disciplines/data/db/postgres share 80% of tool surface"
    suggestion: "extract disciplines/data/db/relational as shared primitive"
```

High-severity conflicts with `requires_human` are surfaced as a DM to OrgAgent — same whisper pattern as ConciergeAgent. The human resolves in a session; the decision is written to Farga.

---

## 9. CLI (`fondament-cli`)

```
fondament check                                    # fast lint — all definitions
fondament check disciplines/data/db/               # fast lint — scoped subtree
fondament sweep                                    # deep semantic sweep, writes report
fondament resolve "project/auth+adversarial" \
  --project acme-auth                              # print assembled system prompt
fondament scaffold discipline                      # interactive: new discipline from template
fondament scaffold role                            # interactive: new role from template
fondament diff HEAD~1                              # changes between definition tree revisions
fondament graph                                    # print extends graph as DOT
```

---

## 10. Key Crates

- `tokio` — async runtime
- `serde` + `serde_yaml` — definition file parsing
- `notify` — file watcher for hot-reload
- `async-trait` — FargaReader trait
- `clap` — CLI argument parsing (`fondament-cli`)
- `anthropic` (or `reqwest`) — LLM call for deep sweep

---

## 11. Out of Scope (v1)

- `fondament-server` — optional REST service for multi-org shared primitive registries
- UI for browsing the definition tree
- Automated conflict resolution (human-in-the-loop only for now)
- Definition versioning beyond git history
- Non-YAML definition formats
