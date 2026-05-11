# ADR-005: Agent Communication Protocol

**Status:** Proposed

**Date:** 2026-05-10

**Authors:** Joseph Pearson

---

## Context

The Host Manager control plane (ADR-001) discovers and identifies endpoints (ADR-003,
ADR-004), then deploys lightweight agents to those endpoints. These agents must communicate
bidirectionally with the control plane to:

- Receive commands (install software, run scripts, update configuration, restart services)
- Report status, health, and metrics
- Stream logs and command output
- Announce presence and maintain a heartbeat
- Resume gracefully after intermittent connectivity loss

### Operational Constraints

The protocol must satisfy a demanding set of real-world constraints:

1. **Agent initiates all connections.** Managed endpoints are routinely behind NAT, corporate
   firewalls, or ISP-controlled networks. The control plane cannot reach in; agents must reach out.

2. **No new firewall rules.** Requiring operators to open a non-standard port (e.g., 4222 for
   NATS TCP) adds friction, creates IT approval bottlenecks, and is a deployment blocker in
   regulated environments. The protocol must traverse existing firewall policy without changes.

3. **TLS 1.3 with Perfect Forward Secrecy (PFS).** Command-and-control traffic for production
   endpoints must be encrypted in transit and must not be retroactively decryptable if a
   long-lived key is compromised. PFS ensures each session's keys are ephemeral and independent.

4. **Intermittent connectivity.** Endpoints may go offline for minutes to days. The protocol
   must queue commands durably and deliver them reliably when the agent reconnects.

5. **Low agent overhead.** Agents run on constrained hardware (Raspberry Pi 3, older servers).
   The agent binary must be small, memory-efficient, and require no runtime dependencies.

6. **Consistency with the existing stack.** The control plane already runs NATS (ADR-001).
   Adding a completely different messaging infrastructure increases operational complexity.

7. **Auditability.** All commands issued and results received must be logged for compliance
   and debugging.

### Precedent: Open Horizon / anax

The Open Horizon project (Linux Foundation / LFedge) faced an identical problem designing
communication between its edge `anax` agent and the management hub. Its design choices are
directly applicable:

> *"The agent is secure from network attacks because it has no listening ports on the host
> network. All communication between the agent and the management hub is accomplished by
> the agent polling the management hub."*
> — Open Horizon documentation

Key properties of the anax model:
- Agent initiates all outbound connections (HTTPS/REST)
- **TLS 1.3** — enforced, not configurable
- **PFS guaranteed** — TLS 1.3 mandates ECDHE; static RSA key exchange is removed from
  the protocol entirely
- **Zero listening ports** on the agent host
- **Exponential backoff** on Exchange unavailability
- **Port 443** — no new firewall rules required in virtually any environment
- Commands persist in the Exchange and are delivered when the agent next polls

The anax model validates that an agent-initiated, outbound-only, HTTPS-based protocol is
operationally sound at production scale. This ADR adopts its core security properties while
preserving NATS integration for lower-latency command delivery where network conditions allow.

---

## Decision

**Host Manager agents will use NATS over WebSocket (WSS) on port 443 as the primary
transport, with automatic fallback to HTTPS polling for environments where WebSocket
connections are blocked.**

Both transport modes share:
- **Port 443** — no new firewall rules in any standard environment
- **TLS 1.3 only** — older protocol versions refused at the server
- **ECDHE key exchange** — Perfect Forward Secrecy on every session
- **Agent-initiated, outbound-only** — zero inbound ports on the agent host
- **Exponential backoff** on connection failure
- **Per-agent NKey credentials** stored in OpenBao (ADR-002)
- **Durable command delivery** — commands queued in JetStream / Exchange and
  re-delivered after reconnection

### Primary Mode: NATS over WebSocket (WSS)

```
Agent → wss://control.example.com:443/nats
         ↑ TLS 1.3 + WebSocket Upgrade (agent-initiated, outbound)
         ↓
    NATS Server (WebSocket listener, port 443)
         ↓
    WasmCloud actors — same NATS subjects, same JetStream streams
```

