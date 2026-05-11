# ADR-011: Client Interface Strategy

**Status:** Proposed

**Date:** 2026-05-10

**Authors:** Joseph Pearson

---

## Context

Host Manager exposes a REST API (ADR-001, ADR-007) but the ADR set did not fully
define how humans, AI agents, and external systems interact with that API. Four
distinct client surfaces are required:

| Surface | Primary Users | Interaction Model |
|---|---|---|
| **Web UI** | Human operators (browser) | OAuth session → REST API |
| **CLI** | Human operators (terminal) | OAuth Device Code → REST API |
| **AI Agents / Automation** | Scripts, CI/CD, AI assistants | API keys → REST API |
| **MCP Client** | Host Manager actors consuming external MCP servers | MCP protocol → external tools |

These surfaces have different authentication requirements, deployment models, and
design constraints. Leaving them implicit creates integration gaps and inconsistent
operator experience.

### The MCP Dimension

The Model Context Protocol (MCP) is an open standard for connecting AI systems to
external tools and data sources. For Host Manager, MCP is relevant in two directions:

- **Expose as MCP server:** Host Manager presents its capabilities as MCP tools,
  allowing AI assistants (Claude, etc.) to query and control it via natural language.
- **Consume MCP servers:** Host Manager actors call out to external MCP servers
  during discovery and identification to enrich endpoint data — for example, querying
  a NetBox MCP server for documented device roles, or an asset management MCP server
  for ownership information.

The initial direction is **consume**: Host Manager gains a WasmCloud capability
provider that implements the MCP client protocol, allowing actors to optionally call
configured external MCP servers as part of their workflows. Exposing Host Manager
as an MCP server is deferred to Phase 2.

### Impact on ADR-007

ADR-007 deferred API keys to Phase 2. This ADR moves them to Phase 1.1 because:
- AI agents and automation scripts cannot use OAuth Device Code flow
- The MCP consume model needs a mechanism for external callers to authenticate
  when they initiate interactions with the Host Manager API
- API keys are simpler to implement than a full machine-to-machine OAuth flow and
  sufficient for the single-operator MVP context

ADR-007 is amended accordingly (see section: Impact on Existing ADRs).

---

## Decision

1. **Web UI:** React, bundled into the WasmCloud host image. Served by the API
   Gateway actor at the same origin as the REST API. Same-origin serving enables
   standard HttpOnly cookie sessions — no cross-origin token exchange required.
   Operators who do not want the UI can ignore it; it adds no runtime cost unless
   accessed.

2. **CLI:** A statically-linked Rust binary (`hostmgr`) distributed via GitHub
   Releases. Authenticates via OAuth Device Code flow (ADR-007). Output defaults to
   human-readable tables; `--output json` for scripting.

3. **AI Agents / Automation:** REST API access via API keys (moved to Phase 1.1).
   Keys are scoped read-only by default. Full-write keys are an explicit operator
   decision.

4. **MCP Client (consume):** A WasmCloud native capability provider implementing
   the MCP client protocol. Actors declare a dependency on this capability and call
   configured external MCP servers during discovery and identification for optional
   data enrichment. Phase 1.1. Exposing Host Manager as an MCP server is Phase 2.

---

## Rationale

### Why React for the Web UI

React is the most widely deployed UI framework in the ecosystem. Its component model,
developer tooling (Vite, React Query, Tanstack Router), and available UI component
libraries (shadcn/ui, Radix) produce maintainable, testable interfaces. For an
open-source project, React maximises the pool of potential contributors.

### Why Bundle the Web UI Into the WasmCloud Host Image

Bundling the React app into the WasmCloud host image means the UI is served from
the exact same origin (`https://control.example.com`) as the REST API and OAuth
callbacks. This unlocks the standard, battle-tested browser session model:

- The API Gateway sets an **HttpOnly, Secure, SameSite=Lax** cookie after OAuth
- The browser sends that cookie automatically on every subsequent API request
- No `Authorization: Bearer` header management in the React app
- No CSRF risk from cross-origin token exchange
- No OTC (one-time code) pattern, no in-memory JWT storage, no sessionStorage fallback

Bundling also means one fewer deployment component. Operators run one container
(or one WasmCloud host) and get both the API and the UI. There is no separate
service to configure, version-pin, or monitor.

