//! Sikuwa 2.0 CLI — Plan 1 (Ver.A2 scaffold)

mod config_util;
mod commands;

use clap::{Parser, Subcommand};
use sikuwa_core::VERSION;
use std::path::PathBuf;

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
    /// Build native shared library from Python entry (Plan 8d)
    Build {
        /// Entry `.py` file (imports define dependency order)
        input: PathBuf,
        /// Output directory for per-module codegen + final `.so`/`.dll`
        #[arg(short = 'o', long, default_value = "dist")]
        out_dir: PathBuf,
        /// Run golden pipeline (PIR O1 → HPGI → O2) before codegen
        #[arg(long)]
        opt: bool,
        /// Path to `sikuwa.toml` (default: search cwd)
        #[arg(short, long)]
        config: Option<PathBuf>,
        /// Allow breaking FFI ABI vs existing manifests
        #[arg(long)]
        allow_abi_break: bool,
        /// Do not link libsikuwa runtime sources
        #[arg(long)]
        no_runtime: bool,
        /// Do not link hotpath asm
        #[arg(long)]
        no_hotpath: bool,
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
        Commands::Build {
            input,
            out_dir,
            opt,
            config,
            allow_abi_break,
            no_runtime,
            no_hotpath,
        } => commands::build::run(
            input,
            out_dir,
            opt,
            config,
            allow_abi_break,
            no_runtime,
            no_hotpath,
        ),
        Commands::Validate { config } => commands::validate::run(&config),
    };
    std::process::exit(code);
}
