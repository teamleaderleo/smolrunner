# ADR 0001: Privilege lanes, adoption, and rollback

- Status: Accepted for the pre-apply architecture
- Date: 2026-07-23

## Context

SmolRunner will eventually prepare Linux hosts, manage official GitHub Actions runners, and build project images through rootless Podman. Those operations cross several authority boundaries and may encounter installations created by hand or by earlier repository-specific scripts.

A generic `apply` loop that runs everything as root would erase the security boundary SmolRunner is intended to provide. A setup process that treats similarly named users, services, directories, images, or GitHub registrations as its own could damage existing Quarry, Glossless, or Starsector Preflight infrastructure.

Host changes also fail partially. Package installation may succeed before user creation fails; a runner may register before service installation fails; compensation may reduce harm without restoring the previous state. The product must report those distinctions honestly.

## Decision

### Execution lanes

Every proposed mutation declares exactly one lane:

- **operator** — the unprivileged process that loads configuration, inspects public state, renders plans, and coordinates confirmation.
- **root** — narrowly scoped operating-system operations such as package installation, Linux accounts, subordinate IDs, protected directories, and system service installation.
- **runner_user** — official runner and rootless Podman operations under the dedicated project account with an explicit environment.
- **github** — authenticated API operations with a bounded credential supplied specifically for that action.

No lane inherits another lane's ambient environment or credentials. Moving between lanes must be an explicit executor decision, not a property of an arbitrary command string.

The `runner_user` lane must set at least `HOME`, `USER`, `LOGNAME`, and `XDG_RUNTIME_DIR` from inspected account state. It must not inherit root's home, SSH keys, Git configuration, cloud credentials, or container storage.

### Action identity and preconditions

Every mutation has:

- an immutable action ID;
- one execution lane;
- a public summary;
- a rollback class;
- recorded precondition evidence.

Duplicate or empty action IDs and missing evidence invalidate the plan before execution. Preconditions describe the state that justified the action; they are not reconstructed after a failure.

### Rollback classes

- **reversible** — an explicit inverse can restore the relevant observed state.
- **compensating** — an explicit follow-up can reduce impact but cannot claim to restore the prior state.
- **irreversible** — no automatic inverse is safe or honest.

A batch containing an irreversible action is blocked before the first mutation unless the operator explicitly confirms irreversible execution. Irreversible actions are never silently mixed into automatic repair.

After an execution failure, completed reversible and compensating actions are processed in reverse completion order. The journal distinguishes `rolled_back`, `compensated`, and `rollback_failed` outcomes. Remaining actions stay pending.

### Public journal contract

The execution journal contains only public action metadata, public receipts, and public failures. Lower process layers are responsible for secret handling and redaction before returning a receipt. Raw child-process errors, registration tokens, private keys, and environment values do not enter the journal.

The journal is appendable state for explanation and recovery, not proof that the world still matches it. A future resume or rollback command must revalidate preconditions before acting.

### Adoption classification

Discovered resources are classified as:

- **managed** — exact durable SmolRunner identity evidence matches the current project;
- **adoptable** — full resource identity is compatible, but explicit operator confirmation and a recorded adoption action are required;
- **foreign** — ownership evidence points elsewhere or cannot be reconciled safely;
- **conflicting** — the desired identity collides with incompatible existing state.

Names alone never establish ownership. A Linux user called `project-runner`, an Actions service containing a repository name, or a matching image tag is insufficient for adoption.

The durable ownership format is intentionally deferred until the first real mutation implementation can enumerate which resources need markers and how those markers survive repository transfers and host migrations. Until then, unknown existing resources remain protected from mutation.

### Registration tokens

Registration tokens are transient inputs. They must not be stored in manifests, journals, state files, process argv, shell history, Debug output, or JSON. Executors should prefer an inherited file descriptor or a narrowly scoped environment value accepted by the official runner command, then remove the value immediately after use.

Token expiry, consumption, and authorization failure are public error classes with recovery instructions; the token value is never part of those errors.

## Consequences

- SmolRunner can test execution and recovery semantics with fake executors before adding root operations.
- Host application requires separate lane executors rather than one unrestricted command runner.
- Existing installations cannot be automatically adopted in the first apply release without a durable ownership specification.
- Some actions will be classified as compensating or irreversible even when a best-effort cleanup command exists.
- Plans may be more verbose because unknown and foreign state must be surfaced rather than guessed away.

## Deferred decisions and current blocker

Before the first host mutation PR, the project still needs a concrete design for:

1. durable ownership markers for users, directories, systemd services, runner installations, images, and registrations;
2. the elevation mechanism for the root lane;
3. the run-as-user mechanism and environment construction for the runner-user lane;
4. journal persistence location, permissions, atomic writes, and crash recovery;
5. how GitHub credentials are obtained for repository versus organization scope;
6. which Debian and Ubuntu package operations are reversible, compensating, or irreversible.

No `apply`, `host prepare`, `runner add`, or `project enroll` mutation is permitted until a follow-up design maps each concrete action to these decisions.
