# ADR-009: Configuration and Packaging

**Status:** Proposed

**Date:** 2026-05-10

**Authors:** Joseph Pearson

---

## Context

Host Manager consists of multiple components that must be deployed, configured, and
operated across a range of environments:

- **Developer laptops** — for local development and integration testing
- **Single Linux servers** — the MVP production topology (ADR-001)
- **Kubernetes clusters** — for teams that standardise on Kubernetes
- **Production edge deployments** — where operational simplicity, reliability, and
  zero-touch lifecycle management are paramount

Each environment has different conventions for configuration injection, packaging,
upgrade, and health monitoring. The choices made here determine how easy Host Manager
is to adopt and operate.

### Components to Package

| Component | Type | Description |
|---|---|---|
| WasmCloud Host | Runtime | Runs all WasmCloud actors |
| NATS Server | Infrastructure | Message bus and JetStream store |
| OpenBao | Infrastructure | Credential and endpoint state storage |
| `hostmgr-agent` | Binary | Runs on managed endpoints (ADR-006) |
| Web UI | Static assets | Served by the API Gateway actor |

### Configuration Requirements

From the decisions in ADR-001 through ADR-008, the following configuration
surface is required at startup:

- Control plane network addresses (NATS URL, OpenBao URL, external API URL)
- TLS certificate and key paths
- OAuth client IDs (GitHub, Google) — ADR-007
- Allowed operator identities — ADR-007
- JWT signing key reference in OpenBao — ADR-007
- Agent communication settings (WSS port, polling interval) — ADR-005
- NATS JetStream stream configuration — ADR-005

### Guiding Constraints

1. **Environment variables for MVP.** The 12-factor app methodology (environment
   variables for configuration) is the lowest-friction approach that works across
   all target packaging formats — Docker Compose, Kubernetes, and Open Horizon
   service definitions all support environment variable injection natively.

2. **Secrets must not appear in environment variables.** Environment variables are
   visible in process listings (`ps aux`) and container inspect output. Secret
   values (OAuth client secrets, JWT signing keys, OpenBao tokens) are stored in
   OpenBao and referenced by path, not by value.

3. **Production packaging must be operationally self-sufficient.** Production
   deployments should not require manual intervention for routine lifecycle events
   (restart, update, configuration reload). This points toward Open Horizon (which
   manages workload lifecycle) and the Kubernetes Operator SDK (which manages
   Kubernetes resource lifecycle).

---

## Decision

**Configuration is injected via environment variables for all deployment targets.
Secret values are referenced by OpenBao path, not set directly.**

**Packaging targets, in order of intended use:**

| Target | Use Case | Format |
|---|---|---|
| **Docker Compose** | Local development and testing | `docker-compose.yml` + `.env` file |
| **Helm 3** | Kubernetes-based testing and staging | Helm chart with `values.yaml` |
| **Open Horizon Service Definition** | Production edge deployments | OH service definition + deployment policy |
| **Operator SDK** | Production Kubernetes deployments | Custom Resource Definition + controller |

The agent (`hostmgr-agent`) packaging is covered in ADR-006 and is out of scope here.
This ADR covers the control plane components only.

---

## Rationale

### Why Environment Variables

Environment variables are the universal configuration interface across all four
packaging targets. Docker Compose uses `env_file` and `environment` blocks; Helm
injects them via `ConfigMap` and `Secret` references; Open Horizon service definitions
accept `userInput` variables that map to container environment variables; the Operator
SDK manages `EnvVar` entries in pod specs. A configuration file format (TOML, YAML)
would need to be generated or mounted differently in each target, adding complexity
without benefit.

The distinction between non-secret configuration (environment variables) and secret
configuration (OpenBao paths) is enforced by convention documented here and by
OpenBao access policies.

### Why Docker Compose for Local Development

Docker Compose is the standard tool for running multi-container development
environments. It starts NATS, OpenBao, and the WasmCloud host in a single command
(`docker compose up`). It supports health checks, volume mounts, and network
isolation out of the box. It is familiar to the widest range of developers.

### Why Helm 3 for Kubernetes Testing

Helm is the de facto package manager for Kubernetes. Teams evaluating Host Manager
on Kubernetes expect a Helm chart. Helm 3 (no Tiller) is secure and widely deployed.
The chart is not the production Kubernetes path (the Operator SDK is) but it
provides a quick way to stand up a test instance on any Kubernetes cluster.

### Why Open Horizon for Production Edge Deployment

Open Horizon is a Linux Foundation project designed precisely for managing workloads
on edge nodes at scale. Its Agreement Bot negotiates deployment policies; its agent
(`anax`) manages workload lifecycle, restarts, and updates. Packaging Host Manager
as an Open Horizon service means:

- **Zero-touch deployment:** OH deploys the control plane to target nodes based on
  policy, with no manual SSH or shell script execution
- **Lifecycle management:** OH handles restarts, version updates, and rollbacks
- **Policy-driven distribution:** Deploy to nodes that match specific properties
  (location, capability, hardware class) without enumerating individual hosts
- **Dogfooding:** Host Manager, a tool for managing edge nodes, is itself managed
  by an edge node management system — validating the architecture it promotes
- **FDO integration:** Open Horizon's FDO support (ADR-006, Phase 3) will bootstrap
  new control plane nodes automatically

### Why Operator SDK for Production Kubernetes

The Kubernetes Operator pattern is the standard for stateful, lifecycle-aware
applications on Kubernetes. The Operator SDK (CNCF project) provides the framework
for building a Host Manager operator that handles:

- CRD-based configuration (`HostManagerInstance` custom resource)
- TLS certificate rotation (via cert-manager integration)
- OpenBao unsealing and initialisation
- NATS cluster management
- Rolling upgrades with health validation
- Horizontal scaling (Phase 2)

The Operator SDK target is production Kubernetes, distinct from the Helm chart
(which is for test/evaluation). A Helm chart cannot safely manage the lifecycle
of stateful infrastructure components like OpenBao.

---

## Architecture

### Environment Variable Reference

All variables are prefixed `HOSTMGR_`. Variables ending in `_REF` are OpenBao
path references; the API Gateway actor resolves them to values at startup via
the credential capability provider.

#### Core / Network

```bash
# External URL operators and agents use to reach the control plane
HOSTMGR_EXTERNAL_URL=https://control.example.com

# Internal NATS URL (WasmCloud host → NATS)
HOSTMGR_NATS_URL=nats://localhost:4222

# NATS WebSocket URL (external agents, ADR-005)
HOSTMGR_NATS_WSS_URL=wss://control.example.com:443/nats

# OpenBao / Vault
HOSTMGR_OPENBAO_URL=https://localhost:8200
HOSTMGR_OPENBAO_TOKEN_REF=secret/config/openbao_token   # resolved at runtime

# TLS (for the API / WSS listener)
HOSTMGR_TLS_CERT=/etc/hostmgr/tls/server.crt
HOSTMGR_TLS_KEY=/etc/hostmgr/tls/server.key
```

#### Authentication (ADR-007)

```bash
# Allowed operator identities (comma-separated)
HOSTMGR_ALLOWED_USERS=github:joewxboy,google:joe@example.com

# GitHub OAuth (client secret stored in OpenBao)
HOSTMGR_AUTH_GITHUB_CLIENT_ID=Ov23li...
HOSTMGR_AUTH_GITHUB_CLIENT_SECRET_REF=secret/config/oauth/github_client_secret

# Google OAuth (client secret stored in OpenBao)
HOSTMGR_AUTH_GOOGLE_CLIENT_ID=123456789-abc...
HOSTMGR_AUTH_GOOGLE_CLIENT_SECRET_REF=secret/config/oauth/google_client_secret

# JWT session tokens
HOSTMGR_JWT_SIGNING_KEY_REF=secret/config/jwt_signing_key
HOSTMGR_JWT_TTL=28800                                    # 8 hours in seconds
```

#### Agent Communication (ADR-005)

```bash
# HTTPS polling fallback endpoint (served by API Gateway actor)
HOSTMGR_AGENT_POLL_INTERVAL=15                           # seconds

# Bootstrap token TTL
HOSTMGR_BOOTSTRAP_TOKEN_TTL=300                         # 5 minutes

# Heartbeat timeout — agent marked offline after N seconds without heartbeat
HOSTMGR_HEARTBEAT_TIMEOUT=90
```

#### Logging and Observability

```bash
HOSTMGR_LOG_LEVEL=info                                  # trace|debug|info|warn|error
HOSTMGR_LOG_FORMAT=json                                 # json|text
```

#### Development Override (compile-guarded in production builds)

```bash
# Bypasses OAuth for local development — not available in production binaries
HOSTMGR_DEV_TOKEN=dev-insecure-token
```

### Target 1: Docker Compose (Local Development)

```
host_manager/
└── deploy/
    └── docker-compose/
        ├── docker-compose.yml
        ├── .env.example          ← copy to .env, fill in values
        ├── config/
        │   ├── nats.conf
        │   └── openbao.hcl
        └── README.md
```