The agent embeds the `async-nats` Rust client, which supports WebSocket transport
natively. The NATS server on the control plane enables a WebSocket listener on port 443
(or TLS-terminating reverse proxy forwards to it). No protocol bridge is required; all
existing actor subjects and JetStream streams work unchanged.

### Fallback Mode: HTTPS Polling

For environments where WebSocket upgrade headers are stripped by a corporate proxy:

```
Agent → HTTPS GET  https://control.example.com/api/v1/agents/{id}/commands
         (poll for pending commands, exponential backoff when empty)
Agent → HTTPS POST https://control.example.com/api/v1/agents/{id}/status
         (push heartbeat, metrics, and command results)
```

The agent detects WebSocket failure on first connect and switches to HTTPS polling
automatically. The API Gateway actor (ADR-001) exposes these endpoints. Commands are
persisted in NATS JetStream until acknowledged; the polling interval is configurable
(default: 15 seconds).

---

## Rationale

### Why NATS over WebSocket (Primary)

**1. Port 443 — no new firewall rules**

Port 443 (HTTPS) is pre-permitted outbound in virtually every network environment:
corporate offices, cloud VPCs, home networks, and air-gapped facilities with external
internet access. Requiring a new outbound rule for port 4222 (standard NATS TCP) means
IT tickets, security review, and deployment friction for every customer. WSS on 443
eliminates this entirely.

**2. TLS 1.3 enforces PFS unconditionally**

TLS 1.3 removes static RSA key exchange from the protocol. ECDHE is not a configurable
option — it is the only permitted key exchange mechanism. This means:

- A captured session recording cannot be decrypted even if the server's certificate
  private key is later compromised.
- Operators cannot accidentally misconfigure weak cipher suites.
- Security auditors reviewing Host Manager can verify PFS without inspecting
  TLS configuration files.

With NATS TCP + TLS 1.2, operators must explicitly configure ECDHE-only cipher suites.
This is frequently overlooked and creates a gap between intended and actual security
posture.

**3. Native NATS integration — no bridge required**

NATS WebSocket is a first-class transport in the NATS server (not a proxy or shim).
WasmCloud actors use the same NATS subjects, the same JetStream streams, and the same
NKey credentials regardless of whether agents connect via TCP or WebSocket. The Agent
Coordinator actor requires no changes.

**4. Sub-millisecond command delivery**

WSS maintains a persistent connection. The control plane pushes commands to agents
immediately via NATS pub/sub. There is no poll interval lag. For time-sensitive
operations — killing a runaway process, applying a security patch, rolling back a
failed deployment — this matters.

**5. Consistent with existing stack**

NATS is already running in the control plane (ADR-001). WSS adds a listener, not a
new system. Operators who already understand NATS monitoring and troubleshooting can
apply that knowledge to agent connectivity.

### Why HTTPS Polling (Fallback)

**1. Works through the strictest proxies**

Some corporate environments run HTTP proxies that inspect and rewrite traffic. Many
strip the `Upgrade: websocket` header, breaking WSS silently. HTTPS polling does not
require any special headers and works through any proxy that permits HTTPS.

**2. Validated at production scale**

Open Horizon / anax has used this exact model — HTTPS polling, TLS 1.3, PFS, port 443,
agent-initiated — in production edge deployments across Linux servers, Raspberry Pis, and
embedded devices. It is not a theoretical fallback; it is a proven approach.

**3. Operationally simple to debug**

When an agent cannot reach the control plane, `curl -v https://control.example.com/api/v1/health`
reproduces the connectivity test exactly. No NATS-specific tooling is required.

### Why Not NATS TCP (Port 4222) as Primary

| Concern | Detail |
|---|---|
| **New firewall rule required** | Port 4222 is non-standard; requires IT change request in most organisations |
| **Security team familiarity** | Security teams understand HTTPS threat models; NATS is unfamiliar and may require additional security review |
| **No PFS guarantee** | TLS 1.2 (the NATS default) requires explicit cipher suite configuration to enforce ECDHE; frequently misconfigured |
| **Proxy traversal** | NATS TCP does not traverse HTTP proxies |
| **Replaced by WSS** | NATS WebSocket on 443 provides all the same capabilities without these drawbacks |

