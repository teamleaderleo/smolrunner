# Roadmap

SmolRunner should remain useful while it is still small. The roadmap favors a dependable CLI and explicit host state over a control-plane service or dashboard.

## Milestone 0 — foundation

- Rust CLI with human and JSON output.
- Host `doctor` checks for Linux, systemd, cgroup v2, Podman, and required commands.
- Threat model and non-goals.
- Continuous formatting, linting, and test verification.

## Milestone 1 — desired state

- Versioned `smolrunner.yml` manifest.
- Typed host, runner, project, and resource-limit models.
- `smolrunner plan` that makes no changes.
- Idempotent command execution with structured events.
- Debian and Ubuntu host preparation.
- Explicit rollback records for every host mutation.

## Milestone 2 — runner lifecycle

- Install a checksum-verified official GitHub Actions runner.
- Repository and organization registration scopes.
- Dedicated Linux user and systemd service management.
- Runner status, version inspection, update, disable, and removal.
- Short-lived registration-token handling without persistent plaintext storage.

## Milestone 3 — project execution

- Project-owned Containerfile and verification command.
- Rootless Podman image build and digest recording.
- Immutable committed-source archives.
- Separate network policy for dependency installation and verification.
- Capability dropping, no-new-privileges, and resource limits.
- Focused and full suite conventions without inventing a pipeline language.

## Milestone 4 — small-fleet operations

- Multi-host inventory over SSH.
- Fleet-wide `doctor`, status, and upgrade planning.
- Disk-pressure and stale-image diagnostics.
- Machine-readable remediation suggestions.
- Optional terminal UI backed by the same core library.

## Later, only with evidence

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
