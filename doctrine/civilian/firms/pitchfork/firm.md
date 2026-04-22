---
slug: pitchfork
kind: firm
project_path: ~/Projects/pitchfork
upstream_url: null
languages: [solidity, rust, typescript]
tier_override: tactical
contract_scope: [fiat-btc-settlement, zk-kyc-attestation, ephemeral-vps-isolation, off-chain-btc-custody, post-quantum-boundaries]
specialists: [settlement-contract, zk-kyc, vps-isolation, btc-custody]
---

# CR-Pitchfork

## Scope

Pitchfork is a privacy-preserving peer-to-peer marketplace for
fiat-to-BTC settlement. Its architecture spans four tightly coupled
technical disciplines:

1. **On-chain settlement.** An EVM smart contract is the authoritative
   source of truth for each trade's state.
2. **Zero-knowledge KYC.** Counterparties prove compliance attestations
   without revealing identity.
3. **Ephemeral VPS isolation.** Each trade runs in a disposable per-
   order VPS with post-quantum cryptographic boundaries.
4. **Off-chain BTC custody.** Bitcoin custody and escrow are coordinated
   off-chain, with settlement anchored to the EVM contract.

CR-Pitchfork is the firm-level civilian. It routes contracts to the
right sub-specialist based on which technical layer the brief touches.

## When to contract CR-Pitchfork

- Cross-layer work: "how should the settlement contract coordinate
  with the VPS lifecycle?", "what does the BTC custody see from the
  ZK-KYC?"
- Unclear-scope briefs where the firm should pick the right
  specialist from the routing rules below.
- Architectural decisions that touch two or more Pitchfork
  disciplines.

## Routing rules

- `settlement-contract` keywords: evm, solidity, smart contract, on-chain, settle, escrow-contract, state machine, solc, foundry, hardhat, reentrancy, gas, ERC
- `zk-kyc` keywords: zero-knowledge, zk, snark, stark, plonk, groth16, attestation, proof, circom, noir, arkworks, nullifier, commitment, kyc
- `vps-isolation` keywords: vps, ephemeral, isolation, firecracker, microvm, post-quantum, pqc, kyber, dilithium, tss, enclave, nitro, SEV
- `btc-custody` keywords: bitcoin, btc, custody, utxo, sighash, PSBT, lightning, multisig, taproot, MuSig, threshold

Default fallback: `settlement-contract` (the contract is the source
of truth; most ambiguous briefs resolve there).

## Out of scope

- Generic DeFi protocol work (AMMs, lending, yield) — Pitchfork is
  specifically fiat-BTC settlement, not a DEX.
- KYC as compliance theater — Pitchfork's ZK-KYC is a specific
  cryptographic construction; generic "KYC provider integration" is
  not this firm's territory.
- Hot-wallet exchange operations — ephemeral-VPS + off-chain custody
  is deliberately not "your funds are on our exchange."
- Payment-rail plumbing (ACH, SWIFT, card networks) — Pitchfork
  assumes counterparties handle their fiat leg out-of-band.

## Deliverables

Firm-level closeouts typically include:
- A routing decision record: "brief -> specialist X because of rule R."
- Cross-specialist synthesis for briefs spanning more than one layer.
- Out-of-scope referrals when the brief is outside all four
  specialties.

## Execution guidance

- **Trade state invariants.** Every specialist answer must respect
  the trade state machine defined in the settlement contract. Don't
  propose changes that invalidate the contract's invariants.
- **Privacy is non-negotiable.** A proposal that weakens ZK-KYC
  unlinkability is rejected at the firm layer.
- **No hot-wallet patterns.** Any recommendation that concentrates BTC
  in a persistent custodial wallet violates the ephemeral-VPS thesis.
- **Post-quantum where specified.** The VPS boundary uses PQC; defer
  to the vps-isolation specialist before proposing changes that touch
  transport crypto.
