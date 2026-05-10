# WASM Feasibility Research

Validate that WASM/WasmCloud can support Host Manager's required capabilities.

## Research Questions

### Capability: Endpoint Discovery & Detection
- [ ] Can WASM modules perform network scanning (ARP, mDNS, port scanning)?
- [ ] What network APIs are available in WASM runtime (component model, wasmtime)?
- [ ] Can discovery agents run in WasmCloud runtime?
- [ ] Performance constraints: How fast can discovery run in WASM vs. native?

### Capability: System Identification
- [ ] Can WASM access system information APIs (hardware detection, OS info)?
- [ ] Available WASM System Interface (WASI) capabilities for identifying:
  - CPU model, RAM, disk
  - OS version and distribution
  - Kubernetes version/cluster info
- [ ] Performance of system introspection in WASM vs. native

### Capability: Credential Storage & Access
- [ ] WASM security model for storing sensitive data (credentials, keys)
- [ ] Available WASM APIs for:
  - Secure storage (WASI? host-provided?)
  - Encryption/decryption
  - Access control within WASM runtime
- [ ] How do agents securely access stored credentials?
- [ ] Can sudo be invoked safely from WASM agents?

### Architecture: Control Plane vs. Agents
- [ ] Is it feasible to run control plane in WASM? (state management, coordination)
- [ ] Is it feasible to run agents in WASM? (resource constraints, native library dependencies)
- [ ] Alternative: WASM for control plane, native for agents (or vice versa)?

### WasmCloud Integration
- [ ] Can WasmCloud provider be built for Host Manager's credential storage?
- [ ] Can WasmCloud host be used to manage distributed agent deployments?
- [ ] Examples of WasmCloud deployments with similar patterns?

## Findings

### WasmCloud 2.x Maturity & Status

**Status:** CNCF Incubating (as of Nov 2024); actively developed with recent HTTP2/gRPC support and Wasmtime 42 updates.

**Production Use:**
- Limited real-world production usage at scale for general-purpose workloads
- American Express built internal FaaS on WasmCloud (specialized use case)
- No widespread general-purpose microservices backends running WASM in production as of early 2026

**Verdict:** WasmCloud is mature enough for experimentation and specialized workloads, but not battle-tested for large-scale distributed systems yet. Good option for learning and MVP, acceptable risk for MVP phase.

---

### WASM Runtime Landscape (2.x versions)

#### **Wasmtime** (Standards-Focused)
- **Latest:** Wasmtime 42+ (targeting WASI Preview 2 with WASIp3 snapshot in progress)
- **Strengths:**
  - First runtime with full WASI Preview 2 support (0.2.0)
  - Winch compiler for cold-start performance, now supports AArch64 (critical for RPi edge deployment)
  - Most ergonomic Rust API of any WASM runtime
  - Standards-focused: only implements official specs
- **Use Case:** Best for standards compliance and edge (ARM) workloads

#### **Wasmer** (Pragmatism-Focused)
- **Latest:** Versions 5.0-6.0 (Meta-Runtime with pluggable backends)
- **Strengths:**
  - WASIX (superset of WASI with fork(), networking, threads—not waiting for standard)
  - Broader language embedding: PHP, Ruby, Swift, Python, Go, Rust, .NET
  - Multiple code generation backends: LLVM (peak performance), Cranelift (balanced), V8 (Chrome engine)
  - Dynamic backend selection at runtime
- **Limitation:** WASIX binaries only run on Wasmer (vendor lock-in), not portable

**Verdict:** For Host Manager, **Wasmtime** is the better choice because portability and standards matter; you want your components to work across runtimes and environments.

---

### WASI Network Capabilities

#### **WASI 0.2 (Current Stable)**
- **Features:** TCP/UDP sockets, wasi-http (HTTP client/server)
- **Security:** Capability-based access control; modules cannot open sockets without capability handles; deny-by-default firewalling
- **Limitation:** Callback-based async makes network I/O awkward; no native async/await
- **Network Discovery:** Direct socket APIs exist (TCP/UDP), but ARP/mDNS scanning would require raw socket access (not granted by default)