```yaml
# deploy/docker-compose/docker-compose.yml
services:

  nats:
    image: nats:2.10-alpine
    command: -c /etc/nats/nats.conf
    ports:
      - "4222:4222"     # internal NATS (localhost only in production)
      - "443:443"       # WSS for agents (mapped to NATS WebSocket port)
      - "8222:8222"     # NATS monitoring UI (dev only)
    volumes:
      - ./config/nats.conf:/etc/nats/nats.conf:ro
      - nats-data:/data
    healthcheck:
      test: ["CMD", "nats-server", "--signal", "status"]
      interval: 10s

  openbao:
    image: openbao/openbao:2.5.0
    cap_add:
      - IPC_LOCK
    ports:
      - "8200:8200"
    volumes:
      - ./config/openbao.hcl:/etc/openbao/openbao.hcl:ro
      - openbao-data:/openbao/data
    healthcheck:
      test: ["CMD", "bao", "status"]
      interval: 10s
      retries: 5

  wasmcloud:
    image: ghcr.io/host-manager/wasmcloud-host:latest
    depends_on:
      nats:
        condition: service_healthy
      openbao:
        condition: service_healthy
    env_file: .env
    ports:
      - "8080:8080"     # REST API / Web UI
    volumes:
      - ./tls:/etc/hostmgr/tls:ro

volumes:
  nats-data:
  openbao-data:
```

Operator setup:
```bash
cp .env.example .env
# edit .env with GitHub/Google OAuth credentials and allowed users
docker compose up -d
# initialise OpenBao (first run only)
docker compose exec openbao bao operator init
```

### Target 2: Helm 3 (Kubernetes Testing / Staging)

```
charts/
└── hostmgr/
    ├── Chart.yaml
    ├── values.yaml           ← default values; override per-environment
    ├── templates/
    │   ├── configmap.yaml    ← non-secret environment variables
    │   ├── secret.yaml       ← Kubernetes Secret for OAuth client IDs
    │   ├── deployment.yaml   ← WasmCloud host deployment
    │   ├── nats.yaml         ← NATS StatefulSet
    │   ├── openbao.yaml      ← OpenBao StatefulSet
    │   ├── service.yaml      ← LoadBalancer / NodePort for API + WSS
    │   └── ingress.yaml      ← optional TLS ingress
    └── README.md
```

```yaml
# charts/hostmgr/values.yaml (excerpt)
externalUrl: "https://control.example.com"

auth:
  allowedUsers: "github:joewxboy"
  github:
    clientId: ""          # set via --set or values override
  google:
    clientId: ""

nats:
  replicaCount: 1         # 1 for testing; 3 for HA (Phase 2)

openbao:
  replicaCount: 1

tls:
  enabled: true
  certManagerIssuer: "letsencrypt-prod"   # or "selfsigned" for testing

image:
  repository: ghcr.io/host-manager/wasmcloud-host
  tag: "latest"
  pullPolicy: IfNotPresent
```

Operator setup:
```bash
helm repo add hostmgr https://charts.host-manager.io
helm install hostmgr hostmgr/hostmgr \
  --namespace hostmgr --create-namespace \
  --set auth.allowedUsers="github:joewxboy" \
  --set auth.github.clientId="Ov23li..." \
  --set externalUrl="https://control.example.com"
```

### Target 3: Open Horizon Service Definition (Production Edge)

The control plane is packaged as an Open Horizon service, allowing Open Horizon's
Agreement Bot to deploy and manage it on any node matching the deployment policy.

```
deploy/open-horizon/
├── service.definition.json     ← OH service definition
├── deployment.policy.json      ← which nodes get this service
├── service.policy.json         ← service constraints and properties
└── README.md
```

```json
{
  "label": "Host Manager Control Plane",
  "description": "WasmCloud-based edge management control plane",
  "public": false,
  "documentation": "https://github.com/host-manager/host-manager",
  "url": "github.com/host-manager/control-plane",
  "version": "0.1.0",
  "arch": "amd64",
  "sharable": "singleton",
  "requiredServices": [],
  "userInput": [
    {
      "name": "HOSTMGR_EXTERNAL_URL",
      "label": "External URL for this control plane",
      "type": "string",
      "defaultValue": ""
    },
    {
      "name": "HOSTMGR_ALLOWED_USERS",
      "label": "Allowed operator identities (comma-separated)",
      "type": "string",
      "defaultValue": ""
    },
    {
      "name": "HOSTMGR_AUTH_GITHUB_CLIENT_ID",
      "label": "GitHub OAuth App Client ID",
      "type": "string",
      "defaultValue": ""
    },
    {
      "name": "HOSTMGR_LOG_LEVEL",
      "label": "Log level",
      "type": "string",
      "defaultValue": "info"
    }
  ],
  "deployment": {
    "services": {
      "nats": {
        "image": "nats:2.10-alpine",
        "environment": ["NATS_CONFIG=/etc/nats/nats.conf"],
        "binds": ["/etc/hostmgr/nats.conf:/etc/nats/nats.conf:ro"]
      },
      "openbao": {
        "image": "openbao/openbao:2.5.0",
        "cap_add": ["IPC_LOCK"],
        "binds": [
          "/etc/hostmgr/openbao.hcl:/etc/openbao/openbao.hcl:ro",
          "/var/lib/hostmgr/openbao:/openbao/data"
        ]
      },
      "wasmcloud": {
        "image": "ghcr.io/host-manager/wasmcloud-host:latest",
        "ports": ["8080:8080", "443:443"],
        "binds": ["/etc/hostmgr/tls:/etc/hostmgr/tls:ro"]
      }
    }
  }
}
```

