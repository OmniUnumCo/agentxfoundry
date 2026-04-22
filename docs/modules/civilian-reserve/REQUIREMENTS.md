# Requirements — Civilian Reserve (CR)

**Module:** Civilian Reserve agent class
**Status:** Implemented in Slice A/B; roster authoring partial (2 of 9 civilians)
**Source:** `src/models.rs` (CR types), `src/c2_router.rs` (contract routing), `src/broker.rs` (firm-internal routing), `src/persistence.rs` (`civilians` + `contracts` tables), `doctrine/civilian/`

## 1. Purpose

Provide a parallel class of agents to the military-rank MOS hierarchy. Civilian Reserve contractors are project-specialized domain experts that military agents *contract* (not order) for work outside their MOS wheelhouse. The goal is to carry per-project expertise (e.g. Arena's consensus primitives, Return42's Morphlex tokenizer) as first-class, auditable entities inside the same state store as military missions.

## 2. Functional requirements

**FR-CR-01.** The system SHALL model civilians as a separate `AgentClass` (Military vs Civilian). Civilians MUST NOT be treated as a rank within the military `RankEnum`.

**FR-CR-02.** The system SHALL support three civilian tiers:
  - **Firm** — umbrella with internal specialists
  - **Specialist** — member of a firm; carries `firm_id`
  - **Independent** — flat civilian with single scope; no parent firm

**FR-CR-03.** Civilians SHALL NOT carry `commander_id` or `subordinates`. They are outside the chain of command.

**FR-CR-04.** Military agents SHALL interact with civilians exclusively through the contract lifecycle. Direct ordering of a civilian via the C2 router MUST be refused with `CommandAuthority` / `SpecialistDirectlyContracted`.

**FR-CR-05.** Contracts SHALL address firms, not specialists. Specialists cannot be contracted directly; a firm-internal broker selects the specialist.

**FR-CR-06.** The system SHALL enforce the following contract states:
`Requested → Accepted | Rejected`
`Accepted → InProgress` (implicit on first SITREP, per design decision)
`InProgress → Closed`

**FR-CR-07.** The system SHALL record civilian lifecycle events: `CIVILIAN_REGISTERED`, `FIRM_REGISTERED`, `CONTRACT_REQUESTED`, `CONTRACT_ACCEPTED`, `CONTRACT_REJECTED`, `CONTRACT_SITREP`, `CONTRACT_CLOSEOUT`.

**FR-CR-08.** Doctrine for civilians SHALL be loaded from filesystem as markdown with YAML frontmatter, under `doctrine/civilian/flat/<slug>.md` and `doctrine/civilian/firms/<firm>/{firm.md, specialist-<spec>.md}`.

**FR-CR-09.** Firm-internal specialist routing (`ContractBroker`) SHALL be deterministic: keyword-based rules parsed from the firm's `## Routing rules` markdown section. No LLM in the routing path.

**FR-CR-10.** Civilian doctrine slugs SHALL be validated against an ASCII allowlist (`[a-z0-9][a-z0-9_-]{0,63}`) before any filesystem access. Built paths SHALL be canonicalized and verified to remain within the doctrine root.

## 3. Non-functional requirements

**NFR-CR-01. Determinism.** Given a stable `(firm_doctrine, brief)` pair, `ContractBroker::route` MUST return the same specialist slug on every call. Byte-identical guarantee.

**NFR-CR-02. Auditability.** Every state transition MUST be reflected in the append-only event log with a stable event type and payload shape.

**NFR-CR-03. Path traversal defense (CWE-22).** Civilian slugs from untrusted sources (MCP tool calls) MUST NOT be able to read files outside `doctrine/civilian/`.

**NFR-CR-04. Doctrine versioning.** The `doctrine_version` stamped on each event SHALL carry both military and civilian version strings (`mil-<X.Y.Z>+civ-<A.B.C>`).

**NFR-CR-05. Separation of concerns.** Adding a new civilian MUST NOT require modifying any military code path (ranks, MOS, C2 router order logic, chain-of-command validation).

## 4. Scope

**In scope:**
- CR data model + persistence
- Contract lifecycle (request → accept → sitrep → closeout)
- Firm-level-only contracting (never address specialists directly)
- Deterministic specialist routing via broker
- YAML-frontmatter doctrine loader with path traversal defense
- 5 new MCP tools exposing the lifecycle

**Out of scope:**
- Cross-firm contracting (firm-to-firm). Military → firm is the only contract path.
- B-class in-product agents (embedded agents that ship inside target products)
- Civilians as members of a military chain of command
- LLM-based broker routing (deterministic only)

## 5. Acceptance criteria

1. A military agent (e.g. 18B SfEngineer) MUST be able to submit a `CONTRACT_REQUESTED` event naming a firm's slug and see a `contracts` row in `state=Requested`.
2. `C2Router::route_order(order_to_civilian)` MUST return `CommandAuthority` error.
3. `ContractBroker::route(firm_doctrine, brief)` MUST be byte-deterministic across 1000 invocations on fixed inputs.
4. A civilian doctrine slug containing `..` or `/` MUST be rejected by `validate_slug` before any I/O.
5. The `contracts` projection MUST survive a daemon restart — contract state is persistent.
6. Every transition (request/accept/sitrep/closeout) MUST append a corresponding event to the event log.
