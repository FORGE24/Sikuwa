//! Sikuwa 2.0 CLI — Plan 1 (Ver.A2 scaffold)

mod commands;

use clap::{Parser, Subcommand};
use sikuwa_core::VERSION;

#[derive(Parser)]
#[command(
    name = "sikuwa",
    about = "Sikuwa — Python build toolchain (2.0 Ver.A2)",
    version = VERSION,
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print version and codename
    Version,
    /// Environment and toolchain diagnostics
    Doctor,
    /// PythonIR tools
    Pir {
        #[command(subcommand)]
        command: commands::pir::PirCommands,
    },
    /// PyStat / PGTE / ITR analysis
    Pystat {
        #[command(subcommand)]
        command: commands::pystat::PystatCommands,
    },
    /// Sikuwa-C native codegen
    Codegen {
        #[command(subcommand)]
        command: commands::codegen::CodegenCommands,
    },
    /// Link Sikuwa-C into shared libraries
    Link {
        #[command(subcommand)]
        command: commands::link::LinkCommands,
    },
    /// Validate sikuwa.toml (schema v2)
    Validate {
        /// Path to config file
        #[arg(short, long, default_value = "sikuwa.toml")]
        config: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Commands::Version => commands::version::run(),
        Commands::Doctor => commands::doctor::run(),
        Commands::Pir { command } => commands::pir::run(command),
        Commands::Pystat { command } => commands::pystat::run(command),
        Commands::Codegen { command } => commands::codegen::run(command),
        Commands::Link { command } => commands::link::run(command),
        Commands::Validate { config } => commands::validate::run(&config),
    };
    std::process::exit(code);
}
