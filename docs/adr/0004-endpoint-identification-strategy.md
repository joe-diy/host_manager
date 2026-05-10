# ADR-004: Endpoint Identification Strategy

**Status:** Proposed

**Date:** 2026-05-10

**Authors:** Joseph Pearson

---

## Context

After discovery (ADR-003), Host Manager has a list of endpoints with IP, MAC, and hostname. But we don't yet know what each endpoint is:
- Linux server or Raspberry Pi (which model)?
- Virtual machine (on which platform)?
- Kubernetes cluster (which version)?
- Something else entirely?

**The problem:** Without identification, Host Manager cannot:
- Determine which agent type to deploy
- Know which commands are available on the endpoint
- Understand resource constraints (memory, CPU, storage)
- Detect required drivers or special handling (e.g., for specific hardware)
- Provide accurate reporting to users

**Identification phase constraints:**
- Cannot assume endpoints are running agents yet (bootstrap problem)
- Requires probing endpoints via SSH, API, or other access
- Requires credentials (stored in OpenBao per ADR-002)
- Some endpoints may be unreachable, protected, or refuse identification
- Identification happens after discovery, before agent deployment

**Endpoints to identify (MVP scope):**
1. **Linux servers** — Determine distro, version, kernel, hardware model
2. **Raspberry Pis** — Detect specific model (1B+, 3B+, 4, 5), Raspbian version
3. **VMs on cloud platforms** — Detect platform (Zededa, Starlight), OS, hardware
4. **Kubernetes clusters** — Detect Kubernetes version, nodes, capabilities
5. **Other devices** — Gracefully handle unknown/unsupported endpoints

---

## Decision

**Host Manager will implement identification via probing: each discovered endpoint is probed via SSH (for Linux/Raspberry Pi), cloud platform API (for VMs), or Kubernetes API (for clusters) to determine its type, OS, hardware, and capabilities. Identification results are stored in OpenBao and trigger appropriate agent deployment.**

### Specific Decisions:

1. **Identification model: Probing-based (not assumptions)**
   - Send identification probes to discovered endpoints
   - Gather system information (OS, hardware, version, capabilities)
   - Match against known profiles (Raspberry Pi 3B+, Ubuntu 22.04 LTS, K8s 1.28, etc.)
   - Store results; use for agent selection and deployment

2. **Probing methods (by endpoint type):**

   **For Linux/Raspberry Pi (SSH probe):**
   - Query: `/etc/os-release` → OS type, version, ID, PRETTY_NAME
   - Query: `lsb_release -a` → More detailed distro info
   - Query: `uname -a` → Kernel version, machine type (armv7l, x86_64, aarch64)
   - Query: `dmidecode` → Hardware model (requires root or CAP_SYS_RAWIO)
   - Query: `/proc/cpuinfo` → CPU details, flags, core count
   - Query: `/proc/meminfo` → Available memory
   - For Raspberry Pi: Check for `/sys/firmware/devicetree/base/model` or MAC OUI (from discovery)
   - Query: `systemctl --version` → Init system (systemd, openrc, etc.)

   **For Cloud Platform VMs:**
   - Zededa Cloud: Query platform API (requires API key/token)
     - Get VM metadata: CPU, RAM, OS, location, status
   - Mainsail Starlight: Query platform API
     - Similar metadata extraction
   - Fallback: SSH probe (same as Linux/Raspberry Pi)

   **For Kubernetes Clusters:**
   - Query: `/api/v1/nodes` endpoint
   - Extract: Kubernetes version, node count, node resources (CPU, memory)
   - Extract: Installed add-ons (Flannel, Calico, CoreDNS, etc.)
   - Query: `/api/v1/namespaces` → Detect if user has namespace access
   - Attempt: `kubectl version` via kubeconfig (if provided)

3. **Identification flow:**
   ```
   Pending Endpoints (from discovery)
     ↓
   For each endpoint:
     ↓
   Attempt probing (SSH → cloud API → K8s API → fallback to generic)
     ↓
   Gather system information
     ↓
   Match against known profiles
     ↓
   Store identification results in OpenBao
     ↓
   Update endpoint status: pending → identified/unknown/unreachable
     ↓
   Trigger agent deployment (ADR-005)
   ```

