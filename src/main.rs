use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use smolrunner::doctor::{inspect_host, render_human};

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
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Doctor { strict } => run_doctor(cli.output, strict),
    }
}

fn run_doctor(output: OutputFormat, strict: bool) -> ExitCode {
    let report = inspect_host();

    match output {
        OutputFormat::Human => print!("{}", render_human(&report)),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("failed to serialize doctor report: {error}");
                return ExitCode::from(2);
            }
        },
    }

    if report.has_failures() || (strict && report.has_warnings()) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
