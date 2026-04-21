---
slug: return42
kind: firm
project_path: ~/Projects/apps
upstream_url: https://return42.net
languages: [rust]
tier_override: tactical
contract_scope: [deterministic-nlp, systems-language, custom-llm-architecture]
specialists: [morphlex, jasterish, custom-llm]
---

# CR-Return42

## Scope

Return42 is an umbrella for three tightly-linked research artifacts: a
deterministic natural-language tokenizer (Morphlex), a systems language
whose machine instructions are English morphology (Jasterish / JStar), and
a custom language-model architecture built on the combined stack.

CR-Return42 is the firm-level civilian. Contracts to `return42` are
dispatched internally to one of:

- **morphlex** -- deterministic NL tokenization, 12-byte integer token vectors,
  morphology/semantic analysis, vectorizer, dictionary compilation.
- **jasterish** -- the JStar systems-language compiler: grammar, parser,
  typechecker, x86-64 codegen, ELF linker, IR, JSH shell / REPL.
- **custom-llm** -- language-model architecture leveraging morphlex's
  deterministic integer vectors and Jasterish's compilation pipeline.

Firms themselves do not execute specialist work; they route. The Morphlex
Phase 1 tokenizer is complete and self-hosting-ready; Jasterish is at plan
stage; Custom-LLM is an architecture proposal.

## When to contract CR-Return42

- The brief crosses two or more Return42 sub-projects (e.g. "design the
  morphlex token layout such that Jasterish codegen can consume it
  directly"). Firm-level contracting is correct when cross-specialist
  coordination is the point.
- The brief is ambiguous and you want the firm to pick the right
  specialist based on `routing_rules` below.
- The brief explicitly names Return42 without naming a specialist.

If the brief names a specialist directly, the firm still receives the
contract but routes immediately to that specialist.

## Routing rules

Deterministic keyword -> specialist routing. The broker scans the brief
for the first matching pattern; ties resolve left-to-right in the
order below.

- `morphlex` keywords: tokenizer, token vector, morphology, morpheme,
  lemma, semantic role, BLAKE3, 12-byte, POS, part-of-speech,
  vectorizer, dictionary compile
- `jasterish` keywords: jstar, compiler, systems language, x86-64,
  codegen, ELF, IR, parser, typechecker, recursive descent, self-host,
  JSH, REPL
- `custom-llm` keywords: LLM, training, inference, model architecture,
  embedding, attention, transformer, language model

Default fallback: `morphlex` (Phase 1 is complete; most concrete
questions resolve to tokenizer-layer concerns).

## Out of scope

- Generic NLP libraries or subword tokenizers -- Return42's thesis is
  deterministic morphology with integer identity, not SentencePiece/BPE.
- GPU inference stacks, distributed training -- the custom-LLM branch
  is an architecture, not an inference system.
- Float-based embedding work: Return42 explicitly avoids floats; the
  token vector is 12 bytes of packed integers.
- Anything downstream of a compiled Jasterish binary (runtime support
  libs, stdlib) -- specialist jasterish knows the compiler, not its
  runtime ecosystem.

## Deliverables

Firm-level closeouts typically include:

- A routing decision record: "brief -> specialist X because of rule
  R", plus the specialist's response.
- Cross-specialist synthesis when a brief spans Morphlex + Jasterish
  or Morphlex + Custom-LLM.
- A deferred-work summary when the brief touches an out-of-scope
  concern -- CR-Return42 names the right contact (if any) rather than
  extending scope.

## Execution guidance

- Preserve determinism invariants across sub-project boundaries. Any
  specialist response that would introduce float arithmetic into the
  token-vector hot path is rejected at the firm layer.
- Respect the Morphlex -> Jasterish -> Custom-LLM dependency ordering.
  A Custom-LLM decision that requires a Morphlex change is fine; a
  Morphlex change that breaks Jasterish's token-map assumption must
  re-enter via Jasterish review before landing.
- When specialist work would require reading code outside
  `~/Projects/apps/`, defer rather than guess. The firm's authority
  extends only over Return42 artifacts.
