# Vault vs OpenBao: Credential Storage Research

Comprehensive comparison of HashiCorp Vault and OpenBao for Host Manager's credential storage backend.

## Executive Summary

| Aspect | HashiCorp Vault | OpenBao |
|--------|---|---|
| **License** | BSL 1.1 (commercial restrictions) | MPL 2.0 (true open source) |
| **Version (2026)** | 1.16+ | 2.5.0+ (Feb 2026) |
| **Maturity** | Battle-tested, mature | Production-ready, actively developed |
| **Governance** | HashiCorp (owned by IBM as of Feb 2025) | Linux Foundation (LF EDGE) |
| **Community** | Large, commercial backing | Growing, community-driven |
| **Free Namespaces** | ❌ Enterprise only | ✅ Included |
| **WasmCloud Support** | ✅ Full | ✅ Full |
| **Best For** | Enterprise with DR needs | Open-source projects, cost-conscious teams |

---

## Licensing Comparison

### HashiCorp Vault: BSL 1.1

**Status:** Changed from MPL 2.0 to Business Source License in August 2023

**Restrictions:**
- Internal use: ✅ Unrestricted
- "Competitive offering": ❌ Prohibited
- Definition: Cannot host or embed Vault as a product sold to third parties that competes with HashiCorp's offerings

**Implication for Host Manager:**
- If you distribute Host Manager as a service (SaaS) with embedded Vault, you need a commercial license
- If users install it on-premises/self-hosted, no issue
- If you open-source it without commercializing it, likely acceptable but legally ambiguous

**Additional context:** IBM acquired HashiCorp in Feb 2025; product sunsets and support changes are ongoing (HCP Vault Secrets, HCP Vault Dedicated Starter tier discontinued mid-2025)

### OpenBao: MPL 2.0

**Status:** True open-source license since day one (Linux Foundation project)

**Freedom:**
- ✅ Use, modify, redistribute for any purpose (including commercial)
- ✅ No competitive restrictions
- ✅ Source code always available
- ✅ No licensing uncertainty as governance/ownership changes

**Implication for Host Manager:**
- Perfect alignment with Apache 2.0 licensed Host Manager
- No restrictions on how you distribute or commercialize
- Future-proof: ownership/governance changes don't affect your rights

---

## Feature Parity

### Fully Compatible (Both Have These)
- Authentication engines: Kubernetes, OIDC, LDAP, AppRole, etc.
- Secret engines: KV, dynamic secrets, database credentials, PKI
- Policies and access control
- API compatibility (mostly)
- Kubernetes integration via Helm
- High availability (HA) clustering

### OpenBao Advantages
- **Namespaces**: Multi-tenancy with complete administrative isolation at no cost
  - Vault requires Enterprise license for this
  - Valuable if you anticipate multi-team deployments
  
- **Horizontal read scaling**: HA standby nodes serve read requests locally
  - Vault sells this as "Performance Standby Nodes" (Enterprise)
  - Included free in OpenBao 2.5.0+

### Vault (Enterprise Only) Advantages
- **Disaster Recovery (DR) Replication**: Async replication to remote cluster for backup
  - OpenBao has no equivalent; requires manual backups
  - Critical for enterprises with strict recovery requirements
  
- **Performance Replication**: Active-active replication across multiple clusters
  - OpenBao not available; only HA (single primary)
  
- **Sentinel**: Pre-policy checks before secret operations
  - OpenBao uses CEL instead (less mature but adequate)

- **Commercial support**: HashiCorp (and now IBM) backing

---

## Production Readiness & Adoption

### HashiCorp Vault
- **Maturity**: Battle-tested in thousands of enterprises since 2015
- **Adoption**: De facto standard for secrets management
- **Support**: Commercial support available (now via IBM)
- **Risk**: Ongoing licensing/pricing uncertainty post-IBM acquisition

### OpenBao
- **Maturity**: Version 2.5.0 released Feb 2026; production-ready
- **Governance**: Linux Foundation (LFedge); IBM engineers contributing
- **Adoption (2026):** 
  - EdgeX Foundry adopted as default secrets store for 4.0 release
  - Joined OpenSSF (Open Source Security Foundation) sandbox in June 2025
  - ~5,900 GitHub stars; 287 active contributors
- **Stability**: Quarterly minor releases; active roadmap (2025-2026 direction approved)
- **Risk**: Smaller community than Vault; less battle-tested at massive scale

---

## Migration & Compatibility

### API Compatibility
- **Good news:** OpenBao API is compatible with Vault
  - Most client libraries work with both (hashicorp/vault libraries, Terraform provider)
  - Existing Vault integrations should port to OpenBao
  
- **Token differences:**
  - Vault: `hvs.<long_random>`, `hvb.<long_random>`, etc.
  - OpenBao: `sbr.<random>`
  - Old Vault tokens still accepted until TTL expires

### Migration Path (If switching from Vault to OpenBao)

**From Vault 1.14.x (Community):** 
- Real but bounded migration path
- Documented: https://openbao.org/docs/guides/migration/

**From Vault 1.15+ or Enterprise:**
- No snapshot/in-place upgrade
- API-driven re-import required:
  - Script recreation of secret engines, auth methods, policies
  - No automated tool as of early 2026
  - Reasonable effort for small deployments, complex for large ones

### Starting Fresh (Recommended for Host Manager)
- No migration concerns if you build with either platform from scratch
- Design secrets architecture to work with both (mostly API-compatible)
- Easy to swap later if needed

---

