# ADR-007: API Authentication and Authorization

**Status:** Proposed — Amended by ADR-011

**Date:** 2026-05-10

**Authors:** Joseph Pearson

---

## Context

The Host Manager API Gateway actor (ADR-001) exposes a REST API used by the CLI,
web UI, and future integrations. Every request to this API must be authenticated
— the system manages credentials for production endpoints, can execute commands
on remote hosts, and stores sensitive network topology information.

Two distinct authentication concerns exist:

1. **Human operator access** — the person (or small team) using the CLI and web UI
   to manage endpoints, view status, and issue commands.
2. **Agent access** — `hostmgr-agent` instances communicating with the control plane.
   Agent authentication is handled separately via NATS NKey credentials (ADR-005)
   and is out of scope for this ADR.

### MVP Scope

For MVP, Host Manager is a **single-operator tool**. One person (or a small, fully
trusted team) administers one control plane instance. There are no untrusted users,
no customer-facing portals, and no need for fine-grained role separation at launch.

The authentication mechanism must:
- Be secure by default (no unauthenticated API access)
- Require no password management infrastructure (no local user database to maintain)
- Support the web UI OAuth flow and CLI token-based access
- Be simple enough to configure with environment variables (ADR-009)

### Identity Provider Choice

Rather than building a local user management system, Host Manager delegates identity
to established, widely-trusted OAuth 2.0 / OpenID Connect (OIDC) providers. For MVP,
two providers are supported:

- **GitHub** — natural fit for an open-source developer tool; operators are likely
  to have GitHub accounts; organisational membership can be used as an additional
  access control layer in future phases
- **Google** — broad coverage for operators using Google Workspace or personal
  Google accounts; mature OIDC implementation

Both providers support the PKCE (Proof Key for Code Exchange) extension, which
eliminates the need for a client secret in browser-based flows.

---

## Decision

**For MVP, all human access to the Host Manager API is authenticated via OAuth 2.0
/ OIDC using GitHub or Google as identity providers. Authenticated sessions are
represented as short-lived JWTs issued by the API Gateway. Access is restricted to
an operator-configured allowlist of provider identities. There is no RBAC; all
authenticated allowlisted users have full access.**

Programmatic access (API keys for CI/CD, scripts, integrations) is deferred to
Phase 2.

---

## Rationale

### Why OAuth 2.0 / OIDC (Not Local Users)

**No credential management burden.** A local user database requires password hashing,
reset flows, brute-force protection, and secure storage. These are solved problems —
but they are solved problems that belong to GitHub and Google, not Host Manager.
Delegating to an established provider means Host Manager inherits their security
posture (MFA, suspicious login detection, breach response) for free.

**Natural fit for the operator profile.** Host Manager is an open-source developer
tool. Its operators are highly likely to have GitHub or Google accounts. Requiring
them to create a separate local account adds friction with no security benefit.

**Stateless session tokens.** OAuth 2.0 + JWT sessions do not require a session
database. The API Gateway verifies the JWT signature on each request and reads claims
from the token itself. This is consistent with the WasmCloud actor model, where actors
should not hold local state (ADR-001).

### Why GitHub and Google Specifically

**GitHub:** The open-source community's de facto identity layer. Operators working
with Host Manager's source code, issues, and releases already have GitHub accounts.
GitHub OAuth supports organisation and team membership as additional claims, enabling
richer access control in Phase 2 without changing providers.

**Google:** The most widely deployed OAuth provider. Covers operators using Google
Workspace (common in SMBs and startups) and personal Gmail accounts. OIDC support
is mature and well-documented.

Supporting both from the start avoids locking operators into a single provider.
The allowlist mechanism means only specific identities gain access regardless of
which provider they use.

### Why an Allowlist (Not "Any GitHub/Google User")