NATS TCP on 4222 was the original recommendation in the first draft of this ADR. The
comparison with Open Horizon / anax surfaced the firewall and TLS 1.3/PFS gaps. NATS WSS
addresses both while preserving native NATS integration.

---

## Architecture

### Connection Model

```
┌──────────────────────────────────────────────────────┐
│  Control Plane                                       │
│                                                      │
│  WasmCloud Host                                      │
│  ├─ Agent Coordinator Actor                          │
│  ├─ API Gateway Actor (HTTPS polling endpoints)      │
│  └─ NATS Server                                      │
│       ├─ TCP listener :4222   (internal actors only) │
│       └─ WSS listener :443    (external agents)      │
│            ↑ TLS 1.3, ECDHE only                    │
│            │ WebSocket upgrade                       │
└────────────┼─────────────────────────────────────────┘
             │ outbound, agent-initiated
┌────────────┼─────────────────────────────────────────┐
│  Managed Endpoint                                    │
│            │                                         │
│  hostmgr-agent                                       │
│  ├─ Transport negotiator                             │
│  │   ├─ Primary: NATS WSS → wss://control:443/nats  │
│  │   └─ Fallback: HTTPS poll → https://control:443  │
│  ├─ Command handler                                  │
│  ├─ Status / metrics reporter                        │
│  └─ Log streamer                                     │
│                                                      │
│  Zero inbound ports                                  │
└──────────────────────────────────────────────────────┘
```

### Transport Negotiation

On startup the agent attempts transports in order:

```
1. NATS WSS  → wss://control.example.com:443/nats
      ↓ success: maintain persistent connection, use NATS pub/sub
      ↓ failure (WSS blocked or server unavailable): exponential backoff × 3, then →

2. HTTPS poll → https://control.example.com:443/api/v1/agents/{id}/commands
      ↓ success: enter polling loop (default interval: 15s)
      ↓ failure: exponential backoff (1s → 2s → 4s → … → 60s max), retry indefinitely

3. On next startup the agent retries WSS first (WSS preference is sticky but not permanent)
```

Both paths share the same NKey credential, the same TLS configuration, and the same
message schema. The Agent Coordinator actor is unaware of which transport the agent is
using.

### TLS Configuration

```toml
# NATS server configuration (control plane)
websocket {
  port: 443
  tls {
    cert_file:  "/etc/hostmgr/tls/server.crt"
    key_file:   "/etc/hostmgr/tls/server.key"
    # TLS 1.3 only — older versions refused
    min_version: "1.3"
  }
}

# Internal TCP listener (not exposed externally)
port: 4222
```

TLS 1.3 is the minimum and only accepted version on the external WSS listener.
The internal TCP listener (port 4222) is for WasmCloud actors on the same host only
and must not be exposed beyond localhost or the cluster-internal network.

### Subject Namespace

All agent subjects follow the pattern `agent.{endpoint_id}.{direction}.{topic}`:

```
# Control plane → agent (commands, via JetStream)
agent.{id}.cmd.exec          # Run a shell command
agent.{id}.cmd.install       # Install a package
agent.{id}.cmd.config        # Push a configuration file
agent.{id}.cmd.restart       # Restart a service
agent.{id}.cmd.update        # Update agent binary

# Agent → control plane (status)
agent.{id}.status.heartbeat  # Alive signal (every 30s)
agent.{id}.status.result     # Command execution result
agent.{id}.status.metrics    # CPU / memory / disk metrics
agent.{id}.logs.stream       # Streaming log output

# Agent → control plane (lifecycle)
agent.{id}.lifecycle.connected     # Agent came online
agent.{id}.lifecycle.disconnected  # Clean shutdown

# Control plane → all agents (broadcast)
agent.broadcast.cmd.ping           # Health check all agents
agent.broadcast.cmd.update         # Broadcast binary update
```

For HTTPS polling, command subjects map to REST endpoints:

