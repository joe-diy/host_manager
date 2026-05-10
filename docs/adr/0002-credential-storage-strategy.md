# ADR-002: Credential Storage Strategy

**Status:** Proposed

**Date:** 2026-05-10

**Authors:** Joseph Pearson

---

## Context

Host Manager's MVP must securely store and manage credentials for accessing diverse endpoints:
- SSH keys and passwords for Linux servers and Raspberry Pis
- VM credentials for cloud platforms (Zededa Cloud, Mainsail Starlight)
- API tokens and kubeconfigs for Kubernetes clusters
- Optional sudo configurations for privileged operations

The control plane (WasmCloud) needs to provide credentials securely to:
- Native agents deployed on endpoints (via secure capability provider)
- Management components coordinating across endpoints
- Future extensions (e.g., operator dashboards, integrations)

**Key Requirements:**
1. Secure storage: Credentials encrypted at rest and in transit
2. Pluggable backend: Users should choose their own credential store
3. Open-source alignment: Host Manager is Apache 2.0 licensed
4. WasmCloud integration: Leverage WasmCloud's secrets architecture
5. Capability-based access: Components only access credentials they need

**Technology Context:**
- WasmCloud has a well-designed secrets backend system (pluggable, just-in-time delivery, encrypted transit via xkeys)
- Two primary options: HashiCorp Vault (BSL, enterprise-focused) vs. OpenBao (MPL 2.0, open-source fork)
- Both have API compatibility; easy to support both without duplicating code

---

## Decision

**Host Manager will adopt OpenBao as the primary credential storage backend for MVP, with support for HashiCorp Vault, Kubernetes Secrets, and other backends as optional alternatives via WasmCloud's pluggable architecture.**

### Specific Decisions:

1. **Primary backend:** OpenBao (v2.5.0+)
   - Default configuration for Host Manager
   - Used in MVP and all development/testing

2. **Secondary backends:** Support via configuration
   - HashiCorp Vault (for users who prefer/have existing infrastructure)
   - Kubernetes Secrets (for K8s-native deployments)
   - NATS KV (for lightweight edge deployments)
   - Extensible: Users/operators can add custom backends

3. **Architecture:** Build credential provider to be backend-agnostic
   - WasmCloud handles credential resolution
   - Components request credentials by reference, not implementation detail
   - Backend selection via configuration (no code changes)

4. **Credential types to support (MVP):**
   - SSH private keys (PEM, OpenSSH format)
   - Passwords/passphrases
   - API tokens (generic string secrets)
   - Kubeconfigs (structured YAML)
   - Sudo configurations (username/password/nopass rules)

5. **Deployment model:** Host Manager provides:
   - Helm charts for OpenBao deployment (recommended)
   - Documentation for Vault integration (if users prefer)
   - Example configurations for each backend

---

## Rationale

### Why OpenBao (Primary)

**1. License Alignment (Critical)**
- Host Manager: Apache 2.0 (permissive open source)
- OpenBao: MPL 2.0 (permissive open source, no commercial restrictions)
- Vault: BSL 1.1 (commercial restrictions on competitive offerings)

OpenBao's licensing creates no ambiguity: users can freely distribute, commercialize, and modify Host Manager without licensing concerns. Vault's BSL introduces risk that a commercial Host Manager deployment could require licensing negotiations with HashiCorp.

**2. Governance Stability**
- OpenBao: Linux Foundation (LFedge) governance; IBM contributes but doesn't control
- Vault: IBM-owned since Feb 2025; history of licensing/pricing changes (BSL adoption in Aug 2023, product sunsets in 2025)

For a long-lived open-source project, community governance is more stable than corporate ownership changes.

**3. Cost for Growth**
- OpenBao: Namespaces (multi-tenancy) and horizontal read scaling included free
- Vault: These features require Enterprise license ($$$$)

If Host Manager grows to support multi-team deployments or high-load scenarios, OpenBao scales without licensing upgrades.

**4. Production Readiness (Feb 2026)**
- v2.5.0 released Feb 2026; actively maintained
- EdgeX Foundry adopted as default secrets store
- Joined OpenSSF (Open Source Security Foundation) for security vetting
- 287 active contributors; quarterly releases
- Linux Foundation backing reduces abandonment risk

OpenBao is not as battle-tested as Vault (since 2015), but is mature enough for MVP and beyond.

**5. No Critical Feature Gaps**
- Both support API-compatible authentication, secret engines, policies
- Host Manager MVP doesn't require Vault Enterprise features:
  - ❌ DR Replication (disaster recovery across regions) — not needed for MVP
  - ❌ Performance Replication (active-active multi-region) — future consideration
  - ✅ Namespaces (multi-tenancy) — included free in OpenBao
  - ✅ Horizontal scaling (read scaling) — included free in OpenBao 2.5.0+

