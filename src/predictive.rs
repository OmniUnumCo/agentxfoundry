//! Predictive analyst agent.
//!
//! Ports `agents/predictive/predictive_agent.py` to pure-Rust deterministic
//! code:
//!
//! - The PyTorch LSTM is replaced with a **closed-form least-squares linear
//!   regression** over the input time series. Forecasts are reproducible to
//!   machine precision: identical input samples always yield identical
//!   forecasts. No weights, no training, no GPU, no floats-in-floats-out
//!   drift.
//! - The 3-provider LLM ensemble is preserved as a generic
//!   `Vec<Arc<dyn LLMClient>>`; analysis concatenates their responses in a
//!   stable order (the order the caller supplied). Wrap each client in
//!   [`crate::llm::DeterministicLLMClient`] for full determinism including
//!   cross-provider replay.
//!
//! Users who need real LSTM inference can layer an ONNX Runtime driver in
//! E.3 — for now, the linear baseline is the deterministic path.

use crate::traits::LLMClient;
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Error, Debug)]
pub enum PredictError {
    #[error("predictive agent needs at least 2 sample points to fit a trend, got {0}")]
    InsufficientData(usize),

    #[error("forecast horizon must be >= 1")]
    ZeroHorizon,

    #[error("LLM analysis failed: {0}")]
    LlmFailure(BoxError),
}

/// Forecast output.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct Forecast {
    /// Predicted values for the next `horizon` steps, in order.
    pub predictions: Vec<f64>,
    /// Slope of the fitted line.
    pub slope: f64,
    /// Intercept of the fitted line.
    pub intercept: f64,
    /// Coefficient of determination (R^2) of the fit on the input series.
    pub r_squared: f64,
}

/// Full predictive analysis combining a numeric forecast + LLM ensemble text
/// commentary.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PredictiveAnalysis {
    pub forecast: Forecast,
    /// Per-provider analysis strings, in the order the ensemble was supplied.
    pub llm_responses: Vec<LlmAnalysis>,
    /// Concatenated summary. Order-stable.
    pub combined: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LlmAnalysis {
    pub provider_label: String,
    pub response: String,
}

// ============================================================================
// Deterministic forecaster
// ============================================================================

pub struct PredictiveForecaster;

impl PredictiveForecaster {
    /// Fit `y = slope * x + intercept` by ordinary least squares over the
    /// input series and project `horizon` points past the last sample.
    ///
    /// `x` values are implicit 0..n. For n=2 with identical values, slope=0
    /// and the forecast is a constant line.
    pub fn forecast(values: &[f64], horizon: usize) -> Result<Forecast, PredictError> {
        if values.len() < 2 {
            return Err(PredictError::InsufficientData(values.len()));
        }
        if horizon == 0 {
            return Err(PredictError::ZeroHorizon);
        }
        let n = values.len() as f64;
        let xs: Vec<f64> = (0..values.len()).map(|i| i as f64).collect();
        let mean_x = xs.iter().sum::<f64>() / n;
        let mean_y = values.iter().sum::<f64>() / n;

        let mut cov = 0.0f64;
        let mut var_x = 0.0f64;
        for i in 0..values.len() {
            cov += (xs[i] - mean_x) * (values[i] - mean_y);
            var_x += (xs[i] - mean_x).powi(2);
        }
        let slope = if var_x == 0.0 { 0.0 } else { cov / var_x };
        let intercept = mean_y - slope * mean_x;

        // R^2
        let mut ss_res = 0.0f64;
        let mut ss_tot = 0.0f64;
        for i in 0..values.len() {
            let yhat = slope * xs[i] + intercept;
            ss_res += (values[i] - yhat).powi(2);
            ss_tot += (values[i] - mean_y).powi(2);
        }
        let r_squared = if ss_tot == 0.0 { 1.0 } else { 1.0 - ss_res / ss_tot };

        let predictions: Vec<f64> = (0..horizon)
            .map(|h| {
                let x = values.len() as f64 + h as f64;
                slope * x + intercept
            })
            .collect();

        Ok(Forecast {
            predictions,
            slope,
            intercept,
            r_squared,
        })
    }
}

