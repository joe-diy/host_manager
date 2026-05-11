# ADR-006: Agent Deployment Strategy

**Status:** Proposed

**Date:** 2026-05-10

**Authors:** Joseph Pearson

---

## Context

Once an endpoint has been discovered (ADR-003) and identified (ADR-004), the next step in
the Host Manager lifecycle is deploying the `hostmgr-agent` binary to that endpoint so it
can be managed via the agent communication protocol (ADR-005).

The deployment strategy must answer three questions:

1. **How does the agent binary reach the endpoint?** (distribution mechanism)
2. **How does the agent run persistently?** (service model)
3. **How does the agent establish its identity with the control plane?** (bootstrap)

### Constraints

- **MVP scope:** Host Manager is in early development. Fully automated, zero-touch
  deployment is a Phase 2+ concern. The MVP must be reliable and well-documented;
  it does not need to be hands-free.
- **Target hardware diversity:** Endpoints include Linux servers, Raspberry Pis, older
  virtual machines, and WSL 2 instances. Not all support systemd. Not all have container
  runtimes. The strategy must work across this range.
- **Security baseline:** Even in manual MVP mode, the installation process must not
  create credentials or open ports that outlast the installation itself. Secrets
  must not appear in shell history, log files, or process arguments.
- **No persistent SSH from control plane:** Host Manager does not maintain a standing
  SSH connection to endpoints. SSH is used only during identification (ADR-004) and,
  where applicable, during agent installation — not for ongoing management.

### Future Context: FIDO Device Onboard 2.0

The FIDO Device Onboard (FDO) standard (FIDO Alliance / Linux Foundation) defines a
protocol for zero-touch, cryptographically verified device onboarding. An FDO-capable
device ships with a credential (the Ownership Voucher) that can be transferred to an
owner service. The owner service then delivers software and configuration to the device
automatically, without any human steps at the device.

Open Horizon has implemented FDO support, making it a natural path for Host Manager's
Phase 2+ deployment strategy: devices are onboarded via FDO, Open Horizon deploys
`hostmgr-agent` as a workload, and the agent registers with the Host Manager control
plane autonomously.

This ADR designs the MVP manual installation path in a way that is explicitly compatible
with FDO/Open Horizon automation in later phases — the same bootstrap token mechanism,
the same binary, the same service configuration.

---

## Decision

**For MVP, `hostmgr-agent` is installed manually by an operator following documented
installation instructions. The agent binary is downloaded from GitHub Releases. The agent
runs as a systemd service on systemd-capable hosts, or as a Docker/Podman container on
hosts without systemd (including WSL 2). Identity with the control plane is established
via a bootstrap token (ADR-005).**

Future phases will introduce automated deployment via FIDO Device Onboard 2.0, optionally
integrated with Open Horizon as the workload delivery mechanism.

---

## Rationale

### Why Manual Installation for MVP

Automated agent deployment requires the control plane to have SSH access to endpoints
using stored credentials (ADR-002), the ability to transfer binaries, and the ability
to configure and start services — all with error handling across diverse OS environments.
This is substantial scope for Phase 1. Manual installation with clear documentation
delivers the same end state with a fraction of the implementation risk.

Manual installation also forces the documentation to be comprehensive and operator-
facing, which is essential for an open-source project where operators will need to
understand what the agent does and how to verify it.

### Why GitHub Releases

GitHub Releases is the standard distribution mechanism for open-source CLI tools and
agents (Prometheus exporters, Grafana Agent, Vector, etc.). Operators know how to use
it. It provides:

- Per-release binary artifacts for multiple platforms/architectures
- SHA256 checksum files alongside each binary
- Optional Cosign signatures for supply chain verification (Phase 2)
- A stable, versioned URL scheme suitable for scripting
- No infrastructure to maintain on the Host Manager side

### Why Both systemd and Docker/Podman

systemd is the standard init system on modern Linux distributions (Ubuntu, Debian,
RHEL, Raspberry Pi OS). It provides automatic restart on failure, structured logging
via journald, and dependency ordering. It is the correct default for bare-metal and VM
deployments.

However, systemd is not universally available:

- **WSL 2:** Systemd is opt-in (requires `[boot] systemd=true` in `.wssconfig`) and
  is not enabled by default in many WSL 2 installations
