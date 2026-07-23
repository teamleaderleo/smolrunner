use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::manifest::Manifest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Presence {
    Present,
    Absent,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CurrentHostState {
    pub commands: BTreeMap<String, Presence>,
    pub runner_user: Presence,
    pub subordinate_uids: Presence,
    pub subordinate_gids: Presence,
    pub linger: Presence,
    pub container_image: Presence,
    pub runner_registration: Presence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesiredHostState {
    pub required_commands: Vec<String>,
    pub runner_user: String,
    pub container_image: String,
    pub repository: String,
}

impl From<&Manifest> for DesiredHostState {
    fn from(manifest: &Manifest) -> Self {
        Self {
            required_commands: vec!["git".to_owned(), "podman".to_owned(), "systemctl".to_owned()],
            runner_user: manifest.runner.user.clone(),
            container_image: manifest.container.image.clone(),
            repository: manifest.repository.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostActionKind {
    EnsurePrerequisite,
    EnsureRunnerUser,
    EnsureSubordinateUids,
    EnsureSubordinateGids,
    EnsureLinger,
    EnsureContainerImage,
    EnsureRunnerRegistration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostActionDisposition {
    Required,
    NeedsInspection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostAction {
    pub id: String,
    pub kind: HostActionKind,
    pub disposition: HostActionDisposition,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostPlan {
    pub desired: DesiredHostState,
    pub current: CurrentHostState,
    pub actions: Vec<HostAction>,
}

#[must_use]
pub fn build_plan(manifest: &Manifest, current: CurrentHostState) -> HostPlan {
    let desired = DesiredHostState::from(manifest);
    let mut actions = Vec::new();

    for command in &desired.required_commands {
        let presence = current
            .commands
            .get(command)
            .copied()
            .unwrap_or(Presence::Unknown);
        push_for_presence(
            &mut actions,
            format!("prerequisite-{command}"),
            HostActionKind::EnsurePrerequisite,
            presence,
            format!("ensure prerequisite command {command}"),
        );
    }

    push_for_presence(
        &mut actions,
        "runner-user",
        HostActionKind::EnsureRunnerUser,
        current.runner_user,
        format!("ensure dedicated runner user {}", desired.runner_user),
    );
    push_for_presence(
        &mut actions,
        "subordinate-uids",
        HostActionKind::EnsureSubordinateUids,
        current.subordinate_uids,
        format!("ensure subordinate UID range for {}", desired.runner_user),
    );
    push_for_presence(
        &mut actions,
        "subordinate-gids",
        HostActionKind::EnsureSubordinateGids,
        current.subordinate_gids,
        format!("ensure subordinate GID range for {}", desired.runner_user),
    );
    push_for_presence(
        &mut actions,
        "linger",
        HostActionKind::EnsureLinger,
        current.linger,
        format!("ensure systemd linger for {}", desired.runner_user),
    );
    push_for_presence(
        &mut actions,
        "container-image",
        HostActionKind::EnsureContainerImage,
        current.container_image,
        format!("ensure rootless image {}", desired.container_image),
    );
    push_for_presence(
        &mut actions,
        "runner-registration",
        HostActionKind::EnsureRunnerRegistration,
        current.runner_registration,
        format!("ensure GitHub runner registration for {}", desired.repository),
    );

    HostPlan {
        desired,
        current,
        actions,
    }
}

fn push_for_presence(
    actions: &mut Vec<HostAction>,
    id: impl Into<String>,
    kind: HostActionKind,
    presence: Presence,
    summary: impl Into<String>,
) {
    let disposition = match presence {
        Presence::Present => return,
        Presence::Absent => HostActionDisposition::Required,
        Presence::Unknown => HostActionDisposition::NeedsInspection,
    };
    actions.push(HostAction {
        id: id.into(),
        kind,
        disposition,
        summary: summary.into(),
    });
}

pub trait HostProbe {
    /// Inspect bounded, read-only host state for one manifest.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when a required operating-system state file cannot be read safely.
    fn inspect(&self, manifest: &Manifest) -> io::Result<CurrentHostState>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxFilesystemProbe;

impl HostProbe for LinuxFilesystemProbe {
    fn inspect(&self, manifest: &Manifest) -> io::Result<CurrentHostState> {
        let runner_user = &manifest.runner.user;
        let commands = ["git", "podman", "systemctl"]
            .into_iter()
            .map(|command| {
                let presence = if find_command(command).is_some() {
                    Presence::Present
                } else {
                    Presence::Absent
                };
                (command.to_owned(), presence)
            })
            .collect();
        let user_present = file_has_key(Path::new("/etc/passwd"), runner_user)?;

        Ok(CurrentHostState {
            commands,
            runner_user: presence(user_present),
            subordinate_uids: if user_present {
                presence(file_has_key(Path::new("/etc/subuid"), runner_user)?)
            } else {
                Presence::Absent
            },
            subordinate_gids: if user_present {
                presence(file_has_key(Path::new("/etc/subgid"), runner_user)?)
            } else {
                Presence::Absent
            },
            linger: if user_present {
                presence(
                    Path::new("/var/lib/systemd/linger")
                        .join(runner_user)
                        .is_file(),
                )
            } else {
                Presence::Absent
            },
            container_image: Presence::Unknown,
            runner_registration: Presence::Unknown,
        })
    }
}

fn presence(value: bool) -> Presence {
    if value {
        Presence::Present
    } else {
        Presence::Absent
    }
}

fn file_has_key(path: &Path, key: &str) -> io::Result<bool> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(contents.lines().any(|line| {
            line.split_once(':')
                .is_some_and(|(candidate, _)| candidate == key)
        })),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn find_command(name: &str) -> Option<PathBuf> {
    env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::manifest::parse;

    use super::{
        CurrentHostState, HostActionDisposition, HostActionKind, Presence, build_plan,
    };

    const MANIFEST: &str = r#"
version: 1
repository: example/project
runner:
  scope: repository
  user: project-runner
  labels: [project-ci]
container:
  image: localhost/project-ci:1
  file: build/ci/Containerfile
verify:
  command: scripts/run-vps-verification.sh
  suites:
    full: full
limits:
  memory: 2GiB
  cpus: 1.5
  pids: 768
trust:
  forks: deny
  trigger: operator
"#;

    #[test]
    fn present_state_produces_no_actions() {
        let manifest = parse(MANIFEST).expect("valid manifest");
        let current = CurrentHostState {
            commands: BTreeMap::from([
                ("git".to_owned(), Presence::Present),
                ("podman".to_owned(), Presence::Present),
                ("systemctl".to_owned(), Presence::Present),
            ]),
            runner_user: Presence::Present,
            subordinate_uids: Presence::Present,
            subordinate_gids: Presence::Present,
            linger: Presence::Present,
            container_image: Presence::Present,
            runner_registration: Presence::Present,
        };

        assert!(build_plan(&manifest, current).actions.is_empty());
    }

    #[test]
    fn absent_and_unknown_state_are_distinguished() {
        let manifest = parse(MANIFEST).expect("valid manifest");
        let current = CurrentHostState {
            commands: BTreeMap::from([
                ("git".to_owned(), Presence::Present),
                ("podman".to_owned(), Presence::Absent),
                ("systemctl".to_owned(), Presence::Present),
            ]),
            runner_user: Presence::Absent,
            subordinate_uids: Presence::Absent,
            subordinate_gids: Presence::Absent,
            linger: Presence::Absent,
            container_image: Presence::Unknown,
            runner_registration: Presence::Unknown,
        };
        let plan = build_plan(&manifest, current);

        assert!(plan.actions.iter().any(|action| {
            action.kind == HostActionKind::EnsurePrerequisite
                && action.disposition == HostActionDisposition::Required
        }));
        assert!(plan.actions.iter().any(|action| {
            action.kind == HostActionKind::EnsureContainerImage
                && action.disposition == HostActionDisposition::NeedsInspection
        }));
    }
}