### Why Support Vault (Secondary)

**1. User Choice & Migration Path**
- Many teams already use Vault (de facto standard since 2015)
- If a user has existing Vault infrastructure, they should be able to use it
- API compatibility means supporting both requires minimal effort

**2. License Flexibility**
- Internal use of Vault is unrestricted (BSL permits this)
- Users who value Vault's enterprise support can use it
- WasmCloud's pluggable architecture means no lock-in

**3. Zero Additional Cost**
- WasmCloud already has a Vault secrets backend
- Vault integration requires only configuration + documentation, not new code

### Why NOT Other Solutions (at MVP stage)

**Cloud Provider Native Secrets (AWS Secrets Manager, Azure Key Vault, GCP Secret Manager):**
- ❌ Couples Host Manager to specific cloud provider
- ❌ Breaks topology-agnostic design (control plane can run anywhere)
- ✅ Defer to Phase 2; can add as backends if needed

**Encrypted Files / Custom Solution:**
- ❌ Reinventing the wheel (Vault/OpenBao already solve this well)
- ❌ Security burden (key rotation, audit, compliance)
- ❌ Harder to integrate with WasmCloud

**K8s Secrets Only:**
- ❌ Requires Kubernetes for credential storage (couples to platform)
- ❌ Host Manager MVP may not run in Kubernetes
- ✅ Support K8s Secrets as optional backend for K8s-native deployments

---

## Consequences

### Positive Impacts

**1. Licensing clarity**
- No ambiguity about redistribution, commercialization, or licensing costs
- Entire Host Manager stack is permissive open source (Apache 2.0 + MPL 2.0)
- Users can fork/modify without licensing concerns

**2. Cost-friendly growth**
- Multi-tenancy (Namespaces) is free in OpenBao
- Horizontal scaling is free in OpenBao
- No unexpected licensing upgrades

**3. Flexible backend choice**
- Users can select OpenBao, Vault, K8s Secrets, or others
- Configuration-driven; no code changes to swap backends
- Future integration with other secret stores is straightforward

**4. Community alignment**
- OpenBao backed by Linux Foundation (same as Linux, CNCF projects)
- Large ecosystem of operators and tools already support OpenBao
- Open governance reduces vendor lock-in risk

### Implementation Challenges

**1. OpenBao is younger than Vault**
- Smaller community and fewer production deployments at massive scale
- May encounter edge cases or bugs that Vault has already solved
- Mitigation: WasmCloud support for Vault allows fallback if needed

**2. No Disaster Recovery Replication (OpenBao)**
- OpenBao lacks built-in DR replication (async backup to remote cluster)
- Vault Enterprise has this as a paid feature
- Mitigation: For MVP, manual backups or external replication tools are sufficient; Vault can be added later if this becomes critical

**3. Migration complexity (if switching from Vault to OpenBao)**
- No automated migration tool; API-driven re-import required
- Not a concern for greenfield Host Manager deployments
- Mitigation: Well-documented migration guide; hybrid approach means users can run both initially

**4. Support burden**
- OpenBao community support is smaller than Vault's commercial support
- No paid support contracts from OpenBao (community-driven)
- Mitigation: Host Manager documentation should clearly guide users on troubleshooting; community Slack/GitHub for questions

### Risks

**1. OpenBao project stagnation**
- Unlikely given Linux Foundation backing and IBM contribution, but possible
- Mitigation: WasmCloud supports multiple backends; easy to add Vault or other alternative if needed

**2. Critical security vulnerability in OpenBao**
- Any credential store could have vulnerabilities
- Mitigation: Use latest stable releases; follow OpenBao security advisories; Host Manager documentation recommends OpenBao security practices

**3. User preference for Vault**
- Some users may insist on Vault due to existing deployments
- Mitigation: Full support for Vault via WasmCloud means users have choice

---

## Alternatives Considered

### Alternative 1: HashiCorp Vault as Primary

**Decision:** Rejected

**Rationale:**
- BSL licensing creates ambiguity for commercial/distributed Host Manager
- IBM acquisition (Feb 2025) introduces governance uncertainty
- Aligns with proprietary software, not open-source principles
- Costs more if users need Enterprise features (Namespaces, performance replication)
- Not necessary for MVP; Vault doesn't provide capabilities Host Manager can't achieve with OpenBao

**Retention:** Support Vault as secondary option for users who have existing infrastructure

### Alternative 2: Kubernetes Secrets Only

**Decision:** Rejected for primary use case; accepted as optional backend

**Rationale:**
- Couples credential storage to Kubernetes platform
- Host Manager must work on standalone servers, VMs, edge devices (not just K8s)
- Limits deployment flexibility and portability