- **Minimal container base images:** Some lightweight Linux environments omit systemd
- **User-space installs:** Operators without root access cannot install systemd units
- **Preference:** Some operators strongly prefer containerised workloads for isolation
  and uniformity

Docker/Podman containers provide a `--restart=unless-stopped` policy that approximates
systemd's restart behaviour. Rootless Podman works without root access. This covers all
the cases where systemd is unavailable or undesirable.

### Why Bootstrap Token for Identity (Not Pre-shared Key)

A bootstrap token (short-lived, single-use, scoped) is the correct mechanism for first-
time agent identity establishment (designed in ADR-005). It avoids:

- Embedding long-lived credentials in installation scripts (which appear in shell history)
- Requiring the operator to generate and manage NKeys manually
- Creating credentials that persist and are reusable if the installation fails partway
  through

The operator copies the bootstrap token from the control plane UI or CLI, passes it
to the install command (via environment variable, never as a CLI argument), and the
agent exchanges it for its permanent NKey credential automatically.

---

## Architecture

### Installation Flow (MVP)

```
Operator                  Control Plane              GitHub Releases
   │                           │                           │
   │  1. Generate bootstrap     │                           │
   │─────────────────────────►│                           │
   │◄─────────────────────────│                           │
   │  token (5min TTL,         │                           │
   │  single-use)              │                           │
   │                           │                           │
   │  2. SSH to endpoint (using stored credentials or manual)
   │──────────────────────────────────────────────────────►(endpoint)
   │                                                        │
   │  3. Download install script + verify checksum          │
   │  curl -fsSL https://github.com/host-manager/           │
   │    releases/latest/download/install.sh | sha256sum -c  │
   │                                                        │
   │  4. Download agent binary for platform/arch            │
   │◄──────────────────────────────────────────────────────(GitHub)
   │  5. Verify SHA256 checksum                             │
   │                                                        │
   │  6. Run installer (bootstrap token via env var)        │
   │  HOSTMGR_BOOTSTRAP_TOKEN=<token> \                     │
   │    HOSTMGR_CONTROL_PLANE=https://control.example.com \ │
   │    ./install.sh                                        │
   │                                                        │
   │      installer:                                        │
   │      ├─ Detects init system (systemd / Docker / none)  │
   │      ├─ Creates hostmgr user (minimal privileges)      │
   │      ├─ Installs binary to /usr/local/bin/             │
   │      ├─ Writes config to /etc/hostmgr/agent.env        │
   │      ├─ Installs service unit / compose file           │
   │      └─ Starts agent                                   │
   │                                                        │
   │  7. Agent exchanges bootstrap token for NKey           │
   │◄──────────────────────────────────────────────────────►Control Plane
   │  8. Agent begins heartbeating                          │
   │──────────────────────────────────────────────────────►Control Plane
   │                                                        │
   │  9. Endpoint appears as "managed" in control plane UI  │
   │◄─────────────────────────────────────────────────────(Control Plane)
```

### Supported Service Modes

#### Mode 1: systemd Service (Primary)

Used when: `systemctl` is present and the system is booted with systemd as PID 1.

```ini
# /etc/systemd/system/hostmgr-agent.service
[Unit]
Description=Host Manager Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=hostmgr
EnvironmentFile=/etc/hostmgr/agent.env
ExecStart=/usr/local/bin/hostmgr-agent
Restart=on-failure
RestartSec=5s
# Prevent credential leakage
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/var/lib/hostmgr

[Install]
WantedBy=multi-user.target
```

The installer runs `systemctl enable --now hostmgr-agent` to start the agent and
enable it at boot.

#### Mode 2: Docker / Podman Container

Used when: systemd is unavailable (WSL 2, minimal environments, user-space installs)
or when the operator prefers containers.

```yaml
# /etc/hostmgr/docker-compose.yml
services:
  hostmgr-agent:
    image: ghcr.io/host-manager/hostmgr-agent:latest
    restart: unless-stopped
    env_file: /etc/hostmgr/agent.env
    network_mode: host        # Required for network discovery reporting
    volumes:
      - hostmgr-data:/var/lib/hostmgr
    read_only: true
    cap_drop:
      - ALL

volumes:
  hostmgr-data:
```

