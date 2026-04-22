# Design Specification — Predictive Agent

## 1. Architecture

```
+-------------------+     +-------------------+
| input: Vec<f64>   |---->| PredictiveForecaster
| horizon: usize    |     | (closed-form OLS) |
+-------------------+     +--------+----------+
                                   |
                                   v
                          +--------+----------+
                          |  Forecast         |
                          |  predictions      |
                          |  slope/intercept  |
                          |  r_squared        |
                          +--------+----------+
                                   |
           +-----------------------+------------------------+
           |                                                |
           v                                                v
   build_analysis_prompt(values, forecast)           PredictiveAnalysis
           |                                          .forecast
           v                                          .llm_responses
  +--------+-----------+                              .combined
  | PredictiveAgent    |
  | ensemble: Vec<     |
  |   LabelledClient>  |
  +--------+-----------+
           |
           v (per member, sequential)
  +--------+-----------+    +-----------------+
  | LabelledClient     |--->| LLMClient::     |
  | label + Arc<LLM>   |    | complete()      |
  +--------------------+    +-----------------+
```

## 2. Why OLS instead of LSTM

The Python original used a PyTorch LSTM with hidden_size=50, output_size=1, but the `forecast` method notes: *"This is a placeholder, in practice train the model"*. The shipped model was untrained — its predictions were driven by random-initialized weights. That is:

1. **Non-deterministic** — weights vary per run unless seeded, and even seeded, determinism depends on PyTorch version + CUDA backend.
2. **Not actually better than a baseline** — an untrained LSTM on normalized data often regresses toward mean, which is strictly worse than linear regression for trending series.
3. **Heavy dependency chain** — pulls in `torch` (~1 GB), `numpy`, `pandas`, `CUDA` runtime.

Closed-form OLS gives us:
- **Exact reproducibility** — `f64` arithmetic is deterministic on a given architecture.
- **Zero dependencies beyond `std`** — just `f64` math.
- **Interpretability** — `slope`, `intercept`, `r_squared` communicate the fit.
- **Strictly better than random-initialized LSTM** on small time series.

A future ONNX Runtime driver (Slice E.3) can restore trained-LSTM inference for users who provide a model, without compromising the current deterministic default.

## 3. Forecast math

Given inputs `values = [y_0, y_1, ..., y_{n-1}]` and implicit `x_i = i`:

```
mean_x = sum(x_i) / n = (n-1)/2
mean_y = sum(y_i) / n

cov   = Σ (x_i - mean_x)(y_i - mean_y)
var_x = Σ (x_i - mean_x)²

slope     = cov / var_x        (0 if var_x == 0, i.e. n < 2 — guarded)
intercept = mean_y - slope * mean_x

For horizon h, predictions[k] = slope * (n + k) + intercept  for k in 0..h

R² = 1 - (Σ (y_i - ŷ_i)²) / (Σ (y_i - mean_y)²)
   where ŷ_i = slope * x_i + intercept
   (1 if ss_tot == 0, i.e. all y_i identical — guarded)
```

Edge cases:
- `n = 1`: rejected with `InsufficientData(1)`
- All `y_i` identical: `slope = 0`, `intercept = mean_y`, `R² = 1`
- Linearly trending: fits exactly, `R² = 1`

## 4. Ensemble structure

```rust
pub struct LabelledClient {
    pub label: String,             // e.g. "openai", "anthropic", "google"
    pub client: Arc<dyn LLMClient>,
}

pub struct PredictiveAgent {
    ensemble: Vec<LabelledClient>,
    analysis_tier: String,         // default "supervision"
}
```

**Why labelled?** Raw `Arc<dyn LLMClient>` doesn't know which provider it is. The label appears in `LlmAnalysis::provider_label` so downstream consumers can attribute quotes. Matches the Python original's hardcoded ["openai", "anthropic", "google"] but generalizes — any labelled set works.

**Why `Vec` (ordered) instead of `HashMap`?** Order stability. Two callers supplying the same ensemble in the same order MUST get identical `combined` output. Hash-based container would be non-deterministic.

## 5. Analysis prompt construction

```
You are analyzing a numeric time series and a deterministic linear forecast.

INPUT SERIES (N points):
- t0: {y_0:.6}
- t1: {y_1:.6}
  ...

FITTED MODEL: y = {slope:.6} * x + {intercept:.6}  (R² = {r2:.6})

FORECAST (H future points):
- t+N: {pred_0:.6}
  ...

Describe the trend, any obvious seasonality or level changes in the input,
and whether the linear projection is likely to hold. Keep the response
under 120 words.
```

**Why `{:.6}` fixed precision?** Two runs of the forecaster produce byte-identical `f64` values on a given architecture, but formatting with `{}` or `{:?}` varies by Rust version. Fixed precision locks the prompt bytes.

**Why "under 120 words"?** Keeps the response scannable and within cache-friendly lengths. LLMs generally obey this constraint.

## 6. Predictor trait

```rust
#[async_trait]
pub trait Predictor: Send + Sync {
    async fn analyze(&self, values: &[f64], horizon: usize)
        -> Result<PredictiveAnalysis, PredictError>;
}
```

Allows substitution with mock predictors in tests (see `EchoLLM` fixture in the test suite).

## 7. Error taxonomy

```rust
pub enum PredictError {
    InsufficientData(usize),        // < 2 samples
    ZeroHorizon,                    // horizon == 0
    LlmFailure(BoxError),           // any ensemble member failed
}
```

Ensemble failure is all-or-nothing: if one member returns `Err`, the whole `analyze` call fails. Alternatives considered:
- **Best-effort** (return partial results): rejected — determinism would be compromised by "which members succeeded this time"
- **Retry**: rejected at this layer — retry is a wrapper concern (caller or provider)
- **First-success**: rejected — defeats the purpose of an ensemble

## 8. Key design decisions

1. **Replace LSTM with OLS.** Determinism over mimicry. An LSTM-capable path can be added via ONNX Runtime in E.3.
2. **Preserve ensemble pattern.** Multi-LLM analysis is the feature the Python original exposed; we keep it.
3. **Sequential ensemble calls.** Order stability over latency; wire `join_all` in E.3.
4. **Fixed-precision prompt.** Byte-stable prompts mean deterministic cache keys.
5. **All-or-nothing ensemble.** Partial success would make outputs non-deterministic.
6. **No built-in normalization.** Python pre-normalized via `(values - mean) / std`. We don't — OLS is scale-invariant for slope/intercept purposes (the intercept absorbs mean shift). Callers who want z-scored inputs can pre-process.

## 9. Determinism profile

- **Forecast:** byte-identical `f64` outputs for fixed inputs (100x-verified)
- **Prompt:** byte-stable via fixed `{:.6}` formatting
- **Ensemble order:** stable; preserves caller order
- **Combined output:** concatenation in ensemble order
- **LLM responses:** delegated to underlying clients (wrap each in `DeterministicLLMClient` for full determinism)

## 10. Future extensions

- **ONNX Runtime driver.** Load a PyTorch-exported LSTM as ONNX, infer in-process with pinned seed. Drop-in replacement for `PredictiveForecaster`.
- **Confidence intervals.** Residual standard error + t-distribution → `prediction_low`, `prediction_high` at alpha=0.05. Additive field on `Forecast`.
- **Seasonal decomposition.** If `values.len() > some_threshold`, detect seasonality via autocorrelation and fit separately. Additive; does not replace linear.
- **Parallel ensemble.** `tokio::join_all` over members. Would need to preserve input order for `llm_responses` — sort-by-index trick.
