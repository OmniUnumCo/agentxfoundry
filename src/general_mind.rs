//! Prompt template library for General-rank cognitive frameworks.
//!
//! Full port of `rr/general_mind.py`. Each builder takes a small set of
//! inputs and returns a `String` with a byte-stable template expansion.
//! Five prompt modes are exposed:
//!
//! - [`recursive_thinking_prompt`] — iterative cycle analysis
//! - [`reactive_complex_prompt`] — PARRY/LUNGE/DISENGAGE/HOLD pivot framework
//! - [`predictive_engine_prompt`] — T+1/T+2/T+3 trajectory projection
//! - [`unified_strategic_prompt`] — combines all three, selectable by mode
//! - [`mission_decomposition_prompt`] — autonomous mission planning (used by
//!   [`crate::mission_planner::MissionPlanner::plan_mission`])
//!
//! Plus [`apply_general_mind`] — the top-level dispatcher that returns a
//! unified prompt only if the caller's rank is `GEN`.
//!
//! # Determinism
//!
//! These builders are pure functions over their inputs. No timestamps, no
//! UUIDs, no HashMap iteration-order dependencies (inputs that would be
//! iterated are `BTreeMap`s or pre-sorted `Vec`s). Byte-stability tests
//! enforce this invariant across all prompts.

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

/// Operational mode for [`unified_strategic_prompt`] / [`apply_general_mind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneralMindMode {
    Full,
    Recursive,
    Reactive,
    Predictive,
}

impl GeneralMindMode {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "full" => GeneralMindMode::Full,
            "recursive" => GeneralMindMode::Recursive,
            "reactive" => GeneralMindMode::Reactive,
            "predictive" => GeneralMindMode::Predictive,
            _ => return None,
        })
    }
}

/// Recursive/iterative thinking prompt.
///
/// Encodes circular reasoning: analyze -> decompose -> plan -> predict
/// iterations -> execute recursively. Inputs render order-stably.
pub fn recursive_thinking_prompt(objective: &str, context: &PromptContext) -> String {
    format!(
        "You are a GENERAL in the Rational Reserve. Your mission: {objective}\n\
        \n\
        RECURSIVE THINKING PROTOCOL:\n\
        \n\
        Everything in strategy is circular. You must think in cycles:\n\
        \n\
        CYCLE 1 - ANALYZE:\n\
        - What is the current state?\n\
        - What are the constraints and resources?\n\
        - What patterns exist in this problem domain?\n\
        - What similar problems have been solved before?\n\
        \n\
        CYCLE 2 - DECOMPOSE:\n\
        - Break the objective into fundamental components\n\
        - Identify dependencies and sequences\n\
        - Recognize which components are themselves circular processes\n\
        - Map the problem space recursively\n\
        \n\
        CYCLE 3 - PLAN:\n\
        - Design the approach with iteration in mind\n\
        - Plan for refinement cycles, not single-pass solutions\n\
        - Build in feedback loops and checkpoints\n\
        - Anticipate where pivots will be needed\n\
        \n\
        CYCLE 4 - PREDICT ITERATIONS:\n\
        - How will this solution evolve?\n\
        - What refinements will be necessary?\n\
        - Where will resistance or blockers appear?\n\
        - How many cycles to convergence?\n\
        \n\
        EXECUTE RECURSIVELY:\n\
        Each task you create should itself contain the seeds of its own refinement.\n\
        Every plan should anticipate its own evolution.\n\
        Every action should create data for the next cycle.\n\
        \n\
        Context:\n\
        {context}\n\
        \n\
        Begin your recursive analysis. Structure your response as cycles of deepening understanding.\n",
        objective = objective,
        context = render_context(context),
    )
}