The installer runs `docker compose -f /etc/hostmgr/docker-compose.yml up -d`.

For rootless Podman environments, the same Compose file works with
`podman-compose` or `podman compose` (Podman 4.0+).

#### Mode 3: Foreground / Manual (Testing Only)

```bash
HOSTMGR_CONTROL_PLANE=https://control.example.com \
HOSTMGR_BOOTSTRAP_TOKEN=<token> \
./hostmgr-agent
```

Not recommended for production. Documented for local development and debugging.

### Installer Script Detection Logic

```
install.sh:
  1. Check for systemd: [ -d /run/systemd/system ]
     → Yes: use Mode 1 (systemd service)
     → No:  check for Docker/Podman
  2. Check for Docker: command -v docker
     → Yes: use Mode 2 (Docker Compose)
     → No:  check for Podman
  3. Check for Podman: command -v podman
     → Yes: use Mode 2 (Podman Compose)
     → No:  print instructions for Mode 3, exit with warning
```

### Security Hardening During Installation

| Concern | Mitigation |
|---|---|
| Bootstrap token in shell history | Token passed via environment variable, not CLI argument; `install.sh` unsets it after use |
| Binary tampering | SHA256 checksum verified before execution; Cosign signature verification in Phase 2 |
| Privilege escalation | Agent runs as dedicated `hostmgr` system user with minimal privileges |
| Credential file permissions | `/etc/hostmgr/agent.env` owned by `hostmgr:hostmgr`, mode `0600` |
| Network exposure | Agent has no listening ports (ADR-005); `NoNewPrivileges=true` in systemd unit |
| Lingering bootstrap token | Token is single-use and 5-minute TTL; invalid after first use regardless |

### Supported Platforms (MVP)

| Platform | Architecture | Service Mode |
|---|---|---|
| Ubuntu 20.04+ | amd64, arm64 | systemd |
| Debian 11+ | amd64, arm64 | systemd |
| Raspberry Pi OS (Bookworm) | armv7, arm64 | systemd |
| RHEL / Rocky / Alma 8+ | amd64, arm64 | systemd |
| WSL 2 (Ubuntu) | amd64 | Docker / Podman |
| Any Linux with Docker | amd64, arm64 | Docker |
| Any Linux with Podman | amd64, arm64 | Podman |

---

## Consequences

### Positive Impacts

**1. Low implementation risk for MVP**
Manual installation with a well-tested shell script is straightforward to implement,
test, and document. No SSH automation, no remote execution framework, no complex
error handling across diverse OS environments.

**2. Operator understanding and trust**
Operators installing software on their own machines should be able to read and understand
exactly what the installer does. A readable shell script with clear steps builds trust
in a way that opaque automation does not. This is especially important for open-source
adoption.

**3. Broad platform coverage**
Supporting both systemd and Docker/Podman covers the full target hardware range,
including edge cases like WSL 2 and minimal Linux environments.

**4. FDO-compatible design**
The bootstrap token mechanism, binary format, and environment-variable configuration
are all compatible with automated FDO-based deployment. Phase 2 automation replaces
the human operator with the FDO owner service and Open Horizon workload delivery —
but the agent itself does not change.

### Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Operator makes mistakes during manual install | Clear, tested documentation with verification steps; installer script does detection and validation automatically |
| Bootstrap token expires before operator completes installation | Token TTL configurable (default 5 min); operator can generate a new token from control plane at any time |
| SHA256 check skipped by operator | Installer script performs the check programmatically; instructions emphasise it prominently |
| WSL 2 without Docker | Mode 3 (foreground) documented as a fallback; WSL 2 systemd enablement documented |
| Raspberry Pi armv7 binary size | Static binary target < 30 MB; acceptable for Pi 3 and later |

### Implementation Considerations

- The install script must be idempotent — running it twice on the same host should
  not create duplicate service units or corrupt the credential store
- The installer should detect an existing installation and offer an upgrade path
  (stop agent, replace binary, restart) — even in MVP
- `/var/lib/hostmgr/` is the agent's data directory: NKey private key (encrypted),
  last-known control plane address, update state
- The agent binary must be statically linked (no libc dependency) to maximise
  platform compatibility — consistent with ADR-005's design

