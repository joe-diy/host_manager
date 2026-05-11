# ADR-010: Agent Lifecycle Management

**Status:** Proposed

**Date:** 2026-05-10

**Authors:** Joseph Pearson

---

## Context

Once `hostmgr-agent` is deployed to an endpoint (ADR-006) and communicating with the
control plane (ADR-005), it must be managed over its operational lifetime:

- **Updates:** New versions of the agent binary are released. How and when does the
  running agent update itself?
- **Rollback:** An update may introduce a regression. How does the system recover?
- **Health monitoring:** Is the running agent in a good state? How does the control
  plane detect and respond to degraded agents?
- **Uninstall:** When an endpoint is decommissioned, how is the agent cleanly removed?

These questions have meaningful architecture and security implications. An update
mechanism that fetches and executes arbitrary code must be cryptographically verified.
A rollback mechanism that relies on the agent being healthy to initiate rollback has
a fundamental limitation: a fatally broken agent cannot roll itself back.

### Constraints

- **MVP scope:** Agent update is out of scope for MVP. The operator manually updates
  agents when needed. The infrastructure designed here supports Phase 1.1+ automation.
- **Service mode diversity:** Agents run as systemd services, Docker/Podman containers,
  or in foreground mode (ADR-006). The lifecycle mechanism must work across all modes.
- **No relay through control plane for binary delivery:** Binary updates are pulled
  from GitHub Releases (ADR-006), not pushed through the NATS message bus. This
  avoids making the control plane a binary distribution point.
- **Security:** The update binary must be verified before execution. A compromised
  update delivery path must not result in arbitrary code execution on managed endpoints.
- **Operator control:** Automatic updates without operator awareness are a risk in
  production environments. The operator must be able to choose the update policy:
  fully automatic, requiring approval, or disabled.
- **Resilience:** A failed update must not leave the agent in an unrecoverable state.
  The previous binary must be preserved until the new binary is confirmed healthy.

---

## Decision

**Agent update is not implemented in MVP. Operators update agents manually by
re-running the installation process on each endpoint.**

**From Phase 1.1, agents support pull-based self-update with a configurable trigger
policy (automatic or manual approval) and an optional automatic rollback mechanism
with a configurable retry threshold.**

The update flow:
1. Agent periodically polls GitHub Releases for a new version
2. If a newer version is found, behaviour depends on the configured trigger:
   - `automatic`: agent downloads, verifies, and installs the update immediately
   - `manual`: agent notifies the control plane; operator approves via UI/CLI
3. Update is verified (SHA256 checksum; Cosign signature in Phase 2)
4. Previous binary is preserved as a rollback target
5. Agent restarts under the new binary
6. If the agent fails to reconnect within the rollback window, it automatically
   reverts to the previous binary

---

## Rationale

### Why No Auto-update in MVP

Implementing agent self-update correctly is non-trivial. The update mechanism must:
handle partial downloads, verify binary integrity, replace a running process safely,
handle restart failures, and preserve a rollback path. Building this for MVP adds
scope without corresponding user value — MVP operators are technical and comfortable
with manual updates. Getting the core management capabilities right (discovery,
identification, communication) is higher priority.

Designing the update infrastructure here (in this ADR) means Phase 1.1 implementation
has a clear specification to follow without reopening architectural decisions.

### Why Pull-based (Agent Polls GitHub Releases)

Push-based update (control plane sends a command: "update yourself") has a fundamental
problem: if the agent is offline or the command is lost, the update doesn't happen.
The operator has no reliable way to ensure all agents are updated.

Pull-based update (agent checks for a new version on a schedule) is resilient to
connectivity gaps. When an agent reconnects after being offline, it checks for updates
at its next scheduled check and catches up without any manual intervention.

GitHub Releases is the distribution source (ADR-006). The agent already knows the
project's release URL. No additional update server infrastructure is required.

The control plane still participates in the pull process: the agent reports its
installed version in heartbeats, the control plane tracks which endpoints are running
outdated versions, and the operator can trigger an immediate update check via the
API/CLI rather than waiting for the next scheduled check.

### Why Configurable Trigger (Automatic vs. Manual)

Different operators have different risk tolerances:

- **Home lab / development:** Automatic updates are fine. Always run the latest version.
- **Production / regulated environment:** No binary should run on a managed endpoint
  without explicit operator approval. Manual trigger with operator sign-off is required.

The `HOSTMGR_UPDATE_TRIGGER` environment variable (ADR-009) controls this per-agent.
The default for MVP is `manual` (conservative); operators who want automatic updates
opt in explicitly.

### Why Optional Automatic Rollback

