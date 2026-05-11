# ADR-008: Endpoint State Data Model

**Status:** Proposed

**Date:** 2026-05-10

**Authors:** Joseph Pearson

---

## Context

Host Manager tracks the state of endpoints through a complete lifecycle: from initial
network discovery through identification, agent deployment, active management, and
eventual decommissioning. Multiple WasmCloud actors read and write this state:

- **Discovery Orchestrator** — creates endpoint records when first seen on the network
- **Identifier** — enriches records with OS, architecture, and type information
- **Agent Coordinator** — records agent deployment status and version
- **Agent Communication** (via NATS subjects, ADR-005) — updates heartbeat and
  real-time connection status
- **API Gateway** — reads endpoint state to serve CLI and web UI queries
- **Endpoint Manager** — orchestrates lifecycle transitions

ADR-001 allocates state across two stores:

- **OpenBao** — durable endpoint records; encrypted at rest; audit-logged access
- **NATS KV** — ephemeral operational state; fast reads/writes; acceptable to lose
  on restart (rebuilt from agent heartbeats and actor reconciliation)

Without a defined schema, actors will make incompatible assumptions about field names,
value formats, and which store is authoritative for each piece of data. This ADR
defines the canonical schema for both stores.

### Design Principles

1. **Stable identity.** An endpoint's ID is assigned at first discovery and never
   changes, even if the endpoint's IP address, hostname, or hardware changes.

2. **Credentials are always referenced, never embedded.** No credential value
   (key, password, token) ever appears in an endpoint record. Only a reference path
   into OpenBao appears.

3. **State transitions are explicit.** The endpoint lifecycle is a defined state
   machine. Actors do not write arbitrary state; they trigger defined transitions.

4. **Durable and ephemeral state are separated.** OpenBao holds what matters long-
   term. NATS KV holds what matters right now. Losing NATS KV state is recoverable;
   losing OpenBao state is a data loss event.

5. **Schema is versioned.** Records include a `schema_version` field. When the
   schema changes, migration tooling handles existing records. Actors check the
   version and refuse to process records from future versions they don't understand.

---

## Decision

**Endpoint state is stored across two layers with a clear ownership boundary:**

- **OpenBao** owns durable, structured endpoint records: identity, network
  configuration, identification results, agent deployment metadata, and credential
  references.
- **NATS KV** owns ephemeral operational state: real-time connection status,
  last heartbeat timestamp, and in-flight command tracking.

The schema defined in this ADR is the contract between all actors. Changes to the
schema require a new ADR or amendment and a migration plan.

---

## Endpoint Lifecycle

```
              ┌──────────┐
              │          │
    Network   │ UNKNOWN  │  (pre-discovery)
    scan ───► │          │
              └────┬─────┘
                   │ Discovery Orchestrator: IP/MAC seen
                   ▼
              ┌──────────────┐
              │  DISCOVERED  │  IP, MAC, hostname, discovery method
              └──────┬───────┘
                     │ Identifier: SSH probe succeeds
                     ▼
              ┌──────────────┐
              │  IDENTIFIED  │  OS, arch, type, SSH fingerprint
              └──────┬───────┘
                     │ Operator triggers agent deployment
                     ▼
              ┌──────────────────┐
              │ AGENT_DEPLOYING  │  (Phase 2: automated SSH install)
              └──────┬───────────┘   (MVP: operator installs manually)
                     │ Agent connects and heartbeats
                     ▼
              ┌──────────────┐
              │   MANAGED    │  Agent connected, accepting commands
              └──────┬───────┘
                     │
          ┌──────────┼──────────────────┐
          │          │                  │
          ▼          ▼                  ▼
    ┌──────────┐ ┌──────────┐  ┌────────────────┐
    │ OFFLINE  │ │ DEGRADED │  │ DECOMMISSIONED │
    │ (no HB   │ │ (errors  │  │ (operator      │
    │  > 90s)  │ │  reported│  │  removes)      │
    └──────┬───┘ └──────┬───┘  └────────────────┘
           │            │
           └─────┬──────┘
                 │ Agent reconnects and heartbeats
                 ▼
            ┌──────────┐
            │  MANAGED │
            └──────────┘
```

State transitions are owned by specific actors:

| Transition | Owner Actor |
|---|---|
| → DISCOVERED | Discovery Orchestrator |
| DISCOVERED → IDENTIFIED | Identifier |
| IDENTIFIED → AGENT_DEPLOYING | Agent Coordinator (Phase 2) / operator (MVP) |
| AGENT_DEPLOYING → MANAGED | Agent Coordinator (on first heartbeat) |
| IDENTIFIED → MANAGED | Agent Coordinator (MVP: manual install detected) |
| MANAGED → OFFLINE | Endpoint Manager (heartbeat timeout) |
| MANAGED → DEGRADED | Agent Coordinator (on error reports) |
| OFFLINE / DEGRADED → MANAGED | Agent Coordinator (on reconnect) |
| Any → DECOMMISSIONED | Endpoint Manager (operator action) |

---

## Schema: OpenBao (Durable Records)

All endpoint data lives under the path prefix `secret/endpoints/{endpoint_id}/`.

### Path Layout

```
secret/endpoints/{endpoint_id}/
├── core          ← identity, status, timestamps
├── network       ← IP addresses, MACs, hostnames, discovery metadata
├── identity      ← OS, architecture, type, SSH fingerprint
├── agent         ← agent version, deployment metadata, transport
└── credentials   ← reference paths only; no credential values
```

### `core` — Identity and Lifecycle

```json
{
  "schema_version": 1,
  "id": "ep-7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "display_name": "pi-livingroom",
  "status": "MANAGED",
  "created_at": "2026-05-10T09:00:00Z",
  "updated_at": "2026-05-10T12:34:56Z",
  "discovered_at": "2026-05-10T09:00:00Z",
  "identified_at": "2026-05-10T09:01:30Z",
  "agent_deployed_at": "2026-05-10T10:15:00Z",
  "decommissioned_at": null,
  "tags": {
    "location": "living-room",
    "owner": "joe",
    "environment": "home"
  },
  "notes": "Primary media server"
}
```

**Field notes:**
- `id`: UUID v4, prefixed `ep-` for readability; assigned at discovery; immutable
- `display_name`: Operator-assigned; defaults to hostname at identification time;
  editable via API
- `status`: One of `DISCOVERED`, `IDENTIFIED`, `AGENT_DEPLOYING`, `MANAGED`,
  `OFFLINE`, `DEGRADED`, `DECOMMISSIONED`
- `tags`: Arbitrary key-value pairs for grouping and filtering; keys and values
  are strings; max 50 tags per endpoint
- `schema_version`: Incremented when the schema changes; actors refuse records
  with a version higher than they understand

### `network` — Network Configuration

```json
{
  "schema_version": 1,
  "primary_ip": "192.168.1.42",
  "ip_addresses": [
    { "address": "192.168.1.42", "type": "ipv4", "interface": "eth0" },
    { "address": "fd00::1a2b:3c4d", "type": "ipv6", "interface": "eth0" }
  ],
  "mac_addresses": [
    { "address": "b8:27:eb:12:34:56", "interface": "eth0" }
  ],
  "hostnames": [
    { "name": "pi-livingroom", "source": "mdns" },
    { "name": "pi-livingroom.local", "source": "mdns" }
  ],
  "discovery_method": "mdns",
  "last_seen_ip": "192.168.1.42",
  "last_seen_at": "2026-05-10T12:34:56Z",
  "network_segment": "192.168.1.0/24"
}
```

**Field notes:**
- `primary_ip`: The IP used for SSH identification and agent communication;
  operator-editable if the device has multiple addresses
- `discovery_method`: One of `arp`, `mdns`, `manual`; records how the endpoint
  was first found
- `ip_addresses` and `mac_addresses` are arrays — endpoints may have multiple
  interfaces
- `last_seen_ip`: Updated by discovery runs even after identification; tracks
  IP changes over time (DHCP roaming)

### `identity` — Identification Results

```json
{
  "schema_version": 1,
  "type": "raspberry_pi",
  "subtype": "raspberry_pi_4b",
  "os": "Raspberry Pi OS",
  "os_version": "12 (Bookworm)",
  "os_codename": "bookworm",
  "kernel": "6.1.21-v8+",
  "architecture": "arm64",
  "cpu_model": "Cortex-A72",
  "cpu_cores": 4,
  "memory_mb": 4096,
  "hostname": "pi-livingroom",
  "ssh_host_fingerprint": "SHA256:abc123...",
  "identification_method": "ssh_probe",
  "identified_at": "2026-05-10T09:01:30Z",
  "identification_confidence": "high"
}
```

