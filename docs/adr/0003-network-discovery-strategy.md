# ADR-003: Network Discovery Strategy

**Status:** Proposed

**Date:** 2026-05-10

**Authors:** Joseph Pearson

---

## Context

Host Manager's MVP must auto-detect endpoints on a network. Users should be able to run Host Manager and have it discover available endpoints without manual inventory entry.

**Endpoints to discover:**
- Linux servers and Raspberry Pis (bare metal, static IPs)
- VMs on cloud platforms (Zededa Cloud, Mainsail Starlight)
- Kubernetes clusters (cluster API discovery, node discovery)
- IoT devices and edge gateways running Host Manager agents

**Discovery mechanisms needed:**
1. **LAN discovery** — Find endpoints on local network via ARP/mDNS
2. **Cloud platform discovery** — Query platform APIs (Zededa, Starlight)
3. **Kubernetes discovery** — API queries to cluster API servers
4. **Manual addition** — Users can also add endpoints explicitly

**Technical constraints from earlier research:**
- WASM sandboxing prevents raw network access (ARP, mDNS scanning)
- WasmCloud components cannot perform network discovery natively
- Host-delegated discovery is the secure, correct approach
- Control plane coordinates; native services perform actual discovery

**MVP scope:** Focus on LAN discovery (ARP/mDNS); defer cloud platform and K8s discovery to Phase 2.

---

## Decision

**Host Manager will implement discovery via a host-delegated capability provider. The WasmCloud control plane coordinates discovery, but delegates actual network scanning to a native discovery service (capability provider) that performs ARP and mDNS scanning on the local network.**

### Specific Decisions:

1. **Discovery architecture: Host-delegated via capability provider**
   - Control plane (WasmCloud actor) triggers discovery operations
   - Discovery capability provider (native service) performs actual ARP/mDNS scanning
   - Provider returns discovered endpoints to control plane
   - Control plane stores discovered endpoints and coordinates further identification/access

2. **Discovery protocols (MVP):**
   - **ARP scanning:** Discover IPv4 addresses on local subnet (most reliable for LAN)
   - **mDNS scanning:** Discover devices advertising mDNS services (Raspberry Pis, printers, etc.)
   - Defer cloud platform APIs (Zededa, Starlight) to Phase 2
   - Defer Kubernetes discovery to Phase 2

3. **Discovery flow:**
   ```
   User triggers discovery (UI/CLI)
     ↓
   WasmCloud Endpoint Manager actor requests discovery
     ↓
   Discovery capability provider executes:
     • ARP scan of local subnet (arp-scan or custom)
     • mDNS multicast query (mdns-sd or custom)
     • Port scanning (optional: SSH 22, HTTP 80, 443 for service detection)
     ↓
   Provider returns discovered hosts (IP, MAC, hostname, services)
     ↓
   Control plane stores pending discoveries in credential store
     ↓
   Identification phase (ADR-004) attempts to access each endpoint
   ```

4. **Discovery provider interface (WasmCloud capability):**
   ```
   DiscoverEndpointsRequest {
     subnet: "192.168.1.0/24" or "auto" (detect from system)
     protocols: ["arp", "mdns"] 
     timeout_seconds: 30
     port_scan_enabled: false  // Optional service detection
   }
   
   DiscoverEndpointsResponse {
     discovered_hosts: [
       {
         ip_address: "192.168.1.42",
         mac_address: "b8:27:eb:xx:xx:xx",  // For Raspberry Pi detection
         hostname: "raspberrypi",
         mdns_services: ["_ssh._tcp", "_http._tcp"],
         vendor: "Raspberry Pi Foundation"  // Optional
       },
       ...
     ]
     scan_duration_ms: 2500,
     errors: ["some devices may have firewalls blocking ARP responses"]
   }
   ```

5. **Control plane coordination:**
   - Endpoint Manager actor maintains discovery state
   - On discovery completion, creates pending endpoint records in OpenBao
   - Triggers Identifier service (next phase) to probe endpoints
   - Stores: IP, MAC, hostname, last_seen, status (pending/identified/offline)

6. **Security model:**
   - Discovery capability provider runs with network access privileges
   - WasmCloud components never execute ARP/mDNS directly
   - Audit trail: All discoveries logged via WasmCloud audit
   - Scope limiting: Operators configure which subnets to discover (e.g., only 192.168.1.0/24, not 0.0.0.0/0)