Rollback is the safety net for a bad update. The logic is:

- After updating, the agent tries to reconnect to the control plane
- If it fails to reconnect within N attempts (configurable), it assumes the new
  version is broken and reverts to the previous binary
- The number of reconnection attempts before rollback is configurable because
  environments vary: a high-latency link might need more retries before a healthy
  agent is declared failed

Rollback is optional because not all operators want it. In some environments,
a failed update should be left in place for diagnosis rather than automatically
reverted. The default is rollback enabled with 3 retries.

---

## Architecture

### Version Tracking

The agent reports its version in every heartbeat message (ADR-005):

```json
{
  "endpoint_id": "ep-7c9e6679...",
  "reported_at": "2026-05-10T12:34:30Z",
  "agent_version": "0.2.1",
  ...
}
```

The Agent Coordinator actor stores the version in the endpoint's `agent` sub-path
in OpenBao (ADR-008: `agent.version`). The API Gateway exposes a control-plane-wide
view of agent versions:

```
GET /api/v1/agents/versions

{
  "current_release": "0.3.0",
  "endpoints": [
    { "id": "ep-7c9e6679...", "name": "pi-livingroom", "version": "0.2.1", "status": "outdated" },
    { "id": "ep-abc12345...", "name": "server-01",     "version": "0.3.0", "status": "current" }
  ]
}
```

### Update Configuration

Agent update behaviour is controlled by environment variables set during installation
(stored in `/etc/hostmgr/agent.env`, ADR-006):

```bash
# Check interval for new releases (default: 24 hours)
HOSTMGR_UPDATE_CHECK_INTERVAL=86400

# "automatic" or "manual" (default: manual)
HOSTMGR_UPDATE_TRIGGER=manual

# Enable automatic rollback (default: true)
HOSTMGR_UPDATE_ROLLBACK_ENABLED=true

# Reconnection attempts before rollback (default: 3)
HOSTMGR_UPDATE_ROLLBACK_RETRIES=3

# Reconnection interval during rollback assessment (seconds, default: 30)
HOSTMGR_UPDATE_ROLLBACK_INTERVAL=30

# Update channel: "stable", "beta", or "none" (default: stable)
HOSTMGR_UPDATE_CHANNEL=stable

# Minimum version to consider an update (prevents downgrade)
HOSTMGR_UPDATE_MINIMUM_VERSION=0.1.0
```

### Update State Machine

```
┌─────────────┐
│    IDLE     │  ← Normal operating state
└──────┬──────┘
       │ Check interval elapsed OR operator triggers check
       ▼
┌─────────────────┐
│ CHECKING_UPDATE │  Polls GitHub Releases API for latest version
└──────┬──────────┘
       │
       ├─ No new version available ──────────────────────► IDLE
       │
       │  New version available
       ├─ trigger=manual ──────────────────────────────► AWAITING_APPROVAL
       │                                                  (notifies control plane;
       │                                                   waits for operator)
       │  trigger=automatic  OR  operator approves
       ▼
┌──────────────────┐
│   DOWNLOADING    │  Downloads binary for platform/arch
└──────┬───────────┘  from GitHub Releases
       │
       │  Download failed ────────────────────────────── retry (3x) → IDLE
       │
       ▼
┌──────────────────┐
│   VERIFYING      │  SHA256 checksum verified against release manifest
└──────┬───────────┘  Cosign signature (Phase 2)
       │
       │  Verification failed ────────────────────────── alert + IDLE (do not install)
       │
       ▼
┌──────────────────┐
│   STAGING        │  Binary written to staging path:
└──────┬───────────┘  /var/lib/hostmgr/agent.new
       │  Current binary copied to rollback path:
       │  /var/lib/hostmgr/agent.prev
       ▼
┌──────────────────┐
│   RESTARTING     │  Service manager instructed to restart:
└──────┬───────────┘  systemd: binary replaced; systemctl restart hostmgr-agent
       │              Docker:  container image updated; docker compose up -d
       │              Podman:  same as Docker
       ▼
┌──────────────────────┐
│ RECONNECTION_CHECK   │  New binary running; agent tries to reconnect
└──────┬───────────────┘  to control plane
       │
       ├─ Reconnects successfully within retry window ─► IDLE
       │   (agent.prev deleted; update reported to control plane)
       │
       │  Fails to reconnect within ROLLBACK_RETRIES × ROLLBACK_INTERVAL
       ▼
┌──────────────────┐
│   ROLLING_BACK   │  agent.new discarded; agent.prev restored as active binary
└──────┬───────────┘  Service restarted with previous binary
       │
       ▼
┌─────────────────────────────┐
│ ROLLBACK_COMPLETE           │  Alert sent to control plane
│ (running previous version)  │  Operator must investigate before next update
└─────────────────────────────┘
```

