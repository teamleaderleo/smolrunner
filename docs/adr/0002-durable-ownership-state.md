# ADR 0002: Durable ownership and host state identity

- Status: Accepted for the pre-persistence architecture
- Date: 2026-07-23

## Context

SmolRunner may encounter existing Linux users, runner directories, systemd services, Podman images, and GitHub runner registrations created by hand or by repository-specific tooling. A matching name or path is not evidence that SmolRunner owns the resource. Treating names as ownership could disable or delete working Quarry, Glossless, Starsector Preflight, or unrelated infrastructure.

The Filesystem Hierarchy Standard defines `/var/lib` for host-specific variable state that persists between invocations and reboots. SmolRunner also needs durable project and installation identity that survives CLI restarts without entering repository configuration, logs, or a runner user's home.

GitHub exposes stable numeric runner IDs through its API. The official runner's local `.runner` configuration can provide registration evidence, but neither a display name nor a label proves SmolRunner ownership.

## Decision

### State root

System-wide SmolRunner state belongs beneath:

```text
/var/lib/smolrunner/
```

The planned layout is:

```text
/var/lib/smolrunner/
  installations/
    INSTALLATION_ID/
      project.json
      resources/
        RESOURCE_RECORD.json
      journals/
        JOURNAL_ID.json
```

This ADR defines the logical location and document identities only. Atomic creation, permissions, symlink defense, locking, migration, and crash recovery remain future implementation work.

The state root must contain no registration tokens, GitHub access tokens, SSH keys, cloud credentials, repository secrets, or captured process environments.

### Project identity

A managed project identity contains:

- exact `OWNER/REPOSITORY`;
- runner scope (`repository` or `organization`);
- dedicated Linux runner user.

Repository transfer or runner-scope migration therefore changes project identity and requires an explicit migration or adoption decision. Redirects and similar names are not silently followed.

### Installation identity

Each SmolRunner-managed project on one host receives a random opaque installation ID. The ID is not a credential; it separates two independent installations that happen to manage the same repository or use similar resource names.

Version one accepts a lowercase ASCII identifier of 16 to 80 characters using letters, digits, and `-`. Generation is deferred until persistence exists.

### Resource identity

Every ownership record identifies one resource by:

- resource kind;
- exact locator;
- optional external ID;
- optional immutable fingerprint.

Initial resource kinds are Linux user, directory, systemd service, official runner installation, Podman image, and GitHub runner registration.

A locator names where to inspect a resource. It does not establish ownership. Examples include a username, canonical absolute path, exact systemd unit name, image reference, or repository-and-runner locator.

External IDs and fingerprints provide compatibility evidence. Examples include a GitHub numeric runner ID, canonical UID, image digest, runner installation identity, service unit digest, or directory marker digest. Labels, display names, mutable tags, and path basenames alone are not immutable evidence.

### Marker identity

A versioned ownership marker binds:

- installation ID;
- project identity;
- exact resource identity and evidence.

A marker is a SmolRunner claim, not unquestionable truth. Classification verifies that the marker, observed resource, desired resource, and evidence agree. A copied or stale marker produces conflict or unknown state rather than authorization.

### Classification

An existing resource is classified as:

- **managed** — marker, project, installation, locator, and all required evidence match exactly;
- **adoptable** — no SmolRunner marker exists, but exact immutable desired evidence matches; explicit adoption is still required;
- **foreign** — a valid marker names another project or installation;
- **conflicting** — locator, marker, or immutable evidence disagrees;
- **unknown** — evidence is missing or the marker version is not understood.

Matching names without immutable evidence are unknown, not adoptable. Unknown and foreign resources are protected from mutation.

### Marker placement

This ADR does not authorize writing metadata into arbitrary existing resources. The canonical marker is the record beneath `/var/lib/smolrunner`. A future implementation may place redundant marker files inside SmolRunner-created directories or runner installations only after proving the location was newly created or explicitly adopted.

SmolRunner must not modify GitHub's `.runner` file to add private metadata. It may read supported fields as observation evidence. Deleting `.runner` is a runner-removal operation, not marker cleanup.

### GitHub registrations

GitHub API runner IDs are preferred external evidence when authenticated access is available. Runner names and labels remain routing and display metadata. A local `.runner` record plus a matching GitHub API runner ID can support adoption, but neither alone creates SmolRunner ownership.

## Consequences

- Existing repository-specific runners remain protected even when their usernames or labels resemble desired SmolRunner values.
- Safe automatic adoption is intentionally impossible; adoption requires exact compatible evidence and explicit operator confirmation.
- Repository transfers and organization migrations require identity migration rather than silent continuation.
- State persistence becomes a privileged, security-sensitive subsystem beneath `/var/lib/smolrunner`.
- A deleted state directory causes resources to become unmarked or unknown; SmolRunner must not reconstruct ownership from names.

## Deferred implementation blocker

Before writing ownership state, SmolRunner still needs:

1. atomic same-filesystem write and rename semantics;
2. directory and file ownership/modes;
3. parent-directory and destination symlink protections;
4. process locking and concurrent invocation behavior;
5. fsync and crash-recovery policy;
6. schema migration and backup behavior;
7. canonical locator and fingerprint definitions for each resource kind;
8. installation-ID generation from an operating-system randomness source.

No host apply command may treat this ADR alone as permission to create, adopt, update, or remove a resource.
