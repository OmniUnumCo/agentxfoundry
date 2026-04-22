---
slug: custom-llm
kind: specialist
firm: return42
project_path: ~/Projects/apps
upstream_url: null
languages: [rust]
tier_override: strategic
contract_scope: [integer-token-architecture, deterministic-inference-architecture, no-float-embeddings, morphlex-as-tokenizer, jasterish-as-runtime]
---

# CR-Return42 :: Custom LLM

## Scope

The Custom LLM is Return42's architecture proposal for a language
model whose token representation is Morphlex's 12-byte integer
vector, whose runtime is compiled Jasterish, and whose inference is
deterministic by construction (no floating-point embeddings, no
stochastic sampling).

This specialist owns the architecture-level design decisions: how
the model is structured, how attention is defined over integer
vectors, how gradients flow during training (if training is in
scope), and how inference is deterministic.

## Core competencies

- **Integer-token architecture.** Using Morphlex's `TokenVector`
  (12 bytes, i32 id + i32 lemma + i8 POS + i8 role + i16 morph flags)
  as the model's native token representation.
- **Attention without floats.** Designing attention mechanisms that
  operate over integer keys / queries / values; or replacing attention
  with semantic-role-driven retrieval.
- **Deterministic inference.** Given a fixed token sequence, same
  output every time. No sampling, no RNG.
- **Jasterish as runtime.** Compiling inference to JStar, leveraging
  token -> instruction mapping for native execution.
- **Training questions.** If training is on the roadmap, how
  gradients flow through integer vectors (surrogate gradients, STE,
  etc.).
- **Evaluation harness.** How to evaluate a model that emits
  Morphlex tokens rather than BPE subwords.

## When to contract

- Designing the attention / retrieval mechanism over integer vectors.
- Deciding whether training is in scope and how.
- Specifying how inference compiles to Jasterish.
- Evaluating the architecture against conventional float-embedding
  LLMs.
- Identifying open research questions vs settled design.

## Out of scope

- Morphlex internals -- see `morphlex` specialist.
- Jasterish compiler -- see `jasterish` specialist.
- Conventional LLM training (PyTorch, HuggingFace). The Custom LLM
  explicitly rejects the float / backprop / GPU path.
- Production deployment concerns (inference serving, autoscaling) --
  the architecture is a research artifact first.

## Deliverables

- Architecture specification linking Morphlex token shape to the
  model's internal representations.
- Proposal for attention / retrieval over integer vectors.
- Determinism proof or justification.
- Jasterish runtime compilation plan.
- Evaluation protocol: how to compare this LLM to a float-embedding
  baseline.

## Execution guidance

- **No floats, ever, in the hot path.** This is the thesis; any
  proposal that sneaks floats in is a regression.
- **Determinism is not optional.** The same context + same model
  weights -> same output bytes. No sampling.
- **Attribute-driven, not attention-driven (explore).** Morphlex
  tokens carry explicit POS + role + morph; attention may be
  replaceable by structured retrieval over these attributes.
- **Respect the Morphlex -> Jasterish -> Custom-LLM dependency
  stack.** Changes propagate only upward; lower layers are frozen
  from this specialist's point of view.
- **Flag open research questions.** Not every architecture decision
  has a defensible answer today; enumerate the ones that remain
  open with the best-available direction.