### Update Flow by Service Mode

#### systemd

```bash
# Staging phase
cp /usr/local/bin/hostmgr-agent /var/lib/hostmgr/agent.prev
cp /var/lib/hostmgr/agent.new  /usr/local/bin/hostmgr-agent
chmod 755 /usr/local/bin/hostmgr-agent

# Restart phase
systemctl restart hostmgr-agent

# Rollback (if reconnection check fails)
cp /var/lib/hostmgr/agent.prev /usr/local/bin/hostmgr-agent
systemctl restart hostmgr-agent
```

The agent has sufficient privilege to restart its own systemd unit. The service
unit grants `ExecReload` permissions to the `hostmgr` user. The agent never
requires root for the update process.

#### Docker / Podman

```bash
# The agent cannot replace its own container image from inside the container.
# Instead, the agent writes a new compose file referencing the new image tag,
# then signals the host-level update helper (a small privileged sidecar).

# The update helper is a minimal process running on the host (not in a container):
# /usr/local/bin/hostmgr-update-helper
# - listens on a Unix socket: /run/hostmgr/update.sock
# - accepts: { "action": "update", "image": "ghcr.io/host-manager/hostmgr-agent:0.3.0" }
# - runs: docker compose -f /etc/hostmgr/docker-compose.yml up -d --pull always
# - accepts: { "action": "rollback" }
# - runs: restore previous image tag in compose file; docker compose up -d
```

The update helper runs as a systemd service with Docker access. It is the only
component that requires elevated privileges for updates in container mode. The
agent communicates with it via a Unix domain socket with strict permissions.

This approach keeps the container itself minimal and read-only while allowing
update operations to be performed by a controlled, auditable helper process.

### Manual Approval Flow (trigger=manual)

When `HOSTMGR_UPDATE_TRIGGER=manual`:

1. Agent detects new version; sends `agent.{id}.lifecycle.update_available` message
   to the control plane
2. Agent Coordinator records the pending update in the endpoint's `agent.pending_update`
   field in OpenBao
3. Operator sees a notification in the UI or via `hostmgr agents list`

```bash
$ hostmgr agents list
ENDPOINT        VERSION   LATEST   STATUS
pi-livingroom   0.2.1     0.3.0    update-available
server-01       0.3.0     0.3.0    current

$ hostmgr agents update ep-7c9e6679   # approve specific endpoint
$ hostmgr agents update --all          # approve all pending updates
```

4. Agent Coordinator publishes `agent.{id}.cmd.update` with the target version
5. Agent proceeds through the DOWNLOADING → ... → IDLE flow

### Binary Integrity Verification

#### MVP: SHA256 Checksum

Each GitHub Release includes a `checksums.txt` file:

```
e3b0c44298fc1c149afb...  hostmgr-agent-linux-amd64
a87ff679a2f3e71d9181...  hostmgr-agent-linux-arm64
...
```

The agent downloads `checksums.txt` first (from a separate HTTPS request), verifies
its content against a pinned public key (Phase 2: Cosign signature on the checksum
file itself), then verifies the binary against the checksum.

#### Phase 2: Cosign Signature Verification

Each release binary is signed with the Host Manager release key via Sigstore/Cosign.
The agent verifies the signature against the pinned public key before staging the
binary. A binary that passes checksum but fails signature verification is treated
as a tampered artifact and rejected.

### Health Monitoring

The agent performs its own health self-assessment and reports in heartbeats:

```json
{
  "endpoint_id": "ep-7c9e6679...",
  "agent_version": "0.2.1",
  "health": {
    "status": "healthy",
    "checks": {
      "control_plane_reachable": true,
      "nats_connected": true,
      "openbao_reachable": true,
      "last_command_success": true,
      "disk_space_ok": true
    }
  },
  "metrics": { ... }
}
```

The Endpoint Manager actor evaluates health status:

| Condition | Duration | Action |
|---|---|---|
| No heartbeat | > 90s | Transition endpoint to `OFFLINE` |
| `health.status = degraded` | Any heartbeat | Transition endpoint to `DEGRADED` |
| Heartbeat resumes | — | Transition endpoint back to `MANAGED` |
| `health.status = healthy` after degraded | — | Transition back to `MANAGED` |
| Agent reports update rollback | — | Endpoint stays `MANAGED`; alert sent to operator |

### Uninstall

The installer script (ADR-006) supports an uninstall mode:

