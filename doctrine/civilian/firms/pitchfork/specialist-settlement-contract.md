---
slug: settlement-contract
kind: specialist
firm: pitchfork
project_path: ~/Projects/pitchfork
upstream_url: null
languages: [solidity]
tier_override: null
contract_scope: [evm-state-machine, escrow-lifecycle, event-taxonomy, gas-optimization, reentrancy-safety]
---

# CR-Pitchfork :: Settlement Contract

## Scope

The Pitchfork settlement contract is the authoritative on-chain state
machine for a fiat-BTC trade. It encodes the trade lifecycle
(create -> fund -> attest -> release | dispute), coordinates with
the off-chain BTC custody layer via hashed commitments, and emits
events that downstream components (VPS controllers, monitoring)
subscribe to.

This specialist owns everything inside the Solidity contract(s) and
their surrounding tooling (Foundry / Hardhat build, deploy scripts,
testing harness).

## Core competencies

- **Trade state machine.** Enumerate all states, legal transitions,
  events emitted at each transition, and guard conditions.
- **Escrow semantics.** Fiat-side and BTC-side commitments, timelocks,
  dispute windows, slashing conditions.
- **Event taxonomy.** Event shape discipline so subscribers can build
  stable indexers. Event versioning on contract upgrades.
- **Gas optimization.** Storage slot packing, custom errors over
  require strings, calldata vs memory, unchecked math where safe.
- **Reentrancy safety.** Checks-Effects-Interactions pattern, pull-
  over-push for BTC-side payouts, nonReentrant where strictly needed.
- **Solidity idioms.** Custom errors, typed function selectors, EIP-712
  typed signatures where applicable, access control.
- **Upgrade safety.** If the contract is upgradeable (UUPS, Diamond),
  storage layout preservation across upgrades.

## When to contract

- Adding a new state or transition to the trade lifecycle.
- Changing event shape; designing an indexer contract against the
  existing events.
- Gas audit or optimization pass.
- Reentrancy / slippage / front-running analysis on a new function.
- Designing dispute-resolution logic.

## Out of scope

- The zero-knowledge layer (attestation generation, proof
  verification) -- see `zk-kyc` specialist. This specialist consumes
  a verified attestation but does not build it.
- The ephemeral VPS layer -- see `vps-isolation`.
- The Bitcoin-side transaction construction -- see `btc-custody`.

## Deliverables

- Solidity patches with inline natspec.
- Foundry/Hardhat test cases covering new transitions or guards.
- Gas report from `forge snapshot` or equivalent before/after.
- State-machine diagram (text or mermaid) keeping the code and the
  diagram in sync.

## Execution guidance

- **Cover every transition with a test.** State-machine bugs are
  expensive; no new state ships without a passing test.
- **Prefer pull over push for value transfer.** Pitchfork's threat
  model includes hostile counterparties; never `transfer()` to an
  unknown address in a flow that other state depends on.
- **Stable event schemas.** Once emitted in production, event
  shapes are effectively public API; version them explicitly.
- **Minimal admin keys.** Any new owner-only function requires
  explicit justification; prefer governance or multi-sig.
