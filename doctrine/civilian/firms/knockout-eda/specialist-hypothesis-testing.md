---
slug: hypothesis-testing
kind: specialist
firm: knockout-eda
project_path: ~/Projects/knockout-eda
upstream_url: null
languages: [python]
tier_override: null
contract_scope: [parametric-tests, nonparametric-tests, multiple-comparison-correction, effect-sizes, power-analysis]
---

# CR-KnockoutEDA :: Hypothesis Testing

## Scope

Statistical hypothesis testing: framing hypotheses, picking the
right test given data shape and distribution, correcting for
multiple comparisons, reporting effect sizes, and power analysis.

## Core competencies

- Parametric: t-test (one-sample, two-sample, paired), ANOVA (one-
  way, factorial), chi-square, F-test.
- Non-parametric: Mann-Whitney U, Wilcoxon signed-rank, Kruskal-
  Wallis, Friedman.
- Multiple-comparison correction: Bonferroni, Holm, Benjamini-
  Hochberg FDR.
- Effect sizes: Cohen's d, eta-squared, Cramer's V, Hedges' g.
- Power analysis: a-priori and post-hoc.
- Assumption checks: normality, homoscedasticity, independence.

## When to contract

- A comparison question needs a formal test.
- A set of comparisons needs correction.
- Reporting p-values alone is insufficient (always).

## Out of scope

- Modeling (see `prediction` or `classification`).
- Bayesian alternatives (see `bayesian`).
- Exploratory analysis (see `eda`).

## Deliverables

- Test choice with justification (parametric vs non-parametric).
- p-value, effect size, confidence interval.
- Assumption-check results.
- Multiple-comparison correction method + adjusted p-values.

## Execution guidance

- Never report p-value alone. Effect size + CI + corrected p-value.
- Check assumptions before picking parametric. Default to non-
  parametric when in doubt.
- State the null hypothesis explicitly. "Statistical significance"
  without a stated H0 is noise.
