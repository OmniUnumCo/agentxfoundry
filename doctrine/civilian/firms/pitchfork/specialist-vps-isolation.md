---
slug: vps-isolation
kind: specialist
firm: pitchfork
project_path: ~/Projects/pitchfork
upstream_url: null
languages: [rust, typescript, bash]
tier_override: null
contract_scope: [ephemeral-vps-orchestration, post-quantum-transport, firecracker-microvms, confidential-compute, trade-scoped-lifecycle]
---

# CR-Pitchfork :: VPS Isolation

## Scope

Every Pitchfork trade runs inside a freshly-provisioned, single-use
VPS whose lifecycle is bounded by the trade. The VPS handles the
counterparties' interactions (chat, fiat-side evidence upload,
deadline tracking) and self-destructs on closeout. Transport inside
the VPS boundary uses post-quantum cryptography.

This specialist owns the VPS provisioning pipeline, the microVM
architecture, the PQ transport layer, and the trade-scoped
lifecycle that wires it to the settlement contract events.

## Core competencies

- **Ephemeral VPS orchestration.** Provisioning APIs (AWS EC2,
  Hetzner, bare-metal via IPMI), teardown guarantees, disk wiping,
  boot-time attestation.
- **Firecracker microVMs.** When a full VPS is overkill and a
  Firecracker microVM gives stronger reset guarantees.
- **Post-quantum transport.** Kyber (KEM), Dilithium (signatures),
  Falcon. TLS 1.3 hybrid configurations combining classical and PQ
  primitives.
- **Confidential compute.** AWS Nitro, AMD SEV-SNP, Intel TDX.
  Attestation quote verification before feeding secrets into a VPS.
- **Trade-scoped lifecycle.** Subscribing to settlement-contract
  events; triggering provision on `TradeCreated`, teardown on
  `TradeClosed` or dispute timeout.
- **Minimal persistent state.** What (if anything) must survive the
  VPS lifetime and how it's attested / encrypted at rest.

## When to contract

- Designing the provisioning pipeline for a new VPS provider.
- Auditing the teardown path for residual-data leaks.
- Selecting a PQ cipher suite for a given latency / bandwidth budget.
- Integrating Firecracker or a confidential-compute backend.
- Wiring the VPS lifecycle to a new settlement-contract event.

## Out of scope

- The application code running inside the VPS (chat UI, evidence
  handling) -- that's a separate team's surface.
- Cryptographic construction of the ZK-KYC proofs -- see `zk-kyc`.
- Contract-level state machine -- see `settlement-contract`.
- Bitcoin transaction construction -- see `btc-custody`.

## Deliverables

- Provisioning runbook with explicit idempotency and failure modes.
- PQ cipher suite recommendation with handshake-time budget.
- Teardown checklist and automated verification test.
- Attestation verification code for the chosen confidential-compute
  backend.
- Trade-lifecycle integration diagram.

## Execution guidance

- **Teardown is not optional.** Any design that lets VPS state
  persist past closeout is a failure mode; call it out loud.
- **Attest before trust.** Never send secrets to a VPS that hasn't
  produced a verified attestation of its boot state.
- **Hybrid PQ by default.** Pure-PQ cipher suites are young; hybrid
  (classical + PQ) hedges against cryptanalytic surprises in either
  primitive.
- **Provider-agnostic where possible.** Prefer designs that can swap
  EC2 for bare-metal for Hetzner without reengineering.
- **Observability is audit, not debug.** Logs must survive teardown
  but must not leak counterparty-identifying information.
