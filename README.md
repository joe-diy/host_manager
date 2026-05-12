# Host Manager

Host Manager is an Apache-2.0 open-source platform for managing heterogeneous endpoints across a network, including Linux hosts, VMs, and Kubernetes clusters.

## Overview

The project is designed as a small, fast, modular control plane with pluggable components and WasmCloud integration where appropriate.

Phase 1 MVP goals:
- Automatic endpoint discovery on local networks
- Endpoint identification (type and hardware)
- Secure credential storage for endpoint access

## Repository Structure

- `agent/` – native Host Manager agent binary
- `cli/` – `hostmgr` CLI
- `crates/types` – shared domain types
- `crates/protocol` – shared protocol/subject/message definitions
- `actors/` – WasmCloud actors (API gateway, discovery orchestration, endpoint management, etc.)
- `providers/` – native capability providers
- `ui/` – React + TypeScript frontend
- `deploy/` – Docker Compose stack and Helm chart
- `config/` – NATS/OpenBao/TLS configuration

## Prerequisites

- Rust stable toolchain
- `cargo fmt`, `clippy`
- Node.js 22+ and npm (for UI)
- Helm (for chart linting)
- Docker + Docker Compose (for local stack)
- WasmCloud tooling (`wash`) for actor builds/deployments

## Build and Verify

### Rust workspace checks

```bash
cargo fmt --all -- --check
cargo check -p hostmgr-types -p hostmgr-protocol -p hostmgr-provider-discovery -p hostmgr-provider-identification -p hostmgr-provider-credentials -p hostmgr-provider-mcp-client -p hostmgr-agent -p hostmgr
cargo clippy -p hostmgr-types -p hostmgr-protocol -p hostmgr-agent -p hostmgr -- -D warnings
cargo test -p hostmgr-types -p hostmgr-protocol
```

### UI checks

```bash
cd ui
npm install
npm run type-check
npm run lint || true
npm run build
```

### Helm lint

```bash
helm lint deploy/charts/hostmgr
```

## Local Development Stack

From `deploy/docker-compose`:

```bash
cp .env.example .env
docker compose up -d
```

The stack starts:
- NATS (JetStream + WebSocket/TLS)
- OpenBao (dev mode)
- WasmCloud host

## License

Apache-2.0
