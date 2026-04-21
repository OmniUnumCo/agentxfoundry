# AAR -- After-Action Review

An AAR is generated when a swarm is disbanded. It captures what happened so that future missions benefit from the lessons of this one. The AAR is the only required deliverable of a completed swarm (besides the mission result itself).

## Required fields
- `mission_id`, `swarm_id` -- the mission and swarm under review
- `success` -- boolean outcome
- `objectives_met`, `objectives_missed` -- lists of specific objectives
- `lessons_learned` -- free-form insights
- `agent_performance` -- per-agent performance metrics (tasks completed, CASREPs raised)
- `resource_usage` -- LLM tokens, API calls, wall clock time
- `timeline` -- ordered list of significant events
- `recommendations` -- specific changes for future missions

## Structure
1. **What we were trying to do.** Restate the mission objective in one sentence.
2. **What actually happened.** Timeline of significant events, including surprises.
3. **What worked.** Things to keep doing.
4. **What did not work.** Things to stop doing or change.
5. **Recommendations.** Concrete, actionable, assigned to a rank or role.

## Rules
- **Be honest about failure.** An AAR that omits setbacks teaches nothing.
- **Separate root causes from symptoms.** A SITREP lag caused by a network blip is different from a SITREP lag caused by a broken sergeant.
- **Write for a future reader who was not present.** AARs live in the event store; they must be decipherable without context.