Without an allowlist, any person with a GitHub or Google account could authenticate
to a Host Manager instance that is reachable from the internet. This is not
acceptable — Host Manager manages production infrastructure. The allowlist
(`HOSTMGR_ALLOWED_USERS`, configured via environment variable) explicitly enumerates
which provider identities are permitted. The default is an empty allowlist, meaning
the API is locked until the operator explicitly grants access to at least one identity.

### Why JWTs for Session Tokens

- **Stateless:** No server-side session store required; consistent with actor model
- **Short-lived:** Default 8-hour expiry reduces window for token misuse
- **Verifiable:** Signed with a key stored in OpenBao; signature validation is
  the only server-side check required
- **Standard:** CLI and future integrations can use Bearer token auth with
  well-understood tooling

### Why No RBAC for MVP

With a single operator and full mutual trust among allowlisted users, role separation
provides no security benefit and adds implementation and configuration complexity.
RBAC requires a permissions model, a role assignment store, and enforcement in every
API handler. For MVP, the correct answer is: if you are on the allowlist, you have
full access; if you are not, you have no access.

---

## Architecture

### Authentication Flow (Web UI — PKCE)

```
Browser                API Gateway             GitHub / Google
   │                       │                        │
   │  1. GET /ui            │                        │
   │──────────────────────►│                        │
   │◄──────────────────────│                        │
   │  302 → /auth/login     │                        │
   │                       │                        │
   │  2. GET /auth/login    │                        │
   │──────────────────────►│                        │
   │     (provider=github)  │                        │
   │  API Gateway generates:│                        │
   │  - code_verifier (random, stored in session cookie)
   │  - code_challenge = SHA256(code_verifier)      │
   │  - state (CSRF token)                          │
   │                       │                        │
   │◄──────────────────────│                        │
   │  302 → github.com/login/oauth/authorize        │
   │    ?client_id=...      │                        │
   │    &redirect_uri=...   │                        │
   │    &code_challenge=... │                        │
   │    &state=...          │                        │
   │                       │                        │
   │  3. User authorises on GitHub                  │
   │────────────────────────────────────────────►  │
   │◄───────────────────────────────────────────── │
   │  302 → /auth/callback?code=...&state=...       │
   │                       │                        │
   │  4. GET /auth/callback │                        │
   │──────────────────────►│                        │
   │  API Gateway:          │                        │
   │  - validates state     │                        │
   │  - POSTs code + code_verifier to GitHub        │
   │───────────────────────────────────────────────►│
   │◄───────────────────────────────────────────────│
   │                  access_token                  │
   │  - fetches user identity from GitHub API       │
   │───────────────────────────────────────────────►│
   │◄───────────────────────────────────────────────│
   │            { login: "joewxboy", ... }          │
   │  - checks allowlist: "github:joewxboy" ∈ HOSTMGR_ALLOWED_USERS?
   │    → Yes: issue signed JWT, set HttpOnly cookie│
   │    → No:  401 Forbidden                        │
   │◄──────────────────────│                        │
   │  Set-Cookie: hostmgr_session=<JWT>; HttpOnly; Secure; SameSite=Lax
   │                       │                        │
   │  5. Subsequent API requests include cookie     │
   │  API Gateway validates JWT signature on each request
   │  (no database lookup required)                 │
```

### Authentication Flow (CLI — Device Code)

The CLI cannot open a browser tab for the PKCE flow. It uses the OAuth 2.0 Device
Authorization Grant (RFC 8628), supported by both GitHub and Google:

```
CLI                         API Gateway             GitHub / Google
 │                               │                        │
 │  hostmgr auth login           │                        │
 │──────────────────────────────►│                        │
 │  POST /auth/device            │                        │
 │                               │ POST device_authorization endpoint
 │                               │───────────────────────►│
 │                               │◄───────────────────────│
 │                               │ { device_code, user_code, verification_uri }
 │◄──────────────────────────────│                        │
 │  "Visit https://github.com/login/device"               │
 │  "Enter code: ABCD-1234"      │                        │
 │                               │                        │
 │  (User opens browser, enters code, authorises)         │
 │                               │                        │
 │  (CLI polls every 5s)         │                        │
 │──────────────────────────────►│                        │
 │  POST /auth/device/token      │ POST token endpoint    │
 │                               │───────────────────────►│
 │                               │◄───────────────────────│
 │                               │ access_token           │
 │                               │ (fetch identity, check allowlist)
 │◄──────────────────────────────│                        │
 │  { token: "<JWT>" }           │                        │
 │                               │                        │
 │  CLI stores JWT in            │                        │
 │  ~/.config/hostmgr/token      │                        │
 │  (mode 0600)                  │                        │
```