4. **Endpoint identification profile (data model):**
   ```rust
   struct EndpointIdentification {
     endpoint_id: String,
     
     // Endpoint classification
     endpoint_type: "linux" | "raspberry_pi" | "kubernetes_cluster" | "vm" | "unknown",
     
     // OS/System information
     os_type: "linux" | "darwin" | "windows" | "kubernetes" | "unknown",
     os_name: String,  // "Ubuntu", "Raspbian", "CentOS", etc.
     os_version: String,  // "22.04 LTS", "11 (bullseye)", etc.
     os_id: String,  // /etc/os-release ID field
     
     // Hardware information
     hardware_model: Option<String>,  // "Raspberry Pi 3 Model B+", "Dell PowerEdge R640", etc.
     cpu_arch: String,  // "x86_64", "aarch64", "armv7l"
     cpu_model: Option<String>,  // "Intel Core i7", "ARM Cortex-A72"
     cpu_cores: Option<u32>,
     memory_mb: Option<u64>,
     
     // Kubernetes-specific
     kubernetes_version: Option<String>,
     k8s_nodes: Option<u32>,
     k8s_namespaces_accessible: Option<Vec<String>>,
     k8s_addons: Option<Vec<String>>,
     
     // Capabilities and constraints
     has_sudo_access: bool,
     init_system: Option<String>,  // "systemd", "openrc", etc.
     systemd_version: Option<String>,
     
     // Probe results
     probe_method: "ssh" | "cloud_api" | "k8s_api" | "unknown",
     probe_success: bool,
     probe_error: Option<String>,
     identified_at: Timestamp,
     
     // Status
     status: "identified" | "unreachable" | "unknown" | "partially_identified",
   }
   ```

5. **Credential-based access:**
   - Request credentials from OpenBao (per ADR-002) for each endpoint
   - SSH: Use stored SSH key + username
   - Cloud API: Use stored API token/credentials
   - Kubernetes: Use stored kubeconfig
   - Handle missing credentials: Mark as "pending_credentials"

6. **Graceful degradation:**
   - If full probe fails, identify what we can from discovery data (MAC vendor, hostname)
   - Mark as "partially_identified" with best-effort results
   - Allow user to provide additional info or credentials later
   - Don't block agent deployment on full identification success

7. **Security considerations:**
   - All probing is read-only (no state changes on endpoints)
   - Credentials retrieved from OpenBao just-in-time (not cached)
   - SSH probes limited to safe commands (no `sudo rm -rf /`)
   - Cloud API calls limited to metadata (no resource modification)
   - Kubernetes API limited to read-only discovery endpoints
   - All identification results stored encrypted in OpenBao

8. **Parallelization and performance:**
   - Probes can run in parallel (WasmCloud async via WASI 0.3)
   - Timeout per probe: 10 seconds (configurable)
   - Retry failed probes: Up to 3 attempts with backoff
   - Overall timeout for identification batch: 5 minutes
   - Large networks (100+ endpoints) identification takes ~2-5 minutes

---

## Rationale

### Why Probing (Not Assumptions)

**Assumptions are wrong:**
- Can't assume all Linux systems are the same (Ubuntu, Debian, CentOS, Alpine all different)
- Can't assume all RPi are the same model (1B+ is very different from 4/5)
- Can't assume endpoint type from MAC alone (some vendors make both servers and IoT)
- Assumptions lead to wrong agent selection and deployment failures

**Probing gives ground truth:**
- OS and version determined via `/etc/os-release` (authoritative)
- Hardware model via `dmidecode` or Raspberry Pi device tree (authoritative)
- Kubernetes version via API (definitive)
- Probing is safe (read-only operations)
- Small overhead (seconds per endpoint)

### Why SSH-First (Not Agent-First)

**Can't deploy agents before identification:**
- Which agent binary to deploy? (Depends on CPU arch: x86_64 vs aarch64 vs armv7l)
- Which features to enable? (Depends on OS, kernel, capabilities)
- Bootstrap problem: Can't run agents without first identifying what can run on each endpoint

**SSH probe is minimal requirement:**
- Available on all Linux/Raspberry Pi systems by default
- Allows reading system files without modification
- Low risk: read-only commands only
- Gives us info to determine which agent to deploy

### Why Store in OpenBao (Not Separate DB)

**OpenBao is our source of truth (ADR-002):**
- Credentials live in OpenBao
- Identification results related to credentials (depends on them)
- Consistency: One system for all endpoint metadata
- Audit trail: OpenBao logs all access
- Just-in-time retrieval: No stale data cached locally

**Alternative of separate database rejected:**
- Adds complexity (sync between OpenBao and DB)
- Credentials and identification results tightly coupled
- OpenBao already designed for this (flexible secret engine)

### Why Parallel Probing

**Network discovery with WasmCloud WASI 0.3 async:**
- Probing is I/O-bound (waiting for SSH/API responses)
- WASI 0.3 native async/await enables concurrent probes
- 100 endpoints: Sequential = 20-30 min; Parallel = 2-5 min
- Reasonable UX: "Identification in progress... 25/100 complete"

### Why Graceful Degradation

**Not all endpoints will be fully identifiable:**
- Behind corporate firewalls blocking SSH
- Credentials not available yet
- Endpoint offline at identification time
- SNMP-only visibility (no SSH access)