```bash
# Graceful uninstall
sudo ./install.sh --uninstall

# Uninstall steps:
# 1. Agent sends agent.{id}.lifecycle.disconnected message to control plane
# 2. Agent Coordinator marks endpoint as no longer having an agent
#    (status → IDENTIFIED; agent sub-path cleared in OpenBao)
# 3. Systemd unit stopped and disabled (or Docker container stopped and removed)
# 4. Binary removed from /usr/local/bin/
# 5. /etc/hostmgr/ and /var/lib/hostmgr/ removed
# 6. hostmgr system user removed
# 7. NKey credential revoked at control plane (operator confirms)
```

The operator must separately revoke the endpoint's NKey credential from the NATS
server configuration and delete or archive the endpoint record in OpenBao (ADR-008).
The uninstall does not automatically purge the control plane record — that is an
explicit decommission operation.

---

## Consequences

### Positive Impacts

**1. Conservative default policy**
`trigger=manual` as the default ensures operators are always aware of what is
running on their endpoints. Automatic updates are opt-in, not opt-out.

**2. Rollback without control plane dependency**
The rollback mechanism is entirely self-contained in the agent. A broken agent
that cannot reach the control plane can still roll itself back using the locally
preserved `agent.prev` binary. This is crucial — the scenarios where rollback
is needed are precisely the scenarios where the new binary may be unable to connect.

**3. No binary distribution infrastructure**
GitHub Releases handles binary hosting, CDN, and availability. The control plane
does not need to store or serve binaries. This avoids a significant operational
burden and potential single point of failure.

**4. Operator visibility into fleet version state**
The `GET /api/v1/agents/versions` endpoint gives a unified view of which endpoints
are current and which need updates. The operator never has to SSH into individual
endpoints to check versions.

**5. Service-mode-agnostic design**
The same update configuration works for systemd, Docker, and Podman. The update
helper sidecar cleanly separates the privileged Docker operations from the
unprivileged agent process.

### Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Rollback window too short for high-latency networks | `HOSTMGR_UPDATE_ROLLBACK_RETRIES` and `HOSTMGR_UPDATE_ROLLBACK_INTERVAL` are configurable; documented guidance for typical network latency ranges |
| GitHub Releases unavailable during update check | Update check failure is logged and retried at next interval; no disruption to running agent |
| Binary verification bypassed by network-level attack | Checksum file and binary downloaded in separate HTTPS sessions; both verified; Cosign signature in Phase 2 adds supply-chain protection |
| Update helper sidecar privilege escalation | Update helper has minimal surface area (single Unix socket, two commands); runs as dedicated user with only Docker group membership; no network exposure |
| Rollback loop (new version always rolls back) | After rollback, agent pins to previous version and does not auto-retry the failed version; operator must explicitly approve retry after investigation |
| Docker image update in air-gapped environment | `HOSTMGR_UPDATE_CHANNEL=none` disables updates; operators manage image distribution manually |

### Implementation Considerations

- The `agent.prev` binary and the `agent.new` download must be on the same filesystem
  as the active binary to ensure atomic rename (`rename(2)`) is possible. The
  installer ensures all three paths are on the same volume.
- The update check must respect GitHub API rate limits (60 unauthenticated requests
  per hour per IP). With `HOSTMGR_UPDATE_CHECK_INTERVAL=86400` (24h), a single
  agent makes approximately 0.00001 requests per second — well within limits.
  Large fleets sharing an IP (via NAT) may need a longer interval or an authenticated
  GitHub token.
- The agent must not initiate an update while a command is in flight. The update
  state machine waits for all in-flight commands to complete or time out before
  proceeding to STAGING.
- For Docker mode, the update helper Unix socket must be mounted into the agent
  container: `- /run/hostmgr/update.sock:/run/hostmgr/update.sock`. The socket
  is created by the helper at startup and has mode `0660`, owned by `hostmgr:docker`.

---

## Alternatives Considered

### Alternative 1: Push-based Update (Control Plane Initiates)

**Decision:** Rejected as primary; push-triggered check is supported

**Rationale:** Pure push-based update requires the agent to be online and connected
when the operator initiates the update. If an agent is temporarily offline, it misses
the update command. JetStream durability (ADR-005) mitigates this, but commands have
a 24-hour expiry — agents offline longer than that would miss the update entirely.
Pull-based update catches up independently of connection gaps.

The operator can still trigger an immediate update check via
`hostmgr agents check-update ep-{id}`, which sends an `agent.{id}.cmd.check_update`
message via NATS. The agent polls when it reconnects and receives this command.
This gives the operator the UX of a push with the resilience of a pull.

