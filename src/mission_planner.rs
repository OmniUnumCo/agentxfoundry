//! Autonomous mission planning.
//!
//! Ports the `plan_mission` half of `rr/mission_planner.py`. Takes a
//! [`Mission`] + swarm composition, builds a decomposition prompt via
//! [`crate::general_mind`], sends it through an [`crate::traits::LLMClient`]
//! (typically wrapped by [`crate::llm::DeterministicLLMClient`]), and parses
//! the JSON response into a `Vec<Task>`.
//!
//! # Slice E.1 policy: fail loud on malformed JSON
//!
//! The Python original has a text-fallback parser and a "single fallback
//! task" last-resort branch. We deliberately skip both. A deterministic
//! planner must either produce a parseable plan or surface the failure to the
//! caller — silent degradation hides bugs.

use crate::general_mind::{mission_decomposition_prompt, PromptContext};
use crate::models::{Mission, StatusEnum, Task};
use crate::traits::LLMClient;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Error, Debug)]
pub enum PlanError {
    #[error("LLM call failed: {0}")]
    LlmFailure(BoxError),

    #[error("planner output was not valid JSON: {reason} (raw: {raw:?})")]
    MalformedJson { reason: String, raw: String },

    #[error("planner output JSON had no 'tasks' array")]
    MissingTasksField,

    #[error("planner returned zero tasks; a mission must decompose into >=1 task")]
    EmptyPlan,
}

/// DTO deserialized from the LLM response. Shape matches the schema
/// described in `general_mind::mission_decomposition_prompt`.
#[derive(Debug, Clone, Deserialize)]
pub struct TaskSpec {
    #[serde(default)]
    pub task_id: Option<String>,
    pub description: String,
    #[serde(default)]
    pub assigned_mos: Option<String>,
    #[serde(default)]
    pub assigned_rank: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub is_iterative: bool,
    #[serde(default)]
    pub pivot_triggers: Vec<String>,
    #[serde(default)]
    pub predicted_duration: Option<String>,
    #[serde(default)]
    pub success_criteria: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlanResponse {
    tasks: Vec<TaskSpec>,
}

/// Strip markdown code fences (```json ... ```) from an LLM response, if the
/// model insisted on wrapping its JSON in them. Robust to leading/trailing
/// whitespace. This is NOT a text-parsing fallback — the payload between the
/// fences still must be valid JSON.
fn unfence(raw: &str) -> &str {
    let trimmed = raw.trim();
    let without_prefix = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed)
        .trim_start();
    without_prefix.strip_suffix("```").unwrap_or(without_prefix).trim_end()
}

/// Convert a [`TaskSpec`] into a persistence-ready [`Task`].
fn to_task(spec: TaskSpec, mission_id: &str) -> Task {
    let mut metadata: HashMap<String, serde_json::Value> = HashMap::new();
    if let Some(mos) = &spec.assigned_mos {
        metadata.insert("assigned_mos".into(), serde_json::Value::String(mos.clone()));
    }
    if let Some(rank) = &spec.assigned_rank {
        metadata.insert("assigned_rank".into(), serde_json::Value::String(rank.clone()));
    }
    metadata.insert("is_iterative".into(), serde_json::Value::Bool(spec.is_iterative));
    if !spec.pivot_triggers.is_empty() {
        metadata.insert(
            "pivot_triggers".into(),
            serde_json::to_value(&spec.pivot_triggers).unwrap_or_default(),
        );
    }
    if let Some(duration) = &spec.predicted_duration {
        metadata.insert(
            "predicted_duration".into(),
            serde_json::Value::String(duration.clone()),
        );
    }
    if let Some(criteria) = &spec.success_criteria {
        metadata.insert(
            "success_criteria".into(),
            serde_json::Value::String(criteria.clone()),
        );
    }
    metadata.insert(
        "mission_id".into(),
        serde_json::Value::String(mission_id.to_string()),
    );
    if let Some(id) = spec.task_id {
        metadata.insert("planner_task_id".into(), serde_json::Value::String(id));
    }

    Task {
        id: uuid::Uuid::new_v4().to_string(),
        description: spec.description,
        assigned_to: None,
        status: StatusEnum::Standby,
        dependencies: spec.dependencies,
        result: None,
        error: None,
        created_at: chrono::Utc::now(),
        completed_at: None,
        metadata,
    }
}

/// `MissionPlanner` ports `rr/mission_planner.py::plan_mission`.
pub struct MissionPlanner {
    llm: Arc<dyn LLMClient>,
}

impl MissionPlanner {
    pub fn new(llm: Arc<dyn LLMClient>) -> Self {
        Self { llm }
    }

