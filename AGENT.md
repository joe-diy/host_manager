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

---

## Learnings (updated as the project evolves)

### Rust / Cargo workspace

- **Package naming matters for `use` statements.** Crates named `types` and `protocol`
  are imported as `types::` and `protocol::` in Rust source. Renaming them to
  `hostmgr-types` and `hostmgr-protocol` makes imports read as `hostmgr_types::` and
  `hostmgr_protocol::`, which is clearer in a multi-crate workspace and avoids
  collisions with common names.

- **Workspace dependency keys must match package names.** If `crates/types/Cargo.toml`
  declares `name = "hostmgr-types"`, the workspace `[workspace.dependencies]` key must
  also be `hostmgr-types`. Mismatches silently resolve to the wrong crate or fail to
  compile.

- **`cargo fmt --all` is toolchain-version-sensitive.** Running `cargo fmt` with an old
  pinned toolchain (e.g. 1.85.0) and then switching `rust-toolchain.toml` to `stable`
  leaves files that older rustfmt accepted but newer rustfmt wants to reformat. Always
  run `cargo fmt --all` *after* the final toolchain is set, and verify locally with
  `cargo fmt --all -- --check` before pushing. Use `rustup update stable` first to
  ensure local == CI.

- **`cargo check` does not work for WasmCloud actor crates without vendored WIT deps.**
  Actor crates use `wit_bindgen::generate!()` which requires the WIT interface files
  from `wit/deps/`. These are fetched automatically by `wash build` but not by plain
  `cargo check --target wasm32-wasip2`. Keep actor `src/lib.rs` files as plain Rust
  stubs (no `wit_bindgen::generate!` call) for `cargo check` compatibility; the full
  implementation is wired in when running `wash build`.

- **`cargo clippy -- -D warnings` catches real scaffold issues early.** Caught during
  initial CI setup:
  - `enum_variant_names`: a variant of enum `Commands` named `Commands` — flattened
    away by removing the unnecessary single-variant wrapper struct.
  - `single_component_path_imports`: bare `use serde_json;` is redundant when only
    macros/paths from the crate are used.
  - `wildcard_in_or_patterns`: `"manual" | _` — the literal is unreachable; just use `_`.
  - `dead_code` on scaffolded struct fields: use `#[allow(dead_code)]` with a comment
    explaining which future feature will read them, rather than silently removing them.

### WasmCloud / WASM

- **WasmCloud actors are built with `wash build`, not `cargo build`.** `wash build`
  fetches WIT dependency packages, generates bindings, compiles to `wasm32-wasip2`, and
  produces a signed `.wasm` component. Plain `cargo build --target wasm32-wasip2` skips
  all of that.

- **Each actor needs four files:** `Cargo.toml` (with `crate-type = ["cdylib"]`),
  `wasmcloud.toml` (build metadata for wash), `wit/world.wit` (the WIT world the actor
  implements), and `src/lib.rs` (the implementation).

- **WIT world imports define the actor's capability requirements.** A messaging actor
  imports `wasmcloud:messaging/consumer` (to publish) and exports
  `wasmcloud:messaging/handler` (to receive). The `api-gateway` exports
  `wasi:http/incoming-handler` instead. The `credential-manager` additionally imports
  `wasmcloud:secrets/store`.

### Agent transport (ADR-005)

- **NATS WSS on port 443 + HTTPS polling fallback** was chosen over plain NATS TCP on
  port 4222. Port 443 requires no new firewall rules. TLS 1.3 is mandatory (ECDHE is
  unconditional at the protocol level, guaranteeing Perfect Forward Secrecy). This
  matches the Open Horizon / anax model that has proven production-ready.

- **Bootstrap uses a single-use token** (5-min TTL, stored in OpenBao). The agent
  POSTs the token to `/api/v1/agents/bootstrap`, receives an NKey seed and endpoint ID,
  writes them to `~/.config/hostmgr/` with mode 0600, and clears the env var. On
  subsequent starts the agent reads the persisted credentials and skips bootstrap.

### CI / CD

- **Always pin Helm to `latest` or a full semver in `azure/setup-helm`.** The string
  `"3.16"` is not a valid semver tag for the Helm installer download URL and will 404.
  Use `"latest"` or an exact version like `"3.16.4"`.

- **Separate CI jobs for native crates vs. actors.** The `rust` job runs `cargo check`,
  `cargo clippy`, and `cargo test` against the native targets only. A separate
  `build-actors` job uses `wash build` to compile actor crates. This avoids impossible
  `cargo check --target wasm32-wasip2` runs that would fail without vendored WIT deps.

- **Cross-compile the agent for musl targets** (`x86_64`, `aarch64`, `armv7`) using
  the `cross` tool in the release workflow. This produces fully static binaries that
  run on any Linux system without libc version constraints — important for Raspberry Pi
  and older distros.

### React UI (ADR-011)

- **Bundling the React UI into the WasmCloud host image eliminates OAuth complexity.**
  When the UI and the API Gateway actor share the same origin, the browser sends
  HttpOnly session cookies automatically. There is no need for CORS, OTC (one-time
  code) exchange, JWT-in-memory storage, or cross-origin credential passing.

- **Same-origin serving means `credentials: "same-origin"` in fetch calls is enough.**
  No `Authorization` header is needed for browser sessions. API key auth (`hm_ro_…` /
  `hm_rw_…`) uses the `Authorization: Bearer` header and is only needed for CLI / MCP
  clients.

- **TanStack Router + TanStack Query** is the chosen stack for React 19. Router handles
  type-safe client-side routing (no Next.js server dependency needed since the UI is
  fully static). Query handles data fetching with 30-second stale time (fleet state
  doesn't change every second).