**Field notes:**
- `type`: One of `linux_server`, `raspberry_pi`, `vm`, `kubernetes_node`,
  `macos`, `unknown`
- `subtype`: More specific variant within the type (e.g., `raspberry_pi_4b`,
  `raspberry_pi_zero_2w`); `null` if not determinable
- `ssh_host_fingerprint`: SHA256 fingerprint of the host's SSH public key;
  used to detect host key changes (potential MITM or reimaging)
- `identification_confidence`: `high` (multiple confirming signals),
  `medium` (primary signal only), `low` (fallback heuristics)
- `identification_method`: `ssh_probe` for MVP; future values include
  `snmp`, `wmi`, `kubernetes_api`

### `agent` — Agent Deployment Metadata

```json
{
  "schema_version": 1,
  "status": "connected",
  "version": "0.2.1",
  "deployed_at": "2026-05-10T10:15:00Z",
  "deployment_method": "manual",
  "service_mode": "systemd",
  "binary_checksum": "sha256:e3b0c44298fc1c149afb...",
  "control_plane_url": "wss://control.example.com:443/nats",
  "transport": "wss",
  "nkey_public": "UABC123...",
  "last_heartbeat_at": "2026-05-10T12:34:30Z",
  "agent_uptime_seconds": 8600,
  "update_channel": "stable",
  "minimum_version": "0.1.0"
}
```

**Field notes:**
- `status`: Mirrors the real-time status in NATS KV; written here on significant
  transitions (connected, offline) for durability; `null` if agent not deployed
- `deployment_method`: `manual` (MVP), `ssh_automated` (Phase 2), `fdo` (Phase 3),
  `open_horizon` (Phase 3)
- `service_mode`: `systemd`, `docker`, `podman`, `foreground`
- `nkey_public`: The agent's Ed25519 public key (NKey format); stored here
  for reference; private key is in `secret/agents/{endpoint_id}/nkey` (ADR-005)
- `transport`: The transport mode currently in use (`wss` or `https_poll`),
  as last reported by the agent
- `update_channel`: `stable`, `beta`, or `none` (no auto-update); used by
  ADR-010 lifecycle management

### `credentials` — Credential References

```json
{
  "schema_version": 1,
  "ssh": {
    "ref": "secret/credentials/ep-7c9e6679/ssh",
    "type": "ssh_key",
    "username": "pi",
    "added_at": "2026-05-10T09:05:00Z",
    "added_by": "github:joewxboy"
  },
  "sudo": {
    "ref": "secret/credentials/ep-7c9e6679/sudo",
    "type": "sudo",
    "added_at": "2026-05-10T09:05:00Z",
    "added_by": "github:joewxboy"
  },
  "additional": []
}
```

**Field notes:**
- This record contains **only reference paths** into OpenBao. No credential
  values appear here. The Credential Manager actor resolves references to
  values when needed, subject to capability-based access control (ADR-001).
- `added_by`: The authenticated operator identity (from JWT `sub` claim, ADR-007)
  who stored this credential; for audit purposes
- `additional`: Array of additional credential references for endpoints with
  multiple access methods (e.g., both SSH key and password, or API token)

---

## Schema: NATS KV (Ephemeral Operational State)

NATS KV buckets for endpoint state use the bucket name `ENDPOINT_STATE`.

### Key Layout

```
ENDPOINT_STATE /
├── {endpoint_id}.status          ← current lifecycle status (fast read)
├── {endpoint_id}.heartbeat       ← last heartbeat timestamp + metrics
├── {endpoint_id}.connection      ← current connection details
└── {endpoint_id}.cmd.{cmd_id}   ← in-flight command tracking
```

### `.status` — Current Status (Fast Read Cache)

```json
{
  "status": "MANAGED",
  "updated_at": "2026-05-10T12:34:56Z",
  "updated_by": "actor/endpoint-manager"
}
```

This mirrors the `status` field in `core` (OpenBao) but is stored in NATS KV
for low-latency reads by the API Gateway when serving list/status endpoints.
The NATS KV value is the cache; OpenBao is authoritative. On NATS restart,
this is rebuilt from OpenBao during reconciliation.

### `.heartbeat` — Real-time Agent Presence