```
GET  /api/v1/agents/{id}/commands      → dequeues pending cmd messages
POST /api/v1/agents/{id}/status        → accepts heartbeat / result payloads
POST /api/v1/agents/{id}/logs          → accepts log chunks
```

### Message Schema

All messages use JSON in Phase 1. Protobuf migration is deferred to Phase 3 if
payload size or throughput becomes a concern.

**Command (control plane → agent):**

```json
{
  "message_id":   "550e8400-e29b-41d4-a716-446655440000",
  "command_id":   "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "issued_by":    "actor/agent-coordinator",
  "issued_at":    "2026-05-10T12:00:00Z",
  "expires_at":   "2026-05-10T13:00:00Z",
  "type":         "exec",
  "payload": {
    "command":           "systemctl restart nginx",
    "timeout_seconds":   30,
    "working_directory": "/",
    "environment":       {}
  }
}
```

**Result (agent → control plane):**

```json
{
  "message_id":    "3f2504e0-4f89-11d3-9a0c-0305e82c3301",
  "command_id":    "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "endpoint_id":   "ep-abc123",
  "reported_at":   "2026-05-10T12:00:01Z",
  "status":        "success",
  "exit_code":     0,
  "stdout":        "...",
  "stderr":        "",
  "duration_ms":   412
}
```

**Heartbeat (agent → control plane, every 30s):**

```json
{
  "endpoint_id":   "ep-abc123",
  "reported_at":   "2026-05-10T12:00:00Z",
  "agent_version": "0.1.0",
  "transport":     "wss",
  "uptime_seconds": 3600,
  "metrics": {
    "cpu_percent":       2.1,
    "memory_used_mb":    512,
    "disk_used_percent": 45.2
  }
}
```

### Authentication and Credential Bootstrap

```
┌─────────────────────────────────────────────────────────┐
│  Authentication Model                                   │
│                                                         │
│  1. Per-agent NKey credentials                          │
│     • Ed25519 keypair generated at installation         │
│     • Public key registered in NATS server config       │
│     • Private key delivered to agent via bootstrap      │
│     • Private key stored in OpenBao: secret/agents/{id} │
│                                                         │
│  2. TLS 1.3 for all transport                           │
│     • Server certificate validated by agent             │
│     • Certificate pinning optional (Phase 2)            │
│                                                         │
│  3. Subject-level NATS authorisation                    │
│     • Each NKey may only:                               │
│       Publish:   agent.{own_id}.*                       │
│       Subscribe: agent.{own_id}.cmd.*                   │
│       Subscribe: agent.broadcast.*                      │
│     • Cannot read or publish to any other agent's       │
│       subjects; compromise of one agent is contained    │
└─────────────────────────────────────────────────────────┘
```

**Bootstrap flow (first-time installation):**

```
1.  Agent Coordinator generates Ed25519 NKey keypair for this endpoint
2.  Public key registered with NATS server (added to account credentials)
3.  Public key + private key stored in OpenBao at secret/agents/{id}/nkey
4.  Agent Coordinator issues a bootstrap token:
    • Short-lived (5 minute TTL)
    • Single-use (invalidated on first retrieval)
    • Scoped only to GET /api/v1/agents/{id}/credential
5.  Bootstrap token is written to the endpoint during agent installation
    (via SSH, as part of the Agent Coordinator install flow)
6.  Agent starts, uses bootstrap token to retrieve its NKey private key
    from the control plane API (HTTPS, TLS 1.3)
7.  Agent stores the NKey locally in an encrypted file
8.  Agent discards the bootstrap token
9.  All future connections use the NKey directly; bootstrap API call
    is never repeated unless the agent is re-provisioned
```

### Reconnection and Durable Delivery