---

## Alternatives Considered

### Alternative 1: Automated SSH Deployment from Control Plane

**Decision:** Deferred to Phase 2

**Rationale:** The Agent Coordinator actor could SSH into each identified endpoint,
upload the binary, and configure the service automatically. This is the natural
"button in the UI" experience operators want. It requires robust error handling,
OS detection, and idempotent remote execution — significant scope for MVP. The SSH
credentials already exist in OpenBao (ADR-002) from the identification phase, so the
prerequisite infrastructure is in place; the implementation is the gap. Designed here
so Phase 2 automation is a UI addition, not an architectural change.

### Alternative 2: Package Repository (.deb / .rpm)

**Decision:** Deferred to Phase 2

**Rationale:** A package repository (apt/yum) provides the cleanest install experience
and integrates with system update tools. It requires maintaining a signed apt/yum
repository, handling multi-distro packaging, and adding CI for package builds. This
is operational overhead that is not justified for MVP but is the right long-term answer
for widely-deployed software. GitHub Releases with a shell installer is the established
interim pattern.

### Alternative 3: curl-pipe-bash (No Checksum)

**Decision:** Rejected

**Rationale:** `curl ... | bash` without checksum verification is a security antipattern.
It trusts the network at the moment of execution. The installer script is downloaded
separately, its checksum verified, and then executed — providing integrity verification
without significantly increasing installation complexity.

### Alternative 4: Ansible / Salt / Puppet Playbook

**Decision:** Documented as optional; not the primary path

**Rationale:** Many operators already use configuration management systems. Providing
an official Ansible role (Phase 2) would allow Host Manager agent deployment to slot
into existing automation. For MVP, the shell installer is the foundation that an
Ansible role would wrap.

---

## Future Phases

### Phase 2: Automated SSH Deployment

- Agent Coordinator actor SSHes to identified endpoints using OpenBao credentials
- Uploads binary, runs installer script, verifies agent appears online
- Operator triggers via "Deploy Agent" button in UI; no manual SSH required

### Phase 2: Ansible Role

- Official `host_manager.agent` Ansible role published to Ansible Galaxy
- Wraps the shell installer; handles idempotency, version pinning, upgrade

### Phase 2: Package Repository

- `.deb` and `.rpm` packages signed with Host Manager GPG key
- Hosted on GitHub Packages or a dedicated apt/yum repository
- Enables `apt upgrade` to update the agent alongside other system packages

### Phase 3: FIDO Device Onboard 2.0

- Devices shipped or provisioned with FDO Ownership Vouchers
- Host Manager control plane acts as FDO Owner Service
- On first boot, device contacts FDO Rendezvous Server, voucher transferred to
  Host Manager, agent deployed without any human steps
- Open Horizon integration: OH Agreement Bot deploys `hostmgr-agent` as a
  workload service to FDO-onboarded devices

---

## Related Decisions

- **ADR-001:** WasmCloud Control Plane — Agent Coordinator actor orchestrates
  deployment in Phase 2
- **ADR-002:** Credential Storage — SSH credentials in OpenBao used for Phase 2
  automated deployment
- **ADR-005:** Agent Communication Protocol — Bootstrap token mechanism defined here;
  NKey credential exchange defined in ADR-005
- **ADR-010:** Agent Lifecycle Management — Update and uninstall procedures build
  on the installation foundation defined here

---

## Open Questions

1. **Installer script language:** Shell (`bash`) is universal on Linux but fragile
   across edge cases. A Go or Rust installer binary would be more robust but adds
   a bootstrapping problem (how do you install the installer?). Shell for MVP;
   revisit for Phase 2 automated deployment.

2. **Uninstall procedure:** `install.sh --uninstall` should cleanly remove the
   agent, service unit, config, and data directory, and deregister from the control
   plane. Design in MVP; implement before Phase 1.1 launch.

3. **Multi-arch binary naming:** GitHub Releases naming convention for binaries
   (e.g., `hostmgr-agent-linux-amd64`, `hostmgr-agent-linux-arm64`) to be
   standardised in the release CI pipeline.

---

## Decision Record

**Approved by:** [To be filled after review]

**Date approved:** [To be filled]

**Concerns / feedback addressed:** [To be filled after discussion]
