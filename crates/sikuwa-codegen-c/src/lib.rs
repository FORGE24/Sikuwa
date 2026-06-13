//! Sikuwa-C codegen — emit native C from PIR + PyStat analysis.

mod artifact_report;
mod compile_report;
mod closure;
mod emit;
mod imports;
mod manifest;
mod abi_guard;
mod build;
mod pipeline;
mod preset;
mod py_shim;
mod slots;
mod structs;
mod verify;

pub use abi_guard::{
    abi_guard_error, annotate_abi_breaks, check_abi_guard, export_abi_diff, find_abi_breaks,
    find_baseline_drift, format_abi_errors,
};
pub use emit::{
    emit_module_c, emit_module_h, module_c_name, skw_c_symbol, skw_c_symbol_dyn, CodegenOptions,
    SKW_ABI_STRING,
};
pub use imports::{collect_manifest_imports, SkwImport};
pub use manifest::{
    emit_manifest, load_baseline_manifest, load_manifest, manifest_to_json, SkwManifest,
};
pub use artifact_report::{
    default_exe_path, emit_entry_main_c, find_entry_main, ArtifactReport, EntryMainInfo,
    ExeBuildStatus,
};
pub use compile_report::{
    compile_report_from_module, CodegenMode, CompileReport, FunctionCodegenEntry,
    ModuleCompileReport,
};
pub use build::{
    build_native_project, codegen_module_to_dir, collect_module_order, entry_import_modules,
    BuildModuleOptions, BuildProjectOptions, BuildProjectResult,
};
pub use pipeline::{
    run_compile_pipeline, run_compile_pipeline_with_options, run_compile_pipeline_with_peers,
    run_golden_pipeline, run_golden_pipeline_with_options, run_golden_pipeline_with_peers,
    CompilePipelineReport, PipelineMode, PipelineModeLabel,
};
pub use preset::{ci_golden_manifest, load_ci_preset_cases, repo_root, CI_GOLDEN_MANIFESTS, CI_PRESET_LIST};
pub use verify::{verify_module_against_manifest, VerifyMode, VerifyReport};
pub use py_shim::emit_pywrap_c;
pub use structs::emit_structs_h;
