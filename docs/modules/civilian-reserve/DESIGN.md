# Design Specification — Civilian Reserve (CR)

## 1. Architecture overview

```
+----------------------------+      +----------------------------+
|  Military agent (RRAgent)  |      |   Doctrine (filesystem)    |
|  rank, MOS, commander      |      |   doctrine/civilian/       |
+-------------+--------------+      +-------------+--------------+
              |                                   |
              | ContractRequest                   | civilian_doctrine()
              v                                   v
+----------------------------+      +----------------------------+
|   C2Router                 |<-----|  FileSystemDoctrineLoader  |
|   route_contract_request() |      |   + validate_slug          |
+-------------+--------------+      |   + ensure_within_root     |
              |                     +----------------------------+
              | persist + event
              v
+----------------------------+      +----------------------------+
|   SqliteStateStore         |<-----|    ContractBroker          |
|   civilians + contracts    |      |    route(firm, brief)      |
|   + events (append-only)   |      |    -> specialist_slug      |
+----------------------------+      +----------------------------+
```

## 2. Agent class model

Three orthogonal dimensions:

| Dimension | Values |
|---|---|
| `AgentClass` | Military, Civilian |
| `CivilianTier` | Firm, Specialist, Independent |
| `ContractState` | Requested, Accepted, InProgress, Closed, Rejected |

**Rationale for separate class vs rank extension:** Adding a `CR` value to `RankEnum` would break `authority()`, `outranks()`, and chain-of-command arithmetic. A sibling class preserves military semantics intact.

**Rationale for Firm/Specialist split:** Project-scoped knowledge is rarely atomic. Return42 holds three distinct disciplines (Morphlex tokenizer, Jasterish compiler, Custom-LLM). Pitchfork spans settlement contract, ZK-KYC, VPS isolation, and BTC custody. A flat civilian would either conflate these or duplicate infrastructure per discipline.

## 3. Contract lifecycle state machine

```
         (military submits request)
                    |
                    v
          +------------------+
          |    Requested     |
          +---+---------+----+
              |         |
(firm accepts)|         |(firm rejects)
              v         v
        +---------+  +----------+
        |Accepted |  | Rejected |    (terminal)
        +----+----+  +----------+
             |
    (first CONTRACT_SITREP)
             |
             v
       +------------+
       | InProgress |
       +-----+------+
             |
  (closeout with outcome)
             |
             v
         +--------+
         | Closed |           (terminal)
         +--------+
```

**Implicit Accepted→InProgress:** Avoids requiring explicit state changes for the common case. First SITREP promotes automatically; the accept event still fires when the broker picks a specialist.

## 4. Firm-internal routing (ContractBroker)

Broker is a pure function over `(CivilianDoctrine, brief: &str) → Option<specialist_slug>`.

**Policy:**
1. Parse `## Routing rules` markdown section of firm doctrine body
2. For each rule `- \`slug\` keywords: kw1, kw2, kw3`: check if any keyword (case-insensitive substring) appears in brief
3. First matching rule wins (document order)
4. Fallback: first specialist in the firm's frontmatter `specialists:` list
5. Return `None` only if the firm has no rules AND no specialists

**Determinism guarantee:** No LLM involvement. Byte-stable for fixed inputs. Tested across 1000 invocations.

## 5. Doctrine layout

```
doctrine/
  VERSION                             # military semver
  ranks/*.md                          # existing military rank doctrine
  mos/*.md                            # existing MOS doctrine
  protocols/*.md                      # existing protocol templates
  civilian/
    VERSION                           # civilian semver (independent)
    flat/
      arena.md
      btcwallet.md                    # pending (Slice C)
      openchoreo.md                   # pending
    firms/
      return42/
        firm.md
        specialist-morphlex.md
        specialist-jasterish.md       # pending
        specialist-custom-llm.md      # pending
      pitchfork/                      # pending
      developer-portal/               # pending
      knockout-eda/                   # pending
      mab/                            # pending
```

## 6. Frontmatter schema

```yaml
---
slug: morphlex                        # required
kind: specialist                      # firm | specialist | independent
firm: return42                        # required iff kind=specialist
project_path: ~/Projects/apps
upstream_url: null
languages: [rust]
tier_override: null                   # strategic|tactical|supervision|execution_guidance|execution
contract_scope: [tokenizer, morphology, ...]
specialists: [a, b, c]                # only on firms
---

# CR-Return42 :: Morphlex

## Scope
## Core competencies
## When to contract this specialist
## Out of scope
## Deliverables
## Execution guidance
```

Firm doctrine additionally carries a `## Routing rules` section consumed by `ContractBroker`.

## 7. Interaction contracts

**Military ↔ C2Router:** military agent builds `ContractRequest`; router validates the contracting agent is military, looks up firm via injected `civilian_lookup`, refuses specialist-directly contracts, persists to event log + `contracts` table.

**C2Router ↔ ContractBroker:** the router does not invoke the broker on request — it defers routing until accept. This matches the lifecycle: a firm may reject. Broker is invoked at `cr_contract_accept` time.

**Daemon ↔ FileSystemDoctrineLoader:** all civilian-slug arguments from MCP tool calls flow through `validate_slug` before path construction. `ensure_within_root` canonicalizes the resolved path and aborts if it escapes `doctrine/civilian/`.

## 8. Key design decisions

1. **Sibling class, not rank extension.** Preserves military semantics. Cost: `AnyAgent` enum wraps both for unified call sites.
2. **Firm-level contracting only.** Keeps the request API narrow. Specialists are discoverable via `cr_firm_detail` but not contractable.
3. **Deterministic broker.** LLM-based broker routing would compromise the determinism thesis. Keyword rules are explicit and debuggable.
4. **Separate civilian doctrine version.** Allows civilian roster to evolve faster than the military core (`mil-X.Y.Z+civ-A.B.C`).
5. **Contract lifecycle in `contracts` projection, not on civilian record.** Multiple open contracts per firm is supported without widening `CivilianRecord`.
6. **Events reuse the existing `events` table.** No schema migration; adds typed event_type strings only.

## 9. Security model

- **Path traversal (CWE-22):** Two-layer defense. `validate_slug` rejects anything not matching `[a-z0-9][a-z0-9_-]{0,63}` (blocks `..`, `/`, `\`, nulls). `ensure_within_root` canonicalizes the joined path and verifies containment. Either layer alone prevents escape; both combined satisfy defense-in-depth.
- **Authority:** Civilians have no commander; C2 router's rank-override path is bypassed entirely by the civilian refusal check added at the top of `validate_command_relationship`.
- **Immutable audit trail:** All state changes are events first, projection second. A cleared `contracts` row can be reconstructed from the event log.

## 10. Open design decisions (tracked separately)

- Civilian CASREP path — escalate failed contracts to the contracting military agent (vs to `None`).
- Roster tool merge — expose both military and civilian rosters behind a class filter.
- AAR integration — include contract timelines in swarm AARs.