```
┌──────────────────────────────────────────────────────────┐
│  Reconnection Strategy                                   │
│                                                          │
│  WSS mode:                                               │
│  • async-nats built-in reconnect loop                    │
│  • Exponential backoff: 1s → 2s → 4s → … → 60s cap     │
│  • Jitter added to prevent thundering herd               │
│  • In-flight messages buffered during reconnect          │
│                                                          │
│  HTTPS polling mode:                                     │
│  • Exponential backoff on HTTP errors (5xx, timeout)     │
│  • 1s → 2s → 4s → … → 60s cap                          │
│  • On 4xx (auth failure): alert and pause — likely a     │
│    credential issue, not a transient error               │
│                                                          │
│  Durable command delivery (both modes):                  │
│  • Commands published to JetStream stream AGENT_CMDS     │
│  • Stream retention: 24 hours                            │
│  • Per-consumer ack required before message is removed   │
│  • Unacked messages re-delivered on reconnect            │
│  • Commands include expires_at; expired commands are     │
│    discarded without execution                           │
│                                                          │
│  Presence tracking:                                      │
│  • Agent marked offline after 90s without heartbeat      │
│  • Commands continue to queue in JetStream               │
│  • Operator alerted via API/UI when agent goes offline   │
└──────────────────────────────────────────────────────────┘
```

### Agent Binary Design

```
hostmgr-agent  (~25 MB static binary, no runtime dependencies)
├─ async-nats  (NATS client: WSS + NATS protocol)   ~2 MB
├─ reqwest     (HTTP client: HTTPS polling fallback) ~3 MB
├─ tokio       (async runtime)
├─ Transport negotiator  (WSS primary → HTTPS fallback)
├─ Command executor      (tokio::process)
├─ Log streamer
├─ Status / metrics reporter  (sysinfo crate)
└─ Credential store      (local encrypted file, age encryption)
```

Target binary size: **< 30 MB** (statically linked, no libc dependency).

---

## Consequences

### Positive Impacts

**1. No new firewall rules in any standard environment**
Both WSS and HTTPS polling use port 443. Outbound 443 is pre-permitted in virtually
every network. This removes a common deployment blocker.

**2. TLS 1.3 PFS is unconditional, not configurable**
By mandating TLS 1.3 on the external listener, PFS is guaranteed at the protocol level.
Operators cannot accidentally weaken it. Security auditors have a single, clear fact to
verify.

**3. Zero inbound ports on agent hosts**
The agent has no open listening ports. It cannot be reached from the network
at all — only it can initiate connections. This eliminates an entire class of
network-based attacks against managed endpoints.

**4. Native NATS integration preserved**
The WSS path reuses the existing NATS server, subjects, JetStream streams, and
NKey credentials. WasmCloud actors require no changes. Adding a WSS listener is
a configuration change, not an architectural change.

**5. Real-time command delivery in the common case**
When WSS is available (the vast majority of environments), commands are pushed
to agents with sub-millisecond latency. The polling fallback is for strict
corporate proxies, not the default experience.

**6. Operationally validated design**
The HTTPS polling fallback is not speculative. Open Horizon / anax has used this
exact model — outbound HTTPS, TLS 1.3, PFS, port 443, polling — in production
edge deployments on constrained hardware. The design is battle-tested.

**7. Per-agent credential isolation**
Each agent has a unique NKey. Compromise of one agent's credential does not
expose any other agent. Subject-level NATS authorisation enforces this at the
server level, not just by convention.

### Risks and Mitigations

| Risk | Mitigation |
|---|---|
| WebSocket blocked by corporate proxy | Automatic fallback to HTTPS polling; detected on first connect attempt |
| Bootstrap token interception | Tokens are single-use, 5-minute TTL, delivered over TLS 1.3 only |
| Agent NKey compromise | Per-agent keys; revoke by removing public key from NATS account config; no blast radius to other agents |
| NATS WSS unfamiliar to operators | Full HTTPS polling fallback is debuggable with standard tools (`curl`); NATS only required for performance path |
| JetStream stream overflow (many offline agents) | 24h retention cap; per-message `expires_at`; configurable stream max bytes |
| Control plane port 443 conflict | If port 443 is occupied by another service, the NATS WSS listener can run on a different port with reverse proxy forwarding to 443 |

### Implementation Considerations

- The NATS server WebSocket listener requires TLS configuration. A self-signed certificate
  is acceptable for development but must be CA-signed (or internally CA-signed) for
  production. Document this clearly.
- The internal NATS TCP listener (port 4222) must be firewalled to localhost or the
  cluster-internal network only. It is not secured with the same TLS requirement
  and must not be exposed externally.