#### **WASI 0.3 (Released Feb 2026)**
- **Headline Feature:** Native async/await built into Component Model via future<T> and stream<T> types
- **Benefit:** Eliminates callback hell; removes awkward workarounds for concurrent I/O
- **Timeline:** Production-ready previews available now; WASI 1.0 targeted for late 2026
- **Impact on Host Manager:** Critical for control plane to manage multiple endpoint connections concurrently

**Verdict:** WASI 0.3 async is essential for your control plane. MVP should plan to adopt WASI 0.3 even if it's preview; it solves a core pain point.

---

### Network Discovery & Endpoint Detection

**Challenge:** WASM sandboxing prevents raw network access by default.

**Options:**

1. **Host-Delegated Discovery (Recommended)**
   - Control plane (WASM or native) calls host capability provider: "discover endpoints"
   - Host performs ARP/mDNS scanning in native code
   - Results returned to control plane as capability response
   - **Pros:** Secure, no privilege escalation, clean separation
   - **Cons:** Control plane can't perform discovery natively in WASM

2. **WASM + Host Network Privileges**
   - Grant WASM module raw socket capability (not standard WASI)
   - Requires Wasmer or custom Wasmtime extensions
   - **Pros:** Discovery logic in WASM
   - **Cons:** Security risk, reduces portability, vendor lock-in (Wasmer)

3. **Hybrid: WASM Control Plane + Native Discovery Agent**
   - Control plane in WASM/WasmCloud
   - Separate native discovery service providing capability interface
   - **Pros:** Best of both worlds; keeps security tight
   - **Cons:** Adds complexity; discovery runs external to control plane

**Verdict:** **Option 1 (Host-Delegated)** is best for MVP. Build a capability provider that wraps native ARP/mDNS libraries.

---

### System Identification (Hardware Detection)

**Challenge:** WASI provides limited system introspection.

**Available WASI Interfaces:**
- `wasi:filesystem` — file/directory access (WASI Preview 2)
- `wasi:process` — limited process information
- **Missing:** Direct CPU model, RAM, disk, hardware serial numbers

**Workarounds:**

1. **Host Capability Provider (Recommended)**
   - WASM components call host capability: "get-system-info"
   - Host runs `lsb_release`, `/sys`, `/proc`, `dmidecode` in native code
   - Returns structured data (CPU, RAM, HW type, Kubernetes version)
   - **Pros:** Clean, portable, no privilege needed
   - **Cons:** Host dependency; can't be purely WASM

2. **WASM with File System Access**
   - Grant `/sys` and `/proc` read access via WASI capabilities
   - Parse in WASM code
   - **Pros:** Works on Linux; data stays in WASM
   - **Cons:** Linux-only; `/proc` parsing is fragile; requires privilege (root) for some data

3. **Custom Hardware Detection Library**
   - Compile existing hardware detection libs (CPU, dmidecode) to WASM
   - **Pros:** Portable; hardware detection logic in WASM
   - **Cons:** Some libraries have OS-specific system calls WASM can't make (e.g., IOCTL for Raspberry Pi model detection)

**Verdict:** **Option 1 + 2 Hybrid** for MVP: Host capability provider for privileged data (hardware serial, full system info); WASM can read `/proc` for basic CPU/RAM on Linux nodes.

---

### Credential Storage & Access

**WasmCloud Secrets Architecture (Excellent):**
- **Design:** Pluggable backends (Vault, AWS Secrets Manager, K8s secrets, in-house)
- **Security:** Just-in-time secret resolution; encrypted in transit (x25519 xkeys); secrets never stored on disk
- **Provider Access:** Secrets supplied at link-creation time; capability providers request secrets from host
- **Encryption:** xkey-based encryption using NaCl Seal/Open; prevents eavesdropping and replay
- **Threat Model:** Secrets safe from inter-component leakage; host mediates all access

**Implementation for Host Manager:**
- Build a `credentials` capability provider
- Store endpoint credentials in chosen backend (Vault/K8s/etc.)
- WASM components request credentials by endpoint ID
- Host resolves and provides encrypted credential to requester
- Agents (native or WASM) receive credentials only when needed

