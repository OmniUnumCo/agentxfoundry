---
slug: full-stack
kind: specialist
firm: knockout-eda
project_path: ~/Projects/knockout-eda
upstream_url: null
languages: [python]
tier_override: tactical
contract_scope: [pipeline-orchestration, workflow-composition, multi-phase-analysis, reproducibility, report-generation]
---

# CR-KnockoutEDA :: Full Stack

## Scope

Pipeline-level orchestration across multiple KnockoutEDA workflows.
Composes data-cleaning + missing-data + EDA + modeling + reporting
into a reproducible DAG with explicit provenance.

## Core competencies

- Workflow composition: which specialists to call, in what order,
  for a given research question.
- Reproducibility: environment pinning (requirements.txt / pyproject
  / conda-lock), data-version pinning, random-seed management.
- Pipeline orchestration: prefect, dagster, or plain makefiles for
  small pipelines.
- Report generation: jupyter + nbconvert, quarto, plain markdown.
- Provenance tracking: what data, which transforms, what code
  version.
- Handing off specialist-to-specialist context without losing the
  prior phase's findings.

## When to contract

- The research question spans multiple KnockoutEDA phases.
- Deliverable needs to be re-runnable by a different analyst.
- Pipeline will be version-controlled and repeated.

## Out of scope

- Any single specialist's domain depth. Full-stack orchestrates; it
  doesn't replace the specialists.
- Infrastructure at scale (Airflow, Kubernetes) -- the pipelines are
  research-scale.

## Deliverables

- Pipeline DAG with explicit phase ordering.
- Pinned-environment recipe.
- End-to-end runbook with input assumptions and output artifacts.
- Provenance log covering each phase's inputs / code version / seed.

## Execution guidance

- Compose specialists; don't duplicate their work. If a phase
  produces a regression, route the next phase to `prediction`, don't
  re-derive the regression.
- Pinned seeds at every stochastic step. Reproducibility is the
  contract.
- Version the data too, not just the code. A pipeline that runs on
  "whatever's in that CSV today" is not reproducible.
- Handoffs between specialists are explicit artifacts (CSV, parquet,
  feather) -- never implicit state.
