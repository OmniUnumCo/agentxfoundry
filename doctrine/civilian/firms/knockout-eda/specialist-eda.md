---
slug: eda
kind: specialist
firm: knockout-eda
project_path: ~/Projects/knockout-eda
upstream_url: null
languages: [python]
tier_override: null
contract_scope: [univariate-profiling, bivariate-correlation, distribution-shape, outlier-detection, data-dictionary]
---

# CR-KnockoutEDA :: EDA

## Scope

Exploratory Data Analysis. First-pass characterization of a dataset
before any modeling. Univariate and bivariate profiling, distribution
shape, outlier detection, correlation structure, data dictionary
generation.

## Core competencies

- Univariate summary: mean, median, mode, std, IQR, skew, kurtosis per
  column.
- Distribution-shape characterization (normal / lognormal / uniform /
  multimodal) via Shapiro / Kolmogorov-Smirnov / Anderson-Darling.
- Outlier detection: Tukey's 1.5*IQR, z-score, MAD, isolation forest.
- Bivariate correlation: Pearson, Spearman, Kendall's tau. Choice of
  correlation based on data type + distribution.
- Missingness map: pattern visualization, correlation of missingness
  between columns.
- Cardinality analysis for categorical columns.

## When to contract

- A new dataset has just landed and we need a characterization.
- Before modeling, to inform feature-engineering decisions.
- To detect data-quality issues surfaceable at univariate level.

## Out of scope

- Imputation (defer to `missing-data`), cleaning (defer to
  `data-cleaning`).
- Hypothesis testing on the profiling results (defer to
  `hypothesis-testing`).
- Any forecasting / prediction / classification.

## Deliverables

- Data dictionary: column -> type, n, n_missing, unique_count,
  distribution_family, outlier_count, correlation_top_3.
- Distribution diagnostics per column with chosen normality test +
  p-value.
- Correlation matrix with method justification (Pearson for linear,
  Spearman for monotonic, etc.).

## Execution guidance

- State the distribution-shape conclusion explicitly; don't leave it
  implicit in a histogram.
- Always report IQR alongside std; either alone can mislead.
- Flag high-cardinality categoricals -- they will blow up downstream
  one-hot encoding.
