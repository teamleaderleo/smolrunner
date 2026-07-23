# ADR 0003: Canonical locators and resource evidence

- Status: Accepted for the pre-probe architecture
- Date: 2026-07-23

## Context

ADR 0002 requires exact locators and immutable evidence before an existing user, directory, service, runner installation, image, or GitHub registration can be classified as managed or adoptable. A generic free-form string is not sufficient: path aliases, mutable image tags, transferred repositories, duplicate runner names, stale `.runner` data, and copied markers can all look plausible while describing a different resource.

This ADR defines version-one canonical constructors and validation. It does not authorize live probing, state persistence, adoption, or host mutation.

## Decision

Every resource identity has a kind, canonical locator, optional external ID, and optional versioned fingerprint. Desired identities must include the minimum evidence defined below. Observed and marker identities may omit evidence so classification can report `unknown`; any evidence that is present must still use the exact canonical format.

Fingerprints use a kind-specific `KIND-v1:` prefix followed by canonical JSON with unknown fields rejected. Digests use lowercase `sha256:` plus 64 hexadecimal characters.

Desired-state validation and observation validation are intentionally separate. A desired identity with missing required evidence is invalid and cannot enter planning. An observed or marker identity with missing evidence is structurally valid but incomplete, allowing ownership classification to return `unknown` rather than misreporting absence. Malformed evidence fails both validation paths.

### Linux user

- Locator: exact safe lowercase username.
- External ID: numeric UID.
- Fingerprint: primary GID, canonical home, canonical shell, and account policy (`login`, `service`, or `locked`).
- Observation lane: `root`.
- A username alone remains unknown.
- UID and account tuples do not automatically survive host restore.

### Managed directory

- Locator: lexical canonical absolute path with no `.`, `..`, duplicate separators, or trailing separator.
- External ID: SmolRunner installation ID.
- Fingerprint: expected owner UID, owner GID, and mode.
- Observation lane: `root`.
- The installation ID prevents an unmarked look-alike directory from becoming adoptable from path and mode alone.
- Device and inode values are diagnostics, not durable identity.

### systemd service

- Locator: exact escaped `.service` unit name. Raw separators, spaces, or malformed `\xHH` escapes are rejected.
- External ID: runner-installation identity.
- Fingerprint: canonical unit-file path, unit content digest, service user, and runner-installation identity.
- Observation lane: `root`.
- Unit descriptions, active state, and similar names are not ownership evidence.

### Official runner installation

- Locator: canonical absolute installation directory.
- External ID: GitHub numeric runner ID when available.
- Fingerprint: canonical HTTPS server URL, verified numeric repository or organization scope plus current canonical name, exact service unit, and installed runner version.
- Observation lanes: `runner_user` for local runner state and `root` for service evidence.
- Local `.runner` data is observation evidence only and is never modified to store SmolRunner metadata.

### Rootless Podman image

- Locator: exact lowercase `localhost/NAME:TAG` reference.
- External ID: immutable image digest or ID expressed as `sha256:`.
- Fingerprint: immutable build-input digest.
- Observation lane: `runner_user`.
- A tag without both immutable values is unknown.

### GitHub runner registration

- Locator: stable numeric repository or organization scope plus exact safe runner name.
- External ID: GitHub numeric runner ID.
- Fingerprint: authenticated API scope, including the current canonical repository slug or organization login.
- Observation lane: `github`.
- Duplicate runner names are disambiguated by scope and numeric runner ID. Labels never establish identity.

## Repository case, transfer, and re-registration

GitHub owner, repository, and organization names are canonicalized to lowercase for resource evidence. Numeric scope IDs form the stable locator component. A repository transfer keeps the numeric scope locator but changes the scope fingerprint and project identity, so explicit migration is required. Runner re-registration changes the numeric runner ID and therefore conflicts with stale registration evidence until reconciled explicitly.

## Evidence survival

The typed policy records whether evidence normally survives three events:

| Resource | Host restore | Repository transfer | Runner re-registration |
| --- | --- | --- | --- |
| Linux user | no | yes | yes |
| Directory | yes, when state and metadata are restored | yes | yes |
| systemd service | yes, when unit files are restored | yes | yes |
| Runner installation | yes, when local state is restored | no | no |
| Podman image | no by default | yes | yes |
| GitHub registration | yes | no | no |

These values describe expected evidence durability, not permission to mutate after an event. Project identity and ownership markers must still match.

## Consequences

- Desired identities cannot be constructed from a mutable tag, display name, path basename, or runner label alone.
- Untrusted markers with missing evidence can be classified as unknown instead of being rejected as absent.
- Evidence that is present but malformed fails validation before ownership classification.
- Live probes must execute only in the lanes listed above.
- State persistence and lane execution remain blocked by issues #11 and #12.

## Deferred work

This ADR does not define parsers for `/etc/passwd`, systemd unit files, Podman inspection output, the official runner's `.runner` file, or GitHub API responses. Those probes must map trusted observations into these constructors and must not bypass canonical validation.
