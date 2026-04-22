# Technical Specification — Civilian Reserve (CR)

## 1. Module map

| File | Responsibility |
|---|---|
| `src/models.rs` (lines ~615-end) | Types: `AgentClass`, `CivilianTier`, `CivilianRecord`, `AnyAgent`, `ContractState`, `ContractOutcome`, `ContractRecord`, `ContractRequest` |
| `src/traits.rs` | `CivilianDoctrine` struct + `DoctrineLoader` trait extensions |
| `src/persistence.rs` | `civilians` + `contracts` projection tables, event reuse |
| `src/c2_router.rs` | Civilian refusal in `validate_command_relationship`, `route_contract_request(ContractRequest)`, new error variants |
| `src/doctrine.rs` | `validate_slug`, `ensure_within_root`, `civilian_doctrine`, `specialist_doctrine`, `assemble_contract_prompt`, YAML frontmatter parser |
| `src/broker.rs` | `ContractBroker::route` + `rules` + private `parse_routing_rules` |
| `src/daemon.rs` | JSON-RPC handlers: `cr_register_civilian`, `cr_list_civilians`, `cr_firm_detail`, `cr_contract_request`, `cr_contract_accept`, `cr_contract_sitrep`, `cr_contract_closeout`, `cr_execute_contract` |
| `src/bin/rr_mcp.rs` | MCP tool descriptors + name → method routing |
| `doctrine/civilian/` | Authored doctrine tree |

## 2. Rust types

