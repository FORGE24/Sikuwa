//! Pass2 — local inference post-checks (strict static eligibility).

use crate::config::{PystatMode, PystatOptions};
use crate::diagnostic::PystatDiagnostic;
use crate::types::{FuncStat, SlotLevel};

/// After local inference + Pass5 floor, enforce `mode = strict`.
pub fn pass2_diagnostics(func: &FuncStat, opts: &PystatOptions) -> Vec<PystatDiagnostic> {
    if opts.mode != PystatMode::Strict {
        return Vec::new();
    }
    if func.static_eligible {
        return Vec::new();
    }
    let reasons = strict_failure_reasons(func);
    vec![PystatDiagnostic::t002(
        format!(
            "cannot staticize under strict mode ({})",
            reasons.join("; ")
        ),
        Some(func.symbol.0.clone()),
    )]
}

fn strict_failure_reasons(func: &FuncStat) -> Vec<String> {
    let mut out = Vec::new();
    if !func.return_ty.bit_width().is_some()
        || matches!(
            func.return_ty,
            crate::types::PhysicalType::Dyn
                | crate::types::PhysicalType::Unknown
                | crate::types::PhysicalType::Object
        )
    {
        out.push(format!("return type {:?}", func.return_ty));
    }
    for slot in func.params.iter().chain(func.locals.iter()) {
        if slot.level == SlotLevel::S3 {
            out.push(format!("slot `{}` is S3", slot.name));
        } else if slot.level != SlotLevel::S0 {
            out.push(format!("slot `{}` is {:?}", slot.name, slot.level));
        }
    }
    if out.is_empty() {
        out.push("not static_eligible".into());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        FuncStat, LogicalSlot, PhysicalType, SlotLevel, SlotStrategy,
    };
    use sikuwa_pir::ids::SymbolRef;

    fn sample_dyn_func() -> FuncStat {
        FuncStat {
            symbol: SymbolRef::new("m.f"),
            params: vec![LogicalSlot {
                name: "x".into(),
                ty: PhysicalType::Dyn,
                strategy: SlotStrategy::Dyn,
                level: SlotLevel::S3,
                tagged: None,
            }],
            locals: vec![],
            return_ty: PhysicalType::Dyn,
            static_eligible: false,
        }
    }

    #[test]
    fn strict_emits_t002_for_dyn() {
        let diags = pass2_diagnostics(&sample_dyn_func(), &PystatOptions::strict());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "SKW-T002");
    }

    #[test]
    fn progressive_skips_t002() {
        let diags = pass2_diagnostics(&sample_dyn_func(), &PystatOptions::default());
        assert!(diags.is_empty());
    }
}
