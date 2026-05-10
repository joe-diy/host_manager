# NetBox and Infragraph for Host Manager Discovery (Phase 3 Research)

Research on integrating NetBox and HashiCorp Infragraph with Host Manager's endpoint discovery system in Phase 3.

## Executive Summary

| Tool | Type | Maturity | Use Case | For Host Manager |
|------|------|----------|----------|---|
| **NetBox** | Open-source IPAM/DCIM | Stable (2016+, widely used) | Network/infrastructure inventory | Phase 3: Optional backend for storing discovered endpoints |
| **Infragraph** | Cloud-native knowledge graph | Early (HCP public preview 2026) | Real-time infrastructure relationships | Phase 3: Optional integration for Terraform-based discovery |

---

## NetBox: Open-Source IPAM/DCIM Platform

### What It Is

NetBox is an open-source web application for managing and documenting computer networks, specifically designed for IPAM (IP Address Management) and DCIM (Data Center Infrastructure Management).

**Features:**
- IP address and prefix management (IPv4/IPv6)
- Device and site inventory
- Network documentation
- Circuit management
- REST API + GraphQL API
- Plugin ecosystem for extensibility
- Available since 2016; widely adopted

### Architecture for Host Manager

**NetBox as an Endpoint Store:**
```
Host Manager Discovery
↓
Discovered endpoints (IP, MAC, hostname)
↓
NetBox Integration Provider
↓
Store in NetBox as "Devices" or "Virtual Machines"
↓
Query NetBox API for inventory queries
```

### Integration Approach (Phase 3)

**1. NetBox as Discovery Backend**
- Host Manager discovery provider pushes discovered endpoints to NetBox
- NetBox becomes the "source of truth" for endpoint inventory
- Users can view/manage endpoints in both Host Manager and NetBox UIs

**2. NetBox API Integration**
- REST API: Create/update devices from discovery results
- GraphQL API: Query devices for advanced filtering
- Plugins: Custom plugins for Host Manager-specific fields (last_probed, online_status, etc.)