```json
{
  "received_at": "2026-05-10T12:34:30Z",
  "agent_version": "0.2.1",
  "uptime_seconds": 8600,
  "transport": "wss",
  "metrics": {
    "cpu_percent": 3.2,
    "memory_used_mb": 512,
    "memory_total_mb": 4096,
    "disk_used_percent": 28.4,
    "load_1m": 0.12,
    "load_5m": 0.08,
    "load_15m": 0.07
  }
}
```

TTL: 120 seconds. The Endpoint Manager watches for heartbeat expiry; if a key
expires without renewal, the endpoint transitions to `OFFLINE`.

### `.connection` — Active Connection Details

```json
{
  "connected_since": "2026-05-10T10:16:00Z",
  "transport": "wss",
  "remote_address": "203.0.113.42",
  "nats_connection_id": "nats-conn-abc123",
  "last_activity_at": "2026-05-10T12:34:30Z"
}
```

Written when an agent connects (via `agent.{id}.lifecycle.connected` subject);
deleted when an agent disconnects cleanly.

### `.cmd.{cmd_id}` — In-flight Command Tracking

```json
{
  "command_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "type": "exec",
  "issued_at": "2026-05-10T12:34:00Z",
  "issued_by": "github:joewxboy",
  "status": "pending",
  "ack_at": null,
  "result_at": null,
  "expires_at": "2026-05-10T13:34:00Z"
}
```

`status` values: `pending` → `delivered` → `executing` → `completed` | `failed` | `expired`

TTL: set to `expires_at - now` at creation. Expired commands are cleaned up
automatically by NATS KV TTL. Results are also written to OpenBao for durability
(Phase 2 command history feature).

---

## Endpoint ID Format

Endpoint IDs are UUID v4 values with an `ep-` prefix:

```
ep-7c9e6679-7425-40de-944b-e07fc1f90ae7
```

- **Generated by:** Discovery Orchestrator, at the moment of first discovery
- **Immutable:** Never changes for the lifetime of the endpoint record
- **Uniqueness:** UUID v4 collision probability is negligible; no central sequence required
- **Stability across IP changes:** The ID remains stable when an endpoint's IP
  address changes (DHCP), the hostname changes, or the endpoint is reimaged
  (as long as the record is not explicitly deleted and recreated)

### Endpoint Deduplication

When a discovery run finds an IP/MAC already associated with an existing endpoint
record, the Discovery Orchestrator updates `network.last_seen_at` and
`network.last_seen_ip` rather than creating a new record. Deduplication key
is the MAC address (most stable identifier), with IP address as a fallback if
MAC is not available (e.g., for remote subnets where ARP is not applicable).

---

## Consequences

### Positive Impacts

**1. Clear actor boundaries**
Each actor knows exactly which OpenBao path it reads and writes. No actor should
read the `credentials` sub-path except the Credential Manager. No actor should write
`network` data except the Discovery Orchestrator. These boundaries are enforced by
OpenBao policies (per ADR-002).

**2. Credentials never co-located with operational data**
Credential values and endpoint operational data are in separate OpenBao paths with
separate access policies. Compromising the endpoint record does not expose credentials.

**3. Fast reads for common queries**
The NATS KV cache serves status and heartbeat queries in microseconds. The API
Gateway does not hit OpenBao for every list-endpoints request.

**4. Self-healing ephemeral state**
NATS KV state is rebuilt automatically when agents reconnect and heartbeat. Losing
NATS KV data (e.g., NATS restart) does not cause data loss — only a brief period
where status reads fall back to OpenBao (authoritative but slower).

**5. Extensible schema**
The tag system allows operators to organise endpoints with arbitrary metadata without
schema changes. The `additional` credentials array accommodates endpoints with multiple
access methods. The `schema_version` field enables safe migrations.

### Risks and Mitigations

| Risk | Mitigation |
|---|---|
| OpenBao unavailability blocks all reads | API Gateway falls back to NATS KV for status/heartbeat; read-only degraded mode documented |
| MAC address not available for deduplication | IP address used as fallback; manual merge operation documented for cases where the same endpoint creates two records |
| Schema drift between actors | `schema_version` checked by all actors; mismatched version causes actor to log error and skip processing rather than corrupt data |
| NATS KV TTL misconfiguration causes false OFFLINE | Heartbeat TTL (120s) and offline detection threshold (90s, ADR-005) are documented together; changing one requires reviewing the other |
| Tag proliferation makes filtering slow | Tag queries implemented via NATS KV index in Phase 2; for MVP with O(100) endpoints, full scan is acceptable |

