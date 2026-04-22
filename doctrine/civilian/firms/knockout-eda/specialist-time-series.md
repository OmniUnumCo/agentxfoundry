---
slug: time-series
kind: specialist
firm: knockout-eda
project_path: ~/Projects/knockout-eda
upstream_url: null
languages: [python]
tier_override: null
contract_scope: [stationarity, arima, seasonal-arima, prophet, state-space, autocorrelation, change-point]
---

# CR-KnockoutEDA :: Time Series

## Scope

Time-series analysis and forecasting. Stationarity diagnostics,
autocorrelation, ARIMA family, Prophet, state-space models, change-
point detection.

## Core competencies

- Stationarity: ADF, KPSS; differencing to achieve stationarity.
- Autocorrelation: ACF, PACF for model-order identification.
- ARIMA, SARIMA, SARIMAX; model selection via AIC / BIC.
- Prophet (Meta's library): trend + seasonality + holidays
  decomposition.
- State-space / structural time-series models.
- Change-point detection: CUSUM, Bayesian online change-point, PELT.
- Backtesting with expanding / rolling origin.
- Forecast evaluation: MAPE, sMAPE, MASE, pinball loss for
  quantile forecasts.

## When to contract

- Data is time-indexed and predicting forward.
- Seasonality present.
- Need forecasts with uncertainty intervals.

## Out of scope

- Non-time-indexed prediction (see `prediction`).
- Deep-learning forecasters (LSTM, N-BEATS) at scale -- deferred.
- Online / streaming forecasting -- batch only.

## Deliverables

- Stationarity test results and differencing decision.
- ACF / PACF interpretation.
- Model fit with residual diagnostics (Ljung-Box for residual
  autocorrelation).
- Backtest result with appropriate metric.
- Forecast with uncertainty interval.

## Execution guidance

- Always test stationarity first; ARIMA on non-stationary data is
  wrong.
- Use expanding-origin backtesting, not k-fold CV.
- Quote sMAPE for scale-free error; MASE for comparison against a
  naive baseline.
- Flag heteroscedasticity; GARCH or log-transform may be required.
