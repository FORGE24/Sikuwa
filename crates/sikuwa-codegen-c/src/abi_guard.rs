//! SKW-T003 — compare emitted manifest against on-disk ABI snapshot.

use std::path::Path;

use sikuwa_core::{Result, SikuwaError};
use sikuwa_pystat::PystatDiagnostic;

use crate::manifest::{load_manifest, SkwExport, SkwManifest};

pub fn check_abi_guard(
    previous_path: &Path,
    current: &SkwManifest,
) -> Result<Vec<PystatDiagnostic>> {
    let previous = match load_manifest(previous_path) {
        Ok(m) => m,
        Err(_) => return Ok(Vec::new()),
    };
    Ok(find_abi_breaks(&previous, current))
}

/// Incremental guard: skip when `source_hash` unchanged (same build artifact).
pub fn find_abi_breaks(previous: &SkwManifest, current: &SkwManifest) -> Vec<PystatDiagnostic> {
    if previous.source_hash == current.source_hash {
        return Vec::new();
    }
    if previous.module != current.module {
        return Vec::new();
    }

    let mut diags = Vec::new();
    for prev in &previous.exports {
        let Some(next) = current.exports.iter().find(|e| e.c_symbol == prev.c_symbol) else {
            continue;
        };
        if let Some(msg) = export_abi_diff(prev, next) {
            diags.push(PystatDiagnostic::t003(
                msg,
                Some(prev.symbol.clone()),
            ));
        }
    }
    diags
}

/// CI baseline: compare against committed golden manifest (ignores `source_hash`).
pub fn find_baseline_drift(expected: &SkwManifest, actual: &SkwManifest) -> Vec<PystatDiagnostic> {
    if expected.module != actual.module {
        return vec![PystatDiagnostic::t003(
            format!(
                "module name `{}` != baseline `{}`",
                actual.module, expected.module
            ),
            None,
        )];
    }

    let mut diags = Vec::new();
    for exp in &expected.exports {
        let Some(act) = actual.exports.iter().find(|e| e.c_symbol == exp.c_symbol) else {
            diags.push(PystatDiagnostic::t003(
                format!(
                    "missing export `{}` (c_symbol `{}`) in current manifest",
                    exp.symbol, exp.c_symbol
                ),
                Some(exp.symbol.clone()),
            ));
            continue;
        };
        if let Some(msg) = export_abi_diff(exp, act) {
            diags.push(PystatDiagnostic::t003(
                format!("baseline drift: {msg}"),
                Some(exp.symbol.clone()),
            ));
        } else if exp.abi_breaking != act.abi_breaking {
            diags.push(PystatDiagnostic::t003(
                format!(
                    "abi_breaking flag mismatch for `{}`: expected {}",
                    exp.symbol, exp.abi_breaking
                ),
                Some(exp.symbol.clone()),
            ));
        }
    }

    for act in &actual.exports {
        if !expected
            .exports
            .iter()
            .any(|e| e.c_symbol == act.c_symbol)
        {
            diags.push(PystatDiagnostic::t003(
                format!(
                    "unexpected export `{}` (c_symbol `{}`) not in baseline",
                    act.symbol, act.c_symbol
                ),
                Some(act.symbol.clone()),
            ));
        }
    }
    diags
}

pub fn export_abi_diff(prev: &SkwExport, next: &SkwExport) -> Option<String> {
    let mut reasons = Vec::new();
    if prev.slot != next.slot {
        reasons.push(format!("slot {} → {}", prev.slot, next.slot));
    }
    if prev.signature.return_type != next.signature.return_type {
        reasons.push(format!(
            "return type `{}` → `{}`",
            prev.signature.return_type, next.signature.return_type
        ));
    }
    if prev.signature.params.len() != next.signature.params.len() {
        reasons.push(format!(
            "param count {} → {}",
            prev.signature.params.len(),
            next.signature.params.len()
        ));
    } else {
        for (p0, p1) in prev.signature.params.iter().zip(&next.signature.params) {
            if p0.ty != p1.ty {
                reasons.push(format!(
                    "param `{}` type `{}` → `{}`",
                    p0.name, p0.ty, p1.ty
                ));
            }
        }
    }
    if reasons.is_empty() {
        None
    } else {
        Some(format!(
            "ABI break for `{}` (c_symbol `{}`): {}",
            prev.symbol,
            prev.c_symbol,
            reasons.join("; ")
        ))
    }
}

