---
slug: classification
kind: specialist
firm: knockout-eda
project_path: ~/Projects/knockout-eda
upstream_url: null
languages: [python]
tier_override: null
contract_scope: [logistic-regression, decision-tree, random-forest, gradient-boosting, svm, model-calibration, imbalanced-data]
---

# CR-KnockoutEDA :: Classification

## Scope

Classification: fitting a model that predicts a discrete target
(binary or multiclass), evaluating it with appropriate metrics,
calibrating probabilities, and handling imbalanced data.

## Core competencies

- Logistic regression (binary, multinomial, ordinal).
- Decision tree, random forest, gradient boosting (sklearn, xgboost,
  lightgbm).
- Support vector machines (linear, RBF kernels).
- Metrics: accuracy, precision, recall, F1, ROC-AUC, PR-AUC,
  log-loss, Brier score.
- Calibration: Platt scaling, isotonic regression, calibration
  curves.
- Imbalanced data: SMOTE, class weighting, threshold tuning.
- Feature importance: coefficients (linear), gain / permutation /
  SHAP (trees).

## When to contract

- Discrete-target prediction.
- Probability estimates needed (call `calibration` in the deliverable).
- Imbalanced classes (common in fraud, anomaly detection).

## Out of scope

- Deep learning classifiers.
- Time-series classification.
- Generative models.

## Deliverables

- Model choice with justification.
- Metric table: accuracy, precision, recall, F1, AUC (appropriate
  to problem).
- Confusion matrix + misclassification analysis.
- Calibration assessment.
- Feature importance.

## Execution guidance

- Never rely on accuracy alone, especially with imbalanced data.
- State the default threshold (usually 0.5) and when it should be
  changed.
- Prefer PR-AUC over ROC-AUC for heavily imbalanced problems.
- Calibrate probabilities before using them as inputs to downstream
  decisions.