### Implementation Considerations

- OpenBao paths use the KV v2 secrets engine, which provides versioning and
  metadata. Actors use versioned reads (`?version=N`) when they need consistency
  guarantees across sub-paths.
- The Endpoint Manager is the only actor that performs status transitions in
  OpenBao `core`. Other actors write to their own sub-paths (`network`, `identity`,
  `agent`) but must not write `core.status` directly.
- NATS KV keys use dots as separators (`{endpoint_id}.heartbeat`). Key names
  must not contain dots in the endpoint ID portion — the `ep-{uuid}` format
  satisfies this since UUIDs use hyphens.
- For MVP, metrics history (CPU, memory over time) is not retained. The last
  heartbeat metrics are available in NATS KV; historical trends are a Phase 2
  feature (time-series store TBD).

---

## Alternatives Considered

### Alternative 1: Single Flat Record per Endpoint in OpenBao

Store all endpoint data in a single JSON blob at `secret/endpoints/{id}`.

**Decision:** Rejected

**Rationale:** A single flat record mixes data with very different access patterns and
owners. Discovery data changes frequently; identity data changes rarely; credential
references change only when operators update them. Separate sub-paths allow
fine-grained OpenBao policies (the Identifier can write `identity` but not `network`),
version each sub-document independently, and avoid large write amplification when
only one field changes.

### Alternative 2: PostgreSQL / SQLite for Structured State

Use a relational database for endpoint records instead of OpenBao + NATS KV.

**Decision:** Rejected for MVP

**Rationale:** Adding a database server is a significant operational dependency.
OpenBao already provides encrypted-at-rest storage with access control and audit
logging — properties a raw database would not provide without additional tooling.
NATS KV provides the fast ephemeral layer. The combination covers the MVP use case
without a third infrastructure component. A database backend may be reconsidered
in Phase 3 for complex query requirements (tag-based filtering at scale, time-series
metrics, command history).

### Alternative 3: NATS JetStream Object Store for All State

Store all endpoint data in NATS JetStream rather than splitting across OpenBao and
NATS KV.

**Decision:** Rejected

**Rationale:** NATS JetStream does not provide encryption at rest or access control
policies at the record level. Credential references stored in NATS would be accessible
to any actor with NATS access. OpenBao's security model is the correct layer for
data that references sensitive credentials. Ephemeral operational state (heartbeat,
in-flight commands) is appropriate for NATS KV because it is non-sensitive and
benefits from NATS's low-latency read/write performance.

---

## Related Decisions

- **ADR-001:** WasmCloud Control Plane — defines actor ownership boundaries
  that this schema enforces via OpenBao path separation
- **ADR-002:** Credential Storage — credential reference paths in `credentials`
  sub-document point into OpenBao paths defined in ADR-002
- **ADR-003:** Network Discovery — Discovery Orchestrator writes `network` sub-path
- **ADR-004:** Endpoint Identification — Identifier writes `identity` sub-path
- **ADR-005:** Agent Communication Protocol — agent heartbeats write to NATS KV
  `.heartbeat` keys; command tracking in `.cmd.{cmd_id}` keys
- **ADR-006:** Agent Deployment — Agent Coordinator writes `agent` sub-path
- **ADR-007:** API Auth — `added_by` fields reference JWT `sub` claim format

---

## Open Questions

1. **Metrics history:** When does Host Manager need to retain historical metric
   data (CPU, memory trends over time)? This drives whether a time-series store
   is needed in Phase 2. Deferred pending operator feedback after MVP.

2. **Command history:** Should completed command results be retained in OpenBao
   for audit purposes? If so, for how long? Likely yes for Phase 2; retention
   period TBD (default: 90 days).

3. **Endpoint record deletion vs. DECOMMISSIONED status:** When an operator
   decommissions an endpoint, should the record be soft-deleted (status →
   DECOMMISSIONED, data retained) or hard-deleted from OpenBao? Soft-delete
   preferred for auditability; hard-delete as an explicit operator option.

4. **Multi-control-plane state sharing:** In Phase 3 (distributed deployments),
   how do two control planes share or partition endpoint state? NATS supercluster
   replication for KV; OpenBao replication for durable records. Deferred to Phase 3.

---

## Decision Record

**Approved by:** [To be filled after review]

**Date approved:** [To be filled]

**Concerns / feedback addressed:** [To be filled after discussion]