The tradeoff is that UI changes require a new WasmCloud host image release. For
MVP this is acceptable — control plane and UI are co-developed and version-pinned
together. If UI and API release cadences diverge significantly in future phases,
a separate deployment can be extracted (see Alternatives).

### Why a Separate CLI Binary

The CLI is a thin REST client that formats API responses for terminal output. A
statically-linked Rust binary on GitHub Releases is the pattern established for
`hostmgr-agent` (ADR-006) — operators already know how to download and install it.
No package manager, runtime, or dependency is required.

### Why API Keys for AI Agent / Automation Access

OAuth flows (Authorization Code, Device Code) require human interaction. Scripts,
CI/CD pipelines, and AI agents cannot click through a browser or enter a device
code. API keys are the standard solution: the operator generates a key once
(via CLI or UI), the automation consumes it, and the key can be revoked at any time.

API keys are simpler to implement than machine-to-machine OAuth (client credentials
flow) and do not require registering the automation tool as an OAuth application.
For a single-operator MVP context, this is the right level of complexity.

### Why Consume MCP Servers (Not Expose as MCP Server) First

Consuming external MCP servers delivers immediate, concrete value during the
discovery and identification workflows: Host Manager can optionally enrich endpoint
records with data from tools the operator already has (NetBox, asset databases,
monitoring systems). This is additive — it does not change the core workflow; it
just adds optional context.

Exposing Host Manager as an MCP server (so Claude or other AI assistants can query
it conversationally) is valuable but has broader security design implications:
defining which tools to expose, what read/write semantics each tool has, and how
MCP-originating commands are distinguished from human-originating commands in the
audit log. These are Phase 2 concerns.

---

## Architecture

### Overall Interface Architecture

```
┌───────────────────────────────────────────────────────────────────┐
│                    Human Operator                                 │
│                                                                   │
│   Browser                        Terminal                         │
│   ┌──────────────┐               ┌────────────────┐              │
│   │  React Web   │               │  hostmgr CLI   │              │
│   │  UI          │               │  (static bin)  │              │
│   └──────┬───────┘               └───────┬────────┘              │
│          │ HTTPS + HttpOnly cookie       │ HTTPS + JWT            │
│          │ (same-origin PKCE OAuth)      │ (Device Code OAuth)    │
└──────────┼───────────────────────────────┼────────────────────────┘
           │                               │
┌──────────┼───────────────────────────────┼────────────────────────┐
│          │                               │   AI Agent /            │
│          │                               │   Automation            │
│          │                               │   ┌───────────────┐    │
│          │                               │   │ Script / CI / │    │
│          │                               │   │ AI Assistant  │    │
│          │                               │   └───────┬───────┘    │
│          │                               │           │ HTTPS       │
│          │                               │           │ + API Key   │
│          └───────────────┬───────────────┘           │            │
│                          │                           │            │
│               ┌──────────▼───────────────────────────▼──────┐    │
│               │        API Gateway Actor (WasmCloud)         │    │
│               │  • OAuth JWT validation                      │    │
│               │  • API key validation                        │    │
│               │  • CORS policy enforcement                   │    │
│               │  • Request routing to actors                 │    │
│               └─────────────────────────────────────────────┘    │
│                                                                   │
│               ┌─────────────────────────────────────────────┐    │
│               │        WasmCloud Actors (internal)          │    │
│               │  Discovery Orchestrator                      │    │
│               │  Identifier           ┌────────────────────┐│    │
│               │  Endpoint Manager     │  MCP Client        ││    │
│               │  Agent Coordinator    │  Capability        ││    │
│               │  Credential Manager   │  Provider          ││    │
│               └───────────────────────┴──────────┬─────────┘│    │
└──────────────────────────────────────────────────┼──────────┘    │
                                                   │ MCP protocol   │
                           ┌───────────────────────┼───────────┐   │
                           │  External MCP Servers  │           │   │
                           │                        ▼           │   │
                           │  ┌──────────┐  ┌──────────────┐   │   │
                           │  │  NetBox  │  │  Asset DB    │   │   │
                           │  │  MCP     │  │  MCP Server  │   │   │
                           │  └──────────┘  └──────────────┘   │   │
                           │  ┌────────────────────────────┐   │   │
                           │  │  Monitoring / Observability │   │   │
                           │  │  MCP Server                 │   │   │
                           │  └────────────────────────────┘   │   │
                           └───────────────────────────────────┘   │
```