/// Reactive complex prompt — PARRY/LUNGE/DISENGAGE/HOLD pivot framework.
///
/// `change_detected` is optional; if `None`, the prompt is a general reactive
/// readiness brief. If `Some`, the specific change is named at the top.
pub fn reactive_complex_prompt(
    objective: &str,
    current_state: &PromptContext,
    change_detected: Option<&str>,
) -> String {
    let change_context = match change_detected {
        Some(c) => format!("\nCHANGE DETECTED: {}\n", c),
        None => String::new(),
    };

    format!(
        "You are a GENERAL in the Rational Reserve. Your mission: {objective}\n\
        \n\
        REACTIVE COMPLEX PROTOCOL:\n\
        \n\
        You must be able to PIVOT AT A MOMENT'S NOTICE. Like a fencer, you parry and lunge.\n\
        {change_context}\n\
        CURRENT STATE:\n\
        {state}\n\
        \n\
        REACTIVE ANALYSIS:\n\
        1. SENSE - What has changed?\n\
        \x20  - Environmental shifts\n\
        \x20  - Resource fluctuations\n\
        \x20  - Blocker emergence\n\
        \x20  - Opportunity windows\n\
        \n\
        2. EVALUATE - Does this require pivot?\n\
        \x20  - Severity: How critical is this change?\n\
        \x20  - Timing: How immediate is the required response?\n\
        \x20  - Impact: What cascades from this change?\n\
        \x20  - Options: What alternative paths exist?\n\
        \n\
        3. DECIDE - Parry or Lunge?\n\
        \x20  - PARRY (Defensive): Adapt current plan to absorb change\n\
        \x20  - LUNGE (Offensive): Exploit change as opportunity\n\
        \x20  - DISENGAGE: Abandon approach entirely, redesign\n\
        \x20  - HOLD: Change is noise, maintain course\n\
        \n\
        4. ACT - Execute pivot with precision\n\
        \x20  - Update mission decomposition\n\
        \x20  - Reassign resources\n\
        \x20  - Communicate FRAGO (fragmentary order) to swarm\n\
        \x20  - Establish new checkpoints\n\
        \n\
        PIVOT DECISION MATRIX:\n\
        - If blockers detected -> PARRY (find alternate path)\n\
        - If opportunity detected -> LUNGE (exploit advantage)\n\
        - If assumptions invalidated -> DISENGAGE (redesign)\n\
        - If minor fluctuation -> HOLD (maintain course)\n\
        \n\
        Your response must include:\n\
        1. Change analysis\n\
        2. Pivot decision (PARRY/LUNGE/DISENGAGE/HOLD)\n\
        3. Immediate actions\n\
        4. Updated plan\n\
        \n\
        REACT NOW. Speed is survival.\n",
        objective = objective,
        change_context = change_context,
        state = render_context(current_state),
    )
}

/// Predictive engine prompt — T+1/T+2/T+3 trajectory projection and
/// "telegraph the move" framework.
pub fn predictive_engine_prompt(
    objective: &str,
    current_trajectory: &PromptContext,
    horizon_depth: u8,
) -> String {
    format!(
        "You are a GENERAL in the Rational Reserve. Your mission: {objective}\n\
        \n\
        PREDICTIVE ENGINE PROTOCOL:\n\
        \n\
        You must SEE BEYOND THE EVENT HORIZON. Predict future states with such precision that\n\
        your actions appear to quantum leap ahead of reality.\n\
        \n\
        CURRENT TRAJECTORY:\n\
        {trajectory}\n\
        \n\
        PREDICTION DEPTH: {horizon} moves ahead\n\
        \n\
        PREDICTIVE ANALYSIS FRAMEWORK:\n\
        \n\
        LAYER 1 - CURRENT STATE VECTOR:\n\
        - Position: Where are we now?\n\
        - Velocity: How fast are we progressing?\n\
        - Acceleration: Is progress speeding up or slowing down?\n\
        - Momentum: What forces drive current movement?\n\
        \n\
        LAYER 2 - TRAJECTORY PROJECTION:\n\
        Predict the next {horizon} states if current trajectory continues:\n\
        \n\
        STATE T+1 (Next immediate state):\n\
        - What happens if we maintain current course?\n\
        - What obstacles emerge?\n\
        - What resources deplete?\n\
        - What opportunities appear?\n\
        \n\
        STATE T+2 (Secondary consequence):\n\
        - What cascades from T+1?\n\
        - What second-order effects manifest?\n\
        - What feedback loops activate?\n\
        \n\
        STATE T+3 (Tertiary horizon):\n\
        - What emerges at the edge of predictability?\n\
        - What patterns become clear only from this distance?\n\
        - What strategic positioning is required now for success then?\n\
        \n\
        LAYER 3 - OPTIMAL PATH CALCULATION:\n\
        - Given predicted future states, what is the optimal current action?\n\
        - What move now sets up the best position at T+{horizon}?\n\
        - What appears to be a quantum leap but is actually precise prediction?\n\
        \n\
        LAYER 4 - TELEGRAPH THE MOVE:\n\
        Communicate your next action with such clarity that it seems prescient:\n\
        - State what you predict will happen\n\
        - State what action you're taking to position for it\n\
        - State why this appears to leap ahead but is actually calculated foresight\n\
        \n\
        PREDICTIVE OUTPUT FORMAT:\n\
        1. Current state vector analysis\n\
        2. Trajectory predictions for T+1, T+2, T+3\n\
        3. Optimal immediate action based on future positioning\n\
        4. Telegraphed next move with precision rationale\n\
        \n\
        Your prediction must be so precise it appears you've already seen the future.\n\
        This is not luck. This is CALCULATED FORESIGHT.\n",
        objective = objective,
        trajectory = render_context(current_trajectory),
        horizon = horizon_depth,
    )
}

