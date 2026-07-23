use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use smolrunner::doctor::{inspect_host, render_human as render_doctor};
use smolrunner::manifest::{ManifestError, load};
use smolrunner::plan::{build, render_human as render_plan};

#[derive(Debug, Parser)]
#[command(
    name = "smolrunner",
    version,
    about = "Tend a small fleet of self-hosted GitHub Actions runners"
)]
struct Cli {
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Human)]
    output: OutputFormat,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect whether the current host is ready for SmolRunner.
    Doctor {
        /// Treat warnings as a non-zero result.
        #[arg(long)]
        strict: bool,
    },
    /// Validate desired state and show the changes SmolRunner would eventually make.
    Plan {
        /// Manifest to validate and plan.
        #[arg(long, default_value = "smolrunner.yml")]
        file: PathBuf,
    },
}

#[derive(Debug, Serialize)]
struct ErrorReport<'a> {
    schema_version: u8,
    error: &'a ManifestError,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Doctor { strict } => run_doctor(cli.output, strict),
        Command::Plan { file } => run_plan(cli.output, &file),
    }
}

fn run_doctor(output: OutputFormat, strict: bool) -> ExitCode {
    let report = inspect_host();

    match output {
        OutputFormat::Human => print!("{}", render_doctor(&report)),
        OutputFormat::Json => {
            if print_json(&report).is_err() {
                return ExitCode::from(2);
            }
        }
    }

    if report.has_failures() || (strict && report.has_warnings()) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run_plan(output: OutputFormat, file: &Path) -> ExitCode {
    let manifest = match load(file) {
        Ok(manifest) => manifest,
        Err(error) => {
            match output {
                OutputFormat::Human => eprint!("{error}"),
                OutputFormat::Json => {
                    let report = ErrorReport {
                        schema_version: 1,
                        error: &error,
                    };
                    if print_json(&report).is_err() {
                        return ExitCode::from(2);
                    }
                }
            }
            return ExitCode::from(2);
        }
    };
    let report = build(&manifest, file);

    match output {
        OutputFormat::Human => print!("{}", render_plan(&report)),
        OutputFormat::Json => {
            if print_json(&report).is_err() {
                return ExitCode::from(2);
            }
        }
    }

    ExitCode::SUCCESS
}

fn print_json(value: &impl Serialize) -> Result<(), serde_json::Error> {
    match serde_json::to_string_pretty(value) {
        Ok(json) => {
            println!("{json}");
            Ok(())
        }
        Err(error) => {
            eprintln!("failed to serialize command output: {error}");
            Err(error)
        }
    }
}
