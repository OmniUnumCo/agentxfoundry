---
slug: data-cleaning
kind: specialist
firm: knockout-eda
project_path: ~/Projects/knockout-eda
upstream_url: null
languages: [python]
tier_override: null
contract_scope: [deduplication, type-coercion, string-normalization, outlier-treatment, encoding-normalization, schema-validation]
---

# CR-KnockoutEDA :: Data Cleaning

## Scope

Pre-analysis data cleaning: deduplication, type coercion, string
normalization, outlier treatment, encoding (UTF-8 vs Latin-1),
schema validation.

## Core competencies

- Deduplication: exact vs fuzzy (Levenshtein, Jaro-Winkler); survivor
  selection rules.
- Type coercion: strings-that-are-numbers, dates in 17 formats,
  booleans-as-strings.
- String normalization: whitespace, case, unicode normalization
  (NFC / NFKC), emoji / control-char stripping.
- Outlier treatment: winsorize / cap / remove / keep-and-flag.
- Encoding detection and conversion.
- Schema validation: pandera, pydantic, JSON Schema.

## When to contract

- Dataset is dirty and analysis / modeling is blocked.
- Integration data from multiple sources with inconsistent shapes.
- Need auditable cleaning provenance.

## Out of scope

- Missingness (see `missing-data`).
- Feature engineering / derived columns for modeling.
- Deep-parsing logs / semi-structured text -- defer to
  text-processing specialists (none in this firm).

## Deliverables

- Cleaning pipeline with explicit per-step counts (rows in, rows out).
- Schema definition validated against cleaned output.
- Audit log: every row modified or dropped with reason.
- Idempotent re-runnability: pipeline must produce identical output
  on identical input.

## Execution guidance

- Every cleaning decision is auditable. No silent row drops.
- Winsorize before removing outliers; removal is last resort.
- Normalize encoding to UTF-8 early; downstream tools fail worse
  with mixed encodings.
- Idempotent by construction: re-running on the cleaned output
  must not change it.