- HTTPS polling endpoints must be idempotent. The agent may re-POST a status update
  if a previous POST timed out. The API Gateway actor must handle duplicate `command_id`
  results gracefully (idempotency key on `message_id`).

---

## Alternatives Considered

### Alternative 1: NATS TCP Leaf Node (Port 4222) — Original Recommendation

**Decision:** Replaced by NATS WSS

**Rationale:** This was the original ADR-005 recommendation. Comparison with Open Horizon /
anax surfaced two critical gaps:

1. Port 4222 requires a new firewall rule in most environments — a deployment blocker.
2. Default NATS + TLS 1.2 does not guarantee PFS without explicit cipher suite configuration,
   which is frequently overlooked.

NATS over WebSocket on port 443 with TLS 1.3 mandatory solves both issues while preserving
all other advantages of the NATS approach (native integration, JetStream, NKey credentials,
sub-millisecond latency). NATS TCP remains available for internal actor communication.

### Alternative 2: HTTPS Polling Only (Open Horizon / anax model)

**Decision:** Retained as fallback; not chosen as primary

**Rationale:** The anax model is operationally sound and has the best firewall and security
story. It is adopted here as the fallback transport. However, NATS WSS is preferred as the
primary transport because:

- Command latency equals the poll interval (15–30s) vs. sub-millisecond for WSS push
- Log streaming requires SSE or a separate WebSocket, duplicating effort
- NATS JetStream provides better durable delivery guarantees than HTTP queue semantics
- The WasmCloud actor model is already NATS-native; bridging through a REST API adds
  unnecessary indirection in the common case

The polling model remains fully supported as an automatic fallback — not an afterthought.

### Alternative 3: MQTT over TLS (Port 8883)

**Decision:** Rejected

**Rationale:**
- Port 8883 is non-standard; requires new firewall rule (same problem as NATS TCP)
- Adds an MQTT broker as a new infrastructure component alongside NATS
- Requires a NATS ↔ MQTT bridge for control plane actors
- MQTT v3 has a weak security model; MQTT v5 is better but less widely deployed
- No native WasmCloud integration
- Port 443 WebSocket-based MQTT (MQTT over WSS) addresses the port issue but doubles
  the protocol complexity (MQTT + WebSocket + NATS bridge) for no advantage over
  NATS WSS directly

### Alternative 4: gRPC Bidirectional Streaming

**Decision:** Rejected for Phase 1; deferred to Phase 3 for evaluation

**Rationale:**
- gRPC is not a first-class WasmCloud capability provider in Phase 1
- Requires a gRPC server actor or sidecar and a gRPC → NATS bridge
- Strong typing via protobuf is an advantage, but JSON is adequate for MVP message
  volumes
- Can run on port 443 (HTTP/2 over TLS), which addresses the firewall concern
- Reconnection and stream management is non-trivial to implement correctly in Rust
- Revisit in Phase 3 if JSON payload overhead becomes measurable at scale

### Alternative 5: SSH Tunnels / WireGuard Overlay

**Decision:** Rejected

**Rationale:**
- Network-level solutions that do not integrate with the application-layer message model
- SSH tunnels require SSH key management for the control plane in addition to the agent —
  doubles the credential surface area
- WireGuard is excellent but over-engineered for the MVP scope; requires kernel module on
  older Linux kernels
- Neither approach provides the durable delivery semantics of NATS JetStream
- Deferred to Phase 3 if a full mesh network between control plane and endpoints is needed

---

## Implementation Plan

### Phase 1.1: Core Agent (Q3–Q4 2026)

| Deliverable | Notes |
|---|---|
| `hostmgr-agent` Rust binary | WSS primary + HTTPS polling fallback; static binary, no runtime deps |
| Transport negotiator | Detects WSS failure, switches to HTTPS poll transparently |
| TLS 1.3 enforcement | NATS WSS listener configured for TLS 1.3 minimum; certificate validation in agent |
| NKey credential bootstrap | Bootstrap token flow; encrypted local storage via `age` |
| JetStream stream `AGENT_CMDS` | 24h retention, per-message TTL, ack-based delivery |
| Agent Coordinator actor | Command dispatch, presence tracking, offline alerting |
| HTTPS polling API | `/api/v1/agents/{id}/commands`, `/status`, `/logs` on API Gateway actor |