**Partial identification is valuable:**
- MAC vendor tells us it's a Raspberry Pi (>90% confidence)
- Hostname may indicate purpose ("k8s-node-1" is probably Kubernetes)
- We can still deploy agents with reasonable defaults
- User can provide more info later

---

## Consequences

### Positive Impacts

**1. Accurate endpoint classification**
- Know exactly what we're managing
- Avoid wrong driver/agent deployments
- Enable targeted, endpoint-specific policies

**2. Proactive issue detection**
- Identify unsupported or deprecated OS versions early
- Detect hardware constraints (low memory, slow CPU)
- Flag endpoints requiring special handling

**3. Audit trail and compliance**
- OpenBao logs all probing activity
- Identification results immutable (stored in OpenBao)
- Clear record of what endpoints are managed

**4. Scalability**
- Parallel probing via WASI 0.3 async enables large network identification
- Reasonable UX even with 100+ endpoints
- Automatic retries handle transient network issues

### Implementation Challenges

**1. Probe reliability across OS variants**
- `/etc/os-release` is standard on modern Linux but not universal
- `lsb_release` deprecated in some newer distros
- `dmidecode` requires privileges on some systems
- Mitigation: Try multiple probing methods; gracefully degrade

**2. Raspberry Pi detection accuracy**
- MAC vendor detection is >99% reliable but not 100%
- Device tree check (`/sys/firmware/devicetree/base/model`) requires Linux 4.0+
- `cat /proc/device-tree/model` alternative for older kernels
- Mitigation: Multiple detection methods; user can verify

**3. Kubernetes cluster detection**
- Requires kubeconfig or in-cluster service account
- Many clusters restrict API access
- Node information may be partial (RBAC restrictions)
- Mitigation: Attempt discovery; gracefully fall back to basic probe

**4. Credential availability**
- Not all endpoints have credentials in OpenBao yet
- User may not have SSH keys for some systems
- Mitigation: Mark as "pending_credentials"; retry when credentials added

**5. Performance at very large scale**
- 1000+ endpoints: Even parallel probing takes 10+ minutes
- Kubernetes clusters: API rate limiting could slow discovery
- Mitigation: Batch size limits; progressive identification (identify subset, deploy agents, agents can help identify others)

### Risks

**1. False positives in identification**
- Probe might misidentify endpoint type
- Example: Generic Linux that looks like Raspberry Pi due to cpuinfo
- Mitigation: Multiple probe methods; user can override/correct

**2. Probe failures blocking agent deployment**
- If identification fails, no agent deployed
- User doesn't know why
- Mitigation: Graceful degradation; "partially_identified" state allows agent deployment with defaults

**3. Security of probing commands**
- SSH commands run on endpoints; could be logged/audited by endpoint
- Administrator might notice "foreign" commands being run
- Mitigation: Document probing; use standard, innocuous commands; audit logging in WasmCloud

**4. Stale identification results**
- Endpoint may change OS/hardware between identifications
- Results stored in OpenBao, not continuously updated
- Mitigation: Periodic re-identification (Phase 2); health checks detect major changes

**5. Credential exposure in probes**
- SSH password probes could be exposed if command is logged
- Mitigation: Use SSH keys (not passwords); credentials retrieved just-in-time from OpenBao

---

## Alternatives Considered

### Alternative 1: Agent-First Identification

**Decision:** Rejected

**Rationale:**
- Bootstrap problem: Can't deploy agent without knowing endpoint type
- Agent binary is specific to CPU arch; need identification first
- Creates chicken-and-egg problem

**Retention:** Agents can perform continuous re-identification after deployment (Phase 2)

### Alternative 2: SNMP-Based Identification

**Decision:** Rejected for MVP

**Rationale:**
- SNMP requires endpoint configuration (not enabled by default)
- Only works for network devices; not Linux servers
- Less detailed information than SSH probe
- Deferred to Phase 2 for specialized network device support

### Alternative 3: Metadata-Only from Discovery

**Decision:** Rejected

**Rationale:**
- MAC vendor + hostname alone is insufficient
- Cannot determine OS version, kernel, Kubernetes version
- Too many false positives
- Leads to wrong agent deployment decisions

**Retention:** Use discovery metadata as fallback when probing fails

### Alternative 4: Separate Identification Database

**Decision:** Rejected

**Rationale:**
- Adds operational complexity
- Credentials in OpenBao; identification in separate DB = sync problems
- OpenBao already designed to store all endpoint metadata
- Better to keep related data together

### Alternative 5: Mandatory Full Identification

**Decision:** Rejected in favor of graceful degradation

