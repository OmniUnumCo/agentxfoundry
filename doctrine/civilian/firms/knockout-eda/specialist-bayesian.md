---
slug: bayesian
kind: specialist
firm: knockout-eda
project_path: ~/Projects/knockout-eda
upstream_url: null
languages: [python]
tier_override: supervision
contract_scope: [prior-elicitation, mcmc-sampling, posterior-analysis, bayes-factors, credible-intervals, hierarchical-models]
---

# CR-KnockoutEDA :: Bayesian

## Scope

Bayesian inference: prior elicitation, posterior sampling via MCMC,
posterior analysis, Bayes factors, credible intervals, hierarchical
models.

## Core competencies

- Prior elicitation: conjugate priors, weakly informative priors,
  empirical-Bayes alternatives.
- MCMC: Metropolis-Hastings, Gibbs, NUTS / HMC (PyMC, Stan, NumPyro).
- Posterior diagnostics: trace plots, R-hat, ESS, divergences.
- Credible intervals (not confidence intervals) and highest-density
  intervals.
- Bayes factors and model comparison.
- Hierarchical / multilevel models for nested data.
- Posterior predictive checks.

## When to contract

- Small-n situations where prior information has real value.
- Need uncertainty distributions, not point estimates + CIs.
- Multilevel / hierarchical structure in the data.
- Calibrated probability of hypotheses (Bayes factors) needed.

## Out of scope

- Black-box deep learning. Bayesian NNs are possible but out of this
  firm's scope.
- Approximate-inference-at-scale beyond what PyMC / NumPyro support.
- Frequentist hypothesis testing (see `hypothesis-testing`).

## Deliverables

- Prior justification.
- Posterior samples + diagnostics (R-hat, ESS, divergences).
- Credible interval on the quantity of interest.
- Posterior predictive check.

## Execution guidance

- State the prior and its justification explicitly. Priors are
  modeling choices that must be defensible.
- Diagnose convergence before reporting posteriors. R-hat > 1.01 is
  suspicious.
- Posterior predictive checks are part of the deliverable, not
  optional.
- Prefer NUTS / HMC over Metropolis when model supports it.