7. **Multi-region/multi-network support (Deferred):**
   - MVP: Single network (auto-detect local subnet)
   - Phase 2: Multiple discovery providers (one per network/region)
   - Phase 2: Cloud platform providers (Zededa, Starlight APIs)

---

## Rationale

### Why Host-Delegated Discovery (Not WASM-Native)

**WASM cannot perform raw network operations:**
- WASI doesn't provide raw socket access for ARP/mDNS scanning
- WASM sandboxing is intentional security boundary
- Granting raw socket access would break WASM's security model
- Wasmer's WASIX (fork) provides this, but breaks portability and adds vendor lock-in

**Host-delegated is the correct architecture:**
- Aligns with WasmCloud's capability-based security model
- Components request capabilities; host decides what's allowed
- Discovery is a privileged operation (network access); should run in native code
- Agents (also native) can also perform local discovery if needed
- Clean separation: discovery logic (native) from coordination logic (WASM)

### Why ARP + mDNS (Not Other Methods)

**ARP (Address Resolution Protocol):**
- ✅ Most reliable for LAN endpoint discovery (every host responds to ARP)
- ✅ Firewall-proof (ARP is link-layer, bypasses IP firewalls)
- ✅ Works for all devices with Ethernet (servers, RPi, VMs, IoT)
- ✅ Fast (completes in seconds for typical subnet)
- ❌ Only works on local subnet (not routable across networks)
- ❌ Some managed networks block ARP scanning

**mDNS (Multicast DNS):**
- ✅ Discovers devices advertising services (mDNS-aware endpoints)
- ✅ Returns hostnames automatically (no reverse DNS lookup needed)
- ✅ Effective for Raspberry Pis, Apple devices, printers, IoT
- ❌ Only works for mDNS-enabled devices (not all Linux servers)
- ❌ Slower than ARP (multicast query timeout)

**Alternative: Passive listening (not for MVP)**
- Could listen to network traffic and infer endpoints
- Too slow and unreliable for MVP
- Deferred to Phase 2 if needed

**Not using ICMP ping sweep:**
- Often blocked by firewalls
- Unreliable
- Less efficient than ARP

### Why Host-Based Discovery Service (Not Cloud APIs in MVP)

**Deferred to Phase 2:**
- Zededa Cloud API discovery (requires authentication setup)
- Mainsail Starlight API discovery (requires authentication setup)
- Kubernetes API discovery (requires kubeconfig)

**MVP focus: LAN discovery**
- Solves the "on-premises" case immediately
- Lowest operational overhead (no additional API credentials)
- Works for the most common Host Manager deployment scenario
- Cloud platform discovery can be added later without changing core discovery architecture

### Why Capability Provider (Not Built-in Service)

**Capability provider is the right WasmCloud abstraction:**
- Decouples discovery implementation from control plane logic
- Host can swap discovery implementations (custom scanner, cloud APIs, etc.) without control plane changes
- Matches WasmCloud's design philosophy (pluggable capabilities)
- Operators can replace discovery provider with custom implementation if needed
- Scales better: discovery provider runs independently (can be rate-limited, monitored separately)

**Alternative: Hardcoded discovery in agent**
- ❌ Agents become heavyweight (discovery + identification + execution)
- ❌ Only works if agent is already deployed (bootstrap problem)
- ❌ Can't discover before deploying agents

---

## Consequences

### Positive Impacts

**1. Security and sandboxing maintained**
- WASM components never touch network directly
- Discovery privilege is isolated to capability provider
- Audit trail through WasmCloud logs

**2. Extensibility**
- Easy to add new discovery methods (cloud APIs, custom scanners)
- Operators can implement custom providers
- WasmCloud's pluggable architecture supports multiple discovery providers

**3. Separation of concerns**
- Control plane focuses on coordination, not network operations
- Discovery logic is testable native code (not WASM)
- Agents can independently perform local discovery if needed (e.g., for edge gateways)

**4. Scalability**
- Discovery provider can be scaled independently
- Can run multiple discovery providers for different networks/regions
- Host Manager control plane remains lightweight

### Implementation Challenges

**1. Subnet detection**
- Must auto-detect local subnet to scan (or require operator configuration)
- Different on macOS, Linux, Windows
- Mitigation: Use standard libraries (ifconfig, ip addr); fallback to manual configuration

**2. ARP tool portability**
- arp-scan is common on Linux but not macOS/Windows
- nmap arp-scan is cross-platform but adds dependency
- Mitigation: Use native OS APIs (getifaddrs + raw sockets) or wrap nmap; document dependencies

