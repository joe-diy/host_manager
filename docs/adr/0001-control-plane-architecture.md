# ADR-001: WasmCloud Control Plane Architecture

**Status:** Proposed

**Date:** 2026-05-10

**Authors:** Joseph Pearson

---

## Context

Host Manager's control plane must:
1. **Orchestrate discovery** (ADR-003) — find endpoints on the network
2. **Coordinate identification** (ADR-004) — determine endpoint types
3. **Manage credentials** (ADR-002) — store and distribute credentials securely
4. **Deploy and manage agents** (ADR-005) — deploy software to endpoints
5. **Provide APIs** — CLI, UI, integrations for users and operators
6. **Handle distributed state** — manage endpoints across multiple networks
7. **Scale and adapt** — work on a single server or across a cluster

**Technology chosen:** WasmCloud (from ADR platform research)
- Capability-based security model aligns perfectly with control plane needs
- Topology-agnostic (runs standalone, in Kubernetes, on edge gateways)
- WASI 0.3 async for concurrent endpoint management
- Pluggable capability providers for discovery, credentials, agent management
- Distributed actor model via NATS messaging

**Design challenges:**
- How are capabilities (discovery, credentials, identification, agents) exposed to components?
- How do control plane actors coordinate across the distributed system?
- How do APIs (CLI, UI) interact with the control plane?
- How is state persisted and recovered?
- How is the control plane deployed (single host vs. distributed)?

---

## Decision

**Host Manager's control plane is implemented as a WasmCloud application comprising multiple actors and capability providers. The control plane uses NATS-based messaging for actor communication, OpenBao for credential storage, and pluggable capability providers for discovery, identification, and agent management. The control plane is deployed as a single WasmCloud host (MVP), with multi-host clustering available for scale.**

### Specific Decisions:

1. **Core architecture: WasmCloud-based distributed actors**
   - Control plane = WasmCloud host(s) running WASM actors
   - Actors communicate via NATS messaging (async, event-driven)
   - Capability providers expose privileged operations (discovery, credential access, agent management)
   - State persisted in OpenBao (credentials) and NATS KV store (endpoint state)

