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
- [ ] **actors/api-gateway**: WasmCloud component (wasm32-wasip2)
  - Status: Implementing WASI HTTP handler
  - Currently debugging wit-bindgen HTTP response API
  - Goal: Serve `/api/v1/health` for docker-compose healthcheck

## Known Issues

1. **wash build** producing WASM artifacts but failing on post-processing
   - Workaround: May need to manually orchestrate the build process or check wash version
   
2. **WASI HTTP API complexity**: Response handling requires careful API usage
   - OutgoingBody::finish() takes (body, Option<Fields>)
   - ResponseOutparam::set() must be called to send response

## Next Steps (for MVP → real implementation)

1. **Fix api-gateway build**: Get one working actor that serves health endpoint
2. **Test control plane**: Bring up docker-compose and verify:
   - WasmCloud host starts and connects to NATS
   - Docker healthcheck passes (GET /api/v1/health → 200 OK)
   - NATS JetStream is operational
   - OpenBao is accessible

3. **Add basic actors**: Implement minimal versions of:
   - `endpoint-manager`: Listens on NATS for lifecycle events
   - `discovery-orchestrator`: Simple NATS pub/sub
   - `credential-manager`: Mock responses from OpenBao

4. **Verify NATS communication**: 
   - Publish test messages to discovery.start
   - Verify endpoint-manager receives them
   - Implement request-reply pattern

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