## Performance Considerations

**General Assessment:**
- Both perform well for typical workloads (few hundred ops/sec)
- Performance differences are operationally invisible at MVP scale
- Benchmarks show mixed results depending on operation type

**One reported benchmark:** OpenBao appeared ~4785x slower in a specific edge-case test, but this was debated; realistic production loads show similar performance.

**For Host Manager:** Performance is not a differentiator at MVP stage.

---

## Integration with WasmCloud

### Both Fully Supported

**WasmCloud secrets backends available for both:**
1. **Vault backend** — connects to HashiCorp Vault
2. **OpenBao backend** — connects to OpenBao
3. **NATS KV backend** — built-in key-value store
4. **Kubernetes secrets** — if running in K8s

**Example WasmCloud manifest:**
```yaml
secrets:
  default:
    backend: vault  # or openbao
    config:
      addr: https://vault.example.com
      mount_path: secret
      role: my_role
```

**How it works:**
- WasmCloud component requests secret (e.g., "endpoint_credentials_prod")
- Host's secrets backend resolves it from Vault/OpenBao
- Secret encrypted over NATS; component receives decrypted credential
- Host handles all authentication to secrets store

---

## Key Decision Factors

### Choose HashiCorp Vault If:
1. ✅ You need commercial support from HashiCorp/IBM
2. ✅ You require Disaster Recovery Replication for high-availability recovery
3. ✅ You need Performance Replication for active-active multi-region setup
4. ✅ Your organization already standardizes on Vault
5. ✅ You're building an enterprise product with premium support tiers

### Choose OpenBao If:
1. ✅ **You prioritize open-source licensing** (aligned with Apache 2.0 Host Manager)
2. ✅ **You want no licensing restrictions** on distribution/commercialization
3. ✅ **You need multi-tenancy (Namespaces)** at no cost
4. ✅ **You want community-driven governance** (Linux Foundation)
5. ✅ **You don't need DR/Performance replication** (or can implement separately)
6. ✅ **You prefer to avoid IBM/HashiCorp dependency** (post-acquisition uncertainty)

### Choose Neither (Not Applicable for Host Manager)
- Only if you have hyper-specific requirements (custom auth engines, proprietary HSM integration, etc.)
- For MVP, either Vault or OpenBao is more than sufficient

---

## Recommendation for Host Manager

### PRIMARY RECOMMENDATION: **OpenBao**

**Rationale:**

1. **License alignment:** Apache 2.0 Host Manager + MPL 2.0 OpenBao = perfect fit
   - No licensing ambiguity
   - Users can freely redistribute and commercialize
   - True open-source stack

2. **Governance trust:** Linux Foundation backing removes risk of future licensing/pricing changes
   - HashiCorp's BSL + IBM acquisition creates ongoing uncertainty
   - OpenBao's LFedge governance is stable and transparent

3. **Cost:** Namespaces and horizontal scaling included free
   - Vault Enterprise = $$$$ for these features
   - Host Manager can grow into multi-tenant without licensing upgrades

4. **Community momentum:** Growing adoption (EdgeX Foundry, OpenSSF)
   - Not as battle-tested as Vault, but production-ready as of Feb 2026

5. **No critical gaps:** You don't need DR/Performance replication for MVP
   - If needed later, can implement backup/restore separately
   - Easy to design secrets API that works with both Vault and OpenBao

### SECONDARY OPTION: **HashiCorp Vault**

If your team already knows Vault well and you:
- Have existing Vault infrastructure to integrate with
- Can justify commercial licensing to stakeholders
- Need commercial support channels

### HYBRID APPROACH (RECOMMENDED FOR FLEXIBILITY)

Design your credential provider to support **both** (API compatible):
- Start with OpenBao (lower cost, true open source)
- If you need Vault Enterprise features, swap backends without code changes
- WasmCloud's pluggable secrets architecture makes this trivial

Example strategy:
```
Host Manager MVP: OpenBao (default)
├── If users want Vault: Configuration change, no code changes
├── If users want K8s Secrets: Configuration change, no code changes
└── If users want AWS Secrets Manager: Add provider later
```

---

## Next Steps for ADR-002

1. **Choose:** OpenBao as primary; Vault as optional secondary
2. **Design:** WasmCloud credential provider interface (agnostic to backend)
3. **Implement:** OpenBao integration first; Vault/K8s as stretch goals
4. **Document:** How users can swap backends via configuration

---

## Sources

- [OpenBao vs HashiCorp Vault Showdown 2026](https://lalatenduswain.medium.com/openbao-vs-hashicorp-vault-the-secrets-management-showdown-every-devops-team-needs-to-read-in-2026-458ae0d9a408)
- [Choosing Secrets Storage: Vault vs OpenBao](https://digitalis.io/post/choosing-a-secrets-storage-hashicorp-vault-vs-openbao)
- [HashiCorp Vault BSL License FAQ](https://www.hashicorp.com/en/license-faq)
- [OpenBao Official Docs](https://openbao.org/)
- [OpenBao Roadmap 2025-2026](https://openbao.org/blog/roadmap-2.0/)
- [OpenBao Migration Guide](https://openbao.org/docs/guides/migration/)
- [WasmCloud Secrets Integration](https://wasmcloud.com/blog/secure-pluggable-webassembly-secrets-with-vault-k8s-secrets/)
- [OpenBao Joins OpenSSF](https://openssf.org/blog/2025/06/17/openbao-joins-the-openssf-to-advance-secure-secrets-management-in-open-source/)