---

### Surface 1: React Web UI

#### Bundling and Serving

The React app is compiled to a static bundle (`index.html` + JS/CSS assets) at
build time and embedded into the WasmCloud host container image under a well-known
path. The API Gateway actor serves these assets at `/` (or `/ui/`) alongside the
API routes at `/api/v1/` and `/auth/`. All traffic flows through a single TLS
endpoint on the same origin.

```
https://control.example.com/           → React app (index.html + assets)
https://control.example.com/api/v1/    → REST API (API Gateway actor)
https://control.example.com/auth/      → OAuth routes (API Gateway actor)
```

No additional environment variables are required for the UI URL — it is always
the same as `HOSTMGR_EXTERNAL_URL` (ADR-009).

#### OAuth Flow (Same-Origin PKCE)

Because the UI and API share an origin, the API Gateway sets an HttpOnly cookie
after OAuth and the browser sends it automatically on every subsequent request:

```
Browser (control.example.com)    API Gateway (control.example.com)    GitHub
        │                                   │                            │
        │  1. GET /                         │                            │
        │  React app loads; detects no      │                            │
        │  session cookie → redirect to     │                            │
        │  /auth/login?provider=github      │                            │
        │──────────────────────────────────►│                            │
        │                                   │                            │
        │  2. API Gateway generates:        │                            │
        │     code_verifier (random, stored │                            │
        │     in short-lived NATS KV entry) │                            │
        │     code_challenge = SHA256(cv)   │                            │
        │     state (CSRF token)            │                            │
        │◄──────────────────────────────────│                            │
        │  302 → github.com/login/oauth/    │                            │
        │    authorize?client_id=...        │                            │
        │    &code_challenge=...&state=...  │                            │
        │                                   │                            │
        │  3. User authorises on GitHub     │                            │
        │──────────────────────────────────────────────────────────────►│
        │◄──────────────────────────────────────────────────────────────│
        │  302 → /auth/callback?code=...&state=...                       │
        │                                   │                            │
        │  4. GET /auth/callback            │                            │
        │──────────────────────────────────►│                            │
        │  API Gateway validates state;     │                            │
        │  exchanges code + verifier for    │                            │
        │  access_token (GitHub API)        │                            │
        │  fetches user identity; checks    │                            │
        │  allowlist; issues JWT            │                            │
        │◄──────────────────────────────────│                            │
        │  302 → /  (React app root)        │                            │
        │  Set-Cookie: hostmgr_session=<JWT>│                            │
        │    HttpOnly; Secure; SameSite=Lax │                            │
        │    Max-Age=28800 (8 hours)        │                            │
        │                                   │                            │
        │  5. All subsequent API calls:     │                            │
        │  Cookie: hostmgr_session=<JWT>    │                            │
        │  (sent automatically by browser)  │                            │
        │──────────────────────────────────►│                            │
```

**Security properties:**
- Cookie is HttpOnly — not accessible to JavaScript; immune to XSS token theft
- Cookie is Secure — only sent over HTTPS
- Cookie is SameSite=Lax — blocks cross-site request forgery in the common case
- PKCE ensures the auth code cannot be exchanged by a third party even if intercepted
- No token stored in `localStorage` or `sessionStorage`
- No custom cross-origin token exchange mechanism

#### CORS Policy

CORS applies only to browser requests from origins other than the control plane.
Since the React UI is same-origin, CORS headers are not needed for UI API calls.
CORS is configured for API key clients that may call from other origins (e.g., a
custom automation dashboard):

```bash
# Optional: allow specific external origins to call the API with API keys
# Default: empty (same-origin only)
HOSTMGR_CORS_ALLOWED_ORIGINS=https://dashboard.mycompany.com
```

```
Access-Control-Allow-Origin: https://dashboard.mycompany.com
Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS
Access-Control-Allow-Headers: Authorization, Content-Type
Access-Control-Max-Age: 86400
```

`Access-Control-Allow-Credentials` is not set — API key clients use
`Authorization: Bearer` headers, not cookies. The UI needs no CORS header at all.

