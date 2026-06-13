//! Plan 8d — multi-module native build (codegen + link with manifest imports).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use sikuwa_core::{Result, SikuwaError};
use sikuwa_link::{default_shared_extension, link_executable, link_shared, LinkOptions};
use sikuwa_pir::{lower_file, verify_module};
use sikuwa_pystat::{peer_summaries_from_stats, PystatOptions, PystatReport};

use crate::{
    abi_guard::{annotate_abi_breaks, check_abi_guard},
    artifact_report::{
        default_exe_path, emit_entry_main_c, find_entry_main, ArtifactReport, ExeBuildStatus,
    },
    compile_report::{compile_report_from_module, CompileReport},
    emit::{emit_module_c, emit_module_h, CodegenOptions},
    manifest::{emit_manifest, load_manifest, manifest_to_json},
    pipeline::{run_compile_pipeline_with_peers, PipelineMode},
};

#[derive(Debug, Clone)]
pub struct BuildModuleOptions {
    pub opt: bool,
    pub allow_abi_break: bool,
    pub pystat: PystatOptions,
    pub emit_module_desc: bool,
}

impl Default for BuildModuleOptions {
    fn default() -> Self {
        Self {
            opt: true,
            allow_abi_break: false,
            pystat: PystatOptions::default(),
            emit_module_desc: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuildProjectOptions {
    pub module: BuildModuleOptions,
    pub link_runtime: bool,
    pub link_hotpath: bool,
    /// Also link a standalone executable calling entry `main()`.
    pub link_exe: bool,
}

impl Default for BuildProjectOptions {
    fn default() -> Self {
        Self {
            module: BuildModuleOptions::default(),
            link_runtime: true,
            link_hotpath: true,
            link_exe: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuildProjectResult {
    pub entry: PathBuf,
    pub output_lib: PathBuf,
    pub output_exe: Option<PathBuf>,
    pub module_dirs: Vec<PathBuf>,
    pub compile_report: CompileReport,
    pub artifact_report: ArtifactReport,
}

/// Topological order: dependencies first, entry last.
pub fn collect_module_order(entry: &Path) -> Result<Vec<PathBuf>> {
    let entry = fs::canonicalize(entry).map_err(SikuwaError::from)?;
    let mut order = Vec::new();
    let mut seen = HashSet::new();
    visit_module(&entry, &mut order, &mut seen)?;
    Ok(order)
}

fn visit_module(path: &Path, order: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) -> Result<()> {
    if !seen.insert(path.to_path_buf()) {
        return Ok(());
    }
    let pir = lower_file(path)?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    for imp in &pir.imports {
        if imp.symbol.ends_with(".*") {
            continue;
        }
        let dep = base.join(format!("{}.py", imp.module));
        if dep.is_file() {
            visit_module(&dep, order, seen)?;
        }
    }
    order.push(path.to_path_buf());
    Ok(())
}

pub fn codegen_module_to_dir(
    input: &Path,
    out_dir: &Path,
    opts: &BuildModuleOptions,
    peer_stats: &[sikuwa_pystat::FuncStat],
) -> Result<(PystatReport, crate::compile_report::ModuleCompileReport)> {
    fs::create_dir_all(out_dir)?;
    let mut pir = lower_file(input)?;
    let mode = if opts.opt {
        PipelineMode::Golden
    } else {
        PipelineMode::None
    };
    let peer_summaries = peer_summaries_from_stats(peer_stats);
    let (report, _) =
        run_compile_pipeline_with_peers(&mut pir, mode, &opts.pystat, &peer_summaries)?;
    if !opts.opt {
        let v = verify_module(&pir);
        if !v.ok() {
            return Err(SikuwaError::pir(v.errors.join("; ")));
        }
    }
    let compile = compile_report_from_module(&pir, &report);
    let cg = CodegenOptions {
        emit_module_desc: opts.emit_module_desc,
        emit_structs: true,
        peer_funcs: peer_stats
            .iter()
            .map(|f| (f.symbol.0.clone(), f.clone()))
            .collect(),
        ..Default::default()
    };
    let h = emit_module_h(&pir, &report, &cg);
    let c = emit_module_c(&pir, &report, &cg);
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module");
    fs::write(out_dir.join(format!("{stem}.h")), h)?;
    fs::write(out_dir.join(format!("{stem}.c")), c)?;
    let mut manifest = emit_manifest(&pir, &report);
    let json_path = out_dir.join(format!("{stem}.skw.json"));
    if !opts.allow_abi_break {
        let breaks = check_abi_guard(&json_path, &manifest)?;
        if !breaks.is_empty() {
            return Err(crate::abi_guard_error(&breaks));
        }
    } else if let Ok(previous) = load_manifest(&json_path) {
        manifest = annotate_abi_breaks(manifest, &previous);
    }
    fs::write(json_path, manifest_to_json(&manifest))?;
    Ok((report, compile))
}

pub fn build_native_project(
    entry: &Path,
    out_dir: &Path,
    opts: &BuildProjectOptions,
) -> Result<BuildProjectResult> {
    fs::create_dir_all(out_dir)?;
    let order = collect_module_order(entry)?;
    if order.is_empty() {
        return Err(SikuwaError::pir("empty build graph"));
    }
    let mut module_dirs = Vec::new();
    let mut peer_stats: Vec<sikuwa_pystat::FuncStat> = Vec::new();
    let mut compile_report = CompileReport::default();
    for path in &order {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");
        let mod_out = out_dir.join(stem);
        let (report, module_compile) =
            codegen_module_to_dir(path, &mod_out, &opts.module, &peer_stats)?;
        compile_report.modules.push(module_compile);
        peer_stats.extend(report.module.functions);
        module_dirs.push(mod_out);
    }
    let entry_canon = fs::canonicalize(entry).map_err(SikuwaError::from)?;
    let entry_stem = entry_canon
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module");
    let entry_dir = out_dir.join(entry_stem);
    let dep_dirs: Vec<PathBuf> = order
        .iter()
        .filter(|p| *p != &entry_canon)
        .filter_map(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .map(|stem| out_dir.join(stem))
        })
        .collect();
    let ext = default_shared_extension();
    let output_lib = out_dir.join(format!("lib{entry_stem}.{ext}"));
    link_shared(&LinkOptions {
        input: entry_dir.clone(),
        output: output_lib.clone(),
        include_dirs: dep_dirs.clone(),
        compiler: None,
        link_runtime: opts.link_runtime,
        link_hotpath: opts.link_hotpath,
        extra_source_dirs: dep_dirs.clone(),
        library_dirs: Vec::new(),
        libraries: Vec::new(),
        extra_sources: Vec::new(),
    })?;

    let entry_pir = lower_file(&entry_canon)?;
    let entry_compile = compile_report
        .modules
        .last()
        .cloned()
        .unwrap_or_default();
    let entry_main = find_entry_main(&entry_pir, &entry_compile);

    let mut exe_status = if opts.link_exe {
        ExeBuildStatus::NoEntryMain
    } else {
        ExeBuildStatus::NotRequested
    };
    let mut output_exe = None;

    if opts.link_exe {
        if let Some(main_info) = &entry_main {
            let _ = fs::remove_file(entry_dir.join("_skw_entry_main.c"));
            let main_path = out_dir.join(format!("_{entry_stem}_entry_main.c"));
            fs::write(&main_path, emit_entry_main_c(main_info, entry_stem))?;
            let exe_path = default_exe_path(out_dir, entry_stem);
            match link_executable(&LinkOptions {
                input: entry_dir.clone(),
                output: exe_path.clone(),
                include_dirs: dep_dirs.clone(),
                compiler: None,
                link_runtime: opts.link_runtime,
                link_hotpath: opts.link_hotpath,
                extra_source_dirs: dep_dirs,
                library_dirs: Vec::new(),
                libraries: Vec::new(),
                extra_sources: vec![main_path],
            }) {
                Ok(()) => {
                    output_exe = Some(exe_path.clone());
                    exe_status = ExeBuildStatus::Linked(exe_path);
                }
                Err(e) => {
                    exe_status = ExeBuildStatus::Failed(e.to_string());
                }
            }
        }
    }

    let artifact_report = ArtifactReport {
        dll_path: output_lib.clone(),
        dll_built: true,
        exe_requested: opts.link_exe,
        exe_status,
        entry_main,
    };

    Ok(BuildProjectResult {
        entry: entry_canon,
        output_lib,
        output_exe,
        module_dirs,
        compile_report,
        artifact_report,
    })
}

pub fn entry_import_modules(entry: &Path) -> Result<Vec<String>> {
    let pir = lower_file(entry)?;
    Ok(pir
        .imports
        .iter()
        .filter(|i| !i.symbol.ends_with(".*"))
        .map(|i| i.module.clone())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_add_before_caller() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
        let caller = root.join("plan5_caller.py");
        let order = collect_module_order(&caller).unwrap();
        assert_eq!(order.len(), 2);
        assert!(order[0].to_string_lossy().contains("add.py"));
        assert!(order[1].to_string_lossy().contains("plan5_caller.py"));
    }
}