**3. NetBox Discovery Plugin**
- NetBox 2026 has native discovery plugins (NetBox Discovery, Slurp'it)
- Host Manager could extend or integrate with these
- Or use NetBox as read-only inventory backend

**Example workflow:**
```
Host Manager discovers: 192.168.1.42 (Raspberry Pi, b8:27:eb:xxx)
↓
Push to NetBox: Create Device "rpi-01" in Site "office"
↓
User queries NetBox: "All Raspberry Pis in office"
↓
Host Manager uses result for bulk operations
```

### Benefits

1. **Single source of truth** — Endpoints documented in both Host Manager and NetBox
2. **Integration with existing tools** — NetBox integrates with Ansible, Terraform, Prometheus, etc.
3. **Plugin ecosystem** — Extensive plugins available for custom fields/integrations
4. **Open-source** — Apache 2.0 license; aligns with Host Manager
5. **REST + GraphQL** — Rich APIs for querying and updating

### Limitations

1. **Not auto-discovery focused** — NetBox designed for manual/scripted entry, not real-time discovery
2. **Complex for small deployments** — Full NetBox setup is heavyweight for MVP users
3. **Data consistency** — Keeping NetBox and Host Manager in sync requires careful design
4. **Learning curve** — Another tool to operate and understand

### When to Use (Phase 3 Decision)

**Integrate if:**
- Users want endpoint inventory visible in NetBox
- Organization already uses NetBox (avoid dual inventory systems)
- Need advanced IP management (VLAN tracking, circuit management, etc.)
- Benefit from NetBox's Ansible/Prometheus integrations

**Skip if:**
- Simple Host Manager inventory is sufficient
- No existing NetBox infrastructure
- Added complexity not justified for MVP user base

---

## HashiCorp Infragraph: Infrastructure Knowledge Graph

### What It Is

Project Infragraph is HashiCorp's infrastructure knowledge graph—a real-time, relational model of infrastructure, applications, services, and ownership. Currently in HCP Terraform public preview (May 2026).

**Features:**
- Real-time infrastructure graph (nodes = resources, edges = relationships)
- Integrates: Terraform state, cloud APIs, Kubernetes, config management
- Bidirectional reasoning (AI agents and humans query/act on graph)
- API hooks and telemetry feeds for updates
- AI-driven automation and self-healing infrastructure
- Designed for agentic infrastructure (AI-powered orchestration)

### Architecture for Host Manager

**Infragraph as a Data Source:**
```
Terraform deploys Host Manager agents
↓
Infragraph ingests: Terraform state, cloud APIs, K8s
↓
Infragraph graph includes: agents, endpoints, topology
↓
Host Manager queries Infragraph API for discovered resources
↓
Combines discovery with Terraform-declared infrastructure
```

### Integration Approach (Phase 3)

**1. Infragraph as Infrastructure Source**
- Host Manager queries Infragraph for existing infrastructure resources
- Discovers endpoints already declared in Terraform
- Reduces need for separate discovery in environments with strong IaC

**2. Bidirectional Sync**
- Infragraph knows about infrastructure from Terraform, Vault, cloud APIs
- Host Manager adds endpoint identifications, credentials, health status
- Infragraph graph enriched with Host Manager metadata

**3. AI-Driven Automation**
- Infragraph could trigger Host Manager actions via API
- Example: "New VM detected in Terraform; Host Manager: probe it and identify type"
- Self-healing: Infragraph detects drift; Host Manager corrects it

**Example workflow:**
```
User applies Terraform: 10 new Raspberry Pi VMs in Zededa Cloud
↓
Infragraph ingests Terraform state (VMs created)
↓
Infragraph notifies Host Manager: "10 new VMs"
↓
Host Manager auto-probes and identifies them
↓
Results stored back to Infragraph graph
```

### Benefits

1. **Infrastructure-as-Code native** — Works seamlessly with Terraform/OpenTofu
2. **Real-time relationships** — Automatically understands dependencies
3. **AI-driven automation** — Agents can coordinate discovery/identification
4. **Multi-source integration** — Terraform, Vault, Consul, cloud APIs, K8s all in one graph
5. **Future-proof** — HashiCorp investing heavily (IBM-backed)

### Limitations

1. **Early maturity** — Public preview (May 2026); not recommended for production yet
2. **US-only** — Currently limited to United States deployments
3. **HCP-coupled** — Requires HCP Terraform (SaaS); may not work with self-hosted Terraform
4. **Vendor lock-in** — Tied to HashiCorp/IBM ecosystem
5. **Complex to deploy** — Requires HCP Terraform subscription and beta acceptance
6. **API stability** — Beta APIs may change; not recommended for production workflows

### When to Use (Phase 3 Decision)

**Integrate if:**
- Users deploy infrastructure via Terraform (HCP or self-hosted)
- Benefit from infrastructure graph queries
- Want AI-driven automation (future state)
- Organization already uses HCP or planning to

**Skip if:**
- Not using Terraform or HCP
- Early maturity is risky for your users
- Prefer open-source alternatives
- SaaS dependency is a blocker

---

## Comparison: NetBox vs Infragraph

| Aspect | NetBox | Infragraph |
|--------|--------|-----------|
| **Type** | IPAM/DCIM inventory | Infrastructure knowledge graph |
| **Maturity** | Stable (10 years) | Early (public preview 2026) |
| **License** | Apache 2.0 | Proprietary (HCP only) |
| **Deployment** | Self-hosted | SaaS (HCP Terraform only) |
| **Data Source** | Manual + plugins | Terraform, cloud APIs, K8s, etc. |
| **Real-time** | No (manual updates) | Yes (APIs + telemetry) |
| **AI/Agents** | No | Yes (planned) |
| **API** | REST + GraphQL | REST (evolving) |
| **Use with Host Manager** | Endpoint storage/query | Infrastructure source + coordination |
| **Ecosystem** | Large plugin community | Growing (HashiCorp ecosystem) |
| **Risk** | Low (established) | Higher (beta/preview) |

---

## Phase 3 Implementation Strategy

### Phase 3a: Optional NetBox Integration (Lower Risk)

**Goal:** Allow Host Manager to push discovered endpoints to NetBox

**Implementation:**
1. Build NetBox capability provider (similar to discovery provider)
2. Discovered endpoints pushed to NetBox on user request
3. Users can view/manage endpoints in both systems
4. Plugin for Host Manager-specific metadata (last_probed, online_status)

**Timeline:** Q3-Q4 2026 (after Phase 1 MVP complete)

**Rollout:** Optional; documented as "advanced feature"

### Phase 3b: Infragraph Integration (Higher Risk, Deferred)

**Goal:** Query Infragraph for existing infrastructure; reduce discovery scope

**Implementation:**
1. Wait for Infragraph to exit beta (likely late 2026/2027)
2. Build Infragraph query provider
3. Host Manager checks Infragraph first: "What infrastructure already exists?"
4. Only probe/discover resources not already in Infragraph
5. Push discovery results back to Infragraph (bidirectional sync)

**Timeline:** Phase 3b (2027+, after Infragraph stable)

**Rollout:** Optional; advanced/Terraform-only deployments

### Phase 3c: AI-Driven Coordination (Future)

**Goal:** Leverage Infragraph's AI agents for automated discovery/identification

**Implementation:**
1. Infragraph agents detect new resources
2. Agents trigger Host Manager identification APIs
3. Host Manager results enriched back to Infragraph
4. Self-healing: Infragraph agents react to Host Manager status

**Timeline:** 2027+ (after Infragraph stable + Host Manager v2.0)

---

## Recommendation for ADR-003 Phase 3

**Phase 3a: NetBox Integration (Recommended for inclusion)**
- Low risk; well-established technology
- Good for users wanting centralized inventory
- Optional feature; doesn't break standalone Host Manager
- Can document as "advanced feature" for Phase 3 roadmap

**Phase 3b: Infragraph Integration (Recommended deferred)**
- Too early; public preview status
- Wait for stable release + wider adoption
- Higher risk for production deployments
- Good long-term vision; defer to Phase 3b/2027

**Phase 3c: AI-Driven Coordination (Future exploration)**
- Interesting vision
- Requires Infragraph stability + maturity
- Aligns with Host Manager's "agentic" control plane (WasmCloud actors)
- Document as "future direction"

---

## Next Steps for Phase 3 Planning

1. Validate NetBox API integration effort (estimate hours for POC)
2. Monitor Infragraph release cycle (target: stable release in late 2026)
3. Gather user feedback: "Do you use NetBox?" (inform decision)
4. Plan Phase 3a NetBox integration after Phase 1 MVP
5. Re-evaluate Infragraph in Q4 2026 for Phase 3b planning

---

## Sources

- [NetBox Labs Documentation](https://netboxlabs.com/docs/netbox/)
- [NetBox REST API](https://github.com/netbox-community/netbox)
- [NetBox Discovery Product](https://netboxlabs.com/products/netbox-discovery/)
- [Infragraph Overview - HCP](https://developer.hashicorp.com/hcp/docs/infragraph)
- [Introducing HCP Terraform powered by Infragraph](https://www.hashicorp.com/en/blog/introducing-hcp-terraform-powered-by-infragraph-in-public-preview)
- [Project Infragraph InfoQ](https://www.infoq.com/news/2025/10/hashicorp-project-infragraph/)
- [NetBox Plugin Development](https://netboxlabs.com/docs/netbox/plugins/development/)
