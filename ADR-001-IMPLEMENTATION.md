# ADR-001: Control Plane Runtime — First Pass Implementation

## Overview

This document tracks the MVP implementation of ADR-001: WasmCloud 1.x + NATS control plane.

## Completed

### Infrastructure
- [x] **docker-compose.yml**: Complete stack definition
  - NATS 2.10 with JetStream, plain TCP (4222), and WebSocket over TLS (443)
  - OpenBao 2.0 (dev mode) on port 8200
  - WasmCloud 1.4 host on port 8080
  
- [x] **TLS certificates**: Self-signed certs for NATS WebSocket (`config/tls/`)
  - For production: replace with real certificates from Let's Encrypt or similar

- [x] **NATS configuration** (`config/nats.conf`):
  - JetStream enabled for durable streams and KV buckets
  - WebSocket TLS on port 443 (primary agent transport per ADR-005)
  - Plain TCP on 4222 (dev/local only)
  - HTTP monitoring on 8222

- [x] **OpenBao configuration** (`config/openbao.hcl`):
  - File storage (dev mode in docker-compose uses `-dev` flag)
  - TLS listener configured (not used in dev mode)
  - Default lease TTLs set

### Application Manifest
- [x] **wadm.yaml**: Minimal deployment manifest (empty components for MVP)

## In Progress

### API Gateway Actor
- [x] **actors/api-gateway**: WasmCloud component (wasm32-wasip2)
  - Status: WASM artifact built (`target/wasm32-wasip2/debug/api_gateway.wasm`)
  - Implements WASI HTTP handler with health check endpoint
  - Issue: `wash build` post-processing fails with workspace (ring crate incompatibility)
  - Workaround: WASM artifact is ready; manual bundling may be needed or wash version update

## Known Issues

1. **wash build** post-processing fails on workspace cargo builds
   - Cause: `ring` crate (used by rustls deps) doesn't support wasm32-wasip2 target
   - Impact: Can't bundle WASM into WasmCloud component format via `wash build`
   - Workaround: Build individual actors from their own directory, or manually create component bundle
   - Fix: May resolve with newer wash version or conditional dependencies
   
2. **WASI HTTP API complexity** (resolved)
   - OutgoingBody::finish() takes (body, Option<Fields>)
   - ResponseOutparam::set() must be called to send response
   - Solution: Use impl block directly, avoid Result type

## Next Steps (Phase 1: Verify Control Plane)

### 1. Resolve Actor Packaging
**Goal**: Get api-gateway bundled as a WasmCloud component and deployable

- [ ] **Option A**: Update wash CLI (check for newer version that handles workspaces better)
- [ ] **Option B**: Build actors individually without workspace context
  - Move api-gateway to standalone `cargo build --target wasm32-wasip2`
  - Use wash from that directory only
- [ ] **Option C**: Manual component wrapping
  - Use `wit-component` directly to wrap WASM + WIT
  - Create component signing workflow without wash

**Acceptance**: `wadm.yaml` can deploy api-gateway and it responds to health checks

### 2. Test Control Plane Startup
**Goal**: Verify WasmCloud + NATS + OpenBao work together

```bash
# In deploy/docker-compose:
docker compose up -d
sleep 30  # wait for services to stabilize

# Verify each service:
docker compose logs nats | grep "JetStream enabled"
docker compose logs openbao | grep "put the unseal key"
docker compose logs wasmcloud | grep "wasmcloud host started"

# Check NATS JetStream:
docker exec hostmgr-nats nats stream ls
docker exec hostmgr-nats nats kv ls

# Check OpenBao:
curl -sk https://localhost:8200/v1/sys/health

# Check WasmCloud API:
curl http://localhost:8080/api/v1/health
```

**Acceptance**: All three services start, communicate over internal network, health endpoints respond

### 3. Deploy api-gateway Actor
**Goal**: Get first actor running in WasmCloud and serving HTTP

- [ ] Resolve packaging issue (Phase 1, step 1)
- [ ] Update wadm.yaml with api-gateway component definition
  - Reference built WASM artifact
  - Define HTTP capability link
  - Set port 8080
- [ ] Deploy: `wash app deploy wadm.yaml`
- [ ] Verify: `curl http://localhost:8080/api/v1/health` returns 200 OK

**Acceptance**: Health endpoint is reachable through WasmCloud actor

---

## Next Steps (Phase 2: Add Core Actors)

### 4. Implement endpoint-manager Actor
**Goal**: Create state machine orchestrator for endpoint lifecycle

Files to create:
- `actors/endpoint-manager/src/lib.rs` — main actor logic
- `actors/endpoint-manager/wit/world.wit` — actor interface
- Update `wadm.yaml` to deploy and link to NATS consumer

Responsibilities:
- Subscribe to `endpoint.{id}.identified` events from identifier
- Manage state transitions: `DISCOVERED → IDENTIFIED → AGENT_DEPLOYING → MANAGED`
- Write durable state to OpenBao (`secret/endpoints/{id}/core`)
- Publish state change events to NATS

NATS subjects (defined in `crates/protocol/src/subjects.rs`):
- Import: `endpoint.{id}.identified` (BrokerMessage with IdentificationResult)
- Export: `endpoint.{id}.status` (endpoint status updates)

