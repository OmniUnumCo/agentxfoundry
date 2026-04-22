---
slug: knockout-eda
kind: firm
project_path: ~/Projects/knockout-eda
upstream_url: null
languages: [python]
tier_override: execution_guidance
contract_scope: [statistical-analysis-via-mcp, nine-combo-workflows, deterministic-stats-pipelines]
specialists: [eda, hypothesis-testing, missing-data, prediction, classification, time-series, data-cleaning, bayesian, full-stack]
---

# CR-KnockoutEDA

## Scope

Diesel Knockout EDA is a statistical analysis tool with an MCP
interface, covering nine combo workflows that span the typical
"take raw data and understand it / model it" pipeline. Each workflow
is a self-contained specialist here; the firm routes briefs to the
one that best fits the phase of analysis requested.

## When to contract CR-KnockoutEDA

- Cross-phase statistical work (e.g. "clean this, then explore it,
  then propose a prediction approach").
- Ambiguous-scope briefs where the firm picks by routing rules.
- Choosing which of the nine workflows is even appropriate for a
  given dataset shape.

## Routing rules

- `eda` keywords: exploratory, EDA, distribution, histogram, correlation, summary statistics, describe, profile, outliers, quantile
- `hypothesis-testing` keywords: hypothesis, p-value, t-test, ANOVA, chi-square, non-parametric, mann-whitney, kruskal-wallis, statistical significance, alpha
- `missing-data` keywords: missing, NaN, null, imputation, MICE, kNN impute, MAR, MCAR, MNAR, listwise
- `prediction` keywords: regression, predict, forecast (non-time-series), linear regression, polynomial, residuals, R-squared, OLS
- `classification` keywords: classify, logistic regression, decision tree, random forest, SVM, gradient boosting, confusion matrix, ROC, AUC, precision recall
- `time-series` keywords: time series, ARIMA, ARMA, Prophet, seasonality, stationarity, ADF test, forecast, autocorrelation, ACF, PACF
- `data-cleaning` keywords: clean, deduplicate, normalize, standardize, type coercion, outlier removal, string cleanup, encoding
- `bayesian` keywords: Bayesian, prior, posterior, MCMC, Stan, PyMC, Metropolis, Gibbs, Bayes factor, credible interval
- `full-stack` keywords: pipeline, orchestration, full stack, end-to-end, workflow composition, DAG, multi-step

Default fallback: `eda` (most briefs that don't specify a phase are
asking for initial exploration).

## Out of scope

- Machine learning at scale (deep learning, distributed training) --
  Knockout-EDA is classical statistics.
- Real-time streaming analytics -- the nine workflows are batch.
- Data engineering (ETL pipelines, warehouse design) -- the data is
  assumed to exist and be accessible.
- Visualization as a deliverable (plots are incidental to the
  statistical output, not the point).

## Deliverables

Firm-level closeouts route to specialists but add:
- Cross-workflow dependency maps when a brief requires multiple
  phases.
- Recommendations for the sequence of specialists to contract.

## Execution guidance

- **Statistics first, visualization second.** Every specialist's
  output is a numerical / model result with commentary; plots are
  supporting, not primary.
- **Determinism by default.** Set random seeds explicitly; state
  them in every deliverable.
- **Assumptions must be checked.** Any result that assumes
  normality / independence / stationarity without stating that
  assumption is incomplete.
- **Effect sizes accompany p-values.** Statistical significance
  without effect size is noise; every hypothesis-test deliverable
  includes both.