### Phase 2: Hardening (Q4 2026)

| Deliverable | Notes |
|---|---|
| Certificate pinning | Agent pins control plane cert on first connect; alerts on mismatch |
| Automated NKey rotation | Agent Coordinator rotates credentials on configurable schedule |
| Agent auto-update | Control plane distributes new agent binaries via NATS object store |
| NATS account server | Move NKey authorisation to dynamic account server for runtime revocation |
| Proxy detection | Detect HTTP proxy in environment; configure transport accordingly |

### Phase 3: Scale and Observability (2027+)

| Deliverable | Notes |
|---|---|
| Protobuf message schema | Replace JSON with protobuf for high-throughput deployments |
| gRPC transport evaluation | Assess if gRPC provides measurable advantage over NATS WSS at scale |
| Distributed tracing | Propagate trace IDs through agent ↔ control plane message flows |
| Multi-hub federation | Agent connects to regional hub; hubs replicate via NATS supercluster |

---

## Related Decisions

- **ADR-001:** WasmCloud Control Plane Architecture — defines NATS as the actor communication
  bus; this ADR extends that bus to external agents via WebSocket
- **ADR-002:** Credential Storage Strategy — agent NKeys stored in OpenBao; bootstrap token
  issued and validated through the credential provider
- **ADR-003:** Network Discovery Strategy — discovers endpoints that agents will be deployed to
- **ADR-004:** Endpoint Identification Strategy — identifies endpoint type before agent deployment

---

## External References

- [Open Horizon Agent (anax)](https://open-horizon.github.io/docs/anax/docs/) — production
  precedent for outbound-only, HTTPS, TLS 1.3, PFS, port 443 agent communication
- [NATS WebSocket Documentation](https://docs.nats.io/running-a-nats-service/configuration/websocket)
- [NATS JetStream Documentation](https://docs.nats.io/nats-concepts/jetstream)
- [async-nats Rust crate](https://docs.rs/async-nats) — WebSocket transport supported
- [TLS 1.3 RFC 8446](https://www.rfc-editor.org/rfc/rfc8446) — PFS via ECDHE is mandatory

---

## Monitoring and Review

**Decision review criteria:**
- WSS fallback rate: if > 20% of agents are falling back to HTTPS polling in a given
  deployment, investigate whether the WSS listener configuration or network topology is
  the cause
- Command delivery latency: track p50/p95/p99 from command issue to agent acknowledgement;
  acceptable p99 < 500ms in WSS mode, < 30s in polling mode
- TLS version in use: monitor TLS handshake metadata; alert if any connection negotiates
  below TLS 1.3

**Review date:** After Phase 1.1 launch (Q4 2026); revisit gRPC and protobuf decisions
in Phase 3 planning (Q1 2027).

---

## Open Questions

1. **Reverse proxy vs. direct NATS WSS listener:** Should the NATS WebSocket listener bind
   directly to port 443, or should an nginx/Caddy reverse proxy terminate TLS and forward
   to NATS on an internal port? The reverse proxy approach is more operationally familiar
   and allows easier certificate management (Certbot/ACME). Likely preference: Caddy as
   reverse proxy for MVP; document both options.

2. **Poll interval tuning:** The 15-second default poll interval in HTTPS fallback mode
   is a placeholder. Real-world testing with constrained hardware should inform the
   final value. A longer interval (30–60s) reduces load on the Exchange; a shorter interval
   reduces command latency in fallback mode.

3. **Agent binary distribution:** How are agent binaries delivered to endpoints before the
   agent is running? Phase 1.1 uses the SSH-based Agent Coordinator install flow. Phase 2
   should define a self-update mechanism.

---

## Decision Record

**Approved by:** [To be filled after review]

**Date approved:** [To be filled]

**Concerns / feedback addressed:** [To be filled after discussion]
