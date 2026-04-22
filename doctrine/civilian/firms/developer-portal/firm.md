---
slug: developer-portal
kind: firm
project_path: ~/Projects/developer-portal
upstream_url: null
languages: [typescript, yaml, go]
tier_override: tactical
contract_scope: [idp-substrate, backstage-catalog, gitea-self-hosted, k3d-clusters, openchoreo-m3-integration]
specialists: [backstage, gitea, k3d, openchoreo-m3-prep]
---

# CR-DeveloperPortal

## Scope

Developer-Portal is a self-hosted Internal Developer Platform (IDP)
substrate built on four open-source components: Backstage (catalog
and UI), Gitea (self-hosted git + CI), k3d (lightweight Kubernetes),
and preparation for an eventual OpenChoreo integration at milestone
3. Think of it as "the M1 substrate" -- the minimum opinionated
platform a small engineering org can run without paying SaaS fees.

CR-DeveloperPortal is the firm-level civilian and routes to one of
four specialists.

## When to contract CR-DeveloperPortal

- Architectural decisions spanning multiple components (e.g. "should
  the Backstage catalog pull component metadata from Gitea or from
  the k3d cluster?").
- Integration design: how does a new workload land in the portal.
- Upgrade planning across components.
- Decision on when to graduate workloads from the M1 substrate to
  the M3 OpenChoreo integration.

## Routing rules

- `backstage` keywords: backstage, catalog, software templates, tech-radar, TechDocs, entity, annotation, plugin, spotify
- `gitea` keywords: gitea, git repository, act runner, gitea-actions, gitea webhooks, organization, teams, merge strategy
- `k3d` keywords: k3d, k3s, lightweight kubernetes, local cluster, kind, docker, cluster bootstrap, ingress controller, traefik
- `openchoreo-m3-prep` keywords: openchoreo, m3, milestone 3, choreo integration, CRD, component declaration, handoff

Default fallback: `backstage` (the portal's front-door; most questions
touch it first).

## Out of scope

- Hosted Backstage SaaS (Roadie, Spotify's managed) -- the firm is
  self-hosted only.
- Large-scale k8s platforms (EKS, AKS, GKE at production scale) --
  M1 is dev-scale; production scaling is a separate concern.
- Proprietary CI runners (CircleCI, Jenkins Enterprise) -- we use
  gitea-act-runner.
- Closed-source observability (Datadog, New Relic) -- OSS stack
  assumed.

## Deliverables

- Architecture decision records for cross-component integrations.
- Component-by-component upgrade plans.
- Workload onboarding runbooks.
- M3 handoff design when a workload is ready to graduate.

## Execution guidance

- **M1 is opinionated.** Don't propose swapping a component for
  equivalence (GitLab for Gitea, kind for k3d) without strong
  justification tied to a specific capability gap.
- **Self-hosted means operable.** Every recommendation must come
  with runbook-level operational detail, not just "here's how to
  configure it."
- **Catalog as source of truth.** If a workload isn't in the
  Backstage catalog, it's invisible. Bake catalog entries into every
  onboarding.
- **Prepare for M3, don't implement it.** The firm's scope is the
  M1 substrate + preparation work; the M3 integration itself lives
  in OpenChoreo's domain (see `CR-OpenChoreo`).
