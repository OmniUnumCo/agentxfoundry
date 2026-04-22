---
slug: recursive-loop-closure
kind: specialist
firm: mab
project_path: ~/Projects/multimodal-agent-builder
upstream_url: null
languages: [python, rust]
tier_override: null
contract_scope: [rlc-ledger, closure-detection, pattern-based-adjustment, adaptive-training-hooks, cycle-termination]
---

# CR-MAB :: Recursive Loop Closure

## Scope

MAB's Recursive Loop Closure (RLC) Ledger: detects when an iterative
process (training, refinement, recursive agent execution) has closed
a meaningful cycle, and uses the pattern to adjust subsequent
iterations (learning rate, stopping criterion, continuation
decision).

## Core competencies

- **Closure-ledger data model.** Append-only record of loop
  iterations, input / output / delta per step, detected cycle
  signatures.
- **Closure detection.** Identifying when successive iterations
  produce bounded-difference output (near-fixed-point), or when
  outputs oscillate (limit cycle).
- **Pattern-based learning-rate adjustment.** Using ledger patterns
  to modulate the step size for the next iteration.
- **Cycle-termination criteria.** Rules for declaring a loop
  converged, diverged, or stalled.
- **Adaptive training hooks.** Callbacks that fire on closure
  detection, letting upstream training frameworks react.

## When to contract

- Designing an iterative refinement loop for an agent.
- Training-loop convergence diagnosis.
- A recursive agent keeps looping on near-identical output and
  needs a termination condition.
- Implementing adaptive step-size logic.

## Out of scope

- Core optimization theory (beyond what RLC covers concretely).
- Distributed training orchestration.
- Specific ML framework integrations; RLC is framework-agnostic.

## Deliverables

- RLC ledger schema for the target system.
- Closure-detection rule set.
- Termination-criteria decision table.
- Integration sketch showing where RLC hooks into an existing loop.

## Execution guidance

- **Append-only ledger.** The ledger is the source of truth; no
  in-place updates.
- **Delta quantification.** Closure detection needs a well-defined
  "difference between iterations" metric; state it explicitly per
  use case.
- **Oscillation vs fixed-point distinction.** Both are forms of
  closure; treat them differently (fixed-point = converge / stop;
  oscillation = diverge / escape or average).
- **Framework-agnostic.** The pattern works for PyTorch training
  loops, recursive agent calls, and refinement chains alike; every
  deliverable should make that explicit.