**Special Case: Sudo Support**
- **Challenge:** WASM cannot directly invoke `sudo`; privilege escalation is fundamentally a host OS concern
- **Solution:** Host agent (native) executes privileged commands; WASM components call host capability provider
- **Security:** WASM never touches credentials or escalated privileges

**Verdict:** **Use WasmCloud secrets architecture for MVP.** Build a custom credential provider. Keep sudo delegation to native agents.

---

### Privileged Operations (sudo)

**Critical Finding:** WASM should NOT attempt privilege escalation.

**Why:**
- WASM runs in sandboxed isolation; cannot directly invoke OS system calls
- Privilege escalation (sudo) is fundamentally a host OS concern
- WasmCloud design: "Modules request capabilities; host decides what's allowed"
- Even if you grant raw socket/process access, escalating privileges from WASM is dangerous

**Safe Pattern:**
```
WASM Component: "Run privileged command: apt-get update"
↓
Host Capability Provider: (validates request, checks sudoers policy)
↓
Native Agent: (executes sudo as configured user)
↓
Result returned to component
```

**Verdict:** Privilege escalation **must** be delegated to native agents. WASM stays sandboxed.

---

### Endpoint Agents: Native vs. WASM

**Requirements for Endpoint Agents:**
- Auto-execute discovery commands (ifconfig, dmidecode, etc.)
- Capture system state (logs, metrics)
- Accept remote commands from control plane
- Run with optional privilege escalation (sudo)
- Deploy on diverse endpoints (RPi, Linux, VMs, K8s pods)

**Option A: Native Agents (Rust/Go binaries)**
- **Pros:** Direct OS access; no sandbox limits; proven deployment model
- **Cons:** Platform-specific builds; larger binary footprint; harder to update atomically
- **Timeline:** Fast to implement; mature tooling

**Option B: WASM Agents**
- **Pros:** Single portable binary; small size (critical for RPi); updates atomic
- **Cons:** Needs WASM runtime on endpoint (adds dependency); sandbox limits require host callbacks
- **Timeline:** Requires mature WASM runtimes on diverse platforms
- **Reality Check:** WASM runtimes on RPi are feasible but less common than native binaries

**Option C: Hybrid (Current Best Practice)**
- Control plane: WASM (WasmCloud)
- Agents: Native (Rust/Go)
- Benefits: Control plane benefits from WASM's portability; agents stay native-fast and battle-tested
- Trade-off: Two different tech stacks

**Verdict for MVP:** **Option C (Hybrid).** Build control plane in WASM/WasmCloud; agents as native binaries. Revisit WASM agents in future if WASM runtimes become lighter and more ubiquitous on endpoints.

---

## Key Constraints Identified

1. **Network discovery requires host delegation** — WASM sandboxing prevents raw network access
2. **System introspection is limited** — WASI lacks hardware APIs; need host callbacks or file system access
3. **Privilege escalation must stay on host** — Sudo cannot be safely run from WASM sandbox
4. **WASI 0.3 async is essential** — WASI 0.2 callback-based async is too awkward for production
5. **Endpoints agents are best as native binaries** — For now; WASM runtime adoption on RPis is increasing but not ubiquitous
6. **WASM is NOT a constraint; it's a design choice** — Nothing in your MVP requires WASM; it's optional for control plane

---

## Recommended WASM Strategy for Host Manager MVP

### Control Plane: WASM/WasmCloud (Tentative Yes)

**Why:** 
- WASI 0.3 async enables concurrent endpoint management
- WasmCloud secrets architecture is well-suited for credential storage
- Control plane benefits from WASM portability; runs anywhere

**How:**
- Build core management logic as WasmCloud components
- Implement capability providers for:
  - Credential storage (pluggable backend)
  - Endpoint discovery (delegates to native service)
  - System info retrieval (delegates to native service)
  - Command execution (delegates to agents)

**Risk:** WasmCloud is Incubating maturity; monitor for stability as you build

### Agents: Native Binaries (Recommended)

**Why:**
- Direct OS access for hardware detection and command execution
- Proven deployment model across diverse endpoints
- Smaller footprint than WASM + runtime for resource-constrained endpoints (RPi)
- No additional runtime dependency on endpoint

