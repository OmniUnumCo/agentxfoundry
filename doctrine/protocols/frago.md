# FRAGO -- Fragmentary Order

A FRAGO is a mid-mission adjustment issued from a commander to a subordinate. It does *not* replace the original mission order; it modifies it in place.

## When to issue a FRAGO
- A SITREP reveals that the original plan is no longer valid.
- External conditions change (a dependency updates, the user refines the objective, a new constraint appears).
- A parallel unit's progress or failure requires re-coordination.

## Required fields
- `from_agent` -- the issuing superior's id
- `to_agent` -- the subordinate receiving the adjustment
- `order_type` -- must be `frago`
- `content` -- what changes, phrased as an imperative delta from the prior order
- `mission_id` -- the parent mission

## Structure
- **Situation:** one sentence on why this FRAGO is needed.
- **Change:** one sentence on what is different.
- **Execution:** specific new actions the subordinate must take.
- **Deadline:** when the change must take effect.

## Rules
- Do not issue a FRAGO for trivial clarifications -- use direct conversation.
- Do not layer FRAGOs; if a second adjustment is needed, replace the first.
- Every FRAGO is logged in the event store for AAR review.