// ============================================================================
// Predictive agent (forecast + LLM ensemble)
// ============================================================================

/// A labelled LLM provider for the ensemble. The label appears in the output
/// so callers can tell which provider returned which analysis.
pub struct LabelledClient {
    pub label: String,
    pub client: Arc<dyn LLMClient>,
}

impl LabelledClient {
    pub fn new(label: impl Into<String>, client: Arc<dyn LLMClient>) -> Self {
        Self {
            label: label.into(),
            client: client.clone(),
        }
    }
}

pub struct PredictiveAgent {
    ensemble: Vec<LabelledClient>,
    analysis_tier: String,
}

impl PredictiveAgent {
    pub fn new(ensemble: Vec<LabelledClient>) -> Self {
        Self {
            ensemble,
            analysis_tier: "supervision".to_string(),
        }
    }

    pub fn with_analysis_tier(mut self, tier: impl Into<String>) -> Self {
        self.analysis_tier = tier.into();
        self
    }
}

#[async_trait]
pub trait Predictor: Send + Sync {
    async fn analyze(
        &self,
        values: &[f64],
        horizon: usize,
    ) -> Result<PredictiveAnalysis, PredictError>;
}

#[async_trait]
impl Predictor for PredictiveAgent {
    async fn analyze(
        &self,
        values: &[f64],
        horizon: usize,
    ) -> Result<PredictiveAnalysis, PredictError> {
        let forecast = PredictiveForecaster::forecast(values, horizon)?;

        // Build a stable prompt containing the numeric prediction. Text is
        // deterministic for fixed inputs because we format with fixed
        // precision.
        let prompt = build_analysis_prompt(values, &forecast);

        let mut responses = Vec::with_capacity(self.ensemble.len());
        for member in &self.ensemble {
            let resp = member
                .client
                .complete(&prompt, &self.analysis_tier, Some(512), Some(0.0))
                .await
                .map_err(PredictError::LlmFailure)?;
            responses.push(LlmAnalysis {
                provider_label: member.label.clone(),
                response: resp,
            });
        }

        let combined = combine_responses(&responses);
        Ok(PredictiveAnalysis {
            forecast,
            llm_responses: responses,
            combined,
        })
    }
}

fn build_analysis_prompt(values: &[f64], forecast: &Forecast) -> String {
    let mut sample = String::new();
    for (i, v) in values.iter().enumerate() {
        sample.push_str(&format!("- t{}: {:.6}\n", i, v));
    }
    let mut pred = String::new();
    for (i, v) in forecast.predictions.iter().enumerate() {
        pred.push_str(&format!(
            "- t+{}: {:.6}\n",
            values.len() + i,
            v
        ));
    }
    format!(
        "You are analyzing a numeric time series and a deterministic linear forecast.\n\
        \n\
        INPUT SERIES ({n} points):\n\
        {sample}\n\
        FITTED MODEL: y = {slope:.6} * x + {intercept:.6}  (R^2 = {r2:.6})\n\
        \n\
        FORECAST ({horizon} future points):\n\
        {pred}\n\
        Describe the trend, any obvious seasonality or level changes in the\n\
        input, and whether the linear projection is likely to hold. Keep the\n\
        response under 120 words.\n",
        n = values.len(),
        sample = sample.trim_end(),
        slope = forecast.slope,
        intercept = forecast.intercept,
        r2 = forecast.r_squared,
        horizon = forecast.predictions.len(),
        pred = pred.trim_end(),
    )
}

