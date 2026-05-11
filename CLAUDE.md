# Host Manager — Project Guide for AI Assistants

Host Manager is an Apache 2.0 open-source platform for managing heterogeneous fleets
of endpoints (Linux servers, Raspberry Pis, VMs, Kubernetes clusters) on a local network.

## Architecture at a Glance

The control plane runs as WasmCloud actors communicating over NATS. Agents run on
managed endpoints and connect back over NATS WebSocket (port 443, TLS 1.3). All
credential and endpoint state is stored in OpenBao.

Full decisions are documented in `docs/adr/`. Read those before making architectural changes.

## Repository Layout

```
host_manager/
├── actors/              # WasmCloud actors — compiled to wasm32-wasip2
│   ├── endpoint-manager/       # Lifecycle orchestrator; owns state transitions
│   ├── discovery-orchestrator/ # Triggers and tracks discovery runs
│   ├── identifier/             # Triggers and tracks identification runs
│   ├── credential-manager/     # All credential reads/writes via OpenBao
│   ├── api-gateway/            # REST API + serves React UI; OAuth + API key auth
│   └── agent-coordinator/      # Agent deployment, update approval, presence tracking
│
├── providers/           # Native capability providers — compiled for host OS
│   ├── discovery/       # ARP + mDNS network scanning (ADR-003)
│   ├── identification/  # SSH probing to detect OS/type (ADR-004)
│   ├── credentials/     # OpenBao client (ADR-002)
│   └── mcp-client/     # MCP protocol client for external enrichment (ADR-011)
│
├── crates/              # Shared library crates (native + wasm32-wasip2 compatible)
│   ├── types/           # Endpoint, DiscoveryResult, IdentificationResult, etc.
│   └── protocol/        # NATS subject constants and message envelope types
│
├── agent/               # hostmgr-agent binary — runs on managed endpoints
│                        # NATS WSS primary transport, HTTPS polling fallback (ADR-005)
│
├── cli/                 # hostmgr CLI binary — thin REST client for operators
│
├── ui/                  # React 19 web UI — bundled into WasmCloud host image (ADR-011)
│
├── deploy/
│   ├── docker-compose/  # Local development environment
│   ├── charts/hostmgr/  # Helm 3 chart for Kubernetes testing
│   └── open-horizon/    # Open Horizon service definition for production edge
│
└── docs/adr/            # Architecture Decision Records (ADR-001 through ADR-011)
```

## Key Architecture Decisions (summary — read ADRs for detail)

| Decision | Choice | ADR |
|---|---|---|
| Control plane runtime | WasmCloud 1.x + NATS | ADR-001 |
| Credential storage | OpenBao (primary), Vault (secondary) | ADR-002 |
| Network discovery | ARP + mDNS via capability provider | ADR-003 |
| Endpoint identification | SSH probing via capability provider | ADR-004 |
| Agent transport | NATS WSS port 443, TLS 1.3; HTTPS polling fallback | ADR-005 |
| Agent deployment | Manual install (MVP); FDO Phase 3 | ADR-006 |
| Human auth | GitHub / Google OAuth; HttpOnly cookie | ADR-007 |
| Machine auth | API keys (`hm_ro_` / `hm_rw_` prefix) | ADR-007, ADR-011 |
| Endpoint state | OpenBao (durable) + NATS KV (ephemeral) | ADR-008 |
| Packaging | Docker Compose → Helm → Open Horizon → Operator SDK | ADR-009 |
| Agent updates | Pull-based; manual approval default | ADR-010 |
| Web UI | React 19, bundled into WasmCloud host image | ADR-011 |
| MCP | Consume external MCP servers (Phase 1.1) | ADR-011 |

## Building

### Prerequisites

```bash
# Rust toolchain (managed by rust-toolchain.toml)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-wasip2

# WasmCloud CLI
cargo install wash-cli

# Node.js 18+ (for UI)
# https://nodejs.org/
```

### Native crates (providers, agent, CLI)

```bash
cargo build                          # all native crates
cargo test --workspace               # all tests
cargo clippy --workspace             # lints
```

### Actor crates (WasmCloud components)

```bash
# Build a single actor
cd actors/endpoint-manager && wash build

# Build all actors
for d in actors/*/; do (cd "$d" && wash build); done
```

### UI

```bash
cd ui && npm install && npm run dev   # development (proxies API to localhost:8080)
cd ui && npm run build                # production build → ui/dist/
```

### Local development environment

```bash
cd deploy/docker-compose
cp .env.example .env                  # fill in OAuth credentials
docker compose up -d
# control plane available at http://localhost:8080
```

## NATS Subject Conventions

All subjects are defined as constants in `crates/protocol/src/subjects.rs`.

```
discovery.start              → Discovery Orchestrator: begin a run
discovery.complete           → Endpoint Manager: run finished
endpoint.{id}.identify       → Identifier: probe this endpoint
endpoint.{id}.identified     → Endpoint Manager: identification complete
agent.{id}.cmd.{type}        → Agent: execute a command
agent.{id}.status.heartbeat  → Agent Coordinator: agent is alive
agent.{id}.lifecycle.*       → Agent Coordinator: connect/disconnect events
```

## OpenBao Path Conventions

All paths are defined as constants in `crates/protocol/src/vault_paths.rs`.

```
secret/endpoints/{id}/core          ← lifecycle status, timestamps, tags
secret/endpoints/{id}/network       ← IPs, MACs, hostnames
secret/endpoints/{id}/identity      ← OS, arch, type
secret/endpoints/{id}/agent         ← agent version, deployment metadata
secret/endpoints/{id}/credentials   ← credential reference paths only
secret/credentials/{id}/ssh         ← actual SSH key/password
secret/agents/{id}/nkey             ← agent NKey private key
secret/config/oauth/*               ← OAuth client secrets
secret/config/jwt_signing_key       ← Ed25519 JWT signing key
secret/config/api_keys/{key_id}     ← hashed API keys
```

## NATS KV Bucket Conventions

```
ENDPOINT_STATE / {id}.status        ← current lifecycle status (fast read)
ENDPOINT_STATE / {id}.heartbeat     ← last heartbeat + metrics (TTL: 120s)
ENDPOINT_STATE / {id}.connection    ← active connection details
ENDPOINT_STATE / {id}.cmd.{cmd_id} ← in-flight command tracking
```

## Endpoint Lifecycle States

```
DISCOVERED → IDENTIFIED → AGENT_DEPLOYING → MANAGED ⇄ OFFLINE
                                                     ⇄ DEGRADED
                                                     → DECOMMISSIONED
```

## Environment Variables (MVP)

See `deploy/docker-compose/.env.example` for the full list.
Key variables:

```bash
HOSTMGR_EXTERNAL_URL          # https://control.example.com
HOSTMGR_NATS_URL              # nats://localhost:4222
HOSTMGR_OPENBAO_URL           # https://localhost:8200
HOSTMGR_ALLOWED_USERS         # github:joewxboy,google:joe@example.com
HOSTMGR_AUTH_GITHUB_CLIENT_ID # GitHub OAuth App Client ID
HOSTMGR_AUTH_GOOGLE_CLIENT_ID # Google OAuth App Client ID
HOSTMGR_LOG_LEVEL             # trace|debug|info|warn|error
```

## Code Style

- `cargo fmt` before every commit (enforced in CI)
- `cargo clippy -- -D warnings` must pass (enforced in CI)
- All public types/functions have doc comments
- Errors use `thiserror` in libraries, `anyhow` in binaries
- No `unwrap()` in production paths — use `?` or explicit error handling
- NATS subjects and OpenBao paths come from `crates/protocol` constants, never hardcoded strings