    /// Decompose a mission into a task DAG using the General-Mind framework.
    ///
    /// `swarm_composition` is a caller-supplied summary of who is available
    /// (e.g. `{"CPT-CombatEngineer": "1", "SGT-Infantry": "3"}`). The prompt
    /// rendering is order-stable via `PromptContext` (BTreeMap) so this is
    /// byte-deterministic for fixed inputs.
    pub async fn plan_mission(
        &self,
        mission: &Mission,
        swarm_composition: PromptContext,
    ) -> Result<Vec<Task>, PlanError> {
        // The mission carries its own free-form constraints bag. Render
        // deterministically via a sorted view.
        let mut constraints = PromptContext::new();
        for (k, v) in &mission.constraints {
            // Values are `serde_json::Value` — render as compact JSON so the
            // prompt is stable regardless of ordering inside the Value.
            constraints.insert(k.clone(), v.to_string());
        }

        let prompt =
            mission_decomposition_prompt(&mission.objective, &constraints, &swarm_composition);

        // Route through the LLM client. We intentionally use the
        // "strategic" tier — mission decomposition is the domain of Generals.
        let response = self
            .llm
            .complete(&prompt, "strategic", Some(4096), Some(0.0))
            .await
            .map_err(PlanError::LlmFailure)?;

        Self::parse_plan(&response, &mission.id)
    }

    /// Parse the LLM response into `Vec<Task>`. Public for tests; callers
    /// should prefer `plan_mission`.
    pub fn parse_plan(response: &str, mission_id: &str) -> Result<Vec<Task>, PlanError> {
        let cleaned = unfence(response);
        let plan: PlanResponse =
            serde_json::from_str(cleaned).map_err(|e| PlanError::MalformedJson {
                reason: e.to_string(),
                raw: response.chars().take(500).collect(),
            })?;
        if plan.tasks.is_empty() {
            return Err(PlanError::EmptyPlan);
        }
        Ok(plan.tasks.into_iter().map(|t| to_task(t, mission_id)).collect())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PriorityEnum;
    use async_trait::async_trait;

    struct FakeLLM {
        response: String,
    }

    #[async_trait]
    impl LLMClient for FakeLLM {
        async fn complete(
            &self,
            _prompt: &str,
            _tier: &str,
            _max_tokens: Option<usize>,
            _temperature: Option<f64>,
        ) -> Result<String, BoxError> {
            Ok(self.response.clone())
        }
    }

    fn sample_mission() -> Mission {
        let mut m = Mission::default();
        m.objective = "Audit the token-vector layout".into();
        m.priority = PriorityEnum::Priority;
        m.constraints.insert(
            "deadline".into(),
            serde_json::Value::String("2026-05-01".into()),
        );
        m
    }

    #[tokio::test]
    async fn plans_happy_path() {
        let fake = Arc::new(FakeLLM {
            response: r#"{"tasks":[
                {"task_id":"t1","description":"Catalogue layout fields","assigned_mos":"35F","assigned_rank":"SGT","dependencies":[],"is_iterative":false,"pivot_triggers":[],"predicted_duration":"2h","success_criteria":"catalogue md committed"},
                {"task_id":"t2","description":"Propose invariants","assigned_mos":"18B","assigned_rank":"CPT","dependencies":["t1"],"is_iterative":true,"pivot_triggers":["spec-drift"],"predicted_duration":"1d","success_criteria":"invariant doc reviewed"}
            ]}"#.to_string(),
        });
        let planner = MissionPlanner::new(fake);
        let mission = sample_mission();
        let tasks = planner.plan_mission(&mission, PromptContext::new()).await.unwrap();

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].description, "Catalogue layout fields");
        assert_eq!(tasks[1].dependencies, vec!["t1".to_string()]);
        assert_eq!(
            tasks[1]
                .metadata
                .get("assigned_rank")
                .and_then(|v| v.as_str()),
            Some("CPT")
        );
        assert_eq!(
            tasks[0]
                .metadata
                .get("mission_id")
                .and_then(|v| v.as_str()),
            Some(mission.id.as_str())
        );
    }

    #[tokio::test]
    async fn plan_strips_markdown_fences() {
        let fake = Arc::new(FakeLLM {
            response: "```json\n{\"tasks\":[{\"description\":\"do a thing\"}]}\n```".to_string(),
        });
        let planner = MissionPlanner::new(fake);
        let tasks = planner
            .plan_mission(&sample_mission(), PromptContext::new())
            .await
            .unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].description, "do a thing");
    }

    #[tokio::test]
    async fn plan_fails_loud_on_malformed_json() {
        let fake = Arc::new(FakeLLM {
            response: "here are the tasks: first, do X. second, do Y.".to_string(),
        });
        let planner = MissionPlanner::new(fake);
        let err = planner
            .plan_mission(&sample_mission(), PromptContext::new())
            .await
            .unwrap_err();
        assert!(matches!(err, PlanError::MalformedJson { .. }));
    }

    #[tokio::test]
    async fn plan_fails_loud_on_empty_tasks_array() {
        let fake = Arc::new(FakeLLM {
            response: r#"{"tasks":[]}"#.to_string(),
        });
        let planner = MissionPlanner::new(fake);
        let err = planner
            .plan_mission(&sample_mission(), PromptContext::new())
            .await
            .unwrap_err();
        assert!(matches!(err, PlanError::EmptyPlan));
    }

    #[tokio::test]
    async fn plan_handles_missing_optional_fields() {
        // Minimal task with only `description` — dependencies, metadata, etc.
        // default cleanly.
        let fake = Arc::new(FakeLLM {
            response: r#"{"tasks":[{"description":"do the minimum"}]}"#.to_string(),
        });
        let planner = MissionPlanner::new(fake);
        let tasks = planner
            .plan_mission(&sample_mission(), PromptContext::new())
            .await
            .unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].dependencies.is_empty());
        assert_eq!(tasks[0].description, "do the minimum");
    }
}
