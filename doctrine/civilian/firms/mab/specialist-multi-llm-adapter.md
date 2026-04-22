---
slug: multi-llm-adapter
kind: specialist
firm: mab
project_path: ~/Projects/multimodal-agent-builder
upstream_url: null
languages: [python, rust, typescript]
tier_override: null
contract_scope: [provider-abstraction, capability-differentiated-routing, fallback-cascades, response-normalization]
---

# CR-MAB :: Multi-LLM Adapter

## Scope

Patterns for provider-agnostic LLM abstraction, drawn from MAB's
multi-LLM adapter layer. The goal: code that calls "an LLM" without
knowing which provider, with principled routing based on capability
differences (not just availability).

## Core competencies

- **Provider abstraction.** Uniform interface (like RR's `LLMClient`
  trait) that hides provider-specific quirks.
- **Capability-differentiated routing.** Routing based on what the
  call actually needs: structured output, function calling, long
  context, vision -- not just "pick the biggest model."
- **Fallback cascades.** Primary -> secondary -> tertiary provider
  logic with explicit failure-mode classification (retry vs escalate
  vs fail).
- **Response normalization.** Provider responses (OpenAI choices
  array, Anthropic content blocks, Google candidates) normalized to
  a single shape.
- **Request shaping.** Provider-specific quirks handled in the
  adapter (Anthropic needs max_tokens; OpenAI optional; seed
  semantics differ).

## When to contract

- Designing an LLM abstraction layer in a new system.
- Adding a new provider to an existing abstraction.
- Building a fallback cascade across providers.
- Normalizing response shapes when downstream code wants uniformity.

## Out of scope

- Implementing specific providers end-to-end -- RR already has that
  in `src/llm/providers/`. This specialist is about the abstraction
  design, not the implementation.
- Authentication / secret management (operational concern).
- Cost optimization strategies beyond capability-based routing.

## Deliverables

- Provider-abstraction trait / interface design.
- Capability matrix: what each provider supports natively.
- Fallback-cascade decision tree.
- Response-normalization schema.
- Request-shaping quirks table.

## Execution guidance

- **Capability first, availability second.** Route based on what the
  call needs; pick among providers that can satisfy that need.
- **Fallback with classification.** Transient network error -> retry
  primary; rate limit -> fall over to secondary; capability mismatch
  -> escalate to caller.
- **Normalize, don't flatten.** A normalized response preserves
  enough structure that downstream code can interrogate it (content
  blocks, role markers, tool calls); flattening to raw text loses
  provenance.
- **RR's `DeterministicLLMClient` is a precedent.** That wrapper is
  a concrete instance of multi-LLM-adapter patterns; cite it when
  translating designs to RR.