```json
{
  "_comment": "deployment.policy.json — deploy to nodes with role=control-plane",
  "label": "Host Manager Control Plane Deployment",
  "description": "Deploy control plane to designated control nodes",
  "service": {
    "name": "github.com/host-manager/control-plane",
    "org": "host-manager",
    "arch": "*",
    "serviceVersions": [{ "version": "0.1.0", "priority": {} }]
  },
  "constraints": ["role == control-plane"],
  "userInput": [
    {
      "serviceUrl": "github.com/host-manager/control-plane",
      "inputs": [
        { "name": "HOSTMGR_EXTERNAL_URL", "value": "https://control.example.com" },
        { "name": "HOSTMGR_ALLOWED_USERS", "value": "github:joewxboy" }
      ]
    }
  ]
}
```

The node designated as the control plane registers with Open Horizon and is tagged
`role=control-plane`. The Agreement Bot then deploys the Host Manager service to it
automatically. Open Horizon manages restarts, health monitoring, and version updates
of the control plane itself.

### Target 4: Operator SDK (Production Kubernetes)

A Custom Resource Definition (CRD) allows operators to declare a Host Manager
instance as a Kubernetes resource, with the controller managing all lifecycle
operations.

```yaml
# Example HostManagerInstance CRD usage
apiVersion: hostmgr.io/v1alpha1
kind: HostManagerInstance
metadata:
  name: production
  namespace: hostmgr
spec:
  externalUrl: "https://control.example.com"
  auth:
    allowedUsers:
      - "github:joewxboy"
      - "google:joe@example.com"
    github:
      clientIdSecretRef:
        name: hostmgr-oauth-secrets
        key: github-client-id
    google:
      clientIdSecretRef:
        name: hostmgr-oauth-secrets
        key: google-client-id
  nats:
    replicaCount: 3              # HA mode
    storageClass: "fast-ssd"
    storageSize: "10Gi"
  openbao:
    replicaCount: 3
    storageClass: "fast-ssd"
    storageSize: "20Gi"
    certManagerIssuerRef:
      name: "letsencrypt-prod"
      kind: "ClusterIssuer"
  tls:
    certManagerIssuerRef:
      name: "letsencrypt-prod"
      kind: "ClusterIssuer"
  wasmcloud:
    replicas: 2
    image: "ghcr.io/host-manager/wasmcloud-host:0.1.0"
```

The operator controller handles:
- Creating and configuring NATS StatefulSets with JetStream persistence
- Bootstrapping OpenBao (init, unseal, AppRole creation)
- Generating and rotating TLS certificates via cert-manager
- Rolling upgrades with readiness validation
- Status reporting via `.status` sub-resource

---

## Consequences

### Positive Impacts

**1. Single configuration model across all targets**
Environment variables are understood by every packaging target and every team
member. No one needs to learn a Host Manager-specific configuration file format.
The variable names are consistent regardless of how they are injected.

**2. Secrets never in plain text**
The `_REF` convention makes it explicit which values are secret paths vs. plain
configuration. This prevents the common mistake of pasting OAuth client secrets
directly into environment variables.

**3. Dogfooding Open Horizon**
Deploying the Host Manager control plane via Open Horizon demonstrates that the
architecture works and gives the team direct experience with the operator's
perspective. Issues discovered this way will improve the product.

**4. Production readiness without custom tooling**
The Operator SDK target brings production-grade Kubernetes lifecycle management
(health checks, rolling updates, CRD-based config) without building custom
orchestration. The Open Horizon target brings the same for edge deployments.

**5. Incremental adoption path**
Docker Compose → Helm → Open Horizon / Operator SDK represents a clear progression
from development to production. Teams can start with Compose and migrate packaging
targets without changing application configuration.

### Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Open Horizon operator unfamiliarity | Docker Compose and Helm targets fully functional; OH is production path, not the only path |
| OpenBao initialisation complexity | First-run initialisation documented step-by-step; automated via operator controller for Kubernetes target |
| TLS certificate management | Docker Compose: self-signed (dev); Helm and Operator: cert-manager; Open Horizon: documented manual cert provisioning |
| OH service definition version drift | Service version pinned in deployment policy; update policy controls rollout |
| Environment variable sprawl | Full variable reference documented in this ADR; `.env.example` kept in sync with CI |

### Implementation Considerations

- The WasmCloud host image (`ghcr.io/host-manager/wasmcloud-host`) bundles all
  WASM actor modules. Operators do not separately manage actor versions.
- The Docker Compose and Helm targets do not run Open Horizon (the OH target
  is self-contained). They use plain Docker networking and Kubernetes networking
  respectively.
- OpenBao must be initialised before the WasmCloud host starts. Health check
  dependencies in Docker Compose and init containers in Kubernetes enforce this
  ordering. The Open Horizon service definition relies on startup ordering within
  the service's container set.
- `HOSTMGR_DEV_TOKEN` must be excluded from the production Docker image. This is
  enforced at compile time via a Rust feature flag (`#[cfg(feature = "dev")]`);
  the production image build does not include the `dev` feature.

---

## Alternatives Considered

### Alternative 1: TOML Configuration File

**Decision:** Rejected for MVP; may be added as an alternative to env vars in Phase 2

**Rationale:** TOML is idiomatic in the Rust ecosystem and provides better structure
for complex nested configuration. However, it requires mounting a config file in
Docker/Kubernetes (volume mounts), templating in Helm (ConfigMap rendering), and
special handling in Open Horizon (file injection). Environment variables work
natively everywhere. A TOML config file can be added in Phase 2 as an alternative
input method (env vars take precedence), allowing operators to choose their preferred
approach.

### Alternative 2: YAML Configuration File

**Decision:** Rejected for the same reasons as TOML

**Rationale:** YAML is more familiar to Kubernetes operators but has the same
injection complexity as TOML. Additionally, YAML's whitespace-sensitivity creates
subtle bugs in generated/templated configs. Env vars are simpler and safer for MVP.

### Alternative 3: Kubernetes-only (No Docker Compose or Open Horizon)

**Decision:** Rejected

**Rationale:** Requiring Kubernetes for local development adds significant friction.
Many contributors and early adopters will run Host Manager on a single Linux machine
or a laptop. Docker Compose enables a zero-infrastructure-dependency local setup.
Open Horizon is the production path for the primary target environment (edge nodes),
which may not be Kubernetes.

### Alternative 4: Single Binary with Embedded NATS and OpenBao

**Decision:** Rejected

**Rationale:** Embedding NATS and OpenBao into a single binary simplifies deployment
but creates serious operational problems: OpenBao data and NATS JetStream data are
not independently backed up or migrated; OpenBao upgrades are coupled to Host Manager
releases; the binary size would be very large. Separation of concerns (each component
upgrades independently) is the correct architecture for a production system.

---

## Related Decisions

- **ADR-001:** WasmCloud Control Plane — defines the components that this ADR packages
- **ADR-002:** Credential Storage — OpenBao configuration and initialisation
- **ADR-005:** Agent Communication Protocol — NATS WSS configuration (port 443, TLS 1.3)
- **ADR-006:** Agent Deployment — `hostmgr-agent` packaging (separate from control plane)
- **ADR-007:** API Authentication — OAuth client ID environment variables defined here

---

## Open Questions

1. **Open Horizon version target:** Which version of Open Horizon (anax) should the
   service definition target? Latest stable at time of Phase 1.1 development. Pin
   the minimum `anax` version in the service definition.

2. **Operator SDK version:** Operator SDK v1.x (controller-runtime based). Pin version
   at start of Operator implementation.

3. **Image registry:** `ghcr.io/host-manager/` is the assumed registry. Confirm
   GitHub Container Registry as the primary distribution registry; document how
   operators with air-gapped Kubernetes clusters can mirror images.

4. **ConfigMap vs. environment variables in Helm:** The Helm chart should use a
   ConfigMap for non-secret variables and a Secret for OAuth client IDs, with the
   deployment referencing both via `envFrom`. Confirm this approach does not conflict
   with the Operator SDK target (which manages these resources directly).

---

## Decision Record

**Approved by:** [To be filled after review]

**Date approved:** [To be filled]

**Concerns / feedback addressed:** [To be filled after discussion]
