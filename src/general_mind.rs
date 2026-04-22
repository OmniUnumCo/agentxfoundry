//! Prompt template library for General-rank cognitive frameworks.
//!
//! This module ports the prompt-generation half of the Python
//! `rr/general_mind.py` — the templates only, not the LLM call sites. Each
//! builder takes a small set of inputs and returns a `String` with a
//! byte-stable template expansion.
//!
//! Slice E.1 ports only `mission_decomposition_prompt` (the one actually used
//! by `MissionPlanner::plan_mission`). The recursive / reactive / predictive /
//! unified templates are retained as TODO-stubs for Slice E.2.
//!
//! # Determinism
//!
//! These builders are pure functions over their inputs. No timestamps, no
//! UUIDs, no HashMap iteration-order dependencies (inputs that would be
//! iterated are `BTreeMap`s or pre-sorted `Vec`s). A byte-stability test
//! enforces this invariant; see `tests::mission_decomposition_is_byte_stable`.

use std::collections::BTreeMap;

/// A deterministic, sorted key-value bag used where the Python original uses
/// `Dict[str, Any]`. Accepts any `Display`-able value; we render it with a
/// stable `k: v` per line format.
pub type PromptContext = BTreeMap<String, String>;

/// Render a `PromptContext` as a stable multi-line block. Empty contexts
/// render as a single line `(none)` so the template retains structure.
fn render_context(ctx: &PromptContext) -> String {
    if ctx.is_empty() {
        return "(none)".to_string();
    }
    let mut out = String::new();
    for (k, v) in ctx {
        out.push_str(&format!("- {}: {}\n", k, v));
    }
    out.trim_end().to_string()
}

/// Render a swarm composition bag. Same stable format as `render_context`.
fn render_composition(composition: &PromptContext) -> String {
    render_context(composition)
}

/// Build the mission-decomposition prompt used by `MissionPlanner::plan_mission`.
///
/// The template is a verbatim port of `general_mind.py::mission_decomposition_prompt`.
/// Output schema is JSON (the planner's parser is strict — invalid JSON fails
/// the planning step loudly, per the Slice E.1 policy of "no text fallback").
pub fn mission_decomposition_prompt(
    objective: &str,
    constraints: &PromptContext,
    swarm_composition: &PromptContext,
) -> String {
    format!(
        "You are a GENERAL in the Rational Reserve executing AUTONOMOUS MISSION PLANNING.\n\
        \n\
        MISSION OBJECTIVE: {objective}\n\
        \n\
        CONSTRAINTS:\n\
        {constraints}\n\
        \n\
        AVAILABLE SWARM COMPOSITION:\n\
        {composition}\n\
        \n\
        YOUR TASK: Decompose this mission into executable tasks for your swarm.\n\
        \n\
        APPLY ALL THREE COGNITIVE FRAMEWORKS:\n\
        \n\
        1. RECURSIVE THINKING:\n\
        \x20  - Break mission into hierarchical components\n\
        \x20  - Identify which tasks require iterative refinement\n\
        \x20  - Plan feedback loops and checkpoints\n\
        \n\
        2. REACTIVE READINESS:\n\
        \x20  - Identify potential pivot points\n\
        \x20  - Mark tasks where adaptation may be needed\n\
        \x20  - Designate contingency plans\n\
        \n\
        3. PREDICTIVE PLANNING:\n\
        \x20  - Predict task dependencies and sequencing\n\
        \x20  - Anticipate bottlenecks and blockers\n\
        \x20  - Position resources for future needs\n\
        \n\
        TASK DECOMPOSITION FORMAT:\n\
        \n\
        For each task, provide:\n\
        - task_id: Unique identifier (string)\n\
        - description: Clear task description\n\
        - assigned_mos: Which MOS should execute (e.g. \"12B\", \"35F\")\n\
        - assigned_rank: Minimum rank required (e.g. \"SGT\", \"LT\")\n\
        - dependencies: List of task_ids that must complete first (array of strings)\n\
        - is_iterative: Boolean — does this require refinement cycles?\n\
        - pivot_triggers: Conditions that would require task adaptation (array of strings)\n\
        - predicted_duration: Estimate based on trajectory (string)\n\
        - success_criteria: How to measure completion (string)\n\
        \n\
        OUTPUT:\n\
        Return a JSON object with a single top-level key \"tasks\" whose value is an\n\
        array of the tasks described above. Return ONLY valid JSON — no prose before\n\
        or after the JSON block, no markdown fences.\n\
        \n\
        THINK STRATEGICALLY. DECOMPOSE PRECISELY. PLAN FOR EVOLUTION.\n",
        objective = objective,
        constraints = render_context(constraints),
        composition = render_composition(swarm_composition),
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_constraints() -> PromptContext {
        let mut m = PromptContext::new();
        m.insert("deadline".into(), "2026-05-01".into());
        m.insert("budget_usd".into(), "1000".into());
        m
    }

    fn sample_composition() -> PromptContext {
        let mut m = PromptContext::new();
        m.insert("CPT-CombatEngineer".into(), "1".into());
        m.insert("SGT-Infantry".into(), "3".into());
        m
    }

    #[test]
    fn mission_decomposition_has_expected_structure() {
        let p = mission_decomposition_prompt(
            "Secure the token-vector layout invariants",
            &sample_constraints(),
            &sample_composition(),
        );
        assert!(p.contains("AUTONOMOUS MISSION PLANNING"));
        assert!(p.contains("MISSION OBJECTIVE: Secure the token-vector layout invariants"));
        assert!(p.contains("budget_usd: 1000"));
        assert!(p.contains("CPT-CombatEngineer: 1"));
        assert!(p.contains("Return ONLY valid JSON"));
    }

    #[test]
    fn mission_decomposition_is_byte_stable() {
        let objective = "Audit recursive loop closure invariants";
        let constraints = sample_constraints();
        let composition = sample_composition();
        let first = mission_decomposition_prompt(objective, &constraints, &composition);
        for _ in 0..1000 {
            let again =
                mission_decomposition_prompt(objective, &constraints, &composition);
            assert_eq!(first, again, "prompt must be byte-identical across calls");
        }
    }

    #[test]
    fn empty_context_renders_none() {
        assert_eq!(render_context(&PromptContext::new()), "(none)");
    }

    #[test]
    fn context_order_is_sorted_by_key() {
        let mut m = PromptContext::new();
        m.insert("zebra".into(), "1".into());
        m.insert("alpha".into(), "2".into());
        m.insert("mike".into(), "3".into());
        let rendered = render_context(&m);
        let alpha_pos = rendered.find("alpha").unwrap();
        let mike_pos = rendered.find("mike").unwrap();
        let zebra_pos = rendered.find("zebra").unwrap();
        assert!(alpha_pos < mike_pos);
        assert!(mike_pos < zebra_pos);
    }
}