#### Technology Stack

| Layer | Choice | Rationale |
|---|---|---|
| Framework | React 19 | Widest contributor pool; mature ecosystem |
| Build tool | Vite | Fast HMR; optimal bundle splitting |
| Routing | TanStack Router | Type-safe; file-based routing |
| Data fetching | TanStack Query | Caching, background refresh, optimistic updates |
| UI components | shadcn/ui (Radix primitives) | Accessible; composable; no lock-in |
| Styling | Tailwind CSS | Utility-first; consistent with shadcn/ui |
| State | React Context + TanStack Query | No global store needed for MVP scope |
| Auth | HttpOnly cookie (set by API Gateway) | Same-origin; no JS token handling needed |

---

### Surface 2: CLI (`hostmgr`)

#### Distribution

Same pattern as `hostmgr-agent` (ADR-006):
- Statically-linked Rust binary, no runtime dependencies
- GitHub Releases: `hostmgr-linux-amd64`, `hostmgr-linux-arm64`, `hostmgr-darwin-amd64`,
  `hostmgr-darwin-arm64`, `hostmgr-windows-amd64.exe`
- SHA256 checksums in `checksums.txt` alongside each release
- Installation: `curl` download + checksum verify + move to `$PATH`

#### Configuration

```bash
# Control plane URL (required; set once)
hostmgr config set control-plane https://control.example.com

# Stored in ~/.config/hostmgr/config.toml
# Token stored in ~/.config/hostmgr/token (mode 0600)
```

#### Command Structure (MVP)

```
hostmgr
├── auth
│   ├── login [--provider github|google]  # Device Code OAuth flow
│   ├── logout                            # Clear stored token
│   └── status                            # Show current identity
│
├── config
│   ├── set <key> <value>                 # Set a config value
│   ├── get <key>                         # Get a config value
│   └── show                              # Show all config
│
├── endpoints
│   ├── list [--status <status>] [--tag <key>=<value>]
│   ├── get <id>
│   ├── tag <id> <key>=<value>
│   └── decommission <id>
│
├── discovery
│   ├── start [--subnet <cidr>]           # Trigger a discovery run
│   └── status                            # Show last discovery run result
│
├── identify
│   └── <id>...                           # Trigger identification for endpoint(s)
│
├── agents
│   ├── list [--status <status>]
│   ├── status <id>
│   └── versions                          # Fleet version overview (ADR-010)
│
├── keys                                  # API key management (Phase 1.1)
│   ├── create --name <label> [--read-only] [--expires <duration>]
│   ├── list
│   └── revoke <key-id>
│
└── health                                # Control plane health check
```

#### Output Formats

```bash
# Default: human-readable table
$ hostmgr endpoints list
ID              NAME             STATUS    VERSION   LAST SEEN
ep-7c9e6679     pi-livingroom    MANAGED   0.2.1     2m ago
ep-abc12345     server-01        MANAGED   0.3.0     30s ago
ep-def67890     old-laptop       OFFLINE   0.1.0     3d ago

# JSON: for scripting and piping
$ hostmgr endpoints list --output json | jq '.[] | select(.status == "OFFLINE")'

# YAML: for configuration-style output
$ hostmgr endpoints get ep-7c9e6679 --output yaml
```

#### Shell Completion

```bash
hostmgr completion bash  >> ~/.bashrc
hostmgr completion zsh   >> ~/.zshrc
hostmgr completion fish  >> ~/.config/fish/completions/hostmgr.fish
```

---

### Surface 3: API Keys for AI Agents and Automation

#### Key Format

```
hm_<scope_prefix>_<32-random-bytes-base62>

Examples:
hm_ro_4xK9mP2nQvR8sL1jW3bA7cF5dH6eG0  (read-only)
hm_rw_9zT3uN7oM4kJ2hY6xQ5wB8cV1pL0eR  (read-write — explicit operator choice)
```

The prefix makes the scope immediately visible in logs and config files.

#### Key Lifecycle

