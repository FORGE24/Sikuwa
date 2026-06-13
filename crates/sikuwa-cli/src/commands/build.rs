//! Plan 8d — `sikuwa build`: lower → golden pipeline → codegen → link.

use std::path::PathBuf;

use sikuwa_codegen_c::{
    build_native_project, collect_module_order, BuildModuleOptions, BuildProjectOptions,
};
use sikuwa_core::{info, init_log_level, resolve_log_level, verbose_block, LogLevel, Result};

use crate::config_util::load_pystat_options;

pub fn run(
    input: PathBuf,
    out_dir: PathBuf,
    opt: bool,
    config: Option<PathBuf>,
    allow_abi_break: bool,
    no_runtime: bool,
    no_hotpath: bool,
    link_exe: bool,
    log_level: Option<LogLevel>,
    quiet: bool,
) -> i32 {
    let level = if quiet {
        LogLevel::Quiet
    } else {
        resolve_log_level(log_level)
    };
    init_log_level(level);

    match run_inner(
        &input,
        &out_dir,
        opt,
        config.as_deref(),
        allow_abi_break,
        no_runtime,
        no_hotpath,
        link_exe,
    ) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn run_inner(
    input: &PathBuf,
    out_dir: &PathBuf,
    opt: bool,
    config: Option<&std::path::Path>,
    allow_abi_break: bool,
    no_runtime: bool,
    no_hotpath: bool,
    link_exe: bool,
) -> Result<()> {
    let pystat = load_pystat_options(config);
    let order = collect_module_order(input)?;
    info(format!(
        "[build] {} module(s): {}",
        order.len(),
        order
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" → ")
    ));
    let result = build_native_project(
        input,
        out_dir,
        &BuildProjectOptions {
            module: BuildModuleOptions {
                opt,
                allow_abi_break,
                pystat,
                emit_module_desc: true,
            },
            link_runtime: !no_runtime,
            link_hotpath: !no_hotpath,
            link_exe,
        },
    )?;

    verbose_block("", result.compile_report.format_verbose_summary());
    verbose_block("", result.compile_report.format_stub_list());
    verbose_block("", result.artifact_report.format_verbose());
    if sikuwa_core::log_level() >= LogLevel::Trace {
        verbose_block("", result.compile_report.format_native_list());
    }

    info(format!(
        "[build] linked {} (entry {})",
        result.output_lib.display(),
        result.entry.display()
    ));
    if let Some(exe) = &result.output_exe {
        info(format!("[build] executable {}", exe.display()));
    }
    Ok(())
}
