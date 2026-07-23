# Threat model

SmolRunner manages persistent GitHub Actions runner listeners and disposable project execution environments on ordinary Linux hosts. That makes its defaults security-sensitive even when the fleet is small.

## Assets to protect

- The host operating system and its service accounts.
- GitHub registration tokens, app credentials, and repository credentials.
- Secrets belonging to other projects on the same host.
- The integrity of runner services, project images, and verification results.
- The operator's ability to understand and reverse every change SmolRunner makes.

## Trust boundaries

1. GitHub schedules a job and sends it to the official Actions runner.
2. The official runner is persistent and therefore treated as part of the host control plane.
3. Checked-out repository code is treated as untrusted input, even for private repositories.
4. Repository code executes only in a project-owned, disposable rootless container.
5. A container shares the host kernel and is not equivalent to a virtual machine.

## Required security invariants

SmolRunner must preserve these invariants unless the operator explicitly selects a weaker mode:

- Never expose a Docker or Podman control socket to repository code.
- Never copy untracked files or host environment files into a job workspace.
- Resolve mutable Git references to an immutable commit before execution.
- Deny fork pull requests on persistent self-hosted runners.
- Do not add automatic `pull_request` execution to a persistent runner by default.
- Use a dedicated, unprivileged Linux account for each runner security boundary.
- Keep runner accounts out of `sudo` and unrelated project groups.
- Drop container capabilities, disable privilege escalation, and apply CPU, memory, and PID limits.
- Pass only the minimum environment required by a job.
- Keep dependency installation and verification network policy independently configurable.
- Verify official runner downloads before installation.
- Make host changes idempotent, inspectable, and reversible.

## Initial supported trust model

The first release targets a solo developer or small trusted team running private repositories on Debian or Ubuntu hosts. Operator-triggered jobs from same-repository branches are supported. Public fork workloads and mutually hostile tenants are not.

## Out of scope for the first release

- Protection against Linux kernel or container-runtime vulnerabilities.
- Safe execution of arbitrary public pull requests.
- Strong tenant isolation between mutually hostile users.
- Secret management for deployment workloads.
- Cloud autoscaling or Kubernetes runner scale sets.
- Reimplementing the GitHub Actions runner protocol.

## Failure behavior

When SmolRunner cannot prove that a security invariant holds, it should stop before executing repository code and explain the failing check. Repair commands must support a dry-run or plan mode before changing the host.

## Reporting vulnerabilities

Do not publish credential exposure or host-escape details in a public issue. Use GitHub's private security advisory flow once it is enabled for the repository.
