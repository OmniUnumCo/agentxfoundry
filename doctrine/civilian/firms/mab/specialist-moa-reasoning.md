---
slug: moa-reasoning
kind: specialist
firm: mab
project_path: ~/Projects/multimodal-agent-builder
upstream_url: null
languages: [any]
tier_override: strategic
contract_scope: [motivation-opportunity-ability-framing, agentic-decision-structure, capability-gating, explanation-generation]
---

# CR-MAB :: MOA Reasoning

## Scope

The MOA framework: every agentic decision is decomposed into three
axes — Motivation (why), Opportunity (when / context permits), and
Ability (can the agent actually do this?). Applies equally to
military MOS agents and civilian contractors as a structured
reasoning scaffold around LLM calls.

## Core competencies

- **Motivation axis.** What goal / urgency / safety / compliance
  reason drives the action? Explicitly elicited, not implied.
- **Opportunity axis.** Does the current context (data available,
  timing, dependencies met) permit this action?
- **Ability axis.** Does the agent have the skill / authorization /
  tooling to perform the action?
- **MOA gating.** Blocking an action when any axis fails; returning
  a structured reason.
- **Explanation generation.** Human-readable explanations phrased
  in MOA terms: "I did X because Motivation Y, Opportunity Z, Ability
  confirmed."
- **Feedback loops.** MOA state updates as new information arrives;
  inflight re-evaluation.

## When to contract

- Designing a new agent's decision loop.
- Adding interpretability to an LLM-backed decision ("why did it
  do that?").
- Gating a potentially-risky action behind explicit capability
  checks.
- Structuring a multi-step agent workflow so each step is auditable.

## Out of scope

- Implementing specific ML models. MOA is a reasoning scaffold, not
  a model architecture.
- Replacing LLM inference. MOA wraps LLM calls; it doesn't substitute
  for them.

## Deliverables

- MOA decomposition of a specific decision: Motivation / Opportunity /
  Ability axes populated with criteria.
- Gating logic that blocks / allows based on axis state.
- Explanation-template for user-facing output.
- Audit log format capturing MOA state at decision time.

## Execution guidance

- **All three axes, always.** A decision that only checks Motivation
  is incomplete; Opportunity and Ability must both be evaluated.
- **Failures are by-axis.** When blocking, cite which axis failed
  and why. "Denied" alone is insufficient.
- **MOA feeds audit, not just decisions.** Every decision's MOA
  state should be persisted for later review.
- **Translate to RR.** Military MOS doctrine already leans on
  rank-authority + MOS-scope checks; frame those as MOA's
  Ability-axis examples when porting designs.
