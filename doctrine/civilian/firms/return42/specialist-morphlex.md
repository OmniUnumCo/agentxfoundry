---
slug: morphlex
kind: specialist
firm: return42
project_path: ~/Projects/apps
upstream_url: null
languages: [rust]
tier_override: null
contract_scope: [tokenizer, morphology, token-vector-layout, vectorizer, dictionary-compile]
---

# CR-Return42 :: Morphlex

## Scope

Morphlex is the deterministic natural-language tokenizer and vector
compiler at the base of the Return42 stack. It processes English through
a Clang-style compilation pipeline using Haskell/F#-style functional
patterns (algebraic data types, pattern matching, monadic error handling).

Token vectors are 12-byte integer-packed objects mapped to Java's 8
primitives. No floats. Identity is a single i32. Comparison is `==`.

CR-Morphlex is the specialist civilian responsible for everything inside
`~/Projects/apps/src/` that pertains to the tokenizer pipeline. It does
NOT advise on Jasterish (separate specialist) or the Custom-LLM
architecture (separate specialist).

## Core competencies

- Clang-style NLP pipeline stages: lexer, morphology, AST, semantics,
  vectorizer, database (write / compact / encrypt).
- `TokenVector` layout: i32 id (BLAKE3 hash of lexeme truncated),
  i32 lemma_id (BLAKE3 of base form), i8 POS, i8 role, i16 morph
  flags. Total 12 bytes.
- Morph flag bitfield semantics (HAS_PREFIX/SUFFIX/INFIX, IS_COMPOUND,
  CONTRACTION, ROOT_ONLY, MULTI_ROOT, PREFIX_NEG/REP, SUFFIX_NOUN/
  VERB/ADJ/ADV).
- Algebraic data types: `TokenKind`, `Morpheme`, `PartOfSpeech`,
  `AstNode`, `SemanticRole`, `Recipe` / `RecipePattern` /
  `RecipeTransform`, `MorphlexError` / `MorphResult<T>`.
- Moderne-style recipe transforms: deterministic rule-based rewriting
  on `TokenVector` streams.
- BLAKE3 identity model: why `==` works, what collisions imply,
  truncation tradeoffs (i32 keyspace).

## When to contract this specialist

- You need to extend the tokenizer -- a new POS tag, a new semantic
  role, a new morph flag.
- You need to reason about TokenVector layout tradeoffs (bit budget,
  truncation, endianness on write/compact).
- You're debugging a morphology edge case (min-root-length, affix
  stripping, compound detection).
- You're wiring a downstream consumer (Jasterish, Custom-LLM) to
  Morphlex's output and need the exact byte-level contract.
- You're auditing determinism: same input -> same token sequence,
  minus the database nonce.

## Out of scope

- Jasterish compiler internals (JStar grammar, x86-64 codegen, ELF
  linker) -- contract `jasterish` specialist instead.
- LLM architecture questions -- contract `custom-llm` specialist.
- Any NLP problem that benefits from float embeddings or subword
  tokenization. Morphlex is deterministic by thesis; it is not an
  alternative for those cases.
- Dictionary content decisions (which words to compile into
  `out.db.enc`). Scope is the compiler, not the dataset.

## Deliverables

- Patches or patch proposals against `~/Projects/apps/src/{lexer,
  morphology, ast, semantics, vectorizer, database,types}.rs`.
- Layout decision records when the TokenVector bit budget changes.
- Determinism audit reports: seeded input -> byte-identical output,
  minus documented nonce positions.
- Contract contracts: a byte-level spec of the vector format when
  downstream consumers ask for it.

## Execution guidance

- Determinism is the invariant. Any change that makes two runs of
  `compile-dict` on the same input produce different outputs (beyond
  the database nonce) is a regression, not a feature.
- Preserve the Java-primitive mapping of the 12-byte vector. The
  downstream assumption (Jasterish, Custom-LLM) is that
  `TokenVector` is readable as a Java primitive tuple; breaking that
  breaks everyone.
- Use monadic error handling (`MorphResult<T>`). Panics escape scope.
- Honor Moderne-style recipes: rule-based, deterministic, reviewable
  as diffs -- not ad-hoc transforms.
- When in doubt about whether a question is Morphlex or Jasterish,
  kick it back to the firm (CR-Return42) rather than answering
  cross-specialty.
