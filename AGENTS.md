# SmolRunner agent instructions

## Product boundary

SmolRunner is a Rust-based steward for a small number of self-hosted GitHub Actions runners on ordinary Linux servers. It manages desired host state, official runner lifecycle, project isolation, and diagnostics.

Do not turn SmolRunner into a new pipeline language, runner protocol, deployment platform, Kubernetes controller, or cloud autoscaler.

## Current priorities

1. Preserve the threat-model invariants in `docs/THREAT_MODEL.md`.
2. Build a dependable CLI and structured state model before adding a daemon, TUI, or web dashboard.
3. Prefer idempotent plans and explicit reconciliation over one-shot shell setup.
4. Keep project-specific build and test behavior inside each enrolled repository.
5. Unknown manifest fields and versions must fail closed.

## Required checks

Before declaring a change ready:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo run --quiet -- --output json doctor
cargo run --quiet -- plan --file examples/quarry.yml
cargo run --quiet -- --output json plan --file examples/glossless.yml
```

A doctor warning is acceptable on a development machine that lacks Podman or systemd. A doctor failure must be understood and documented. Planning must never mutate the filesystem, users, services, containers, or GitHub state.

## Implementation rules

- Unsafe Rust is forbidden.
- Human output and JSON output must be derived from the same typed report.
- Never print registration tokens, app keys, repository credentials, or secret environment values.
- Every host mutation must eventually support plan/dry-run behavior and a clear rollback path.
- Use stable system interfaces and invoke existing tools where that is safer than recreating package-manager, systemd, Git, or container-runtime behavior.
- Avoid adding dependencies without a concrete need and maintenance rationale.
- Keep Linux-specific code behind a narrow host abstraction so unsupported platforms fail clearly.
- Tests must not require root, systemd, Podman, or live GitHub credentials unless explicitly marked as integration tests.
- Keep manifests limited to host and execution policy. Language-specific build behavior belongs in repository-owned scripts and Containerfiles.

## Pull requests

Keep changes small enough to review. State the security impact, commands run, and any host assumptions. Do not claim a VPS or GitHub runner path passed unless the exact tested commit and result are available.
