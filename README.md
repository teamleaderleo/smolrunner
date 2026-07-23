# SmolRunner

**GitHub Actions on the Linux boxes you already own.**

SmolRunner is a Rust-based steward for small fleets of self-hosted GitHub Actions runners. It is aimed at solo developers and small teams who have ordinary Linux servers, several repositories, and no desire to operate Kubernetes or inherit a full platform-engineering stack.

> [!IMPORTANT]
> SmolRunner is pre-alpha. The current executable provides host diagnostics; runner installation and reconciliation are roadmap work.

## The problem

The official GitHub Actions runner is straightforward to install once. The operational burden appears afterward:

- several repositories and organizations share one server;
- persistent runner services need upgrades and repair;
- repository code should not inherit host credentials or state;
- container limits, cgroup delegation, users, and systemd units drift;
- agents and humans need a trustworthy answer to “what is broken?”;
- setup should be repeatable on the next boring VPS.

SmolRunner keeps GitHub as the workflow scheduler, status UI, and log store. It focuses on host desired state, official runner lifecycle, disposable project execution, and diagnostics.

## Current command

```bash
cargo run -- doctor
```

Machine-readable output is available for agents and automation:

```bash
cargo run -- --output json doctor
```

Use strict mode when warnings should fail a provisioning check:

```bash
cargo run -- doctor --strict
```

The first doctor probes Linux support, architecture, systemd, cgroup v2, Podman, and Git. Human and JSON output are produced from the same typed report.

## Intended workflow

The planned interface is deliberately small:

```text
smolrunner doctor
smolrunner plan
smolrunner host prepare
smolrunner runner add
smolrunner project enroll
smolrunner status
smolrunner remove
```

Individual repositories continue to own their Containerfiles, dependency installation, test commands, and GitHub workflow YAML. SmolRunner will not introduce another pipeline language.

## Design principles

- **Official runner, managed safely.** SmolRunner does not reimplement the GitHub Actions protocol.
- **Persistent listener, disposable execution.** Repository code belongs in bounded rootless containers, not directly on the host.
- **Plan before mutation.** Host changes should be idempotent, inspectable, and reversible.
- **Secure defaults.** Fork execution, host sockets, untracked files, and secret inheritance are denied by default.
- **Boring infrastructure.** Debian or Ubuntu, systemd, cgroup v2, Podman, and one native binary.
- **Human and agent friendly.** Stable JSON is a first-class interface, not terminal output scraped after the fact.
- **Stay smol.** No mandatory daemon, database, dashboard, cloud controller, or Kubernetes cluster.

## Development

Rust 2024 stable is used. The repository checks formatting, Clippy, tests, and the JSON doctor path:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo run --quiet -- --output json doctor
```

## Project documents

- [Threat model](docs/THREAT_MODEL.md)
- [Roadmap](docs/ROADMAP.md)
- [Agent instructions](AGENTS.md)

## Project status

The first milestone is a dependable diagnostic and desired-state foundation. A dashboard, daemon, cloud autoscaling, and broader distribution support are intentionally deferred until the CLI and security model have proven themselves.
