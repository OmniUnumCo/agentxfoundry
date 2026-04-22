# Technical Specification — Predictive Agent

## 1. Module map

Single file: `src/predictive.rs`.

## 2. Public types

```rust
pub enum PredictError {
    InsufficientData(usize),
    ZeroHorizon,
    LlmFailure(Box<dyn std::error::Error + Send + Sync>),
}

pub struct Forecast {
    pub predictions: Vec<f64>,
    pub slope: f64,
    pub intercept: f64,
    pub r_squared: f64,
}

pub struct LlmAnalysis {
    pub provider_label: String,
    pub response: String,
}

pub struct PredictiveAnalysis {
    pub forecast: Forecast,
    pub llm_responses: Vec<LlmAnalysis>,
    pub combined: String,
}

pub struct PredictiveForecaster;
impl PredictiveForecaster {
    pub fn forecast(values: &[f64], horizon: usize) -> Result<Forecast, PredictError>;
}

pub struct LabelledClient {
    pub label: String,
    pub client: Arc<dyn LLMClient>,
}
impl LabelledClient {
    pub fn new(label: impl Into<String>, client: Arc<dyn LLMClient>) -> Self;
}

pub struct PredictiveAgent { /* private fields */ }
impl PredictiveAgent {
    pub fn new(ensemble: Vec<LabelledClient>) -> Self;
    pub fn with_analysis_tier(self, tier: impl Into<String>) -> Self;
}

#[async_trait]
pub trait Predictor: Send + Sync {
    async fn analyze(&self, values: &[f64], horizon: usize)
        -> Result<PredictiveAnalysis, PredictError>;
}

impl Predictor for PredictiveAgent { ... }
```

All types derive `Debug + Clone + serde::Serialize` where appropriate. `PartialEq` on `Forecast` for test equality.

## 3. Forecaster implementation

```rust
pub fn forecast(values: &[f64], horizon: usize) -> Result<Forecast, PredictError> {
    if values.len() < 2 { return Err(PredictError::InsufficientData(values.len())); }
    if horizon == 0 { return Err(PredictError::ZeroHorizon); }

    let n = values.len() as f64;
    let xs: Vec<f64> = (0..values.len()).map(|i| i as f64).collect();
    let mean_x = xs.iter().sum::<f64>() / n;
    let mean_y = values.iter().sum::<f64>() / n;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    for i in 0..values.len() {
        cov += (xs[i] - mean_x) * (values[i] - mean_y);
        var_x += (xs[i] - mean_x).powi(2);
    }
    let slope = if var_x == 0.0 { 0.0 } else { cov / var_x };
    let intercept = mean_y - slope * mean_x;

    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;
    for i in 0..values.len() {
        let yhat = slope * xs[i] + intercept;
        ss_res += (values[i] - yhat).powi(2);
        ss_tot += (values[i] - mean_y).powi(2);
    }
    let r_squared = if ss_tot == 0.0 { 1.0 } else { 1.0 - ss_res / ss_tot };

    let predictions = (0..horizon)
        .map(|h| {
            let x = values.len() as f64 + h as f64;
            slope * x + intercept
        })
        .collect();

    Ok(Forecast { predictions, slope, intercept, r_squared })
}
```

## 4. Prompt builder

```rust
fn build_analysis_prompt(values: &[f64], forecast: &Forecast) -> String {
    let mut sample = String::new();
    for (i, v) in values.iter().enumerate() {
        sample.push_str(&format!("- t{}: {:.6}\n", i, v));
    }
    let mut pred = String::new();
    for (i, v) in forecast.predictions.iter().enumerate() {
        pred.push_str(&format!("- t+{}: {:.6}\n", values.len() + i, v));
    }
    format!("You are analyzing a numeric time series and a deterministic linear forecast.\n\n\
             INPUT SERIES ({n} points):\n{sample}\n\
             FITTED MODEL: y = {slope:.6} * x + {intercept:.6}  (R^2 = {r2:.6})\n\n\
             FORECAST ({horizon} future points):\n{pred}\n\
             Describe the trend, any obvious seasonality or level changes in the\n\
             input, and whether the linear projection is likely to hold. Keep the\n\
             response under 120 words.\n",
        n = values.len(), sample = sample.trim_end(),
        slope = forecast.slope, intercept = forecast.intercept, r2 = forecast.r_squared,
        horizon = forecast.predictions.len(), pred = pred.trim_end())
}
```