**Language:** Rust (small binaries, good cross-platform support, memory safe)

### Optional WASM for Agents: Future

Once WASM runtimes become standard on edge/IoT (perhaps 2027+), agents could be:
- WASM components compiled to tiny binaries (<1MB)
- Deployed via WasmCloud or standalone Wasmtime
- Hot-deployed and updated atomically
- Deferred to Phase 2 of the project

---

## Alternative Strategy: All-Native

If WASM introduces too much complexity, a simpler MVP is:

- **Control plane:** Native service (Rust, Go, Python) with standard REST API
- **Agents:** Native binaries (Rust, Go)
- **Credentials:** Built-in or existing key-value store (Vault, NATS KV)
- **Trade-off:** Less innovative, slower to build distributed systems, but lower risk and proven

This avoids WasmCloud entirely and gets to MVP faster. WASM adoption could happen post-launch.

---

---

## Comparison: WasmCloud vs Spin/SpinKube vs Project Ocre

### **Project Ocre (Atym)**

**What it is:** A minimal WASM container runtime from the Linux Foundation, optimized for resource-constrained embedded systems.

**Runtime footprint:** 
- 128KB (RTOS version on Zephyr)
- <1MB (Linux version)

**Target devices:**
- MCUs with 256KB+ memory (Zephyr RTOS)
- Constrained CPUs with 1MB-1GB RAM
- Runs containerized WASM applications on firmware

**Architecture:**
- Hardware abstraction layer
- Fine-grained permissions model
- OCI-like container interface
- Inter-container communication

