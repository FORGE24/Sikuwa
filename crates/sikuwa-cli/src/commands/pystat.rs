//! PyStat CLI commands.

use std::path::PathBuf;

use clap::Subcommand;
use sikuwa_core::Result;
use sikuwa_pir::lower_file;
use sikuwa_pystat::{analyze_module, pstat_to_json, write_pstat};

#[derive(Subcommand)]
pub enum PystatCommands {
    /// Analyze Python → PGTE / ITR report
    Report {
        /// Input `.py` file
        input: PathBuf,
        /// Write `.pstat` binary
        #[arg(long)]
        output: Option<PathBuf>,
        /// Print JSON to stdout
        #[arg(long)]
        json: bool,
    },
}

pub fn run(cmd: PystatCommands) -> i32 {
    match run_inner(cmd) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn run_inner(cmd: PystatCommands) -> Result<()> {
    match cmd {
        PystatCommands::Report { input, output, json } => {
            let pir = lower_file(&input)?;
            let report = analyze_module(&pir);
            println!(
                "module: {}  functions: {}  ITR slots: {}  dyn: {}",
                report.module.module,
                report.module.functions.len(),
                report.itr_slots,
                report.dyn_slots
            );
            for f in &report.module.functions {
                let level = if f.static_eligible { "S0" } else { "dyn" };
                println!(
                    "  {}  ret={:?}  static={level}",
                    f.symbol.0, f.return_ty
                );
                for slot in f.params.iter().chain(f.locals.iter()) {
                    println!(
                        "    slot `{}` {:?} {:?}",
                        slot.name, slot.ty, slot.strategy
                    );
                }
            }
            if json {
                println!("{}", pstat_to_json(&report.module)?);
            }
            if let Some(path) = output {
                write_pstat(&path, &report.module)?;
                println!("wrote {}", path.display());
            }
        }
    }
    Ok(())
}