**3. Firewall filtering**
- Some networks block ARP responses or mDNS
- Enterprise networks may disable ARP scanning entirely
- Mitigation: Document this limitation; recommend manual endpoint addition in restricted networks

**4. Performance at scale**
- ARP scanning on large subnets (e.g., /16) could take minutes
- mDNS timeout adds latency per scan
- Mitigation: MVP targets typical small-to-medium networks (/24 subnets, <256 hosts); optimize for common case

**5. Accuracy of hardware detection**
- MAC address vendor detection (to identify Raspberry Pi) requires MAC lookup database
- Accuracy depends on database freshness
- Mitigation: Use IEEE OUI database; pre-populate with common vendors (Raspberry Pi Foundation, etc.)

### Risks

**1. Network misconfiguration**
- Operators might accidentally scan 0.0.0.0/0 or scan too-large subnets
- Discovery could flood network with traffic
- Mitigation: Add validation (no /8 or larger subnets without explicit override); add throttling

**2. False positives**
- Discovery might find non-endpoint devices (printers, cameras, smart home devices)
- Need additional identification to distinguish endpoints
- Mitigation: Identification phase (ADR-004) filters and validates; discovery is first pass only

**3. Race conditions during discovery**
- Endpoint comes online/offline during discovery
- Control plane sees stale endpoint data
- Mitigation: Discovery is periodic; endpoint health checks detect offline endpoints

**4. DoS vector (if misused)**
- A compromised control plane component could hammer network with discovery requests
- WasmCloud's capability-based access control limits this
- Mitigation: Capability provider can rate-limit discovery requests; audit logging

---

## Alternatives Considered

### Alternative 1: WASM-Native Discovery with Raw Sockets

**Decision:** Rejected

**Rationale:**
- WASI doesn't provide raw socket access (security boundary)
- Would require Wasmer WASIX (vendor lock-in, breaks portability)
- Violates WasmCloud's security model (capabilities, mediated host access)
- Harder to test; requires WASM runtime environment

**Retention:** None; host-delegation is correct model

### Alternative 2: Manual Endpoint Registration Only

**Decision:** Rejected for MVP; accepted as fallback

**Rationale:**
- Auto-discovery is a core MVP requirement
- Manual inventory is tedious and error-prone
- Defeats the "auto-detection" use case

**Retention:** Manual endpoint addition via API remains supported for:
- Endpoints behind firewalls that block ARP/mDNS
- Cloud platform endpoints (before Phase 2 APIs ready)
- Enterprise environments with restrictive policies

### Alternative 3: Cloud Platform APIs in MVP (Zededa, Starlight, Kubernetes)

**Decision:** Rejected for MVP; deferred to Phase 2

**Rationale:**
- Cloud platform discovery requires authentication (API credentials/kubeconfigs)
- Adds deployment complexity for MVP
- LAN discovery covers most MVP use cases
- Can be added later without changing discovery architecture
- MVP ships with LAN discovery; Phase 2 adds cloud/K8s discovery providers

### Alternative 4: Passive Network Listening

**Decision:** Rejected for MVP

**Rationale:**
- Too slow (discovery takes minutes/hours)
- Unreliable (depends on network traffic patterns)
- Harder to implement correctly
- ARP/mDNS is faster and more reliable for active discovery

**Retention:** Could be considered in Phase 3 for continuous endpoint monitoring (supplement to periodic active discovery)

### Alternative 5: Centralized Discovery Service (Not WasmCloud Capability)

**Decision:** Rejected

**Rationale:**
- Adds extra microservice dependency
- WasmCloud capability provider is the natural abstraction
- Simpler deployment (discovery runs as part of WasmCloud host)
- Easier to scale multiple discovery providers

---

## Implementation Approach

### Phase 1: LAN Discovery (MVP)

**1. Discovery capability provider (Rust)**
   - Takes subnet as input (or auto-detect)
   - Executes ARP scan (using arp-scan or nmap)
   - Executes mDNS scan (mdns-sd or similar)
   - Returns discovered hosts with IP, MAC, hostname
   - Built as WasmCloud capability provider (native code)

**2. WasmCloud integration**
   - Define WIT (WebAssembly Interface Type) for discovery capability
   - Endpoint Manager actor calls capability provider
   - Routes response to persistence layer (OpenBao)

**3. Subnet auto-detection**
   - Query system network interfaces (ifaddrs on Unix)
   - Extract primary subnet (assume /24)
   - Allow override via configuration

