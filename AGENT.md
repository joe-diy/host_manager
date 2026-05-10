# Host Manager project

This is an Apache 2.0 licensed open-source project to create a platform to manage a fleet of heterogeneous endpoints (hosts, clusters, VMs) running on a network (usually a local network, not the open internet).

## Vision
Small, fast, lightweight platform with a pluggable architecture featuring swappable modular components. Target: WASM-compiled code with WasmCloud integration where it makes sense.

## Supported Endpoints
1. **Linux servers and Raspberry Pis** — bare metal Linux nodes
2. **VM platforms** — Zededa Cloud, Mainsail Industries' Starlight, and similar VM management systems
3. **CNCF Kubernetes clusters** — any Kubernetes distribution

## MVP Use Cases (Phase 1)
1. **Auto-detection of endpoints** — discover endpoints on the network
2. **Endpoint identification** — detect endpoint type and hardware (e.g., Raspberry Pi 3B+ 1G)
3. **Credential storage** — securely store credentials for endpoint access, including optional sudo support

## Architecture Approach
- Pluggable, modular design with swappable components
- WASM compilation for both control plane and agents (where sensible)
- Exploration of WasmCloud runtime

## Planning Process
Detailed interactive planning sessions with ADRs and design artifacts before implementation. Validate WASM feasibility for required capabilities.

## Team
Solo for now, with potential for team expansion if the project shows promise.