**Maturity:** Early/research phase (Linux Foundation's LFedge project)

**Best for:** IoT/MCU edge devices with severe memory constraints (not your primary target)

**Verdict for Host Manager:** ❌ Not suitable for control plane. Focus is ultra-constrained devices; your control plane needs distributed system capabilities, not minimal footprint.

---

### **Spin/SpinKube (Fermyon)**

**What it is:** Serverless WASM framework (Spin) + Kubernetes operator (SpinKube) for running WASM workloads natively in K8s.

**Components:**
- **Spin**: Developer framework for serverless WASM functions (built-in bindings for KV, DB, AI, etc.)
- **SpinKube**: Kubernetes operator + containerd shim for scheduling Spin apps as WASM workloads

**Performance:**
- 0.5ms cold start (vs. 100-500ms traditional serverless)
- Minimal memory footprint
- Sub-second scaling

**Architecture:**
- Serverless-focused (event-driven, request-response)
- Kubernetes-native scheduling
- CNCF Sandbox status (accepted January 2025)
- Tight integration with K8s primitives (Secrets, ConfigMaps, volumes)

**Maturity:** Stable for Kubernetes workloads (CNCF Sandbox)

**Best for:** Serverless functions in Kubernetes clusters; event-driven microservices

**Limitations for Host Manager:**
- Designed for serverless/FaaS patterns (stateless, request-response)
- Tight coupling to Kubernetes (good if control plane runs IN K8s, bad if control plane must be independent)
- Less suited for long-running coordination services (your control plane needs to maintain state across endpoint connections)

**Verdict for Host Manager:** ⚠️ Partial fit. Good IF your control plane runs inside a Kubernetes cluster, but adds a hard dependency on Kubernetes. Not portable to standalone deployments.

---

### **WasmCloud**

**What it is:** Distributed, topology-agnostic WASM microservices runtime with capability providers and secure inter-component communication.

**Architecture:**
- Actor model with WebAssembly components
- Capability providers (external services)
- Pluggable secrets backends (Vault, K8s, AWS Secrets Manager)
- NATS-based messaging
- Declarative topology (wash)

**Key capabilities:**
- Topology-agnostic: runs on Linux, Kubernetes, cloud, edge (no platform coupling)
- Distributed component linking
- Pluggable secrets + credential providers
- Security-first: mediated host access, capability-based controls

**Maturity:** CNCF Incubating (since Nov 2024); actively maintained; limited production scale use

**Best for:** Distributed microservices that need to adapt to different deployment environments; control planes coordinating diverse endpoints

**Verdict for Host Manager:** ✅ Best fit. Topology-agnostic; capability providers align perfectly with your architecture (discovery, system info, credentials); can run anywhere (standalone server, inside K8s, edge).

---

## Platform Comparison Matrix

| Aspect | Project Ocre | Spin/SpinKube | WasmCloud |
|--------|---|---|---|
| **Runtime footprint** | <1MB | Bundled with Spin (~10-50MB) | Configurable, ~50-100MB |
| **Memory efficiency** | ★★★★★ (best) | ★★★★☆ | ★★★☆☆ |
| **Deployment targets** | MCU/constrained edge | Kubernetes | Linux, K8s, edge, cloud |
| **Architecture pattern** | Container runtime | Serverless/FaaS | Microservices/distributed |
| **Statefulness** | Stateless containers | Stateless functions | Stateful actors |
| **Coordination capability** | Basic (inter-container) | None (FaaS) | Rich (distributed) |
| **Maturity** | Research | Stable (CNCF Sandbox) | Incubating (CNCF) |
| **Secrets management** | Not built-in | K8s-integrated | Pluggable backends |
| **Platform coupling** | Minimal | Tight (K8s) | None (topology-agnostic) |

---

## Recommendation: WasmCloud for MVP

**WasmCloud is the best fit for Host Manager's Hybrid Strategy control plane.**

**Why:**

1. **Topology-agnostic:** Works on your developer laptop, a single Linux server, inside Kubernetes clusters, or on edge gateways—without code changes or different deployment models.

2. **Capability providers align perfectly:**
   - Discovery provider (network scanning)
   - System info provider (hardware detection)
   - Credentials provider (credential storage and access)
   - Execution provider (delegate to agents)

3. **Distributed coordination:** Actor model with NATS messaging is well-suited for managing multiple endpoints concurrently and maintaining state across them.

4. **Secrets architecture:** Already thought through; pluggable backends means you choose Vault, K8s secrets, or in-house solution without code changes.

5. **Scalability:** NATS-based messaging is battle-tested for distributed systems; supports many-to-many component linking.

**When to consider Spin/SpinKube:** If your MVP constraint is "control plane MUST run as a Kubernetes workload with minimal overhead," Spin/SpinKube is excellent. But this adds a hard dependency on Kubernetes for the control plane, which may not be necessary yet.

**Defer Project Ocre:** Relevant for Phase 2 if you add support for ultra-constrained MCU devices as endpoints (not Raspberry Pi level). MCUs are a different deployment story.

---

## WasmCloud + Agents: Refined Hybrid Strategy

**Control Plane (WASM/WasmCloud):**
- Core management logic as WasmCloud actors
- Capability providers for:
  - Credential storage (Vault-backed)
  - Discovery service (delegates to native service or built-in provider)
  - System info provider (delegates to native service)
  - Command execution (routes to agents)
- WASI 0.3 async for concurrent endpoint management
- Runs anywhere: standalone, K8s, edge

**Agents (Native Rust binaries):**
- Lightweight binary deployed to each endpoint
- Direct OS access for hardware detection and privileged operations
- Secure credential access (requests from control plane)
- Connection back to control plane (gRPC, WebSocket, or NATS)

**Deployment in MVP:**
- Single WasmCloud host (or small cluster) for control plane
- Native agents on Linux/RPi/VM endpoints
- Secrets backend: Vault (or K8s secrets if in Kubernetes)

**Future options:**
- SpinKube for K8s endpoints (if control plane scales to many K8s clusters)
- Project Ocre for MCU endpoints (if scope expands to IoT/embedded)

---

## Immediate Next Steps (Updated)

1. ✅ **Confirm: Use WasmCloud for control plane** (with WASI 0.3 async)
2. **Design ADR-001:** WasmCloud control plane architecture + capability providers
3. **Design ADR-002:** Credential storage (Vault integration via WasmCloud)
4. **Design ADR-003:** Network discovery and delegation strategy
5. **Design ADR-004:** Agent communication protocol with control plane
6. **Prototype:** Build minimal WasmCloud component + first capability provider
7. **Prototype:** Sketch agent binary (Rust) with control plane integration