```
Operator: hostmgr keys create --name "claude-assistant" --read-only
  → API Gateway generates key + random salt
  → Stores SHA256(key + salt) in OpenBao at secret/config/api_keys/{key_id}
  → Returns the full key ONCE (not stored in plaintext anywhere)
  → Operator copies key to the AI agent / script configuration

AI agent: Authorization: Bearer hm_ro_4xK9mP2nQvR8sL1jW3bA7cF5dH6eG0
  → API Gateway receives key
  → Computes SHA256(key + stored_salt) for each key in the store
  → If match found: check scope, check expiry, allow/deny request

Operator: hostmgr keys revoke <key-id>
  → API Gateway deletes secret/config/api_keys/{key_id} from OpenBao
  → Key immediately invalid (no cache to flush)
```

#### Key Scopes (Phase 1.1)

| Scope | Prefix | Permitted Operations |
|---|---|---|
| `read-only` | `hm_ro_` | GET all endpoints; GET discovery results; GET agent status |
| `read-write` | `hm_rw_` | All read-only + POST discovery; POST identify; POST agent commands |

Default for new keys: `read-only`. A `read-write` key requires an explicit `--read-write`
flag and a confirmation prompt:

```bash
$ hostmgr keys create --name "automation" --read-write
⚠  Read-write API keys allow command execution on managed endpoints.
   Confirm? [y/N]: y
Created key: hm_rw_9zT3...  (shown once; store it securely)
```

#### API Key Authentication in the API Gateway

API key validation runs before OAuth JWT validation. The API Gateway checks the
`Authorization: Bearer` header for the `hm_` prefix:

```
Request: Authorization: Bearer hm_ro_4xK9mP2n...

API Gateway:
  1. Detect hm_ prefix → API key path
  2. Look up all key IDs in OpenBao (cached in memory, refreshed every 60s)
  3. For each key record: verify HMAC(incoming_key, stored_salt) == stored_hash
  4. If match: check scope against requested operation
  5. If scope permits: set request context { sub: "apikey:claude-assistant", scope: "read-only" }
  6. Forward to actor

No match: 401 Unauthorized
Scope mismatch: 403 Forbidden
```

The `sub` format `apikey:{name}` integrates with the audit log format defined in
ADR-007, so API key actions are distinguishable from human operator actions in logs.

---

### Surface 4: MCP Client Capability Provider

#### Role

A WasmCloud native capability provider that implements the MCP client protocol.
Actors that declare a dependency on this capability can call configured external
MCP servers during their workflows. The capability abstracts the MCP transport,
authentication, and tool invocation — actors call a simple interface:

```rust
// Actor pseudocode
let result = mcp_client
    .call_tool("netbox", "get_device_by_ip", json!({ "ip": "192.168.1.42" }))
    .await?;
```

