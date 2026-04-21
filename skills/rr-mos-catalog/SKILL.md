---
name: rr-mos-catalog
description: Catalog of the 14 Military Occupational Specialties (MOS) in the Rational Reserve. Use when choosing which specialty to assign to a task, interpreting an MOS code returned by RR tools, or selecting primary_mos when deploying a swarm.
---

# Rational Reserve -- MOS Catalog

Every RR agent has a Military Occupational Specialty (MOS) that shapes its task prompting. When deploying a swarm with `rr_deploy_swarm`, pick the `primary_mos` that best matches the kind of work the swarm will do.

## Intelligence (MI) -- analysis, pattern recognition

| Code | Name | Use for |
|------|------|---------|
| 35F  | Intelligence Analyst    | Reading many sources and producing a judgment/brief |
| 35L  | Counterintelligence     | Security review, threat modeling, supply-chain risk |
| 35N  | SIGINT Analyst          | Log analysis, trace correlation, noise filtering    |

## Operations (OPS) -- execution and building

| Code | Name | Use for |
|------|------|---------|
| 11B  | Infantry                | General-purpose execution, adaptable work |
| 12B  | Combat Engineer         | Code generation, building features, setting up infra |
| 13B  | Artillery               | Batch processing, large-scale sweeps, bulk transformations |
| 19D  | Cavalry Scout           | Reconnaissance, codebase exploration, mapping unknowns |

## Support (SPT) -- the plumbing

| Code | Name | Use for |
|------|------|---------|
| 25B  | IT Specialist           | Sysadmin, deployment, DevOps |
| 42A  | HR Specialist           | Agent roster management, headcount audits |
| 88M  | Transport               | Data transfer, API orchestration, message routing |
| 92G  | Logistics               | Resource allocation, dependency management |

## Special Operations (SOF) -- hard problems, fast response

| Code | Name | Use for |
|------|------|---------|
| 18B  | Special Forces Engineer | Architecture, critical design decisions |
| 75R  | 75th Ranger             | Emergency fixes, crisis response, hotfixes |
| 160  | 160th SOAR              | Time-critical ops, synchronized multi-agent handoffs |

## Choosing a MOS

- "Build something new" -> **12B** Combat Engineer
- "Figure out how X works" -> **19D** Cavalry Scout
- "Review this PR for security" -> **35L** Counterintelligence
- "Debug a log file" -> **35N** SIGINT Analyst
- "Migrate many files" -> **13B** Artillery
- "Fix a prod incident" -> **75R** Ranger
- "Design a new subsystem" -> **18B** SF Engineer
- "General-purpose work" -> **11B** Infantry

## Full doctrine

Authoritative per-MOS guidance lives at `doctrine/mos/*.md` in the RR source tree.
