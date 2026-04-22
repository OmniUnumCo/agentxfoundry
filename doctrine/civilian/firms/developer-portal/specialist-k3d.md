---
slug: k3d
kind: specialist
firm: developer-portal
project_path: ~/Projects/developer-portal
upstream_url: https://k3d.io
languages: [yaml, bash, go]
tier_override: null
contract_scope: [k3d-cluster-bootstrap, k3s-flavors, docker-host-networking, ingress-traefik, registry-mirror, local-dev-lifecycle]
---

# CR-DeveloperPortal :: k3d

## Scope

k3d wraps k3s (lightweight Kubernetes) in Docker containers, giving a
fully-featured single-host Kubernetes cluster suitable for local
development and the M1 substrate. This specialist owns cluster
bootstrap, host-Docker networking, the bundled traefik ingress, an
optional registry-mirror for air-gapped use, and the lifecycle
between k3d and the workloads running on it.

## Core competencies

- **Cluster bootstrap.** `k3d cluster create` flags, config files,
  multi-server vs single-server topologies, port mapping from host
  to ingress.
- **k3s flavors.** What k3s disables by default (traefik, servicelb,
  local-path-provisioner); how to re-enable or swap each.
- **Docker host networking.** Port binding, host-network mode vs
  exposed ports, macOS / Linux / WSL2 host differences.
- **Traefik ingress.** Default IngressController behavior, overriding
  the bundled version, TLS via cert-manager.
- **Registry mirror.** Running a pull-through cache for container
  images; configuring k3d to use it.
- **Local dev lifecycle.** Idempotent recreate scripts, persisting
  state across cluster restarts, tearing down cleanly.

## When to contract

- Bootstrapping a new developer workstation onto the M1 substrate.
- Configuring multi-node k3d for integration testing.
- Debugging ingress / networking issues in a k3d cluster.
- Setting up a registry mirror for air-gapped or bandwidth-constrained
  environments.
- Migrating from kind, minikube, or a hosted dev cluster to k3d.

## Out of scope

- Production k8s operations -- k3d is dev/M1-scale only.
- Non-k3s distributions (kind, minikube, microk8s, k0s) -- comparison
  is fine; deep operations are out of scope.
- Cluster-hosted workload semantics -- that's workload / OpenChoreo
  territory.

## Deliverables

- `k3d` config files (declarative bootstrap).
- Host-networking reference card per OS.
- Ingress / TLS / cert-manager setup runbooks.
- Registry-mirror bootstrap + k3d configuration.
- Teardown + rebuild scripts that preserve explicit state.

## Execution guidance

- **Declarative over imperative.** Use `k3d cluster create --config`
  instead of long flag sets; the config is checkable into the portal.
- **OS-aware advice.** macOS Docker Desktop, Linux native, and WSL2
  have different networking quirks; cite which OS the advice applies
  to.
- **Dev != prod.** Any solution that "works in k3d" must carry an
  explicit note about whether it generalizes to real clusters.
- **Clean teardown.** Left-over volumes and networks accumulate fast;
  teardown scripts must be exhaustive.