OpenBao paths (defined in `crates/protocol/src/vault_paths.rs`):
- Write: `secret/endpoints/{id}/core` (lifecycle status, timestamps)

WIT interface:
```wit
import wasmcloud:messaging/consumer@0.2.0;
import wasmcloud:secrets/reveal@0.2.0;  # for OpenBao reads/writes
```

**Acceptance**: Actor deploys, listens on NATS, persists state to OpenBao

### 5. Implement discovery-orchestrator Actor
**Goal**: Trigger and track network discovery runs

Responsibilities:
- Subscribe to `discovery.start` (HTTP endpoint will call this)
- Coordinate with discovery provider (capability provider, ADR-003)
- Collect ARP + mDNS results
- Publish `discovery.complete` with DiscoveryResult
- Update endpoint state in OpenBao

NATS subjects:
- Import: `discovery.start` (empty or with filters)
- Export: `discovery.complete` (DiscoveryResult)

Capability links:
- `wasmcloud:discovery` (native provider from `providers/discovery`)

**Acceptance**: Can trigger discovery via REST, results published to NATS, state updated

### 6. Implement credential-manager Actor
**Goal**: Proxy all secret reads/writes through OpenBao

Responsibilities:
- Subscribe to credential requests from other actors (RPC-style)
- Call OpenBao API (ADR-002)
- Cache credentials in memory with TTL (for performance)
- Support read, write, rotate operations

OpenBao paths:
- Read/write: `secret/credentials/{id}/ssh`
- Read/write: `secret/endpoints/{id}/credentials`

**Acceptance**: Other actors can request credentials via NATS, OpenBao integration works

---

## Next Steps (Phase 3: Verify NATS Communication)

### 7. Test Full NATS Flow
**Goal**: Verify pub/sub and request-reply patterns work end-to-end

Test scenario:
1. Trigger discovery via: `curl -X POST http://localhost:8080/api/v1/discovery/start`
2. discovery-orchestrator should publish to `discovery.complete`
3. endpoint-manager should receive and process results
4. State should appear in OpenBao: `curl -sk https://localhost:8200/v1/secret/endpoints/...`

Manual verification:
```bash
# Terminal 1: Watch NATS messages
docker exec hostmgr-nats nats sub "discovery.*"

# Terminal 2: Trigger discovery
curl -X POST http://localhost:8080/api/v1/discovery/start

# Terminal 3: Check OpenBao state
curl -sk https://localhost:8200/v1/secret/endpoints/ -H "X-Vault-Token: dev-only-token"
```

**Acceptance**: Messages flow through NATS, state persists in OpenBao

### 8. Implement Request-Reply Pattern
**Goal**: Support RPC-style calls between actors (needed for agent bootstrap, commands)

Pattern:
- Caller publishes to `actor.{name}.request.{id}` with inbox subject in header
- Callee processes, publishes response to inbox
- Caller subscribes on inbox with timeout

Example (credential lookup):
```
Request: actor.credential-manager.request.{uuid}
  Body: CredentialRequest { endpoint_id: "ep-123", type: "ssh" }
  Reply-To: _INBOX.{random}

Response: _INBOX.{random}
  Body: CredentialResponse { private_key: "...", username: "root" }
```

Actors need this for:
- Identifier asking credential-manager for SSH keys
- Agent coordinator asking endpoint-manager for bootstrap token
- CLI asking endpoint-manager for endpoint details

**Acceptance**: Bidirectional RPC patterns work between actors

---

## Deployment Checklist

Before marking ADR-001 complete:

- [ ] All 6 core actors deployed and linked in `wadm.yaml`
- [ ] Each actor logs startup message on deploy
- [ ] Health endpoint returns 200 OK consistently
- [ ] NATS JetStream streams for durable messages
- [ ] NATS KV bucket for ephemeral endpoint state
- [ ] OpenBao integration working (read/write secrets)
- [ ] Request-reply RPC pattern verified
- [ ] docker-compose stack starts cleanly and passes all healthchecks
- [ ] CLAUDE.md updated with operational notes

## Running Locally (once ready)

```bash
# Prerequisites
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-wasip2
cargo install wash-cli

# Setup
cd deploy/docker-compose
cp .env.example .env
docker compose up -d

# Deploy
wash app deploy wadm.yaml

# Monitor
wash app status hostmgr
nats pub discovery.start ""  # if NATS client is available
```

## Architecture Notes

- **Control plane**: All state orchestration via WasmCloud actors on NATS
- **Agent transport**: NATS WebSocket (TLS 1.3) on 443; fallback HTTPS polling (ADR-005)
- **Credential storage**: All secrets in OpenBao (path conventions in protocol crate)
- **State**: Durable in OpenBao, ephemeral in NATS KV buckets
- **Development**: Plain TCP NATS + self-signed certs; production replaces with real TLS

## References

- **CLAUDE.md**: Full project context and conventions
- **ADR-001**: WasmCloud + NATS decision
- **ADR-002**: Credential storage (OpenBao)
- **ADR-005**: Agent transport (NATS WebSocket)
- **ADR-008**: Endpoint state (dual storage)
