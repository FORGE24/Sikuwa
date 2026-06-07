//! Pass5 — downgrade decisions (`min_slot` floor, dyn fallback, profile check).

use crate::config::{MinSlot, PystatMode, PystatOptions};
use crate::diagnostic::PystatDiagnostic;
use crate::types::{FuncStat, LogicalSlot, PhysicalType, SlotLevel, SlotStrategy, TaggedLayout};

pub fn pass5_finalize_func(
    mut func: FuncStat,
    opts: &PystatOptions,
) -> (FuncStat, Vec<PystatDiagnostic>) {
    let mut diags = Vec::new();
    let floor = opts.min_slot.floor_level();

    func.params = func
        .params
        .drain(..)
        .map(|s| apply_min_slot(s, floor, opts))
        .collect();
    func.locals = func
        .locals
        .drain(..)
        .map(|s| apply_min_slot(s, floor, opts))
        .collect();

    func.return_ty = apply_min_return(func.return_ty, floor);

    func.static_eligible = func.params.iter().chain(func.locals.iter()).all(|s| s.level == SlotLevel::S0)
        && func.return_ty.bit_width().is_some()
        && !matches!(
            func.return_ty,
            PhysicalType::Dyn | PhysicalType::Unknown | PhysicalType::Object
        );

    if opts.mode == PystatMode::Strict
        && !opts.allow_dyn_fallback
        && func.params.iter().chain(func.locals.iter()).any(|s| s.level == SlotLevel::S3)
    {
        diags.push(PystatDiagnostic::t005(
            "strict profile disallows S3 slots without dyn fallback",
            Some(func.symbol.0.clone()),
        ));
    }

    if opts.mode == PystatMode::Strict
        && opts.min_slot == MinSlot::Static
        && !func.static_eligible
        && diags.is_empty()
    {
        // T005: configured min static but analysis produced non-S0
        diags.push(PystatDiagnostic::t005(
            "min_slot=static but function is not S0-static",
            Some(func.symbol.0.clone()),
        ));
    }

    (func, diags)
}

fn apply_min_slot(mut slot: LogicalSlot, floor: SlotLevel, opts: &PystatOptions) -> LogicalSlot {
    if slot_level_ord(slot.level) >= slot_level_ord(floor) {
        return slot;
    }
    slot.level = floor;
    match floor {
        SlotLevel::S0 => {}
        SlotLevel::S1 => {
            if slot.tagged.is_none() {
                slot.tagged = Some(TaggedLayout {
                    arms: vec![physical_arm_name(slot.ty).unwrap_or_else(|| "dyn".into())],
                });
            }
            slot.strategy = SlotStrategy::Dyn;
            slot.ty = PhysicalType::Dyn;
        }
        SlotLevel::S2 | SlotLevel::S3 => {
            slot.strategy = SlotStrategy::Dyn;
            slot.ty = PhysicalType::Dyn;
            slot.tagged = None;
        }
    }
    if slot.level == SlotLevel::S3 && !opts.allow_dyn_fallback && opts.mode != PystatMode::Compat {
        // progressive: allowed; strict handled in pass2/t005
    }
    slot
}

fn apply_min_return(ty: PhysicalType, floor: SlotLevel) -> PhysicalType {
    if floor == SlotLevel::S0 || ty.bit_width().is_some() && !matches!(ty, PhysicalType::Dyn) {
        return ty;
    }
    if floor == SlotLevel::S1 && ty.bit_width().is_some() {
        return ty;
    }
    if matches!(ty, PhysicalType::Dyn | PhysicalType::Object | PhysicalType::Unknown) {
        return ty;
    }
    if slot_level_ord(floor) >= slot_level_ord(SlotLevel::S1) {
        PhysicalType::Dyn
    } else {
        ty
    }
}

fn physical_arm_name(ty: PhysicalType) -> Option<String> {
    Some(match ty {
        PhysicalType::Int64 | PhysicalType::Bool => "int64".into(),
        PhysicalType::Float64 => "float64".into(),
        PhysicalType::Str => "str".into(),
        PhysicalType::None => "none".into(),
        _ => return None,
    })
}

fn slot_level_ord(l: SlotLevel) -> u8 {
    match l {
        SlotLevel::S0 => 0,
        SlotLevel::S1 => 1,
        SlotLevel::S2 => 2,
        SlotLevel::S3 => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LogicalSlot, SlotStrategy};
    use sikuwa_pir::ids::SymbolRef;

    #[test]
    fn min_slot_tagged_raises_s0_local() {
        let func = FuncStat {
            symbol: SymbolRef::new("m.f"),
            params: vec![LogicalSlot {
                name: "a".into(),
                ty: PhysicalType::Int64,
                strategy: SlotStrategy::Itr {
                    primary: PhysicalType::Int64,
                },
                level: SlotLevel::S0,
                tagged: None,
            }],
            locals: vec![],
            return_ty: PhysicalType::Int64,
            static_eligible: true,
        };
        let mut opts = PystatOptions::default();
        opts.min_slot = MinSlot::Tagged;
        let (out, _) = pass5_finalize_func(func, &opts);
        assert_eq!(out.params[0].level, SlotLevel::S1);
        assert!(!out.static_eligible);
    }
}
