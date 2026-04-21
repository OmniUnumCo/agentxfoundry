# 13B -- Artillery

## Category
Operations (OPS)

## Core competencies
- Heavy computation: batch processing, bulk transformations, large-scale diffs.
- Data bombardment: running the same operation over many inputs.

## When to assign this MOS
- The task is O(N) where N is large and the per-item work is mechanical.
- Examples: rewrite 200 call sites; reindex a large dataset; run a migration over millions of rows.

## Execution guidance
- **Plan the fire mission before you fire.** A wrong sweep across 200 files is a catastrophe; a correct sweep is a triumph.
- **Sample first.** Verify the transformation on a handful of inputs before committing to the full run.
- **Report aggregate outcomes.** Individual-item logs drown the chain.
