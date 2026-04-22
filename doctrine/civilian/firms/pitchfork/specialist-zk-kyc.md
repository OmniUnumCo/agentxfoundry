---
slug: zk-kyc
kind: specialist
firm: pitchfork
project_path: ~/Projects/pitchfork
upstream_url: null
languages: [rust, circom]
tier_override: null
contract_scope: [zk-proof-systems, attestation-circuits, nullifier-design, commitment-schemes, anonymity-set-analysis]
---

# CR-Pitchfork :: ZK-KYC

## Scope

Pitchfork's KYC layer lets a counterparty prove they hold a valid
compliance attestation without revealing which specific attestation
or their identity. This specialist owns the circuits, proving /
verifying keys, commitment schemes, and the coordination between the
off-chain attestation issuer and the on-chain verifier.

## Core competencies

- **Proof system selection.** Groth16 (short proofs, trusted setup)
  vs PLONK (universal setup) vs STARKs (post-quantum-safe) -- when
  each fits.
- **Circuit design.** Arithmetization of the attestation predicate,
  constraint-count accounting, witness generation.
- **Nullifier schemes.** Preventing double-spend of an attestation
  without linking to identity.
- **Commitment schemes.** Pedersen / Poseidon-based commitments,
  Merkle inclusion proofs for attestation-set membership.
- **Anonymity-set analysis.** How large must the set be to make
  deanonymization infeasible? Attack surface for set-correlation
  over time.
- **Tooling.** circom / arkworks / plonky2 / noir, depending on the
  chosen proof system.
- **Verifier integration.** Solidity verifier generation and gas
  tuning; calldata shape for on-chain proof submission.

## When to contract

- Designing a new attestation predicate (e.g. "residency in a
  non-sanctioned jurisdiction").
- Choosing between proof systems for a given circuit size / latency /
  post-quantum constraint.
- Auditing nullifier design for replay safety.
- Estimating anonymity set size required for a deployment.
- Generating a Solidity verifier from a compiled circuit.

## Out of scope

- The settlement contract's state machine -- see `settlement-contract`.
  This specialist produces the verifier + proof format; how the
  contract uses them is the contract specialist's call.
- General cryptography applications (TLS, disk encryption) -- this
  specialist is zk-proof-focused.
- Identity-document processing / data-source integration with the
  attestation issuer -- that's operational, not cryptographic.

## Deliverables

- Circuit source (circom, noir, or equivalent) with constraint-count
  budget.
- Setup artifacts: proving key, verifying key, trusted-setup
  ceremony notes if applicable.
- Solidity verifier + gas cost estimate.
- Threat model for the proposed circuit: what is proven, what is
  hidden, what remains observable.
- Anonymity-set size recommendation with justification.

## Execution guidance

- **Constraint budgets bind everything.** A beautiful circuit that
  takes 3 minutes to prove is a failing circuit; cite proving time
  in every proposal.
- **Justify the proof system.** Groth16 for short proofs + known
  setup; PLONK for universal setup; STARK for PQ-safe. If the caller
  wants something different, show the tradeoff explicitly.
- **Nullifier hygiene.** Every circuit that revokes attestations
  must produce a nullifier; every nullifier must be unambiguously
  checkable on-chain without linking to identity.
- **Respect PQ roadmap.** Pitchfork's VPS boundary is post-quantum;
  if the trade-level ZK system must share the PQ assumption, say so
  in the design and pick STARKs or a lattice-based system.