**Rationale:**
- Some endpoints will be unreachable or restricted
- Blocking agent deployment until full identification = poor UX
- Partial identification + defaults is better than nothing
- Progressive identification (agents help identify each other) in Phase 2

---

## Implementation Approach

### Phase 1: SSH-Based Identification (MVP)

**1. Identification probe executor (Rust/native code)**
   - Takes endpoint IP + credentials from OpenBao
   - Executes SSH commands to gather system information
   - Parses outputs into structured data
   - Returns identification results

**2. Probe commands (read-only, safe):**
   ```bash
   # Get OS info
   cat /etc/os-release
   
   # Get kernel info
   uname -a
   
   # Get hardware model (if privileged)
   dmidecode -t system
   
   # Alternative hardware detection (unprivileged)
   cat /sys/firmware/devicetree/base/model  # RPi device tree
   
   # Get CPU info
   cat /proc/cpuinfo
   
   # Get memory info
   cat /proc/meminfo
   
   # Get init system
   systemctl --version
   
   # Get Raspberry Pi specific info
   vcgencmd measure_temp  # (Optional; requires videocore access)
   ```

**3. WasmCloud integration:**
   - Identification capability provider (native code)
   - Endpoint Manager actor orchestrates identification for all endpoints
   - Stores results in OpenBao via credentials provider

**4. Profile matching:**
   - Define known profiles (Raspberry Pi 3B+, Ubuntu 22.04, etc.)
   - Match probe results against profiles
   - Return best-match endpoint type

**5. Endpoint object model (in OpenBao):**
   - Store identification results as secure secrets
   - One record per endpoint with all metadata
   - Retrieve by endpoint ID; update after re-identification

**6. Control plane flow:**
   - User triggers identification (CLI: `hostmgr identify`)
   - Endpoint Manager calls identification capability provider
   - Provider SSH-probes all pending endpoints
   - Results stored in OpenBao
   - Endpoint status updated: pending → identified/partially_identified/unreachable
   - Trigger agent deployment (next phase)

### Phase 2: Cloud Platform & K8s Identification (Post-MVP)

**1. Zededa Cloud identification provider**
   - Query Zededa API for VM metadata
   - Extract: OS, hardware, version, location

**2. Mainsail Starlight identification provider**
   - Query platform API

**3. Kubernetes cluster identification provider**
   - Query `/api/v1` endpoints
   - Extract: Kubernetes version, nodes, capabilities

**4. Progressive identification**
   - Agents perform local identification and report back
   - Reduces need for separate SSH probes
   - Agents probe each other in cluster

### Phase 3: Continuous Health & Re-identification (Future)

**1. Periodic re-identification**
   - Refresh endpoint information every N hours
   - Detect OS upgrades, hardware changes, drift

**2. Agent-driven identification**
   - Agents run local probes; report to control plane
   - More detailed information (installed packages, running services)
   - Reduces dependency on SSH access

**3. Anomaly detection**
   - Flag endpoints that change type unexpectedly
   - Alert on hardware resource changes

---

## Related Decisions

- **ADR-002:** Credential Storage — credentials retrieved for probing
- **ADR-003:** Network Discovery — generates list of endpoints to identify
- **ADR-005:** Agent Communication — agent deployment follows successful identification
- **ADR-001:** Control Plane Architecture — Identifier actor orchestrates identification

---

## Monitoring & Review

**Identification-specific metrics to track:**
- Identification success rate (% successfully identified)
- Identification time per endpoint (seconds)
- Common failure modes (unreachable, credential errors, etc.)
- False positive rate (misidentified endpoints)
- Endpoint type distribution (# Linux, # RPi, # K8s, # unknown)

**Decision review criteria:**
- Identification success rate >90% (goal for Phase 1)
- Mean identification time <30 seconds per endpoint
- User satisfaction with endpoint type detection
- No critical misidentifications blocking deployments

**Review date:** After Phase 1 MVP completion; reassess every 6-12 months

---

## Open Questions / Future Exploration

1. **SNMP-based identification:** Should Phase 2 add SNMP discovery for network devices? (Yes, for specialized networks)

2. **Agent-driven identification:** How much identification can agents do locally? (Explored in Phase 2)

3. **Continuous health monitoring:** How often should endpoints be re-identified? (Phase 3; probably hourly for critical endpoints)

4. **Identification plugins:** Should users be able to write custom identification probes? (Phase 3; plugin interface)

5. **Integration with existing tools:** Should identification import from Ansible facts, Puppet reports, etc.? (Phase 3; import/sync tools)

6. **Troubleshooting endpoints:** What should Host Manager do when identification fails repeatedly? (Phase 2; suggestion engine)

---

## Decision Record

**Approved by:** [To be filled after review]

**Date approved:** [To be filled]

**Concerns/feedback addressed:** [To be filled after discussion]
