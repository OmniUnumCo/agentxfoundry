---
slug: mab
kind: firm
project_path: ~/Projects/multimodal-agent-builder
upstream_url: null
languages: [python, typescript]
tier_override: tactical
contract_scope: [multimodal-io, moa-reasoning, recursive-loop-closure, multi-llm-adapter]
specialists: [multimodal-io, moa-reasoning, recursive-loop-closure, multi-llm-adapter]
---

# CR-MAB

## Scope

Multimodal Agent Builder was shelved in favor of market-ready stacks,
but its conceptual IP survives as doctrine. Four distinct patterns
are preserved:

1. **Multimodal I/O** — text / image / audio input processing with
   consistent typing across modalities.
2. **MOA Reasoning** — Motivation-Opportunity-Ability agentic
   framework.
3. **Recursive Loop Closure Ledger** — training-pattern recognition
   for adaptive learning rate and closure detection.
4. **Multi-LLM Adapter** — provider-agnostic LLM switching patterns.

CR-MAB is the firm-level civilian. The code itself is NOT being
revived; these specialists exist to answer design questions that
draw on MAB's conceptual work.

## When to contract CR-MAB

- A new civilian (or military MOS) needs multimodal input handling.
- An LLM-backed decision needs MOA framing (motivation, opportunity,
  ability axes).
- A training or execution loop needs closure detection.
- Provider-agnostic LLM abstractions need a design precedent.

## Routing rules

- `multimodal-io` keywords: image, audio, video, multimodal, modality, vision, speech, transcription, captioning, OCR
- `moa-reasoning` keywords: MOA, motivation, opportunity, ability, agentic reasoning, decision framework, capability check
- `recursive-loop-closure` keywords: recursive loop closure, RLC, ledger, closure detection, training pattern, learning rate adjustment, feedback loop
- `multi-llm-adapter` keywords: multi-LLM, provider switching, adapter, fallback, LLM abstraction

Default fallback: `moa-reasoning` (the framework that most other
MAB patterns rest on).

## Out of scope

- Reviving the MAB codebase (`~/Projects/multimodal-agent-builder/`)
  itself. The code is shelved by thesis.
- Production multimodal inference serving -- the patterns are
  architectural, not operational.
- Novel research in agentic reasoning beyond the specific MOA
  framework.

## Deliverables

Firm-level closeouts:
- Pattern-of-the-right-shape identification for a caller's problem.
- Cross-specialist synthesis where a design spans multiple MAB
  patterns.
- Explicit referral when the problem is outside the MAB IP scope.

## Execution guidance

- **The code is shelved; the patterns are live.** Don't propose
  direct reuse of MAB code; translate patterns into the caller's
  target stack (Rust RR core, typically).
- **MOA is a framework, not an algorithm.** Most MAB-flavored
  designs frame decisions in Motivation + Opportunity + Ability
  terms; that framing is the product, not the implementation.
- **Determinism where possible.** Even non-deterministic LLM
  invocations can be framed around deterministic MOA-axis checks
  that happen before / after.
- **Adapt to Rational Reserve.** This civilian exists to inform RR
  designs, not to re-impose MAB's abstractions. Always translate.
