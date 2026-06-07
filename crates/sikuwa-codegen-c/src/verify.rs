//! Verification helpers — type diagnostics + manifest ABI checks.

use std::collections::HashMap;
use std::path::Path;

use sikuwa_pir::module::FuncDef;
use sikuwa_pir::Module;
use sikuwa_pystat::{analyze_module, PystatDiagnostic, PystatReport};

use crate::abi_guard::{check_abi_guard, find_baseline_drift};
use crate::manifest::{emit_manifest, load_baseline_manifest, slot_level_str, SkwManifest};
use crate::slots::{tier_for, CodegenTier};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyMode {
    /// Compare with on-disk `{stem}.skw.json` (skip when `source_hash` matches).
    OnDisk,
    /// Compare with committed golden manifest (CI preset).
    Baseline,
}

#[derive(Debug, Clone)]
pub struct VerifyReport {
    pub pystat: PystatReport,
    pub manifest: SkwManifest,
    pub type_diags: Vec<PystatDiagnostic>,
    pub abi_diags: Vec<PystatDiagnostic>,
}

impl VerifyReport {
    pub fn all_diagnostics(&self) -> impl Iterator<Item = &PystatDiagnostic> {
        self.type_diags
            .iter()
            .chain(self.abi_diags.iter())
    }

    pub fn ok(&self) -> bool {
        self.type_diags.is_empty() && self.abi_diags.is_empty()
    }
}

pub fn verify_module_against_manifest(
    module: &Module,
    manifest_path: Option<&Path>,
    allow_abi_break: bool,
    mode: VerifyMode,
) -> VerifyReport {
    let pystat = analyze_module(module);
    let manifest = emit_manifest(module, &pystat);
    let mut type_diags = pystat.diagnostics.clone();
    type_diags.extend(codegen_slot_diagnostics(module, &pystat));
    let abi_diags = if allow_abi_break {
        Vec::new()
    } else if let Some(path) = manifest_path {
        match mode {
            VerifyMode::OnDisk => check_abi_guard(path, &manifest).unwrap_or_default(),
            VerifyMode::Baseline => load_baseline_manifest(path)
                .map(|baseline| find_baseline_drift(&baseline, &manifest))
                .unwrap_or_else(|e| {
                    vec![PystatDiagnostic::t003(e, None)]
                }),
        }
    } else {
        Vec::new()
    };
    VerifyReport {
        pystat,
        manifest,
        type_diags,
        abi_diags,
    }
}

fn codegen_slot_diagnostics(module: &Module, pystat: &PystatReport) -> Vec<PystatDiagnostic> {
    let funcs = module_func_map(module);
    let mut diags = Vec::new();
    for stat in &pystat.module.functions {
        let Some(func) = funcs.get(&stat.symbol.0) else {
            continue;
        };
        let manifest_slot = slot_level_str(stat);
        let Some(tier) = tier_for(stat, func) else {
            continue;
        };
        let codegen_slot = tier_slot_label(tier);
        if manifest_slot != codegen_slot {
            diags.push(PystatDiagnostic::t005(
                format!(
                    "manifest slot {manifest_slot} inconsistent with codegen tier {codegen_slot}"
                ),
                Some(stat.symbol.0.clone()),
            ));
        }
    }
    diags
}

fn tier_slot_label(tier: CodegenTier) -> &'static str {
    match tier {
        CodegenTier::S0 | CodegenTier::ClassMethod => "S0",
        CodegenTier::S1 | CodegenTier::Closure => "S1",
        CodegenTier::S3 => "S3",
    }
}

fn module_func_map(module: &Module) -> HashMap<String, &FuncDef> {
    let mut map = HashMap::new();
    for f in &module.functions {
        map.insert(f.symbol.0.clone(), f);
        for nested in &f.nested {
            map.insert(nested.symbol.0.clone(), nested);
        }
    }
    for class in &module.classes {
        for method in &class.methods {
            map.insert(method.symbol.0.clone(), method);
        }
    }
    map
}
