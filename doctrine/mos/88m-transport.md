# 88M -- Transport

## Category
Support (SPT)

## Core competencies
- Data transfer: moving information between systems, APIs, storage layers.
- API orchestration: chaining calls, handling pagination, managing retries.
- Message routing: getting the right payload to the right destination.

## When to assign this MOS
- The task is "call API X, transform, call API Y" without heavy analysis.
- Examples: export data from one service, import to another; sync state between two systems; bulk-fetch and persist.

## Execution guidance
- **Respect rate limits.** A transport that gets throttled is a transport that fails.
- **Handle partial failures explicitly.** Half-completed transfers must be resumable or reversible.
- **Log every boundary crossing.** The route is the deliverable.
