# Requirements — Predictive Agent

**Module:** `src/predictive.rs`
**Status:** Implemented with deterministic forecaster; multi-LLM ensemble operational.
**Source:** `agents/predictive/predictive_agent.py` (Python original)

## 1. Purpose

Replace the PyTorch-LSTM-based Python predictive agent with a compiled, deterministic Rust module. Combines a closed-form numerical forecaster (replacing LSTM) with a multi-LLM ensemble for qualitative commentary on the forecast.

## 2. Functional requirements

**FR-PR-01.** The module SHALL provide a `PredictiveForecaster` type that, given `values: &[f64]` and `horizon: usize`, returns a `Forecast { predictions, slope, intercept, r_squared }`.

**FR-PR-02.** The forecaster SHALL use ordinary least-squares linear regression (`y = slope*x + intercept` fitted to input series) with a closed-form solution — no iterative optimization, no random initialization, no training.

**FR-PR-03.** The forecaster SHALL reject inputs with fewer than 2 samples (`InsufficientData`) or zero horizon (`ZeroHorizon`).

**FR-PR-04.** The forecaster SHALL compute `r_squared` as the coefficient of determination on the input series (a diagnostic for fit quality).

**FR-PR-05.** A `PredictiveAgent` SHALL wrap an ordered list of `LabelledClient` (label + `Arc<dyn LLMClient>`) and call each member in turn to produce analysis commentary on the forecast.

**FR-PR-06.** `PredictiveAgent::analyze(values, horizon)` SHALL:
  1. Compute the forecast
  2. Build a deterministic analysis prompt embedding the input series and fitted line at fixed precision (6 decimals)
  3. Call each ensemble member at tier `"supervision"` (configurable)
  4. Return `PredictiveAnalysis { forecast, llm_responses, combined }`

**FR-PR-07.** Ensemble responses SHALL preserve caller-supplied order in `llm_responses` (no reordering, no deduplication).

**FR-PR-08.** The combined output SHALL be a Markdown-style concatenation: `### {label}\n{response}\n\n...` per member.

## 3. Non-functional requirements

**NFR-PR-01. Forecast determinism.** For any `(values, horizon)` pair, the forecast MUST be byte-identical across 100 invocations. Proven by unit test.

**NFR-PR-02. Precision stability.** Prompt embeds fixed `{:.6}` float formatting. Two forecasts differing by 1e-7 would still produce the same prompt (rounded).

**NFR-PR-03. No GPU dependency.** The module MUST compile and run without CUDA / MPS / any ML framework.

**NFR-PR-04. No floating-point drift between runs.** Same hardware + same Rust compiler produces byte-identical `f64` output. BLAKE3-hashed forecast would be reproducible.

**NFR-PR-05. Ensemble concurrency.** All ensemble members SHALL be called sequentially for Slice E.1/E.2 to preserve order stability. Parallel `tokio::join_all` deferred to E.3.

## 4. Scope

**In scope:**
- Linear least-squares forecaster (slope, intercept, R², point predictions)
- Multi-LLM ensemble for analysis commentary
- Stable prompt construction with fixed-precision numerics
- `Predictor` async trait for substitution in tests

**Out of scope:**
- True LSTM inference (Slice E.3 via ONNX Runtime)
- Seasonal decomposition (ARIMA, Prophet-style models)
- Confidence intervals on predictions
- Multi-dimensional time series (scalar only)
- Anomaly detection
- Training / fitting from labeled data (closed-form only)

## 5. Acceptance criteria

1. Input `[1.0, 2.0, 3.0, 4.0, 5.0]` with horizon 3 MUST produce `predictions = [6.0, 7.0, 8.0]`, `slope = 1.0`, `intercept = 1.0`, `r_squared = 1.0`.
2. Input `[3.0, 3.0, 3.0, 3.0]` MUST produce `slope = 0.0` and constant predictions equal to 3.0.
3. Running the same `(values, horizon)` 100 times MUST produce identical `Forecast` each time.
4. Input of 0 or 1 element MUST return `InsufficientData`.
5. Horizon of 0 MUST return `ZeroHorizon`.
6. `PredictiveAgent::analyze` with a 3-provider ensemble MUST return `llm_responses.len() == 3` in caller-supplied order.
7. A zero-norm input to `cosine`-adjacent helpers MUST NOT panic.