```rust
pub enum AgentClass { Military, Civilian }

pub enum CivilianTier { Firm, Specialist, Independent }

pub struct CivilianRecord {
    pub id: String,
    pub name: String,
    pub tier: CivilianTier,
    pub firm_id: Option<String>,         // Some iff Specialist
    pub specialists: Vec<String>,        // populated on Firm only
    pub specialty_slug: String,
    pub project_path: Option<String>,
    pub upstream_url: Option<String>,
    pub status: StatusEnum,
    pub active_contracts: Vec<String>,
    pub performance_score: f64,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub enum AnyAgent { Mil(AgentRecord), Civ(CivilianRecord) }

pub enum ContractState { Requested, Accepted, InProgress, Closed, Rejected }

pub enum ContractOutcome { Complete, Failed, Cancelled }

pub struct ContractRecord {
    pub id: String,
    pub contracting_agent: String,       // military agent id
    pub firm_id: String,                 // civilian id (firm or independent)
    pub routed_specialist_id: Option<String>,
    pub state: ContractState,
    pub brief: String,
    pub mission_id: Option<String>,
    pub priority: PriorityEnum,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub outcome: Option<ContractOutcome>,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct ContractRequest {
    pub contracting_agent: String,
    pub firm_id: String,
    pub brief: String,
    pub mission_id: Option<String>,
    pub priority: PriorityEnum,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

## 3. Persistence schema (additions to SQLite)

```sql
CREATE TABLE civilians (
    id                  TEXT PRIMARY KEY,
    name                TEXT NOT NULL,
    tier                TEXT NOT NULL,
    firm_id             TEXT,
    specialists         TEXT NOT NULL DEFAULT '[]',
    specialty_slug      TEXT NOT NULL,
    project_path        TEXT,
    upstream_url        TEXT,
    status              TEXT NOT NULL,
    active_contracts    TEXT NOT NULL DEFAULT '[]',
    performance_score   REAL NOT NULL DEFAULT 0.0,
    created_at          TEXT NOT NULL,
    metadata            TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX civilians_by_firm ON civilians(firm_id);
CREATE INDEX civilians_by_slug ON civilians(specialty_slug);

CREATE TABLE contracts (
    id                      TEXT PRIMARY KEY,
    contracting_agent       TEXT NOT NULL,
    firm_id                 TEXT NOT NULL,
    routed_specialist_id    TEXT,
    state                   TEXT NOT NULL,
    brief                   TEXT NOT NULL,
    mission_id              TEXT,
    priority                TEXT NOT NULL,
    created_at              TEXT NOT NULL,
    closed_at               TEXT,
    outcome                 TEXT,
    metadata                TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX contracts_by_firm ON contracts(firm_id);
CREATE INDEX contracts_by_state ON contracts(state);
CREATE INDEX contracts_by_contracting_agent ON contracts(contracting_agent);
```

Events use the existing `events` table — no schema migration required.

## 4. Event payload shapes

```jsonc
// CONTRACT_REQUESTED
{
  "contract_id": "uuid",
  "from_agent": "military-agent-id",
  "firm_id": "return42",
  "brief": "Extend tokenizer to handle zero-width joiners",
  "mission_id": null,
  "priority": "priority"
}

// CONTRACT_ACCEPTED
{
  "contract_id": "uuid",
  "firm_id": "return42",
  "routed_specialist_id": "morphlex"
}

// CONTRACT_SITREP
{
  "contract_id": "uuid",
  "firm_id": "return42",
  "status": "active",
  "progress": 0.4,
  "message": "drafted the morpheme-boundary decision table",
  "blockers": []
}

// CONTRACT_CLOSEOUT
{
  "contract_id": "uuid",
  "firm_id": "return42",
  "outcome": "complete",
  "deliverables": "decision table + proposed morphology rule patch"
}

// CIVILIAN_REGISTERED / FIRM_REGISTERED
{
  "civilian_id": "arena",
  "tier": "independent",
  "firm_id": null,
  "specialty_slug": "arena"
}
```

## 5. Slug validation (CWE-22 guard)

```rust
fn validate_slug(slug: &str) -> Result<(), DoctrineError> {
    if slug.is_empty() || slug.len() > 64 { return Err(InvalidSlug); }
    let mut chars = slug.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) { return Err(InvalidSlug); }
    for c in chars {
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_') {
            return Err(InvalidSlug);
        }
    }
    Ok(())
}
```

Additionally, `ensure_within_root` canonicalizes the resolved path (`path.canonicalize()`) and verifies `.starts_with(self.root.canonicalize())`.

## 6. MCP tool surface

| Tool | Required params | Optional | Returns |
|---|---|---|---|
| `cr_register_civilian` | `slug` | — | `{civilian}` |
| `cr_list_civilians` | — | — | `{count, civilians[]}` |
| `cr_firm_detail` | `firm_id` | — | `{firm, specialists[], contract_scope[], routing_rules[], open_contracts[]}` |
| `cr_contract_request` | `contracting_agent`, `firm_id`, `brief` | `mission_id`, `priority` | `{contract}` |
| `cr_contract_accept` | `contract_id`, `specialist_id` | — | `{contract}` |
| `cr_contract_sitrep` | `contract_id` | `status`, `progress`, `message`, `blockers[]` | `{contract}` |
| `cr_contract_closeout` | `contract_id`, `outcome` | `deliverables` | `{contract}` |
| `cr_execute_contract` | `contract_id` | `auto_close` (bool, default true) | `{contract, response}` |

## 7. Invariants (test-enforced)

1. **Specialist not directly contractable.** `route_contract_request` with `firm_id` resolving to `CivilianTier::Specialist` returns `SpecialistDirectlyContracted(specialist_id, parent_firm_id)`.
2. **Only requested contracts can be accepted.** `cr_contract_accept` on a contract in any state other than `Requested` returns a Handler error.
3. **Specialist must be in firm's roster.** Accept rejects specialist_id not present in `firm_doctrine.specialists`.
4. **Broker is deterministic.** `ContractBroker::route(doctrine, brief)` returns the same `Option<String>` across any number of invocations with identical inputs.
5. **Slug rejects traversal.** Inputs matching `..`, `/`, `\`, `.`, empty, or >64 chars all error.
6. **Canonicalized path stays inside doctrine root.** Verified by `ensure_within_root` calling `canonicalize()` on both the target path and `self.root`.

## 8. Test coverage

- `tests/test_civilian_reserve.rs` — 8 tests covering parsing, slug rejection, persistence round-trip, full contract lifecycle (military → firm → broker → SITREP → closeout), specialist-directly-contract refusal.
- `src/broker.rs` unit tests — 7 tests covering keyword matching, case insensitivity, earlier-wins tie-break, fallback, empty firm, h1-section termination, parsed-rules accessor.
- `src/doctrine.rs` unit tests — slug validator via civilian doctrine path (covered by integration tests).

Total: **15 tests specifically covering CR semantics**, 0 failures.

## 9. Extension points

- **New firm:** Add `doctrine/civilian/firms/<slug>/firm.md` + any `specialist-*.md` files. No Rust changes required.
- **New flat civilian:** Add `doctrine/civilian/flat/<slug>.md`. No Rust changes required.
- **New event type:** Append a new `event_type` string to `daemon.rs` handlers; no schema migration needed (the events table is schemaless in payload).
- **New contract state:** Add variant to `ContractState` enum; update state transition guards in relevant handlers.

## 10. Dependencies

- `rusqlite` for the projection tables
- `uuid` for contract / registration ids
- `chrono` for timestamps
- `serde_json` for payload serialization
- No new external dependencies vs pre-CR state

## 11. File paths cited in code

- Civilian refusal top-of-function guard: `src/c2_router.rs:validate_command_relationship`
- Contract routing entry: `src/c2_router.rs:route_contract_request`
- Deterministic broker: `src/broker.rs:ContractBroker::route`
- Slug validation: `src/doctrine.rs:validate_slug`
- Canonical path containment: `src/doctrine.rs:ensure_within_root`
- Doctrine loader: `src/doctrine.rs:civilian_doctrine` / `specialist_doctrine`
- Daemon handlers: `src/daemon.rs:handle_cr_*`
- MCP descriptors: `src/bin/rr_mcp.rs:tools/list` + name mapping
