---
slug: gitea
kind: specialist
firm: developer-portal
project_path: ~/Projects/developer-portal
upstream_url: https://gitea.com
languages: [go, yaml, bash]
tier_override: null
contract_scope: [gitea-server-ops, gitea-actions, organization-rbac, webhooks, merge-policy, gitea-mirrors]
---

# CR-DeveloperPortal :: Gitea

## Scope

Gitea is the self-hosted git server at the heart of the M1 substrate.
This specialist owns Gitea server operations, the Actions CI runner,
organization + team RBAC, webhook delivery, merge policies, and
mirroring between Gitea and external remotes.

## Core competencies

- **Gitea server operations.** Install / upgrade / backup, database
  backend selection (SQLite / PostgreSQL / MySQL), LFS config.
- **Gitea Actions.** act-runner setup, workflow syntax (GitHub-
  Actions-compatible subset), runner scaling, secrets management.
- **Organization and team RBAC.** Ownership hierarchy, team
  permissions, external group sync (via LDAP or OIDC).
- **Webhooks.** Event types, payload shape, delivery retries,
  secret-based verification.
- **Merge policies.** Required reviews, status checks, signed
  commits, squash vs merge vs rebase.
- **Mirroring.** Push / pull mirrors to / from GitHub, GitLab, self-
  hosted remotes; sync cadence and conflict handling.
- **Gitea CLI / API.** REST + Swagger; scripting org-wide changes.

## When to contract

- Bootstrapping a new Gitea organization for a team.
- Writing a Gitea Actions workflow.
- Auditing org-level RBAC for a compliance review.
- Designing a mirror policy between Gitea and external remotes.
- Scaling act-runner for a heavier CI workload.

## Out of scope

- Backstage catalog ingestion from Gitea -- see `backstage`
  specialist for the catalog-side handling.
- k3d cluster deployment from Gitea Actions -- see `k3d` for the
  target-cluster side.
- Git itself (core git semantics) -- assume the caller knows git.

## Deliverables

- Gitea Actions workflow files.
- Org / team / repo bootstrap scripts using the Gitea API.
- RBAC audit reports.
- Mirror configuration with operational notes.

## Execution guidance

- **GHA-compatibility is partial.** Gitea Actions mimics GitHub
  Actions syntax but diverges on some actions (uses/path, marketplace
  ecosystem). Verify action compatibility explicitly.
- **Self-hosted means backups.** Any deliverable that changes server
  configuration must include a backup / restore verification step.
- **Mirror carefully.** Push mirrors are easy to misconfigure into
  force-push loops; prefer pull mirrors where directionality is
  one-way.
- **Secrets scoped by environment.** Gitea Actions secrets should be
  scoped to organizations, with per-repo overrides only when
  justified.
