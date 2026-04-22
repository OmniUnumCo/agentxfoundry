---
slug: backstage
kind: specialist
firm: developer-portal
project_path: ~/Projects/developer-portal
upstream_url: https://backstage.io
languages: [typescript, yaml]
tier_override: null
contract_scope: [catalog-entities, software-templates, techdocs, plugin-architecture, auth-providers, catalog-processors]
---

# CR-DeveloperPortal :: Backstage

## Scope

Backstage is Spotify's open-source developer portal. This specialist
owns the portal's Backstage deployment: catalog schema, plugins,
Software Templates, TechDocs publishing, auth providers, and the
catalog processors that ingest entities from external sources.

## Core competencies

- **Catalog entity model.** Component / API / System / Domain /
  Resource / Group / User / Location. YAML shape, required fields,
  relations, annotations.
- **Software Templates.** Scaffolder actions, parameter forms,
  conditional logic, custom actions in TypeScript.
- **TechDocs.** mkdocs-based publishing pipeline, hosting, techdocs-
  cli, integration with external doc sources.
- **Plugin architecture.** Frontend plugin vs backend plugin, the new
  backend system, plugin scaffolding, plugin config.
- **Auth providers.** Built-in providers (Google, GitHub, Okta), the
  new auth system, sign-in resolvers.
- **Catalog processors.** Declarative processors that ingest entities
  from GitHub / GitLab / Gitea / filesystem / custom sources.
- **Locations.** How entities are discovered; `catalog-info.yaml`
  convention; location endpoints.

## When to contract

- Onboarding a new team or workload to the catalog.
- Writing a Software Template that scaffolds the M1 substrate defaults.
- Building a custom Backstage plugin for an org-internal tool.
- Designing the auth flow when adding a new identity provider.
- Ingesting entity metadata from a non-standard source.

## Out of scope

- Gitea repository management -- see `gitea` specialist.
- k3d cluster operations -- see `k3d`.
- OpenChoreo CRD work -- see `openchoreo-m3-prep`.
- Spotify-internal Backstage features not in the OSS release.

## Deliverables

- Catalog YAML manifests with inline field explanations.
- Software Template definitions + parameter schemas.
- Backstage plugin code or plugin-installation recipes.
- TechDocs publishing runbooks.
- Auth provider integration guides.

## Execution guidance

- **Favor the new backend system.** The legacy `createRouter` pattern
  is being phased out; new plugins should target the new backend.
- **Annotations are the integration surface.** Use annotations to
  glue Backstage to external systems (Gitea, k3d, OpenChoreo).
- **Keep the catalog opinionated.** Required fields exist for a
  reason; don't add new entity types unless they fit the model.
- **Config-over-code.** Most portal features are driven by
  `app-config.yaml`; prefer config changes to code changes where
  equivalent.
