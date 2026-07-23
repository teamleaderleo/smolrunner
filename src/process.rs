use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

const REDACTED: &str = "[REDACTED]";

#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(REDACTED)
    }
}

impl Serialize for SecretString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(REDACTED)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "sensitivity", content = "value", rename_all = "snake_case")]
pub enum CommandValue {
    Plain(String),
    Secret(SecretString),
}

impl CommandValue {
    fn exposed(&self) -> &str {
        match self {
            Self::Plain(value) => value,
            Self::Secret(value) => value.expose(),
        }
    }

    fn displayed(&self) -> String {
        match self {
            Self::Plain(value) => value.clone(),
            Self::Secret(_) => REDACTED.to_owned(),
        }
    }

    fn secret(&self) -> Option<&str> {
        match self {
            Self::Plain(_) => None,
            Self::Secret(value) if value.expose().is_empty() => None,
            Self::Secret(value) => Some(value.expose()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub arguments: Vec<CommandValue>,
    pub environment: BTreeMap<String, CommandValue>,
}

impl CommandSpec {
    #[must_use]
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            arguments: Vec::new(),
            environment: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn argument(mut self, value: impl Into<String>) -> Self {
        self.arguments.push(CommandValue::Plain(value.into()));
        self
    }

    #[must_use]
    pub fn secret_argument(mut self, value: impl Into<String>) -> Self {
        self.arguments
            .push(CommandValue::Secret(SecretString::new(value)));
        self
    }

    #[must_use]
    pub fn environment(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment
            .insert(key.into(), CommandValue::Plain(value.into()));
        self
    }

    #[must_use]
    pub fn secret_environment(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment
            .insert(key.into(), CommandValue::Secret(SecretString::new(value)));
        self
    }

    #[must_use]
    pub fn displayed_argv(&self) -> Vec<String> {
        std::iter::once(self.program.display().to_string())
            .chain(self.arguments.iter().map(CommandValue::displayed))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecutionRecord {
    pub argv: Vec<String>,
    pub environment_keys: Vec<String>,
    pub status: Option<i32>,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandExecutor {
    /// Execute one explicit program without an implicit shell.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the program path is unsafe or the process cannot be started.
    fn execute(&self, spec: &CommandSpec) -> io::Result<ExecutionRecord>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessExecutor;

impl CommandExecutor for ProcessExecutor {
    fn execute(&self, spec: &CommandSpec) -> io::Result<ExecutionRecord> {
        ensure_absolute_program(&spec.program)?;

        let mut command = Command::new(&spec.program);
        command.env_clear();
        command.args(spec.arguments.iter().map(CommandValue::exposed));
        for (key, value) in &spec.environment {
            command.env(key, value.exposed());
        }

        let output = command.output()?;
        let secrets = spec
            .arguments
            .iter()
            .chain(spec.environment.values())
            .filter_map(CommandValue::secret)
            .collect::<Vec<_>>();

        Ok(ExecutionRecord {
            argv: spec.displayed_argv(),
            environment_keys: spec.environment.keys().cloned().collect(),
            status: output.status.code(),
            success: output.status.success(),
            stdout: redact(&String::from_utf8_lossy(&output.stdout), &secrets),
            stderr: redact(&String::from_utf8_lossy(&output.stderr), &secrets),
        })
    }
}

fn ensure_absolute_program(program: &Path) -> io::Result<()> {
    if program.is_absolute() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "command program must be an absolute path: {}",
                program.display()
            ),
        ))
    }
}

fn redact(value: &str, secrets: &[&str]) -> String {
    secrets.iter().fold(value.to_owned(), |output, secret| {
        output.replace(secret, REDACTED)
    })
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;

    use super::{CommandExecutor, CommandSpec, ProcessExecutor, REDACTED};

    #[test]
    fn serialization_and_debug_output_redact_secrets() {
        let spec = CommandSpec::new("/usr/bin/example")
            .argument("visible")
            .secret_argument("very-secret")
            .secret_environment("TOKEN", "environment-secret");

        let debug = format!("{spec:?}");
        let json = serde_json::to_string(&spec).expect("serialize command spec");
        assert!(!debug.contains("very-secret"));
        assert!(!debug.contains("environment-secret"));
        assert!(!json.contains("very-secret"));
        assert!(!json.contains("environment-secret"));
        assert!(json.contains(REDACTED));
    }

    #[test]
    fn process_output_is_redacted() -> io::Result<()> {
        let printf = Path::new("/usr/bin/printf");
        if !printf.is_file() {
            return Ok(());
        }

        let spec = CommandSpec::new(printf)
            .argument("%s")
            .secret_argument("top-secret");
        let record = ProcessExecutor.execute(&spec)?;

        assert!(record.success);
        assert_eq!(record.stdout, REDACTED);
        assert!(!record.argv.join(" ").contains("top-secret"));
        Ok(())
    }

    #[test]
    fn relative_programs_are_rejected() {
        let error = ProcessExecutor
            .execute(&CommandSpec::new("printf"))
            .expect_err("relative program must fail");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
}
