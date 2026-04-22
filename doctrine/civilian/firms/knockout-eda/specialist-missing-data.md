---
slug: missing-data
kind: specialist
firm: knockout-eda
project_path: ~/Projects/knockout-eda
upstream_url: null
languages: [python]
tier_override: null
contract_scope: [missingness-mechanism, imputation-selection, MICE, knn-imputation, multiple-imputation, listwise-deletion]
---

# CR-KnockoutEDA :: Missing Data

## Scope

Missingness analysis and imputation. Diagnose the missingness
mechanism (MCAR, MAR, MNAR), pick an imputation strategy, execute
single or multiple imputation, evaluate the impact on downstream
analyses.

## Core competencies

- Missingness-mechanism diagnosis: Little's MCAR test, logistic
  regression of missingness indicators.
- Imputation methods: mean / median / mode (fast, biased), kNN,
  MICE (multiple imputation by chained equations), regression-based,
  last-observation-carried-forward for time series.
- Multiple imputation: generating N imputed datasets, pooling
  estimates via Rubin's rules.
- Listwise / pairwise deletion tradeoffs.
- Sentinel value detection (-999, 9999 as pseudo-missing).

## When to contract

- Dataset has >5% missing values and imputation strategy matters.
- Suspicion that missingness is informative (MNAR).
- Need to preserve variance across imputed datasets.

## Out of scope

- Data cleaning broadly (see `data-cleaning`).
- Downstream modeling on the imputed data (see relevant modeling
  specialist).

## Deliverables

- Missingness-mechanism report (MCAR / MAR / MNAR with justification).
- Imputation method recommendation + execution.
- Comparison of listwise-delete vs imputation on downstream estimate.
- For multiple imputation: pooled result with Rubin's-rule SEs.

## Execution guidance

- Default to MICE for MAR data with >3 columns missing.
- Don't impute if MNAR and mechanism is unknown -- flag the issue
  instead of hiding it.
- Report variance inflation from imputation -- it matters for
  downstream inference.
