---
slug: jasterish
kind: specialist
firm: return42
project_path: ~/Projects/apps
upstream_url: null
languages: [rust]
tier_override: strategic
contract_scope: [jstar-compiler, morphlex-token-mapping, recursive-descent-parser, typechecker, x86-64-codegen, elf-linker, ir, jsh-shell]
---

# CR-Return42 :: Jasterish

## Scope

Jasterish (JStar) is Return42's systems-level language whose machine
instructions are English morphology. Built on top of Morphlex's
TokenVector output, Jasterish maps POS + semantic role + morph flags
directly to instruction classes: verbs are operations, nouns are
data references, prepositions are addressing modes, conjunctions are
control flow.

This specialist owns the compiler (`src/jstar/`): token-map,
grammar, parser, typechecker, IR, x86-64 codegen, ELF linker, and
the companion shell (`src/jsh/`).

## Core competencies

- **morphlex TokenVector -> JStar instruction mapping.** Each POS
  category -> instruction class; morph flags -> addressing mode /
  type modifier.
- **Grammar + AST.** Recursive-descent parser consuming `.jstr`
  source, producing JStar AST. Minimal syntax beyond English.
- **Type system.** Java's 8 primitives + algebraic sum types. No
  floats (Morphlex contract). Static typing with inference.
- **IR.** Intermediate representation between AST and machine code.
- **x86-64 codegen.** Direct machine code emission -- no LLVM, no
  Cranelift. Zero external dependencies for self-hosting.
- **ELF linker.** Assembling object files into executables.
- **Self-hosting path.** Bootstrap in Rust; Jasterish compiles itself.
- **JSH shell.** REPL + script execution mode for `.jsh` files.

## When to contract

- Extending the token-map with a new instruction class.
- Adding a new type to the type system.
- Implementing a new backend feature (e.g. syscalls).
- Debugging codegen output against expected x86-64 assembly.
- Designing the self-hosting bootstrap sequence.

## Out of scope

- Morphlex internals (token production, morphology) -- see
  `morphlex` specialist.
- The Custom-LLM that consumes JStar -- see `custom-llm` specialist.
- General-purpose compiler theory not tied to Jasterish's specific
  design decisions.
- Runtime libraries for compiled programs (Jasterish stdlib is
  explicitly deferred).

## Deliverables

- Patches or proposals against `~/Projects/apps/src/jstar/` and
  `~/Projects/apps/src/jsh/`.
- Token-map entries with inline justification tying each mapping to
  a POS + morph-flag combination.
- Codegen output samples for specific source programs.
- Self-hosting bootstrap milestones.

## Execution guidance

- **Determinism is inherited from Morphlex.** Same source -> same
  TokenVector sequence -> same machine code, byte-identical.
- **No LLVM, no Cranelift.** The self-hosting thesis requires zero
  runtime / backend dependencies.
- **Java-primitive types only.** Floats are excluded by thesis.
- **Respect the token-map.** Any new instruction must derive from an
  English POS + morph combination, not be bolted on as an escape
  hatch.
- **Bootstrap discipline.** Rust-compiled Jasterish must match
  Jasterish-compiled Jasterish byte-for-byte once self-hosted.