**Retention:** K8s Secrets as optional backend for users running Host Manager in Kubernetes

### Alternative 3: Cloud Provider Secrets (AWS Secrets Manager, etc.)

**Decision:** Rejected for MVP; deferred to Phase 2

**Rationale:**
- Breaks topology-agnostic design (control plane must work anywhere)
- Couples Host Manager to specific cloud provider
- Not necessary for MVP; can add later as optional backends

**Retention:** Design credential provider to allow plugging in cloud provider backends in future

### Alternative 4: Custom Encrypted Credential Store

**Decision:** Rejected

**Rationale:**
- Reinventing wheel (Vault/OpenBao already well-designed)
- Security burden: key rotation, audit, compliance
- Maintenance cost; harder to operate than established solutions
- Host Manager should focus on endpoint management, not credential storage

---

## Implementation Approach

### Phase 1: OpenBao Integration (MVP)
1. Deploy OpenBao (Helm chart or standalone) in dev/test environment
2. Build WasmCloud credential provider that:
   - Connects to OpenBao via HTTPS
   - Authenticates (AppRole or similar)
   - Retrieves credentials by reference (e.g., "endpoint_creds_prod/server1")
   - Returns encrypted credentials to requesting component
3. Define credential schema:
   - SSH keys: `{type: "ssh_key", private_key: "...", passphrase: "..."}`
   - Passwords: `{type: "password", username: "...", password: "..."}`
   - Kubeconfigs: `{type: "kubeconfig", data: "..."}`
   - Sudo rules: `{type: "sudo", user: "...", allow_no_pass: true}`
4. Document: How to deploy and configure OpenBao for Host Manager
5. Test: Agent credential retrieval via control plane

### Phase 2: Vault Support (Post-MVP)
1. Add Vault secrets backend configuration to WasmCloud manifests
2. Test credential provider with Vault
3. Document: How to use Vault instead of OpenBao
4. Users can choose OpenBao or Vault via simple configuration change

### Phase 3: Other Backends (Future)
- K8s Secrets backend (for K8s-native deployments)
- Cloud provider secrets (AWS Secrets Manager, etc.)
- Custom backends (via documented interface)

---

## Related Decisions

- **ADR-001:** WasmCloud Control Plane Architecture — sets context for credential provider design
- **ADR-003:** Network Discovery — will need credentials to access endpoints
- **ADR-004:** Agent Communication Protocol — agents request credentials from control plane

---

## Monitoring & Review

**Decision review criteria:**
- OpenBao stability: Track releases, security advisories, community activity
- User feedback: Gather input on credential backend preferences in early deployments
- Licensing changes: Monitor Vault and OpenBao licensing developments
- Feature gaps: If Host Manager needs Vault Enterprise features (DR replication, etc.), evaluate switching

**Review date:** After MVP launch (Q3 2026); reassess every 6-12 months

---

## Open Questions / Future Exploration

1. **High-availability credential store:** Should Host Manager recommend HA configuration for OpenBao? (Yes, for production; no for MVP)

2. **Credential rotation:** Should Host Manager support automatic credential rotation? (Deferred to Phase 2; OpenBao/Vault both support this)

3. **Audit logging:** Should Host Manager emit audit logs for credential access? (WasmCloud + OpenBao handle this separately; document best practices)

4. **Encryption at rest:** Should Host Manager encrypt credentials before storing in secret store? (No; rely on OpenBao's encryption; redundant)

5. **Credential templates:** Should Host Manager support dynamic credential generation (e.g., generate temporary SSH keys on-demand)? (Deferred to Phase 2; OpenBao supports this)

---

## Appendix: OpenBao vs Vault Feature Matrix

| Feature | OpenBao | Vault | WasmCloud Support |
|---------|---------|-------|---|
| API Compatibility | ✅ Yes (mostly) | N/A | Both |
| Namespaces (Multi-tenancy) | ✅ Free | ❌ Enterprise | OpenBao, via WasmCloud |
| Horizontal Read Scaling | ✅ Free (2.5.0+) | ❌ Enterprise | OpenBao, via WasmCloud |
| DR Replication | ❌ | ✅ Enterprise | Vault, via WasmCloud |
| Performance Replication | ❌ | ✅ Enterprise | Vault, via WasmCloud |
| Kubernetes Integration | ✅ Helm | ✅ Helm | Both |
| Commercial Support | ❌ Community | ✅ IBM/HashiCorp | Vault |
| License | MPL 2.0 | BSL 1.1 | OpenBao (for open projects) |
| Governance | Linux Foundation | IBM | OpenBao (community-driven) |

---

## Decision Record

**Approved by:** [To be filled after review]

**Date approved:** [To be filled]

**Concerns/feedback addressed:** [To be filled after discussion]
