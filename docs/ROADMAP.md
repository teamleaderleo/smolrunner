# Roadmap

SmolRunner should remain useful while it is still small. The roadmap favors a dependable CLI and explicit host state over a control-plane service or dashboard.

## Milestone 0 — foundation

- [x] Rust CLI with human and JSON output.
- [x] Host `doctor` checks for Linux, systemd, cgroup v2, Podman, and required commands.
- [x] Threat model and non-goals.
- [x] Continuous formatting, linting, and test verification.

## Milestone 1 — desired state

- [x] Versioned `smolrunner.yml` manifest.
- [x] Typed host, runner, project, and resource-limit models.
- [x] `smolrunner plan` that makes no changes.
- [x] Typed current-host observations with present, absent, and unknown state.
- [x] Shell-free command execution records with an empty child environment and explicit secret redaction.
- [x] Typed execution lanes, precondition evidence, rollback classes, and partial-failure journals.
- [x] ADR for privilege separation, adoption, token handling, and rollback semantics.
- [x] Pure ownership identity and managed/adoptable/foreign/conflicting/unknown classification.
- [x] ADR for `/var/lib/smolrunner`, marker identity, and name-safe adoption policy.
- [ ] Canonical locators and immutable evidence for each real resource kind.
- [ ] Atomic ownership and journal persistence with crash recovery.
- [ ] Root and runner-user lane implementations.
- [ ] Debian and Ubuntu host preparation.

## Milestone 2 — runner lifecycle

- [ ] Install a checksum-verified official GitHub Actions runner.
- [ ] Repository and organization registration scopes.
- [ ] Dedicated Linux user and systemd service management.
- [ ] Runner status, version inspection, update, disable, and removal.
- [ ] Short-lived registration-token handling without persistent plaintext storage.

## Milestone 3 — project execution

- [ ] Project-owned Containerfile and verification command.
- [ ] Rootless Podman image build and digest recording.
- [ ] Immutable committed-source archives.
- [ ] Separate network policy for dependency installation and verification.
- [ ] Capability dropping, no-new-privileges, and resource limits.
- [ ] Focused and full suite conventions without inventing a pipeline language.

## Milestone 4 — small-fleet operations

- [ ] Multi-host inventory over SSH.
- [ ] Fleet-wide `doctor`, status, and upgrade planning.
- [ ] Disk-pressure and stale-image diagnostics.
- [ ] Machine-readable remediation suggestions.
- [ ] Optional terminal UI backed by the same core library.

## Later, only with evidence

- Project cache lifecycle with explicit ownership, quotas, isolation, and cleanup (#21).
- Bounded workload observations and transparent duration estimates (#21).
- Agent-readable queued and running status with host capability recommendations (#21).
- Web dashboard.
- Background daemon.
- GitHub App authentication.
- Ephemeral machine provisioning.
- Additional Linux distributions and service managers.

## Non-goals

- Replacing GitHub Actions workflow YAML.
- Reimplementing the GitHub Actions runner protocol.
- Kubernetes runner scale sets.
- Public-fork execution on persistent personal hosts.
- Becoming a general-purpose deployment platform.
