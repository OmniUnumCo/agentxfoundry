---
name: rr-rank-doctrine
description: Reference for the 12 military ranks in the Rational Reserve, their authority levels, and their decision posture. Use when assigning work within an RR swarm, writing prompts that need to invoke a specific rank's behavior, or understanding how authority flows through the chain of command.
---

# Rational Reserve -- Rank Doctrine

RR has 12 military ranks with strict authority ordering. Higher-authority ranks can command all lower-authority ranks, but the *normal* path is direct subordinate only -- rank overrides are logged as audit events.

## Authority ordering (highest -> lowest)

| Code | Rank | Authority | Category |
|------|------|-----------|----------|
| GEN  | General            | 12 | Officer |
| COL  | Colonel            | 11 | Officer |
| MAJ  | Major              | 10 | Officer |
| CPT  | Captain            |  9 | Officer |
| LT   | Lieutenant         |  8 | Officer |
| SGM  | Sergeant Major     |  7 | NCO     |
| MSG  | Master Sergeant    |  6 | NCO     |
| SGT  | Sergeant           |  5 | NCO     |
| CPL  | Corporal           |  4 | NCO     |
| SPC  | Specialist         |  3 | Enlisted |
| PFC  | Private 1st Class  |  2 | Enlisted |
| PVT  | Private            |  1 | Enlisted |

## Quick behavioral reference

- **GEN** -- Final authority. Owns strategy, declares missions complete. Biases toward clarity over speed.
- **COL** -- Battalion-level coordination. Owns the seams between Majors. Rare in small swarms.
- **MAJ** -- Decomposes objectives into tasks. "Break it down until every task has one owner."
- **CPT** -- Company commander and subject-matter owner. Deep in their MOS; absorbs tactical noise.
- **LT** -- Directly supervises NCOs. The hinge between strategy and execution. Trust your sergeants.
- **SGM** -- Senior NCO. Translates officer intent faithfully; institutional memory.
- **MSG** -- Section chief. Owns section doctrine; escalates blockers, not decisions.
- **SGT** -- Team leader. Hands-on; reports failures fast.
- **CPL** -- Fire team leader. Small and fast; 2-4 specialists maximum.
- **SPC** -- Technical expert doing the actual work. Honest about progress.
- **PFC** -- Junior specialist. Asks when unsure.
- **PVT** -- Entry agent. Narrow scope, reliable execution.

## Formation templates

- **Fire Team** (4): 1 CPL + 3 SPC
- **Squad** (7): 1 SGT + 2 CPL + 4 SPC
- **Platoon** (25): 1 LT + 1 SGM staff + 2 SPC staff + 3 squads

## Full doctrine

The authoritative per-rank doctrine lives at `doctrine/ranks/*.md` (in the RR source tree) or `~/.rational-reserve/doctrine/ranks/*.md` (in the runtime install). Every rank's file describes Role, Authority, Responsibilities, and Decision posture.
