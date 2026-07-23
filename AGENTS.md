# SmolRunner agent instructions

## Product boundary

SmolRunner is a Rust-based steward for a small number of self-hosted GitHub Actions runners on ordinary Linux servers. It manages desired host state, official runner lifecycle, project isolation, and diagnostics.

Do not turn SmolRunner into a new pipeline language, runner protocol, deployment platform, Kubernetes controller, or cloud autoscaler.

## Current priorities

1. Preserve the threat-model invariants in `docs/THREAT_MODEL.md`.
2. Follow the privilege, adoption, and rollback decisions in `docs/adr/0001-privilege-adoption-and-rollback.md`.
3. Build a dependable CLI and structured state model before adding a daemon, TUI, or web dashboard.
4. Prefer idempotent plans and explicit reconciliation over one-shot shell setup.
5. Keep project-specific build and test behavior inside each enrolled repository.
6. Unknown manifest fields and versions must fail closed.
7. Distinguish proven absence from unknown state; never mutate based on an unproven assumption.

## Required checks

Before declaring a change ready:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo run --locked --quiet -- --output json doctor
cargo run --locked --quiet -- plan --file examples/quarry.yml
cargo run --locked --quiet -- --output json plan --file examples/glossless.yml
cargo run --locked --quiet -- --output json host plan --file examples/quarry.yml
```

A doctor warning is acceptable on a development machine that lacks Podman or systemd. A doctor failure must be understood and documented. Planning must never mutate the filesystem, users, services, containers, or GitHub state.

## Implementation rules

- Unsafe Rust is forbidden.
- Human output and JSON output must be derived from the same typed report.
- Never print registration tokens, app keys, repository credentials, or secret environment values.
- Commit `Cargo.lock` and use locked Cargo operations for this binary application.
- Pin third-party GitHub Actions to reviewed commit SHAs.
- Every host mutation must eventually support plan/dry-run behavior and a clear rollback path.
- Invalid mutation plans must fail before the first executor call.
- Irreversible actions must block the entire batch before the first mutation unless explicitly confirmed.
- Rollback and compensation run in reverse completion order; do not describe compensation as restoration.
- Public journals may contain only public receipts and public failures.
- Do not add an apply path until durable ownership markers, root elevation, runner-user execution, journal persistence, GitHub credential acquisition, and package-operation rollback classes are concretely designed.
- Generated subprocesses must use explicit absolute program paths and argument vectors; do not introduce `sh -c` or equivalent implicit shells.
- Child-process environments must start empty and receive only explicit allowlisted values.
- Treat output redaction as defense in depth, not proof that a child process cannot transform or leak a secret.
- Use stable system interfaces and invoke existing tools where that is safer than recreating package-manager, systemd, Git, or container-runtime behavior.
- Avoid adding dependencies without a concrete need and maintenance rationale.
- Keep Linux-specific code behind a narrow host abstraction so unsupported platforms fail clearly.
- Tests must not require root, systemd, Podman, or live GitHub credentials unless explicitly marked as integration tests.
- Keep manifests limited to host and execution policy. Language-specific build behavior belongs in repository-owned scripts and Containerfiles.

## Pull requests

Keep changes small enough to review. State the security impact, commands run, and any host assumptions. Do not claim a VPS or GitHub runner path passed unless the exact tested commit and result are available.