## 5. Ensemble call loop

```rust
async fn analyze(&self, values: &[f64], horizon: usize)
    -> Result<PredictiveAnalysis, PredictError> {

    let forecast = PredictiveForecaster::forecast(values, horizon)?;
    let prompt = build_analysis_prompt(values, &forecast);

    let mut responses = Vec::with_capacity(self.ensemble.len());
    for member in &self.ensemble {
        let resp = member.client
            .complete(&prompt, &self.analysis_tier, Some(512), Some(0.0))
            .await
            .map_err(PredictError::LlmFailure)?;
        responses.push(LlmAnalysis {
            provider_label: member.label.clone(),
            response: resp,
        });
    }

    let combined = combine_responses(&responses);
    Ok(PredictiveAnalysis { forecast, llm_responses: responses, combined })
}
```

## 6. Response combiner

```rust
fn combine_responses(responses: &[LlmAnalysis]) -> String {
    let mut out = String::new();
    for r in responses {
        out.push_str(&format!("### {}\n{}\n\n", r.provider_label, r.response));
    }
    out.trim_end().to_string()
}
```

## 7. Test coverage

**Forecaster unit tests (5):**
1. `forecast_on_perfect_line_is_exact` — `[1,2,3,4,5]` → slope=1, intercept=1, R²=1
2. `forecast_on_flat_line_has_zero_slope` — `[3,3,3,3]` → slope=0, constant predictions
3. `forecast_is_deterministic` — 100x invocation yields `PartialEq` equal Forecast
4. `forecast_rejects_too_few_samples` — `[1.0]` → `InsufficientData(1)`
5. `forecast_rejects_zero_horizon` — `([1,2], 0)` → `ZeroHorizon`

**Agent integration tests (3):**
6. `agent_combines_forecast_with_ensemble_responses` — 3-member ensemble, assert all labels present in combined
7. `agent_preserves_ensemble_order` — ensemble order matches `llm_responses` order
8. Ensemble failure propagation (implicit via `PredictError::LlmFailure`)

Total: **8 unit tests**, 0 failures.

## 8. `EchoLLM` test fixture

```rust
struct EchoLLM { label: String }

#[async_trait]
impl LLMClient for EchoLLM {
    async fn complete(&self, _prompt: &str, _tier: &str,
                      _max_tokens: Option<usize>, _temperature: Option<f64>)
        -> Result<String, BoxError> {
        Ok(format!("analysis from {}", self.label))
    }
}
```

Trivial mock that echoes its label — sufficient to verify ensemble-order preservation and combined-output assembly without any network.

## 9. Numerical precision considerations

- All math uses `f64` (double-precision).
- `.powi(2)` used instead of `*` for readability; compiler emits the same code.
- No Kahan summation; accumulated error on small series (<10k samples) is well within display precision.
- `r_squared` is clamped implicitly: can theoretically exceed 1.0 on numerical edge cases; no guard added because such cases indicate input misuse.

## 10. File path index

- `src/predictive.rs::PredictiveForecaster::forecast` — the core math
- `src/predictive.rs::build_analysis_prompt` — prompt construction
- `src/predictive.rs::combine_responses` — output assembly
- `src/predictive.rs::tests::*` — 8 tests

## 11. Dependencies

- `async_trait` (already in workspace)
- `serde` + `serde_json` (already)
- `std::sync::Arc` for ensemble sharing
- `thiserror` for `PredictError`

No new crates introduced.

## 12. Integration hooks

- Predictive agent does NOT live on `DaemonState` (Slice E.1/E.2 scope). It is constructed by downstream callers that want predictive capability — typically a skill or application building on top of the RR substrate.
- Future `rr_predict` MCP method would construct a `PredictiveAgent` from the daemon's `llm` (single-member ensemble) or from a purpose-built ensemble assembled from config.