2. **Actor model (MVP scope):**

   **Endpoint Manager (core orchestrator)**
   - Coordinates entire endpoint lifecycle
   - Responsibilities:
     - Trigger discovery when user requests
     - Monitor discovery progress
     - Trigger identification after discovery
     - Monitor identification progress
     - Store endpoint state (pending, identified, online, offline)
     - Trigger agent deployment after identification
     - Monitor agent health and status
   - Exposes endpoints via API (list, get, status, update)

   **Discovery Orchestrator**
   - Manages discovery operations
   - Responsibilities:
     - Receive discovery requests from Endpoint Manager
     - Call discovery capability provider
     - Parse and normalize results
     - Create pending endpoint records
     - Report completion to Endpoint Manager
   - Receives: `{ subnet: "192.168.1.0/24", timeout: 30 }`
   - Returns: `{ discovered_hosts: [...], duration_ms: 2500 }`

   **Identifier Actor**
   - Manages endpoint identification
   - Responsibilities:
     - Receive identification requests from Endpoint Manager
     - Request credentials from Credential Manager
     - Call identification capability provider
     - Match results against known profiles
     - Store identification results
     - Report completion to Endpoint Manager
   - Receives: `{ endpoint_ids: [id1, id2, ...] }`
   - Returns: `{ identified: [...], failed: [...] }`

   **Credential Manager**
   - Mediates all credential access
   - Responsibilities:
     - Receive credential requests from other actors
     - Query OpenBao for secrets
     - Decrypt and return credentials
     - Log all access for audit trail
     - Enforce access policies (which actor can access which credentials)
   - Exposes credential interface to discovery/identification providers
   - Never caches credentials; always retrieves from OpenBao

   **Agent Coordinator (MVP v1.1)**
   - Manages agent deployment and lifecycle
   - Responsibilities:
     - Receive agent deployment requests
     - Download agent binary (for endpoint's CPU arch/OS)
     - Deliver to endpoint (SSH, cloud API, etc.)
     - Monitor agent connectivity
     - Trigger agent actions (probe, collect, command)
   - Deferred to Phase 1.1 (after MVP discovery/identification)

   **API Gateway (exposes HTTP API)**
   - REST endpoint for CLI/UI/integrations
   - Responsibilities:
     - Receive HTTP requests (POST /discovery, GET /endpoints, etc.)
     - Route to appropriate actor
     - Return JSON responses
     - Handle authentication/authorization
   - Built as WasmCloud HTTP server capability

3. **Capability providers (native code, WasmCloud-integrated):**

   **Discovery Provider**
   - Performs ARP/mDNS network scanning
   - Native code (Rust) for network access
   - WasmCloud interface:
     ```
     Input: { subnet: "192.168.1.0/24", timeout: 30, protocols: ["arp", "mdns"] }
     Output: { discovered_hosts: [{ip, mac, hostname, services}], duration_ms: N }
     ```

   **Identification Provider**
   - SSH probes to gather endpoint information
   - Native code (Rust) for SSH client
   - Calls Credential Manager for SSH credentials
   - WasmCloud interface:
     ```
     Input: { endpoints: [{ip, credentials_id}], timeout: 10 }
     Output: { identified: [{endpoint_id, os, hardware, ...}], failed: [...] }
     ```

   **Credential Provider**
   - Mediates access to OpenBao
   - Native code (Rust) for OpenBao client
   - WasmCloud interface:
     ```
     Input: { credential_id: "endpoint_1_ssh" }
     Output: { secret: {ssh_key, username, passphrase} }
     ```

   **Agent Management Provider (Phase 1.1)**
   - Manages agent deployment to endpoints
   - Calls Credential Provider for endpoint access
   - WasmCloud interface:
     ```
     Input: { endpoint_id, agent_binary_url, cpu_arch, os }
     Output: { deployed: true/false, agent_id: "...", endpoint_address: "..." }
     ```

4. **Communication patterns:**

   **Actor-to-actor: NATS messaging**
   - All inter-actor communication via NATS
   - Async, event-driven (no blocking RPC)
   - Example flow:
     ```
     User: POST /discovery
     ↓
     API Gateway: publish discovery.start {subnet, timeout}
     ↓
     Discovery Orchestrator: subscribe discovery.start
     ↓
     Discovery Orchestrator: call discovery_provider.scan()
     ↓
     Discovery Orchestrator: publish discovery.complete {results}
     ↓
     Endpoint Manager: subscribe discovery.complete
     ↓
     Endpoint Manager: update endpoint state
     ↓
     Endpoint Manager: publish identification.start {endpoint_ids}
     ```

   **Actor-to-capability: WasmCloud bindings**
   - Actors call capabilities via WasmCloud imports
   - Synchronous within actor, but async underneath (WASI 0.3)
   - Example:
     ```rust
     let discovered = discovery_provider.scan(
       subnet: "192.168.1.0/24",
       timeout: 30
     ).await;  // WASI 0.3 async/await
     ```

5. **State management:**

   **Endpoint state (stored in OpenBao)**
   ```rust
   struct Endpoint {
     id: String,
     ip_address: String,
     mac_address: String,
     hostname: String,
     
     // Discovery state
     discovery_status: "pending" | "discovered" | "failed",
     discovered_at: Timestamp,
     
     // Identification state
     identification_status: "pending" | "identified" | "failed",
     identified_at: Timestamp,
     endpoint_type: "linux" | "raspberry_pi" | "kubernetes" | "vm" | "unknown",
     os: {name, version, distro},
     hardware: {model, cpu_arch, cores, memory},
     
     // Agent state
     agent_status: "not_deployed" | "deploying" | "running" | "error",
     agent_id: Option<String>,
     agent_version: Option<String>,
     
     // Health
     last_seen: Timestamp,
     online: bool,
     health_status: "healthy" | "degraded" | "offline",
   }
   ```

   **Session/operation state (stored in NATS KV)**
   ```rust
   // In-flight operations tracked in NATS KV
   struct DiscoveryOperation {
     operation_id: String,
     status: "running" | "complete" | "failed",
     started_at: Timestamp,
     endpoints_found: u32,
     endpoints_processed: u32,
   }
   ```

   **No local actor state** — all state persisted in OpenBao or NATS KV
   - Allows actor restarts without data loss
   - Enables horizontal scaling (multiple actor instances)

6. **Deployment topology (MVP):**

   **Single-host deployment (most common)**
   ```
   ┌─────────────────────────────────────────┐
   │  WasmCloud Host (single machine)        │
   │                                         │
   │  NATS embedded                          │
   │  ├─ Endpoint Manager actor              │
   │  ├─ Discovery Orchestrator actor        │
   │  ├─ Identifier actor                    │
   │  ├─ Credential Manager actor            │
   │  ├─ API Gateway actor                   │
   │  │                                      │
   │  ├─ Discovery provider                  │
   │  ├─ Identification provider             │
   │  ├─ Credential provider                 │
   │  └─ HTTP provider (API endpoint)        │
   │                                         │
   │  External:                              │
   │  └─ OpenBao (credential storage)        │
   └─────────────────────────────────────────┘
   
   User interacts via:
   - CLI: hostmgr discover, hostmgr identify, hostmgr status
   - UI: HTTP API at :8080
   - Integrations: REST API
   ```

   **High-availability deployment (Phase 2)**
   ```
   NATS cluster (HA)
   ├─ Node 1: WasmCloud host (Endpoint Manager, Discovery Orchestrator)
   ├─ Node 2: WasmCloud host (Identifier, Credential Manager, API Gateway)
   └─ Node 3: WasmCloud host (backup)
   
   External:
   ├─ OpenBao cluster (HA)
   └─ NATS KV (replicated)
   ```

   **Kubernetes deployment (Phase 2)**
   ```
   Kubernetes cluster
   ├─ StatefulSet: WasmCloud hosts
   ├─ Service: NATS cluster
   ├─ ConfigMap: WasmCloud config
   ├─ Secret: OpenBao credentials
   └─ Ingress: API Gateway
   ```

7. **API surface (REST, exposed via HTTP provider):**

   ```
   # Discovery
   POST   /api/v1/discovery               # Start discovery
   GET    /api/v1/discovery/{op_id}       # Get discovery status
   
   # Endpoints
   GET    /api/v1/endpoints                # List all endpoints
   GET    /api/v1/endpoints/{endpoint_id}  # Get endpoint details
   PATCH  /api/v1/endpoints/{endpoint_id}  # Update endpoint
   DELETE /api/v1/endpoints/{endpoint_id}  # Remove endpoint
   
   # Identification
   POST   /api/v1/endpoints/{ids}/identify # Trigger identification
   
   # Credentials
   POST   /api/v1/credentials              # Add credentials
   GET    /api/v1/credentials/{cred_id}    # Get credential (masked)
   DELETE /api/v1/credentials/{cred_id}    # Remove credential
   
   # Agents (Phase 1.1)
   POST   /api/v1/agents/deploy            # Deploy agents
   GET    /api/v1/agents/{agent_id}        # Get agent status
   POST   /api/v1/agents/{agent_id}/command # Send command
   
   # Health & status
   GET    /api/v1/health                   # Control plane health
   GET    /api/v1/status                   # Overall system status
   ```

8. **Error handling & resilience:**

   **Transient failures:**
   - Discovery provider timeout → retry with backoff
   - SSH probe failure → retry up to 3 times
   - OpenBao unavailable → circuit breaker; queue operations

   **Permanent failures:**
   - Endpoint unreachable → mark as offline; don't retry
   - Credential missing → mark endpoint as pending_credentials
   - Unsupported endpoint type → mark as unknown; alert user

   **Observability:**
   - WasmCloud logs all actor activity
   - OpenBao audit logs credential access
   - NATS provides message history for debugging
   - Prometheus metrics (in Phase 2)

---

## Rationale

### Why WasmCloud (Not Native Service or Serverless Framework)

**Native service (Go/Rust microservices):**
- ❌ Topology-specific (K8s, Docker, VM, etc.)
- ❌ Reinvent capabilities (discovery, credentials, etc.)
- ✅ Well-established pattern

**Serverless framework (Lambda, Cloud Functions):**
- ❌ Vendor lock-in (AWS, GCP, Azure)
- ❌ Topology-coupled (SaaS only)
- ❌ Cold start latency for periodic discovery

**WasmCloud:**
- ✅ Topology-agnostic (runs anywhere: laptop, server, K8s, edge)
- ✅ Capability providers abstract privileged operations
- ✅ WASI 0.3 async/await for concurrent endpoint management
- ✅ Distributed actor model matches control plane's needs
- ✅ NATS messaging for inter-actor communication
- ✅ Pluggable secrets (integrates with OpenBao naturally)
- ✅ Small footprint (single binary can run entire control plane)

### Why Actor Model (Not Traditional Microservices)

**Traditional microservices (separate containers/processes):**
- ❌ Deployment complexity (multiple services)
- ❌ Network overhead (inter-process communication)
- ❌ Operational burden (monitor multiple services)

**WasmCloud actors:**
- ✅ Single deployment unit (one WasmCloud host)
- ✅ NATS messaging (low latency, high throughput)
- ✅ Lightweight (actors are WASM modules, 10s-100s KB each)
- ✅ Scales horizontally (add more hosts; same code)
- ✅ Matches control plane's async, event-driven nature

### Why NATS for Inter-Actor Communication

**HTTP/REST between microservices:**
- ❌ Synchronous (blocking)
- ❌ Tight coupling (each service knows others' URLs)
- ❌ Polling for async operations

**NATS:**
- ✅ Async, event-driven (publish/subscribe)
- ✅ Loose coupling (actors publish events; others subscribe)
- ✅ Builtin NATS KV for distributed state
- ✅ Embedded in WasmCloud host (no external dependency)
- ✅ Scales to 1000s of actors

### Why No Local State in Actors

**In-memory state:**
- ❌ Lost on actor restart
- ❌ Inconsistent across multiple actor instances
- ❌ Can't scale horizontally

**Persistent state (OpenBao + NATS KV):**
- ✅ Survives actor/host restarts
- ✅ Consistent across deployments
- ✅ Enables horizontal scaling
- ✅ Audit trail (OpenBao logs all changes)

### Why Separate Discovery/Identification Providers

**Discovery + Identification in one actor:**
- ❌ Complex actor (too many responsibilities)
- ❌ Harder to test and debug
- ❌ Harder to replace/upgrade individually

**Separate providers:**
- ✅ Single responsibility per actor
- ✅ Easy to swap providers (custom discovery, etc.)
- ✅ Scales independently (can prioritize discovery over identification)
- ✅ Testable in isolation

---

## Consequences

### Positive Impacts

**1. Topology-agnostic control plane**
- Same code runs on laptop, single server, Kubernetes, edge gateways
- No code changes for different deployments
- Easy to move/migrate control plane

**2. Scalability**
- NATS messaging enables 1000s of concurrent endpoint operations
- WASI 0.3 async allows concurrent probes, discoveries, identifications
- Actors can scale horizontally (multiple instances behind NATS)
- State persisted externally (no actor-to-actor synchronization)

**3. Extensibility**
- Custom capability providers easy to add (native code)
- Custom actors can be added to extend functionality
- Pluggable secrets, discovery, agent management
- Operators can write plugins without modifying core

**4. Security**
- Capability-based access control (actors only access what they need)
- Credentials retrieved just-in-time from OpenBao (never cached)
- Full audit trail via WasmCloud + OpenBao logs
- WASM sandboxing prevents actors from escaping isolation

**5. Operational simplicity**
- Single WasmCloud host runs entire control plane
- Embedded NATS (no external broker)
- Embedded metrics/observability
- Simple deployment (one binary, one config file)

### Implementation Challenges

**1. WASM ecosystem maturity**
- WasmCloud Incubating (not Stable) maturity level
- Smaller community than traditional microservices
- May encounter edge cases or bugs
- Mitigation: Start with MVP; community is growing; fallback to native if needed

**2. WASI 0.3 async is preview**
- Async/await only available in preview (not stable)
- APIs may change before final release
- Mitigation: Use preview versions for MVP; Wasmtime 37+ fully supports it

**3. Operational knowledge**
- Teams unfamiliar with WASM/WasmCloud
- Learning curve for troubleshooting (WASM runtime, NATS, etc.)
- Mitigation: Documentation; examples; active community

**4. Debugging in production**
- WASM module errors are harder to inspect than native code
- Stack traces may be less informative
- Mitigation: Comprehensive logging; structured error handling

**5. Performance overhead**
- WASM runtime adds small CPU/memory overhead vs. native
- Typically <5-10% for compute-bound operations
- I/O operations (network) dominate; WASM overhead negligible
- Mitigation: Profile and optimize critical paths; async eliminates blocking

### Risks

**1. WasmCloud adoption stalls**
- If WasmCloud community doesn't grow, may be stranded
- Mitigation: Design for portability; can migrate to native if needed; open-source, can fork if needed

**2. WASI specs change**
- Network, async APIs still stabilizing
- May require code changes as specs finalize
- Mitigation: Use stable APIs; avoid bleeding-edge features; plan for updates

**3. Deployment complexity exceeds expectations**
- NATS cluster setup, OpenBao integration, WASM runtime management
- May be too complex for small teams
- Mitigation: Provide Helm charts, Docker Compose, pre-built binaries; document operations

**4. Performance issues with many endpoints**
- Parallel probing with 1000+ endpoints might overwhelm NATS or OpenBao
- Mitigation: Load testing; rate limiting; progressive scanning (batch size limits)

**5. Security misconfiguration**
- NATS/OpenBao misconfiguration could expose credentials
- WASM module access controls need careful setup
- Mitigation: Security-first defaults; clear documentation; audit logging

---

## Alternatives Considered

### Alternative 1: Traditional Microservices (Go/Rust)

**Decision:** Rejected

**Rationale:**
- Topology-specific (must choose K8s, Docker, VMs, etc.)
- Operational complexity (manage multiple services)
- Overkill for MVP (single control plane, not at massive scale)
- Reinventing WASM's capability model ourselves

**Retention:** Can migrate to native services later if WasmCloud becomes bottleneck

### Alternative 2: Serverless (AWS Lambda, Cloud Functions)

**Decision:** Rejected

**Rationale:**
- Vendor lock-in (SaaS only)
- Can't run on-premises or edge
- Cold start latency (minutes for discovery operations)
- Not suitable for topology-agnostic design

### Alternative 3: Kubernetes-first (Operators, CRDs)

**Decision:** Rejected for MVP

**Rationale:**
- Only works in Kubernetes
- Overkill for MVP (can add later)
- Coupling to K8s limits on-premises/edge deployments

**Retention:** Phase 2 can add Kubernetes operator pattern

### Alternative 4: Single monolithic process (not distributed)

**Decision:** Rejected

**Rationale:**
- All actors in one process = restart cascades affect everything
- No horizontal scaling
- Harder to extend/customize

**Retention:** Simple for MVP, but actors and NATS enable better scaling

### Alternative 5: Custom capability providers (not WasmCloud)

**Decision:** Rejected

**Rationale:**
- Reinventing WasmCloud's design ourselves
- More work; error-prone
- WasmCloud already solved this problem

---

## Implementation Approach

### Phase 1: MVP Control Plane (Single Host)

**1. WasmCloud setup**
   - Create Rust project for actors and providers
   - Actors: Endpoint Manager, Discovery Orchestrator, Identifier, Credential Manager, API Gateway
   - Providers: Discovery, Identification, Credentials, HTTP
   - WasmCloud manifest (wash.toml) defining actors and capabilities

**2. NATS integration**
   - Embedded NATS in WasmCloud host
   - KV store for endpoint state
   - Pub/sub for inter-actor communication

**3. OpenBao integration**
   - Credential provider connects to OpenBao
   - Stores endpoint data (discovered, identified, agents)
   - Audit logging enabled

**4. API Gateway**
   - HTTP server exposing REST API
   - Routes to appropriate actors
   - JSON request/response format

**5. Deployment**
   - Single WasmCloud host binary
   - Configuration file (host config, NATS settings, OpenBao address)
   - Docker container optional (Phase 1.1)
   - Helm chart for Kubernetes (Phase 2)

**6. Testing**
   - Unit tests for each actor
   - Integration tests with mock providers
   - E2E tests with real discovery/identification

### Phase 1.1: Agent Deployment (Agent Coordinator)

**1. Agent Coordinator actor**
   - Manages agent lifecycle
   - Calls Credential Provider for endpoint access
   - Deploys agent binary to endpoint

**2. Endpoint-specific agent binaries**
   - Separate builds for x86_64, aarch64, armv7l
   - Optimized for low-resource endpoints (RPi)

### Phase 2: Distributed Control Plane (HA & Kubernetes)

**1. Multiple WasmCloud hosts**
   - Replicate actors across hosts
   - NATS cluster for inter-host communication
   - OpenBao cluster for HA credential storage

**2. Kubernetes operator**
   - Custom resource: HostManager (defines control plane config)
   - StatefulSet for WasmCloud hosts
   - Service for API Gateway
   - Ingress for external access

**3. Load balancing**
   - API Gateway behind load balancer
   - Automatic failover if host goes down
   - Horizontal scaling (add more hosts)

### Phase 3: Advanced Features

**1. Prometheus metrics**
   - Discovery duration, success rate
   - Identification success rate
   - Agent deployment status
   - Endpoint health distribution

**2. Custom actors & providers**
   - Plugin interface for user-defined actors
   - Custom discovery, identification, agent providers

**3. Multi-region support**
   - Multiple control plane instances per region
   - Region-specific discovery providers
   - Cross-region agent management

---

## Related Decisions

- **ADR-002:** Credential Storage (OpenBao integration)
- **ADR-003:** Network Discovery (Discovery Orchestrator uses discovery provider)
- **ADR-004:** Endpoint Identification (Identifier actor, identification provider)
- **ADR-005:** Agent Communication (Agent Coordinator, agent communication protocol)

---

## Monitoring & Review

**Control plane-specific metrics:**
- Actor message latency (NATS pub/sub timing)
- Capability provider response time (discovery, identification)
- OpenBao credential retrieval latency
- API Gateway request latency
- NATS message queue depth

**Decision review criteria:**
- Can control plane handle 100+ concurrent endpoint operations? (Yes)
- Can actors be restarted without data loss? (Yes)
- Can new actors be added without code changes? (Yes)
- Deployment complexity acceptable? (Yes for MVP; Phase 2 improves)

**Review date:** After Phase 1 MVP completion; reassess every 6-12 months

---

## Open Questions / Future Exploration

1. **Cluster vs. single host trade-off:** When should users switch to HA deployment? (Probably >1000 endpoints)

2. **Custom actor plugins:** How should users write custom actors? (Phase 3 feature)

3. **Actor versioning:** How to upgrade control plane without downtime? (Rolling updates via NATS clustering)

4. **State machine for endpoints:** Should we formalize endpoint lifecycle state machine? (Yes, for Phase 2)

5. **Metrics & observability:** What metrics should be exposed? (Phase 2; Prometheus metrics)

6. **Integration with other systems:** How should external tools integrate (Ansible, Terraform, etc.)? (Phase 2; webhooks, event streaming)

7. **Policy engine:** Should control plane support policies (e.g., "auto-deploy agents to all Linux servers")? (Phase 3 feature)

---

## Decision Record

**Approved by:** [To be filled after review]

**Date approved:** [To be filled]

**Concerns/feedback addressed:** [To be filled after discussion]