/// Mark exports whose ABI differs from `previous` with `abi_breaking: true`.
pub fn annotate_abi_breaks(mut manifest: SkwManifest, previous: &SkwManifest) -> SkwManifest {
    for export in &mut manifest.exports {
        let Some(prev) = previous
            .exports
            .iter()
            .find(|e| e.c_symbol == export.c_symbol)
        else {
            continue;
        };
        if export_abi_diff(prev, export).is_some() {
            export.abi_breaking = true;
        }
    }
    manifest
}

pub fn format_abi_errors(diags: &[PystatDiagnostic]) -> String {
    diags
        .iter()
        .map(PystatDiagnostic::format_line)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn abi_guard_error(diags: &[PystatDiagnostic]) -> SikuwaError {
    SikuwaError::pystat(format!(
        "{}\n(use --allow-abi-break to override)",
        format_abi_errors(diags)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{SkwParam, SkwSignature};

    fn sample_export(slot: &str, ret: &str) -> SkwExport {
        SkwExport {
            symbol: "m.add".into(),
            c_symbol: "skw_m_add".into(),
            slot: slot.into(),
            signature: SkwSignature {
                params: vec![SkwParam {
                    name: "a".into(),
                    ty: "int64".into(),
                }],
                return_type: ret.into(),
            },
            static_eligible: true,
            abi_breaking: false,
        }
    }

    #[test]
    fn same_hash_skips_check() {
        let prev = SkwManifest {
            manifest_version: "1.0".into(),
            abi: "1".into(),
            module: "m".into(),
            source_hash: "abc".into(),
            exports: vec![sample_export("S0", "int64")],
            imports: vec![],
            itr_slots: vec![],
            tagged_slots: vec![],
        };
        let mut next = prev.clone();
        next.source_hash = "abc".into();
        assert!(find_abi_breaks(&prev, &next).is_empty());
    }

    #[test]
    fn slot_change_is_breaking() {
        let prev = SkwManifest {
            manifest_version: "1.0".into(),
            abi: "1".into(),
            module: "m".into(),
            source_hash: "old".into(),
            exports: vec![sample_export("S0", "int64")],
            imports: vec![],
            itr_slots: vec![],
            tagged_slots: vec![],
        };
        let mut next = prev.clone();
        next.source_hash = "new".into();
        next.exports[0].slot = "S3".into();
        let diags = find_abi_breaks(&prev, &next);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "SKW-T003");
    }

    #[test]
    fn baseline_detects_slot_drift() {
        let mut expected = sample_export("S0", "int64");
        let golden = SkwManifest {
            manifest_version: "1.0".into(),
            abi: "1".into(),
            module: "m".into(),
            source_hash: "golden".into(),
            exports: vec![expected.clone()],
            imports: vec![],
            itr_slots: vec![],
            tagged_slots: vec![],
        };
        expected.slot = "S3".into();
        expected.abi_breaking = true;
        let actual = SkwManifest {
            source_hash: "live".into(),
            exports: vec![expected],
            ..golden.clone()
        };
        let diags = find_baseline_drift(&golden, &actual);
        assert!(!diags.is_empty());
    }

    #[test]
    fn annotate_sets_abi_breaking_flag() {
        let prev = SkwManifest {
            manifest_version: "1.0".into(),
            abi: "1".into(),
            module: "m".into(),
            source_hash: "old".into(),
            exports: vec![sample_export("S0", "int64")],
            imports: vec![],
            itr_slots: vec![],
            tagged_slots: vec![],
        };
        let mut next = prev.clone();
        next.source_hash = "new".into();
        next.exports[0].slot = "S3".into();
        let annotated = annotate_abi_breaks(next, &prev);
        assert!(annotated.exports[0].abi_breaking);
    }
}
