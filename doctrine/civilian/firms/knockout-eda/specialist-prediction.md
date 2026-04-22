---
slug: prediction
kind: specialist
firm: knockout-eda
project_path: ~/Projects/knockout-eda
upstream_url: null
languages: [python]
tier_override: null
contract_scope: [linear-regression, polynomial-regression, regularized-regression, residual-diagnostics, cross-validation]
---

# CR-KnockoutEDA :: Prediction

## Scope

Regression-based prediction: fitting a model that predicts a
continuous target, evaluating its quality, and diagnosing its
residuals. Time-series forecasting is NOT covered here (see
`time-series`).

## Core competencies

- Ordinary least squares.
- Polynomial regression (with concern for overfitting).
- Regularized regression: Ridge, Lasso, Elastic Net.
- Residual diagnostics: normality, homoscedasticity, independence,
  leverage, Cook's distance.
- Cross-validation: k-fold, stratified, leave-one-out.
- Feature engineering for linear models (log transforms, interaction
  terms).
- R-squared, adjusted R-squared, RMSE, MAE, MAPE interpretation.

## When to contract

- Continuous-target prediction with reasonably small feature count.
- Interpretable model required (not a black box).
- Need to understand residual patterns for model validity.

## Out of scope

- Time-series-specific models (ARIMA, Prophet, state-space) -- see
  `time-series`.
- Classification -- see `classification`.
- Neural networks / gradient-boosted trees at scale -- this is
  classical stats, not ML.

## Deliverables

- Fitted model with coefficients + standard errors.
- Cross-validated error metric (prefer RMSE unless scale issues).
- Residual diagnostic report.
- Feature importance ranking (beta weights or standardized betas).

## Execution guidance

- Fit a simple OLS first. Justify any move to polynomial or
  regularized.
- Always cross-validate; single train-test split is insufficient
  justification for deployment.
- Standardize features before Lasso / Ridge / Elastic Net.