/// Unified strategic prompt combining recursive / reactive / predictive
/// frameworks. Mode selects the active framework; `Full` returns an integrated
/// brief that walks through all three.
pub fn unified_strategic_prompt(
    objective: &str,
    context: &PromptContext,
    mode: GeneralMindMode,
) -> String {
    match mode {
        GeneralMindMode::Recursive => recursive_thinking_prompt(objective, context),
        GeneralMindMode::Reactive => reactive_complex_prompt(objective, context, None),
        GeneralMindMode::Predictive => predictive_engine_prompt(objective, context, 3),
        GeneralMindMode::Full => format!(
            "You are a GENERAL in the Rational Reserve. Your mission: {objective}\n\
            \n\
            You operate with THREE INTEGRATED COGNITIVE FRAMEWORKS:\n\
            \n\
            ===============================================================\n\
            FRAMEWORK 1: RECURSIVE/ITERATIVE THINKING\n\
            ===============================================================\n\
            \n\
            Everything in life is circular. Strategy is iterative refinement.\n\
            \n\
            THINK IN CYCLES:\n\
            1. Analyze current state\n\
            2. Decompose into components\n\
            3. Plan with iteration in mind\n\
            4. Predict refinement needs\n\
            5. Execute and gather feedback\n\
            6. Return to step 1 (REPEAT)\n\
            \n\
            Your plans must contain the seeds of their own evolution.\n\
            \n\
            ===============================================================\n\
            FRAMEWORK 2: REACTIVE COMPLEX (Pivot Capability)\n\
            ===============================================================\n\
            \n\
            You must pivot at a moment's notice. Parry and lunge.\n\
            \n\
            CONSTANT MONITORING:\n\
            - Detect changes in environment, resources, blockers, opportunities\n\
            - Evaluate: Does this require pivot?\n\
            - Decide: PARRY (defend), LUNGE (exploit), DISENGAGE (redesign), or HOLD (maintain)\n\
            - Act: Execute pivot with precision\n\
            \n\
            Speed is survival. Hesitation is defeat.\n\
            \n\
            ===============================================================\n\
            FRAMEWORK 3: PREDICTIVE ENGINE (Event Horizon Analysis)\n\
            ===============================================================\n\
            \n\
            See beyond the event horizon. Telegraph moves with quantum precision.\n\
            \n\
            PREDICT FUTURE STATES:\n\
            - Calculate current trajectory vector\n\
            - Project T+1, T+2, T+3 states\n\
            - Identify optimal positioning\n\
            - Take action NOW that sets up future success\n\
            - Telegraph your move with prescient clarity\n\
            \n\
            Your foresight must be so precise it appears you've seen the future.\n\
            \n\
            ===============================================================\n\
            UNIFIED STRATEGIC EXECUTION\n\
            ===============================================================\n\
            \n\
            CONTEXT:\n\
            {context}\n\
            \n\
            YOUR STRATEGIC RESPONSE MUST INCLUDE:\n\
            \n\
            1. RECURSIVE ANALYSIS:\n\
            \x20  - Problem decomposition\n\
            \x20  - Iterative refinement plan\n\
            \x20  - Feedback loop design\n\
            \n\
            2. REACTIVE ASSESSMENT:\n\
            \x20  - Change detection\n\
            \x20  - Pivot decision (if needed)\n\
            \x20  - Adaptive action plan\n\
            \n\
            3. PREDICTIVE CALCULATION:\n\
            \x20  - Trajectory projection (T+1, T+2, T+3)\n\
            \x20  - Optimal positioning\n\
            \x20  - Telegraphed next move\n\
            \n\
            4. INTEGRATED ACTION PLAN:\n\
            \x20  - Immediate actions\n\
            \x20  - Iterative checkpoints\n\
            \x20  - Pivot triggers\n\
            \x20  - Success metrics\n\
            \n\
            Think like a master strategist who sees the battlefield as a living, breathing,\n\
            evolving system. Be circular. Be reactive. Be prescient.\n\
            \n\
            BEGIN YOUR STRATEGIC ANALYSIS.\n",
            objective = objective,
            context = render_context(context),
        ),
    }
}

