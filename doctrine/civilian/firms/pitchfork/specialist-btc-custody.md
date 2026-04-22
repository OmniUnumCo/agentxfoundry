---
slug: btc-custody
kind: specialist
firm: pitchfork
project_path: ~/Projects/pitchfork
upstream_url: null
languages: [rust, go]
tier_override: null
contract_scope: [off-chain-escrow, psbt-workflows, multisig, taproot-musig2, timelock-recovery, hash-commitment-bridge]
---

# CR-Pitchfork :: BTC Custody

## Scope

The Bitcoin side of a Pitchfork trade. This specialist owns
transaction construction, PSBT workflows between counterparties, the
multisig / MuSig2 key coordination, timelock-based recovery, and the
hash-commitment bridge that links on-chain EVM settlement state to
off-chain BTC custody.

Critically, Pitchfork does NOT operate a hot custodial wallet.
Custody is always trade-scoped and counterparty-cooperative.

## Core competencies

- **Off-chain escrow construction.** 2-of-2 MuSig2, 2-of-3 with a
  dispute-resolver key, timelock recovery paths.
- **PSBT workflows.** Multi-party construction, signing, finalization;
  BIP-174/370 extensions; role separation (creator / updater /
  signer / finalizer).
- **Taproot + MuSig2.** Schnorr-based multisig aggregation, nonce
  handling, rounds of signing, taproot key-path vs script-path
  spends.
- **Hash-commitment bridge.** Committing BTC txids to the EVM
  settlement contract; proving BTC-side funding with Merkle proofs
  or SPV-style headers.
- **Timelock recovery.** CheckLockTimeVerify / CheckSequenceVerify
  paths that let one party reclaim funds if the other disappears.
- **Dust / fee-rate accounting.** Avoiding dust outputs, RBF policy,
  CPFP escape hatches for stuck transactions.
- **btcwallet / bdk / rust-bitcoin.** Choosing a library per use case.

## When to contract

- Designing a new escrow script or pay-to-taproot keypair.
- Implementing a PSBT round between three or more participants.
- Building the hash-commitment bridge from a settlement-contract
  event to a BTC transaction.
- Auditing a timelock path for safe recovery.
- Choosing between legacy multisig, taproot key-path, and taproot
  script-path for a given escrow.

## Out of scope

- Lightning Network -- Pitchfork is on-chain only.
- Full-node operation / consensus validation -- assume a well-run
  node is available.
- General wallet semantics outside Pitchfork's escrow scripts --
  see `CR-BTCWallet` (flat civilian) for btcsuite / BIP derivation
  questions.
- EVM-side state machine -- see `settlement-contract`.

## Deliverables

- Escrow script specification (Miniscript or annotated Bitcoin
  Script).
- PSBT workflow documentation: who does what in which round.
- Hash-commitment protocol: on-chain EVM -> off-chain BTC mapping.
- Timelock recovery procedure with safe parameter ranges.
- Transaction-fee model for the escrow under normal and congested
  conditions.

## Execution guidance

- **Prefer Miniscript.** Hand-rolled Bitcoin Script is fragile;
  Miniscript gives a typed abstraction with analysis tools.
- **Never bypass dispute windows.** Any design that lets one party
  recover funds without the timelock expiring is a regression on
  Pitchfork's trust model.
- **Fee accounting is part of the design.** An escrow that silently
  fails on high fees is a broken escrow.
- **Nonce hygiene for MuSig2.** Nonce reuse is catastrophic; every
  MuSig2 session must generate fresh nonces per round.
- **Publicly auditable BTC commitments.** The hash-commitment bridge
  must let an auditor reconstruct the relationship between EVM
  state and BTC transactions without trusted intermediaries.