**4. MAC vendor detection**
   - Embed IEEE OUI database (or download)
   - Detect Raspberry Pi Foundation MACs
   - Useful for identification phase (ADR-004)

**5. Endpoint object model**
   ```rust
   struct DiscoveredEndpoint {
     ip_address: String,
     mac_address: String,
     hostname: Option<String>,
     vendor: Option<String>,  // e.g., "Raspberry Pi Foundation"
     services: Vec<String>,   // mDNS services found
     discovery_source: String, // "arp", "mdns"
     discovered_at: Timestamp,
     status: "pending",       // Awaiting identification
   }
   ```

**6. Control plane flow**
   - User triggers discovery (CLI: `hostmgr discover`)
   - Endpoint Manager calls discovery capability provider
   - Provider scans network; returns results
   - Control plane stores each discovered endpoint with status="pending"
   - Trigger identification phase (next ADR) to probe/identify endpoints

### Phase 2: Cloud Platform Discovery (Post-MVP)

**1. Zededa Cloud provider**
   - Query Zededa API for VMs on account
   - Extract IP, status, metadata

**2. Mainsail Starlight provider**
   - Query Starlight API for VMs

**3. Generic cloud provider abstraction**
   - Multiple discovery providers can coexist
   - Control plane coordinates discovery across all providers

### Phase 3: Enhanced Discovery (Future)

**1. Kubernetes discovery**
   - Query cluster API for nodes
   - Extract node IP, status, labels

**2. NetBox integration (Optional, Low Risk)**
   - Push discovered endpoints to NetBox for centralized inventory
   - Discovered endpoints visible in both Host Manager and NetBox
   - Query NetBox API to reduce discovery scope (check existing inventory first)
   - Plugin for Host Manager-specific metadata (last_probed, online_status)
   - See: RESEARCH-NETBOX-INFRAGRAPH.md for details

**3. Infragraph integration (Optional, Deferred)**
   - Wait for Infragraph stable release (late 2026/2027)
   - Query Infragraph knowledge graph for existing infrastructure (from Terraform, cloud APIs)
   - Reduce discovery scope: only probe resources not already in Infragraph
   - Bidirectional sync: push Host Manager findings back to Infragraph
   - See: RESEARCH-NETBOX-INFRAGRAPH.md for details

**4. Passive monitoring**
   - Continuous listening to network for endpoint state changes
   - Supplements periodic active discovery

**5. SSDP discovery**
   - Simple Service Discovery Protocol (used by UPnP devices)
   - Low priority (niche device support)

---

## Related Decisions

- **ADR-002:** Credential Storage — discovered endpoints stored in OpenBao
- **ADR-004:** Identification Strategy — follows discovery; probes endpoints to determine type
- **ADR-001:** Control Plane Architecture — Endpoint Manager actor coordinates discovery

---

## Open Questions / Future Exploration

1. **Multi-region discovery:** How should Host Manager discover endpoints across multiple networks? (Phase 2: multiple discovery providers, operator configuration for each region)

2. **Dynamic endpoint lifecycle:** How does Host Manager handle endpoints that appear/disappear frequently? (Health checks; periodic re-discovery; endpoint state machine in ADR-004)

3. **Credential less discovery:** Should discovery work without any credentials? (Yes for MVP; just lists IPs/hostnames. Identification requires credentials.)

4. **Reverse discovery:** Can agents discover the control plane (for nat-traversal)? (Deferred to Phase 2; out-of-scope for MVP)

5. **User-defined discovery rules:** Should users be able to filter/tag discovered endpoints during discovery? (Phase 2: discovery filters; tagging in identification)

6. **Integration with existing inventory:** Should Host Manager import from Ansible inventory, Terraform state, etc.? (Phase 2 feature; manual import scripts)

---

## Monitoring & Review

**Discovery-specific metrics to track:**
- Discovery execution time (seconds)
- Endpoints discovered (count)
- False positive rate (endpoints discovered but unreachable)
- Network errors (ARP/mDNS failures)
- Operator feedback on discovery accuracy

**Decision review criteria:**
- Does LAN discovery cover >80% of MVP use cases? (Target: yes by Phase 1 complete)
- Are false positives acceptable? (Target: <10% of discovered endpoints unreachable)
- Does host-delegation model scale to 1000+ endpoints? (Yes; test in Phase 1)

**Review date:** After Phase 1 MVP completion; reassess for Phase 2 planning

---

## Decision Record

**Approved by:** [To be filled after review]

**Date approved:** [To be filled]

**Concerns/feedback addressed:** [To be filled after discussion]