/// Top-level dispatcher. Mirrors Python's `apply_general_mind`:
/// returns a unified strategic prompt only if `agent_rank == "GEN"`, else
/// `None`. Callers that want cognitive scaffolding for any rank should call
/// [`unified_strategic_prompt`] directly.
pub fn apply_general_mind(
    agent_rank: &str,
    objective: &str,
    context: &PromptContext,
    mode: GeneralMindMode,
) -> Option<String> {
    if agent_rank != "GEN" {
        return None;
    }
    Some(unified_strategic_prompt(objective, context, mode))
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

    #[test]
    fn recursive_prompt_has_four_cycles() {
        let p = recursive_thinking_prompt("Test mission", &PromptContext::new());
        assert!(p.contains("CYCLE 1 - ANALYZE"));
        assert!(p.contains("CYCLE 2 - DECOMPOSE"));
        assert!(p.contains("CYCLE 3 - PLAN"));
        assert!(p.contains("CYCLE 4 - PREDICT ITERATIONS"));
        assert!(p.contains("Test mission"));
    }

    #[test]
    fn recursive_prompt_is_byte_stable() {
        let ctx = sample_constraints();
        let first = recursive_thinking_prompt("audit", &ctx);
        for _ in 0..500 {
            assert_eq!(first, recursive_thinking_prompt("audit", &ctx));
        }
    }

    #[test]
    fn reactive_prompt_includes_change_when_supplied() {
        let p = reactive_complex_prompt(
            "Hold the line",
            &PromptContext::new(),
            Some("supply line severed"),
        );
        assert!(p.contains("CHANGE DETECTED: supply line severed"));
        assert!(p.contains("PARRY/LUNGE/DISENGAGE/HOLD"));
    }

    #[test]
    fn reactive_prompt_omits_change_block_when_none() {
        let p = reactive_complex_prompt("Hold the line", &PromptContext::new(), None);
        assert!(!p.contains("CHANGE DETECTED"));
    }

    #[test]
    fn predictive_prompt_threads_horizon_depth() {
        let p5 = predictive_engine_prompt("observe", &PromptContext::new(), 5);
        assert!(p5.contains("PREDICTION DEPTH: 5 moves ahead"));
        // The template also hardcodes T+1/T+2/T+3 labels regardless of depth;
        // confirm those are present as an invariant of the wording.
        assert!(p5.contains("STATE T+1"));
        assert!(p5.contains("STATE T+2"));
        assert!(p5.contains("STATE T+3"));
    }

    #[test]
    fn unified_mode_includes_all_three_framework_markers() {
        let p = unified_strategic_prompt(
            "the mission",
            &PromptContext::new(),
            GeneralMindMode::Full,
        );
        assert!(p.contains("FRAMEWORK 1: RECURSIVE/ITERATIVE THINKING"));
        assert!(p.contains("FRAMEWORK 2: REACTIVE COMPLEX"));
        assert!(p.contains("FRAMEWORK 3: PREDICTIVE ENGINE"));
        assert!(p.contains("UNIFIED STRATEGIC EXECUTION"));
    }

    #[test]
    fn unified_mode_dispatch_matches_standalone_templates() {
        let ctx = sample_constraints();
        assert_eq!(
            unified_strategic_prompt("m", &ctx, GeneralMindMode::Recursive),
            recursive_thinking_prompt("m", &ctx)
        );
        assert_eq!(
            unified_strategic_prompt("m", &ctx, GeneralMindMode::Reactive),
            reactive_complex_prompt("m", &ctx, None)
        );
        assert_eq!(
            unified_strategic_prompt("m", &ctx, GeneralMindMode::Predictive),
            predictive_engine_prompt("m", &ctx, 3)
        );
    }

    #[test]
    fn apply_general_mind_returns_none_for_non_general_ranks() {
        let ctx = PromptContext::new();
        assert!(apply_general_mind("CPT", "m", &ctx, GeneralMindMode::Full).is_none());
        assert!(apply_general_mind("SPC", "m", &ctx, GeneralMindMode::Full).is_none());
        assert!(apply_general_mind("GEN", "m", &ctx, GeneralMindMode::Full).is_some());
    }

    #[test]
    fn mode_parse_round_trips() {
        assert_eq!(GeneralMindMode::parse("full"), Some(GeneralMindMode::Full));
        assert_eq!(
            GeneralMindMode::parse("recursive"),
            Some(GeneralMindMode::Recursive)
        );
        assert_eq!(
            GeneralMindMode::parse("reactive"),
            Some(GeneralMindMode::Reactive)
        );
        assert_eq!(
            GeneralMindMode::parse("predictive"),
            Some(GeneralMindMode::Predictive)
        );
        assert_eq!(GeneralMindMode::parse("bogus"), None);
    }
}
