//! PyStat CLI commands.

use std::path::{Path, PathBuf};

use clap::Subcommand;
use sikuwa_codegen_c::{
    ci_golden_manifest, load_ci_preset_cases, repo_root, verify_module_against_manifest,
    VerifyMode, VerifyReport,
};
use sikuwa_core::{Result, SikuwaError};
use sikuwa_pir::lower_file;
use sikuwa_pystat::{analyze_module_with_options, pstat_to_json, write_pstat, PystatOptions};

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
        /// Strict static analysis (SKW-T002 on non-S0)
        #[arg(long)]
        strict: bool,
        /// Minimum slot floor: static | tagged | dyn
        #[arg(long, value_name = "LEVEL")]
        min_slot: Option<String>,
    },
    /// Verify type evidence + ABI vs on-disk or golden manifest
    Verify {
        /// Input `.py` file (omit with `--preset ci --all`)
        #[arg(required_unless_present = "all")]
        input: Option<PathBuf>,
        /// Verification preset (`ci` → golden manifests under `tests/golden/manifests/`)
        #[arg(long)]
        preset: Option<String>,
        /// Run all cases listed in `tests/golden/manifests/preset.txt` (requires `--preset ci`)
        #[arg(long)]
        all: bool,
        /// Baseline `.skw.json` (default: `{input_dir}/{stem}.skw.json`, or CI golden with `--preset ci`)
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// Skip SKW-T003 ABI comparison
        #[arg(long)]
        allow_abi_break: bool,
        /// Exit 0 even when SKW-T001 type warnings exist
        #[arg(long)]
        allow_type_warnings: bool,
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
        PystatCommands::Report {
            input,
            output,
            json,
            strict,
            min_slot,
        } => {
            let pir = lower_file(&input)?;
            let mut opts = PystatOptions::default();
            if strict {
                opts = PystatOptions::strict();
            }
            if let Some(ms) = min_slot {
                opts.min_slot = PystatOptions::from_min_slot_str(&ms);
            }
            let report = analyze_module_with_options(&pir, &opts);
            print_report_summary(&report);
            if json {
                println!("{}", pstat_to_json(&report.module)?);
            }
            if let Some(path) = output {
                write_pstat(&path, &report.module)?;
                println!("wrote {}", path.display());
            }
        }
        PystatCommands::Verify {
            input,
            preset,
            all,
            manifest,
            allow_abi_break,
            allow_type_warnings,
        } => {
            if all {
                verify_ci_preset(allow_abi_break, allow_type_warnings)?;
            } else {
                let input = input.ok_or_else(|| SikuwaError::pystat("missing input .py"))?;
                verify_one(
                    &input,
                    preset.as_deref(),
                    manifest.as_deref(),
                    allow_abi_break,
                    allow_type_warnings,
                )?;
            }
        }
    }
    Ok(())
}

fn verify_ci_preset(allow_abi_break: bool, allow_type_warnings: bool) -> Result<()> {
    let root = repo_root();
    let cases = load_ci_preset_cases(&root).map_err(SikuwaError::pystat)?;
    let total = cases.len();
    let mut failed = 0usize;
    for py in &cases {
        println!("[preset ci] {}", py.display());
        if verify_one(
            &py,
            Some("ci"),
            None,
            allow_abi_break,
            allow_type_warnings,
        )
        .is_err()
        {
            failed += 1;
        }
    }
    if failed > 0 {
        return Err(SikuwaError::pystat(format!(
            "CI preset: {failed}/{total} case(s) failed",
        )));
    }
    println!("[preset ci] ok ({total} cases)");
    Ok(())
}

fn verify_one(
    input: &Path,
    preset: Option<&str>,
    manifest: Option<&Path>,
    allow_abi_break: bool,
    allow_type_warnings: bool,
) -> Result<()> {
    let pir = lower_file(input)?;
    let (manifest_path, mode) = resolve_manifest(input, preset, manifest)?;
    let verify = verify_module_against_manifest(
        &pir,
        manifest_path.as_deref(),
        allow_abi_break,
        mode,
    );
    print_verify(&verify);
    let type_ok = allow_type_warnings || verify.type_diags.is_empty();
    let abi_ok = allow_abi_break || verify.abi_diags.is_empty();
    if !(type_ok && abi_ok) {
        return Err(SikuwaError::pystat("verification failed"));
    }
    Ok(())
}

fn resolve_manifest(
    input: &Path,
    preset: Option<&str>,
    manifest: Option<&Path>,
) -> Result<(Option<PathBuf>, VerifyMode)> {
    if let Some(path) = manifest {
        let mode = if preset == Some("ci") {
            VerifyMode::Baseline
        } else {
            VerifyMode::OnDisk
        };
        return Ok((Some(path.to_path_buf()), mode));
    }
    if preset == Some("ci") {
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| SikuwaError::pystat("invalid input path"))?;
        let path = ci_golden_manifest(&repo_root(), stem);
        if !path.is_file() {
            return Err(SikuwaError::pystat(format!(
                "CI golden manifest not found: {}",
                path.display()
            )));
        }
        return Ok((Some(path), VerifyMode::Baseline));
    }
    Ok((default_manifest_path(input), VerifyMode::OnDisk))
}

fn default_manifest_path(input: &Path) -> Option<PathBuf> {
    let stem = input.file_stem()?.to_str()?;
    let dir = input.parent().unwrap_or_else(|| Path::new("."));
    Some(dir.join(format!("{stem}.skw.json")))
}

fn print_report_summary(report: &sikuwa_pystat::PystatReport) {
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
    for d in &report.diagnostics {
        eprintln!("{}", d.format_line());
    }
}

fn print_verify(verify: &VerifyReport) {
    print_report_summary(&verify.pystat);
    for d in verify.all_diagnostics() {
        eprintln!("{}", d.format_line());
    }
    if verify.ok() {
        println!("[verify] ok");
    } else {
        println!(
            "[verify] failed: {} type, {} abi",
            verify.type_diags.len(),
            verify.abi_diags.len()
        );
    }
}