CLI stores the JWT locally. Subsequent `hostmgr` commands include it as a
`Authorization: Bearer <JWT>` header. Token refresh is triggered automatically
when the JWT is within 15 minutes of expiry.

### JWT Structure

```json
{
  "header": {
    "alg": "EdDSA",
    "typ": "JWT"
  },
  "payload": {
    "iss": "https://control.example.com",
    "sub": "github:joewxboy",
    "provider": "github",
    "provider_id": "12345678",
    "iat": 1746878400,
    "exp": 1746907200,
    "jti": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

- **Algorithm:** EdDSA (Ed25519) — compact, fast, no weak-key risks
- **Signing key:** Ed25519 keypair stored in OpenBao at `secret/config/jwt_signing_key`;
  generated on first run if not present
- **Expiry:** 8 hours (configurable via `HOSTMGR_JWT_TTL`)
- **`sub` format:** `{provider}:{username_or_id}` — matches allowlist format
- **`jti`:** UUID for revocation logging (Phase 2)

### Allowlist Configuration

The allowlist is configured via the `HOSTMGR_ALLOWED_USERS` environment variable
(ADR-009):

```bash
# Format: comma-separated provider:identity pairs
HOSTMGR_ALLOWED_USERS="github:joewxboy,google:joe@example.com"
```

The API Gateway actor reads this at startup. Changes require a restart (Phase 2 will
support dynamic reload).

Provider identity formats:

| Provider | Format | Example |
|---|---|---|
| GitHub | `github:{login}` | `github:joewxboy` |
| Google | `google:{email}` | `google:joe@example.com` |

An empty allowlist (`HOSTMGR_ALLOWED_USERS=""`) causes the API Gateway to reject
all authenticated requests. This is the secure default — the operator must
explicitly grant access.

### OAuth Application Configuration

Each deployment requires its own OAuth application registrations:

**GitHub:**
```
Application name:   Host Manager (your-instance-name)
Homepage URL:       https://control.example.com
Callback URL:       https://control.example.com/auth/callback/github
```

**Google:**
```
Application name:   Host Manager (your-instance-name)
Authorised redirect URI: https://control.example.com/auth/callback/google
Scopes: openid, email, profile
```

Client IDs are public; client secrets are stored in OpenBao:
```
secret/config/oauth/github_client_id
secret/config/oauth/github_client_secret
secret/config/oauth/google_client_id
secret/config/oauth/google_client_secret
```

The API Gateway retrieves these at startup via the credential capability provider
(ADR-002). They are never written to environment variables or disk.

### API Request Authentication

All API requests (except `/auth/*` and `/health`) require a valid JWT:

```
GET /api/v1/endpoints
Authorization: Bearer eyJ...

→ API Gateway:
  1. Decode JWT header/payload (no signature check yet)
  2. Verify issuer matches this control plane's URL
  3. Verify expiry (exp > now)
  4. Verify signature using Ed25519 public key from OpenBao
  5. Check subject (sub) against in-memory allowlist
  → All pass: forward request to appropriate actor
  → Any fail: 401 Unauthorized, log reason
```

Step 4 (signature verification) is the cryptographic proof. Steps 2, 3, 5 are
defence-in-depth checks.

---

## Consequences

### Positive Impacts

**1. No local credential management**
No user database, no password reset flows, no brute-force protection to implement.
Security of operator identity is fully delegated to GitHub and Google.

**2. MFA inherited automatically**
If an operator has MFA enabled on their GitHub or Google account (strongly encouraged),
that protection applies to Host Manager access with no additional configuration.

**3. Stateless session validation**
JWT verification requires only a signature check. No database or cache lookup. The
API Gateway actor remains stateless, consistent with the WasmCloud actor model.

**4. Simple configuration**
Two environment variables (`HOSTMGR_ALLOWED_USERS` plus provider client IDs/secrets)
fully configure the authentication system. No user management UI needed for MVP.

**5. Extensible foundation**
The allowlist and JWT approach is forward-compatible with RBAC (add a `roles` claim
to the JWT), additional providers (add a new callback handler), and API keys (issue
JWTs with `sub: apikey:{id}` and no expiry, store in OpenBao).

### Risks and Mitigations

| Risk | Mitigation |
|---|---|
| GitHub or Google outage blocks login | JWT sessions last 8 hours; existing sessions continue working during provider outages |
| JWT signing key compromise | Key stored in OpenBao with audit logging; rotation invalidates all active sessions (intentional); key rotation procedure documented |
| Allowlist misconfiguration (empty list locks out operator) | Control plane provides an emergency `--emergency-access` startup flag that bypasses allowlist for one session (logs prominently); documented recovery procedure |
| CSRF attack on OAuth callback | `state` parameter validated on callback; SameSite=Lax cookie policy; PKCE eliminates code injection risk |
| JWT stored insecurely on CLI host | Stored at `~/.config/hostmgr/token` with mode `0600`; documentation warns against sharing; token rotation on `hostmgr auth logout` |

### Implementation Considerations

- The API Gateway actor handles all `/auth/*` routes. Other actors never see
  unauthenticated requests — the Gateway validates the JWT before routing.
- The OAuth client secret is retrieved from OpenBao at startup, not cached beyond
  the startup sequence. If OpenBao is unavailable at startup, the API Gateway
  cannot initialise (fail-closed is correct).
- For local development without internet access, a `HOSTMGR_DEV_TOKEN` environment
  variable can bypass OAuth entirely (disabled in production builds via compile flag).
- Token refresh: the CLI automatically refreshes the JWT by re-initiating the device
  flow when the stored token is within 15 minutes of expiry. The web UI refreshes
  silently using a short-lived refresh token stored in an HttpOnly cookie.

---

## Alternatives Considered

### Alternative 1: Local Username / Password

**Decision:** Rejected

**Rationale:** Requires building and maintaining a local user database with password
hashing, reset flows, rate limiting, and secure storage. All of this is solved by
delegating to GitHub/Google. The only scenario where local auth is necessary is
air-gapped deployments with no external identity provider access — a Phase 3
consideration, where a self-hosted OIDC provider (Dex, Authentik) would be the
correct answer rather than a custom implementation.

### Alternative 2: API Keys Only (No OAuth)

**Decision:** Rejected for human access; implemented in Phase 1.1 for programmatic
access (amended from original Phase 2 by ADR-011)

**Rationale:** API keys are the right model for machine-to-machine access (CI/CD
pipelines, scripts, integrations) but a poor model for human operators. Humans
benefit from provider-enforced MFA, suspicious login detection, and single sign-on
across devices. Storing and rotating API keys securely is an operational burden that
OAuth eliminates for human users. API keys are implemented in Phase 1.1 (moved
from Phase 2) because the MCP client capability and AI agent access defined in
ADR-011 require non-interactive authentication from the start of Phase 1.1.

### Alternative 3: mTLS (Mutual TLS)

**Decision:** Rejected for human access; noted for agent access (ADR-005)

**Rationale:** mTLS is the correct authentication mechanism for service-to-service
communication, including agent ↔ control plane (handled via NKeys in ADR-005).
For human operators using a browser or CLI, mTLS requires managing client certificates
— generating, distributing, rotating, and revoking them. This is more complex and
less user-friendly than OAuth without providing meaningfully better security for
the MVP human access case.

### Alternative 4: Self-hosted OIDC Provider (Dex, Keycloak, Authentik)

**Decision:** Deferred to Phase 3

**Rationale:** A self-hosted OIDC provider adds a significant infrastructure component
but is necessary for:
- Air-gapped deployments (no internet access to GitHub/Google)
- Enterprise LDAP/SAML integration
- Fine-grained RBAC beyond what GitHub/Google claims provide

The JWT and allowlist architecture chosen here is fully compatible with a self-hosted
OIDC provider — the API Gateway would simply accept tokens from a different issuer.
This migration path is low-friction.

### Alternative 5: RBAC for MVP

**Decision:** Rejected; deferred to Phase 2

**Rationale:** With a single operator and full mutual trust among all allowlisted users,
RBAC provides no security benefit for MVP. Implementing RBAC correctly (role model,
assignment storage, enforcement, UI) is substantial scope. The decision to defer is
intentional: Phase 2 will add a `roles` JWT claim and enforcement middleware once
there is a real access control requirement from multi-user deployments.

---

## Future Phases

### Phase 1.1: API Keys for Programmatic Access *(amended from Phase 2 — see ADR-011)*

- Operator generates API keys via CLI (`hostmgr keys create`) or Web UI
- Keys stored as HMAC-hashed values in OpenBao
- API Gateway accepts `Authorization: Bearer hm_ro_<key>` or `hm_rw_<key>`
  alongside OAuth JWT Bearer tokens
- Default scope: read-only; read-write requires explicit opt-in with confirmation
- Full key design, format, and validation flow specified in ADR-011
- One-time code (OTC) OAuth pattern for cross-origin React Web UI added (ADR-011)

### Phase 2: RBAC

- Roles: `admin` (full access), `operator` (read + execute), `viewer` (read-only)
- Role assigned per allowlist entry: `github:joewxboy:admin`
- Enforced in API Gateway before routing to actors

### Phase 3: Self-hosted OIDC / Enterprise Identity

- Dex or Authentik as a self-hosted OIDC provider for air-gapped deployments
- LDAP and SAML connectors for enterprise SSO
- API Gateway accepts tokens from any configured issuer (not just GitHub/Google)

### Phase 3: Audit Log

- All authenticated API calls logged with `sub`, timestamp, action, and result
- Logs shipped to operator-configured destination (NATS JetStream, syslog, S3)

---

## Related Decisions

- **ADR-001:** WasmCloud Control Plane — API Gateway actor handles authentication;
  other actors receive only pre-authenticated requests
- **ADR-002:** Credential Storage — OAuth client secrets and JWT signing key stored
  in OpenBao
- **ADR-005:** Agent Communication Protocol — Agent authentication via NKeys;
  separate from human access and out of scope for this ADR
- **ADR-009:** Configuration & Packaging — `HOSTMGR_ALLOWED_USERS` and provider
  client IDs configured via environment variables

---

## Open Questions

1. **Token refresh for web UI:** Short-lived refresh tokens (stored HttpOnly) vs.
   silent re-auth on JWT expiry. Refresh tokens are more seamless but require
   server-side state (refresh token store) or careful rotation. Decision: implement
   silent re-auth first (simpler); add refresh tokens if session interruption is
   reported as a pain point.

2. **GitHub organisation restriction:** Should the allowlist support
   `github-org:my-org` as a group entry (all members of a GitHub organisation)?
   Useful for team deployments. Deferred to Phase 2; the allowlist format is
   extensible.

3. **HOSTMGR_DEV_TOKEN in production builds:** Compile-time flag vs. runtime check.
   Compile-time is safer (cannot be enabled by accident) but requires separate
   dev/prod build. Runtime check with an explicit `HOSTMGR_INSECURE_DEV_MODE=true`
   guard is simpler but requires documentation to warn against production use.

---

## Decision Record

**Approved by:** [To be filled after review]

**Date approved:** [To be filled]

**Concerns / feedback addressed:** [To be filled after discussion]