fn combine_responses(responses: &[LlmAnalysis]) -> String {
    let mut out = String::new();
    for r in responses {
        out.push_str(&format!("### {}\n{}\n\n", r.provider_label, r.response));
    }
    out.trim_end().to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn forecast_on_perfect_line_is_exact() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let f = PredictiveForecaster::forecast(&values, 3).unwrap();
        assert!(approx_eq(f.slope, 1.0));
        assert!(approx_eq(f.intercept, 1.0));
        assert!(approx_eq(f.r_squared, 1.0));
        assert!(approx_eq(f.predictions[0], 6.0));
        assert!(approx_eq(f.predictions[1], 7.0));
        assert!(approx_eq(f.predictions[2], 8.0));
    }

    #[test]
    fn forecast_on_flat_line_has_zero_slope() {
        let values = vec![3.0, 3.0, 3.0, 3.0];
        let f = PredictiveForecaster::forecast(&values, 2).unwrap();
        assert!(approx_eq(f.slope, 0.0));
        assert!(approx_eq(f.intercept, 3.0));
        assert_eq!(f.predictions, vec![3.0, 3.0]);
    }

    #[test]
    fn forecast_is_deterministic() {
        let values = vec![1.7, 2.3, 3.1, 3.9, 4.6, 5.4];
        let f1 = PredictiveForecaster::forecast(&values, 5).unwrap();
        for _ in 0..100 {
            let fi = PredictiveForecaster::forecast(&values, 5).unwrap();
            assert_eq!(f1, fi);
        }
    }

    #[test]
    fn forecast_rejects_too_few_samples() {
        assert!(matches!(
            PredictiveForecaster::forecast(&[1.0], 3),
            Err(PredictError::InsufficientData(1))
        ));
    }

    #[test]
    fn forecast_rejects_zero_horizon() {
        assert!(matches!(
            PredictiveForecaster::forecast(&[1.0, 2.0], 0),
            Err(PredictError::ZeroHorizon)
        ));
    }

    struct EchoLLM {
        label: String,
    }

    #[async_trait]
    impl LLMClient for EchoLLM {
        async fn complete(
            &self,
            _prompt: &str,
            _tier: &str,
            _max_tokens: Option<usize>,
            _temperature: Option<f64>,
        ) -> Result<String, BoxError> {
            Ok(format!("analysis from {}", self.label))
        }
    }

    #[tokio::test]
    async fn agent_combines_forecast_with_ensemble_responses() {
        let agent = PredictiveAgent::new(vec![
            LabelledClient::new(
                "openai",
                Arc::new(EchoLLM { label: "openai".into() }),
            ),
            LabelledClient::new(
                "anthropic",
                Arc::new(EchoLLM { label: "anthropic".into() }),
            ),
            LabelledClient::new(
                "google",
                Arc::new(EchoLLM { label: "google".into() }),
            ),
        ]);
        let result = agent.analyze(&[1.0, 2.0, 3.0, 4.0], 2).await.unwrap();
        assert_eq!(result.llm_responses.len(), 3);
        assert_eq!(result.llm_responses[0].provider_label, "openai");
        assert_eq!(result.llm_responses[1].provider_label, "anthropic");
        assert_eq!(result.llm_responses[2].provider_label, "google");
        assert!(result.combined.contains("### openai"));
        assert!(result.combined.contains("### anthropic"));
        assert!(result.combined.contains("### google"));
        assert!(approx_eq(result.forecast.slope, 1.0));
    }

    #[tokio::test]
    async fn agent_preserves_ensemble_order() {
        let agent = PredictiveAgent::new(vec![
            LabelledClient::new("third", Arc::new(EchoLLM { label: "third".into() })),
            LabelledClient::new("first", Arc::new(EchoLLM { label: "first".into() })),
            LabelledClient::new("second", Arc::new(EchoLLM { label: "second".into() })),
        ]);
        let result = agent.analyze(&[1.0, 2.0], 1).await.unwrap();
        let labels: Vec<&str> = result
            .llm_responses
            .iter()
            .map(|r| r.provider_label.as_str())
            .collect();
        assert_eq!(labels, vec!["third", "first", "second"]);
    }
}