The capability provider handles:
- MCP protocol transport (HTTP/SSE or stdio, depending on the MCP server)
- Authentication to each MCP server (bearer token, API key, or none)
- Tool discovery (caching the server's tool manifest)
- Timeout and retry behaviour
- Result schema validation

#### Configuration

MCP servers are configured via environment variables (ADR-009):

```bash
# List of configured MCP server names (comma-separated)
HOSTMGR_MCP_SERVERS=netbox,assets

# Per-server configuration
HOSTMGR_MCP_NETBOX_URL=https://netbox.internal/mcp
HOSTMGR_MCP_NETBOX_TOKEN_REF=secret/config/mcp/netbox_token
HOSTMGR_MCP_NETBOX_TIMEOUT=5000        # ms

HOSTMGR_MCP_ASSETS_URL=https://assets.internal/mcp
HOSTMGR_MCP_ASSETS_TOKEN_REF=secret/config/mcp/assets_token
HOSTMGR_MCP_ASSETS_TIMEOUT=3000
```

MCP calls are always **optional enrichment**. If an MCP server is unavailable or
times out, the actor logs a warning and continues with the data it has. A failed
MCP call never blocks endpoint discovery or identification.

#### Integration Points

The two actors that benefit most from MCP enrichment:

**Discovery Orchestrator** — after finding an IP/MAC, optionally queries:
- NetBox: documented device name, role, site, rack position
- Asset database: owner, purchase date, support contract status
- IPAM: subnet description, VLAN, DNS name

```json
// Enriched discovery result with MCP data
{
  "ip": "192.168.1.42",
  "mac": "b8:27:eb:12:34:56",
  "hostname": "pi-livingroom",
  "mcp_enrichment": {
    "netbox": {
      "device_name": "rpi-livingroom-01",
      "role": "media-server",
      "site": "home",
      "status": "active"
    },
    "assets": {
      "owner": "Joseph Pearson",
      "asset_tag": "HW-2024-0042"
    }
  }
}
```

**Identifier** — after SSH probing, optionally queries:
- NetBox: expected OS and role (validates identification result)
- Monitoring system: current metrics (confirms the device is reachable and healthy)
- CVE/patch database MCP server: known vulnerabilities for the identified OS version

#### What MCP Servers Host Manager Can Consume

The capability provider is server-agnostic — it works with any MCP-compliant server.
Likely candidates in the Host Manager ecosystem:

| MCP Server | Provides | ADR Reference |
|---|---|---|
| NetBox | Network inventory, IPAM, device roles | ADR-003 (Phase 3 integration) |
| Infragraph | Infrastructure graph relationships | ADR-003 (Phase 3 integration) |
| Asset management | Ownership, purchase history | New enrichment |
| Monitoring systems | Live metrics, alert status | New enrichment |
| CVE database | Known vulnerabilities by OS/version | New enrichment |
| Claude / AI assistant | Natural language endpoint classification | Future |

#### Phase 2: Expose as MCP Server

When Host Manager exposes itself as an MCP server, external AI assistants (Claude,
etc.) can call it as a tool:

```
Claude: "What endpoints are offline right now?"
  → MCP tool call: list_endpoints(status="OFFLINE")
  → Host Manager returns structured data
  → Claude formats a natural language response

Claude: "Identify all devices on the 10.0.1.0/24 subnet"
  → MCP tool call: trigger_discovery(subnet="10.0.1.0/24")
  → Host Manager queues the discovery run
  → Claude confirms the action was taken
```

Phase 2 MCP server design will require:
- Defining the tool manifest (which operations are exposed as MCP tools)
- Authentication (MCP clients authenticate via API key, mapped to read-only scope)
- Audit log integration (MCP-originating calls labelled as `mcp:{server_name}`)
- Rate limiting per MCP client to prevent runaway discovery triggers

---

## Impact on Existing ADRs

### ADR-007: API Authentication — Amendment

**API keys move from Phase 2 to Phase 1.1.**

Rationale: API keys are required for AI agent and MCP client access, which are Phase
1.1 features. The key design in this ADR supersedes the Phase 2 placeholder in ADR-007.

Specific changes to ADR-007:
- Phase 1.1 now includes: key generation, HMAC-based validation, scope enforcement,
  key revocation
- Phase 2 retains: RBAC role assignment, per-endpoint-group key scoping,
  organisation-level GitHub team membership checks

### ADR-001: Control Plane Architecture — Amendment

The MCP Client Capability Provider is added to the capability provider list:

| Provider | Function |
|---|---|
| Discovery | ARP/mDNS network scanning |
| Identification | SSH probes |
| Credentials | OpenBao integration |
| Agent Management | Deploy/manage agents |
| HTTP | REST API endpoint |
| **MCP Client** | **Call external MCP servers for enrichment** |

### ADR-009: Configuration & Packaging — Amendment

The React Web UI is bundled into the WasmCloud host image at build time. No separate
UI service is added to Docker Compose. The `wasmcloud` service already serves the UI
at `http://localhost:8080/` alongside the API at `http://localhost:8080/api/v1/`.

The CI pipeline gains a build step:

```
1. cd ui && npm ci && npm run build     # Vite build → dist/
2. COPY ui/dist /usr/share/hostmgr/ui   # into WasmCloud host image
3. API Gateway actor configured to serve /usr/share/hostmgr/ui at /
```

No new environment variables are required for the UI. `HOSTMGR_EXTERNAL_URL` is
the single URL for both API and UI access.

The CLI binary is added to the GitHub Releases matrix:

| Artifact | Platform |
|---|---|
| `hostmgr-linux-amd64` | Linux x86_64 |
| `hostmgr-linux-arm64` | Linux ARM64 |
| `hostmgr-darwin-amd64` | macOS Intel |
| `hostmgr-darwin-arm64` | macOS Apple Silicon |
| `hostmgr-windows-amd64.exe` | Windows x86_64 |

---

## Consequences

### Positive Impacts

**1. Simplest possible auth flow**
Same-origin serving enables standard HttpOnly cookies. The browser handles session
management automatically. No JWT-in-memory, no OTC exchange, no CORS configuration
for the UI, no custom refresh logic. This is the auth model browsers were designed for.

**2. AI agent access without human auth complexity**
API keys allow scripts, CI/CD pipelines, and AI assistants to interact with Host
Manager without OAuth flows. Read-only default scope limits blast radius if a key
is exposed.

**3. MCP enrichment is non-blocking and additive**
MCP calls are best-effort. No existing workflow changes; enriched data is additional
context on top of what discovery and identification already produce. Operators without
MCP servers configured experience no difference.

**4. Consistent CLI patterns**
Single statically-linked binary with consistent subcommand structure, table and JSON
output, and shell completion covers the common operator workflows without requiring
any runtime dependencies.

**5. Extensible MCP surface**
The capability provider abstraction means any MCP-compliant server can be integrated
without changes to actor code. As the MCP ecosystem grows, new enrichment sources
are a configuration addition, not a code change.

### Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Session cookie stolen via XSS | HttpOnly prevents JS access to the cookie entirely; Content Security Policy headers restrict script sources |
| CSRF attack using session cookie | SameSite=Lax blocks cross-site form submissions and navigations; state parameter in OAuth flow provides additional CSRF protection |
| API key leaked in logs or config files | `hm_ro_` / `hm_rw_` prefix makes keys easily detectable in log scraping and secret scanning CI tools (e.g., truffleHog, gitleaks) |
| API key compromised grants full access | Default is read-only; read-write requires explicit opt-in with confirmation; keys can be revoked instantly via CLI |
| MCP server returns malicious data used in commands | MCP enrichment data is stored as metadata only; it never flows into command execution paths; actors treat it as untrusted input |
| React bundle injected with malicious JS | Content Security Policy headers served with UI assets; Subresource Integrity (SRI) for external scripts; no CDN for critical auth code |
| CLI token exfiltrated from developer machine | Token stored at `~/.config/hostmgr/token` mode 0600; `hostmgr auth logout` wipes it; JWT TTL is 8 hours |

### Implementation Considerations

- The React app does not manage authentication tokens at all — the browser handles
  the session cookie automatically. The React app simply makes `fetch()` calls to
  `/api/v1/...` and relies on the browser to attach the cookie.
- Session expiry is detected by the API returning `401 Unauthorized`. The React app
  intercepts this response and redirects to `/auth/login` to re-authenticate.
- The WasmCloud host image build must run `npm ci && npm run build` before the
  Docker image build step. This requires Node.js in the CI environment but not in
  the production runtime image (the built static assets are copied in).
- In local development, Vite's dev server (`npm run dev`) proxies API calls to the
  WasmCloud host running on a different port. This avoids needing a built image for
  UI development. The Vite config's `server.proxy` setting routes `/api` and `/auth`
  to `http://localhost:8080`.
- The MCP capability provider must enforce a timeout on every MCP call. The default
  (configurable) is 5 seconds. Actors must not await an MCP call without a timeout.
- The CLI must redact the API key from its own log output. The key is visible to the
  operator exactly once, at creation time. After that, only the key ID is shown.
- Windows support for the CLI is included in the binary matrix but is best-effort for
  MVP. The primary development and test environment is Linux/macOS.

---

## Alternatives Considered

### Alternative 1: Separate Web UI Deployment

**Decision:** Rejected for MVP; remains viable for Phase 2 if release cadences diverge

**Rationale:** A separately deployed React app (Nginx container, CDN, GitHub Pages)
decouples UI and control plane release cycles. However, it forces a cross-origin
OAuth solution. The standard HttpOnly cookie approach does not work across origins,
requiring either a one-time code exchange pattern or JWT storage in JavaScript memory
— both of which add implementation complexity and reduce security posture compared to
HttpOnly cookies. For MVP, bundled deployment is simpler to build, simpler to operate,
and more secure. If UI and control plane release cadences diverge significantly in
Phase 2, the UI can be extracted without changing the API or auth model.

### Alternative 2: Machine-to-Machine OAuth (Client Credentials Flow) Instead of API Keys

**Decision:** Rejected for Phase 1.1; may be added in Phase 3

**Rationale:** Client credentials flow requires registering each automation tool as
an OAuth application (client ID + secret), an OAuth server that supports it (GitHub
does not; Google does), and a token endpoint. API keys achieve the same goal with
significantly less infrastructure for the single-operator MVP context.

### Alternative 3: GraphQL API Instead of REST

**Decision:** Rejected

**Rationale:** GraphQL has advantages for flexible querying (useful for a UI with
varied data requirements) but adds schema definition, resolver implementation, and
client tooling complexity. For MVP, the REST API surface is small enough that
over-fetching is not a concern. GraphQL can be added as an alternative API layer
in Phase 3 if the UI data requirements warrant it.

### Alternative 4: Expose as MCP Server First (Not Consume)

**Decision:** Rejected; deferred to Phase 2

**Rationale:** Exposing as an MCP server immediately enables AI assistant interactions
but requires careful definition of write-capable tool semantics, audit logging for
AI-originating commands, and rate limiting. Consuming MCP servers is lower-risk and
delivers immediate value (data enrichment) without introducing AI agents as command
issuers. Phase 2 will design the expose-as-server surface with the lessons learned
from the consume implementation.

### Alternative 5: Svelte / SvelteKit Instead of React

**Decision:** Rejected in favour of React

**Rationale:** SvelteKit produces smaller bundles and has a simpler component model,
making it attractive for a lean stack. React was chosen for maximum contributor
familiarity and ecosystem depth (component libraries, accessibility tooling, testing
utilities). Either would be technically sound; React reduces the barrier for
open-source contributions.

---

## Future Phases

### Phase 1.1

- API keys (read-only and read-write scopes)
- MCP Client capability provider (NetBox, asset DB integration)
- CLI complete command set (all MVP commands above)
- React Web UI: endpoint list, status, discovery trigger, agent status
- One-time code OAuth flow for separate web UI

### Phase 2

- Expose Host Manager as an MCP server (read-only tool manifest)
- CLI: `hostmgr mcp` subcommand for testing MCP server interactions
- Web UI: command execution, log streaming, update approval
- API key scoping to specific endpoint groups
- GitHub organisation membership as allowlist group

### Phase 3

- GraphQL API layer for complex UI queries
- MCP server write tools (with confirmation and audit trail)
- Natural language endpoint management via AI assistant integration
- MCP server for Claude: query fleet state, trigger discovery via conversation
- Open Horizon service definition for Web UI (edge-deployed management console)

---

## Related Decisions

- **ADR-001:** WasmCloud Control Plane — API Gateway actor is extended with API key
  validation and CORS enforcement; MCP Client capability provider added
- **ADR-007:** API Authentication — amended: API keys moved to Phase 1.1; one-time
  code OAuth pattern added for cross-origin web UI
- **ADR-008:** Endpoint State Data Model — MCP enrichment data stored in `network`
  and `identity` sub-paths as optional fields
- **ADR-009:** Configuration & Packaging — web UI added as optional Docker Compose
  service; CLI binary added to GitHub Releases matrix; MCP server env vars defined
- **ADR-010:** Agent Lifecycle Management — CLI `hostmgr agents versions` and
  `hostmgr agents update` commands surface the update workflow

---

## Open Questions

1. **React component library pinning:** shadcn/ui copies components into the repo
   (not a traditional npm dependency). This means updates are manual. Is this the
   right trade-off for an open-source project, or should a traditional component
   library (e.g., Mantine, Chakra) with npm versioning be used instead?

2. **CLI version pinning:** Should the CLI enforce a minimum API version and refuse
   to connect to a control plane that is too old? Semver compatibility checks between
   CLI and API are standard practice but add versioning discipline overhead. Recommend
   yes; implement in Phase 1.1.

3. **MCP server authentication variety:** Some MCP servers may use mTLS rather than
   bearer tokens. The capability provider should support multiple auth schemes. Design
   the auth configuration to be extensible from Phase 1.1.

4. **Web UI offline behaviour:** When the control plane is unreachable, should the
   React app show cached data with a stale indicator, or a hard error? TanStack Query's
   stale-while-revalidate pattern supports the former with minimal effort.

---

## Decision Record

**Approved by:** [To be filled after review]

**Date approved:** [To be filled]

**Concerns / feedback addressed:** [To be filled after discussion]