### Alternative 2: Automatic Update Only (No Manual Trigger)

**Decision:** Rejected

**Rationale:** Automatic-only update is unacceptable for production environments where
change control processes require operator approval before any software update. A system
that updates production endpoints without approval would be rejected by the operators
who manage regulated infrastructure. The configurable trigger satisfies both preferences.

### Alternative 3: Control Plane Distributes Binaries

**Decision:** Rejected

**Rationale:** Serving binary files from the control plane requires significant storage,
CDN/caching consideration, and high-bandwidth capacity. For a fleet of 100 agents each
downloading a 30MB binary simultaneously, that is 3GB of outbound traffic from the
control plane. GitHub Releases handles this with GitHub's global CDN. The control plane
is not a content delivery network.

### Alternative 4: Package Manager Updates (.deb / .rpm)

**Decision:** Deferred to Phase 2

**Rationale:** Package manager updates (`apt upgrade`, `yum update`) are operationally
clean on systems that use them. They integrate with existing patch management tooling.
However, they require a hosted package repository (ADR-006, Phase 2) and only work
on systemd mode (Docker mode agents cannot use system package managers). When the
package repository exists (Phase 2), package manager updates should be documented as
an alternative to the GitHub Releases pull mechanism.

### Alternative 5: No Rollback

**Decision:** Rejected

**Rationale:** Without rollback, a broken update that prevents agent reconnection
requires manual operator intervention on the endpoint (SSH in, restore the binary
manually). For a fleet of many endpoints, a bad release could require manual recovery
on every affected machine simultaneously. Automatic rollback is a safety net that
keeps the operator in control without requiring emergency manual access to endpoints.

---

## Future Phases

### Phase 1.1: Update Implementation

- GitHub Releases polling and version comparison
- SHA256 checksum verification
- systemd and Docker/Podman restart mechanisms
- Update helper sidecar for Docker mode
- Manual approval flow (NATS message + CLI command)
- Automatic trigger mode
- Rollback mechanism with configurable retry threshold
- `GET /api/v1/agents/versions` fleet view

### Phase 2: Hardening

- Cosign signature verification on all update binaries
- Package repository (`.deb` / `.rpm`) as alternative update path
- Scheduled maintenance windows (update only between 02:00–04:00 local time)
- Per-endpoint update policy override (different policy from fleet default)
- Update dry-run mode (verify binary without installing)

### Phase 3: Advanced Lifecycle

- Canary updates: update a percentage of the fleet first; observe error rate; proceed or halt
- Blue/green agent: run new and old version side-by-side; cut over after validation
- Open Horizon workload lifecycle: if the agent runs as an OH service (ADR-006 Phase 3),
  OH's agreement bot manages update deployment across the fleet

---

## Related Decisions

- **ADR-005:** Agent Communication Protocol — heartbeat carries `agent_version`; update
  commands delivered via NATS; control plane notified of update completion/rollback
- **ADR-006:** Agent Deployment Strategy — installation creates the filesystem layout
  (`agent.prev`, `agent.new` paths) that the update mechanism depends on; uninstall
  procedure extends the install script
- **ADR-008:** Endpoint State Data Model — `agent.version`, `agent.update_channel`, and
  `agent.pending_update` fields in OpenBao track update state per endpoint
- **ADR-009:** Configuration & Packaging — update behaviour configured via environment
  variables (`HOSTMGR_UPDATE_TRIGGER`, `HOSTMGR_UPDATE_ROLLBACK_*`)

---

## Open Questions

1. **Update helper sidecar distribution:** The update helper for Docker mode must
   be installed alongside the agent. Does it ship in the same GitHub Release artifact,
   or as a separate download? Same release, separate binary
   (`hostmgr-update-helper-linux-amd64`) is simplest.

2. **Maintenance window support:** Should Phase 1.1 include basic maintenance window
   configuration (e.g., only update between certain hours), or defer this to Phase 2?
   Given the conservative `manual` default, defer to Phase 2.

3. **Fleet update ordering:** When the operator approves updates for all endpoints,
   should the Agent Coordinator stagger updates (e.g., 10 at a time) to avoid all
   agents restarting simultaneously? Likely yes for large fleets; design in Phase 2.

4. **Minimum version enforcement:** Should the control plane refuse to communicate with
   agents below a minimum supported version? This prevents legacy agents from
   accumulating silently. Implement as a warning in Phase 1.1; hard refusal in Phase 2.

---

## Decision Record

**Approved by:** [To be filled after review]

**Date approved:** [To be filled]

**Concerns / feedback addressed:** [To be filled after discussion]
