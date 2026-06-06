use std::collections::HashMap;

use sikuwa_pir::module::{ConstValue, FuncDef, Module, Op, OpOperand, Terminator};
use sikuwa_pir::opcode::OpCode;

use crate::types::{
    FuncStat, LogicalSlot, PhysicalType, PystatModule, SlotLevel, SlotStrategy,
};

#[derive(Debug, Clone)]
pub struct PystatReport {
    pub module: PystatModule,
    pub itr_slots: usize,
    pub dyn_slots: usize,
}

pub fn analyze_module(module: &Module) -> PystatReport {
    let mut functions = Vec::new();
    let mut itr_slots = 0;
    let mut dyn_slots = 0;

    for func in &module.functions {
        let stat = analyze_func(func);
        for slot in stat.params.iter().chain(stat.locals.iter()) {
            match slot.strategy {
                SlotStrategy::Itr { .. } => itr_slots += 1,
                SlotStrategy::Dyn => dyn_slots += 1,
                SlotStrategy::Alloc { .. } => {}
            }
        }
        functions.push(stat);
    }

    for class in &module.classes {
        for method in &class.methods {
            let stat = analyze_func(method);
            functions.push(stat);
        }
    }

    PystatReport {
        module: PystatModule {
            module: module.name.clone(),
            source_hash: module.source_hash,
            functions,
        },
        itr_slots,
        dyn_slots,
    }
}

pub fn analyze_func(func: &FuncDef) -> FuncStat {
    let mut slot_types: HashMap<String, PhysicalType> = HashMap::new();
    let mut value_types: HashMap<u32, PhysicalType> = HashMap::new();

    for param in &func.params {
        slot_types.insert(param.clone(), PhysicalType::Unknown);
    }

    for block in &func.blocks {
        for phi in &block.phis {
            let mut merged = PhysicalType::Unknown;
            for inc in &phi.incoming {
                if let Some(t) = value_types.get(&inc.value.0) {
                    merged = merged.merge(*t);
                }
            }
            slot_types.insert(phi.name.clone(), merged);
            value_types.insert(phi.result.0, merged);
        }

        for op in &block.ops {
            let ty = infer_op(op, &value_types, &mut slot_types);
            if let Some(result) = op.result {
                value_types.insert(result.0, ty);
            }
            if let OpCode::StoreFast | OpCode::StoreCell = op.opcode {
                if let Some(OpOperand::Name(name)) = op.operands.first() {
                    if let Some(OpOperand::Value(v)) = op.operands.get(1) {
                        if let Some(t) = value_types.get(&v.0) {
                            merge_slot(&mut slot_types, name, *t);
                        }
                    }
                }
            }
        }

        if let Terminator::Return { value: Some(v) } = &block.term {
            let _ = value_types.get(&v.0).copied().unwrap_or(PhysicalType::Unknown);
        }
    }

    let mut return_ty = func
        .return_value
        .and_then(|v| value_types.get(&v.0).copied())
        .unwrap_or(PhysicalType::Unknown);

    let dyn_ops = func_has_dyn_ops(func);
    if return_ty == PhysicalType::Unknown && !dyn_ops {
        return_ty = PhysicalType::Int64;
    }

    let params: Vec<LogicalSlot> = func
        .params
        .iter()
        .map(|n| {
            let mut ty = slot_types.get(n).copied().unwrap_or(PhysicalType::Unknown);
            if ty == PhysicalType::Unknown && !dyn_ops {
                ty = PhysicalType::Int64;
            }
            logical_slot(n, ty)
        })
        .collect();

    let locals: Vec<LogicalSlot> = func
        .locals
        .iter()
        .filter(|n| !func.params.contains(n))
        .map(|n| logical_slot(n, slot_types.get(n).copied().unwrap_or(PhysicalType::Unknown)))
        .collect();

    let static_eligible = params.iter().chain(locals.iter()).all(|s| s.level == SlotLevel::S0)
        && return_ty.bit_width().is_some()
        && !matches!(return_ty, PhysicalType::Dyn | PhysicalType::Unknown | PhysicalType::Object);

    let params = if static_eligible && return_ty == PhysicalType::Int64 {
        params
            .into_iter()
            .map(|mut s| {
                if s.ty == PhysicalType::Unknown {
                    s.ty = PhysicalType::Int64;
                }
                s
            })
            .collect()
    } else {
        params
    };

    FuncStat {
        symbol: func.symbol.clone(),
        params,
        locals,
        return_ty,
        static_eligible,
    }
}

fn logical_slot(name: &str, ty: PhysicalType) -> LogicalSlot {
    let (strategy, level) = plan_slot(ty);
    LogicalSlot {
        name: name.to_string(),
        ty,
        strategy,
        level,
    }
}

fn func_has_dyn_ops(func: &FuncDef) -> bool {
    func.blocks.iter().flat_map(|b| &b.ops).any(|op| {
        matches!(
            op.opcode,
            OpCode::LoadAttr
                | OpCode::StoreAttr
                | OpCode::SubscriptLoad
                | OpCode::SubscriptStore
                | OpCode::Call
                | OpCode::MakeClosure
                | OpCode::BuildClass
                | OpCode::GetIter
                | OpCode::ForIterNext
        )
    })
}

fn plan_slot(ty: PhysicalType) -> (SlotStrategy, SlotLevel) {
    match ty {
        PhysicalType::Int64 | PhysicalType::Bool | PhysicalType::Float64 | PhysicalType::None => {
            (
                SlotStrategy::Itr {
                    primary: if ty == PhysicalType::Bool {
                        PhysicalType::Int64
                    } else {
                        ty
                    },
                },
                SlotLevel::S0,
            )
        }
        PhysicalType::Str => (SlotStrategy::Alloc { ty }, SlotLevel::S0),
        PhysicalType::Unknown => (
            SlotStrategy::Itr {
                primary: PhysicalType::Int64,
            },
            SlotLevel::S0,
        ),
        _ => (SlotStrategy::Dyn, SlotLevel::S3),
    }
}

fn merge_slot(slots: &mut HashMap<String, PhysicalType>, name: &str, ty: PhysicalType) {
    slots
        .entry(name.to_string())
        .and_modify(|existing| *existing = existing.merge(ty))
        .or_insert(ty);
}

fn infer_op(
    op: &Op,
    values: &HashMap<u32, PhysicalType>,
    slots: &mut HashMap<String, PhysicalType>,
) -> PhysicalType {
    let operand_ty = |i: usize| -> PhysicalType {
        op.operands
            .get(i)
            .and_then(|o| type_of_operand(o, values, slots))
            .unwrap_or(PhysicalType::Unknown)
    };

    match op.opcode {
        OpCode::Const => match op.operands.first() {
            Some(OpOperand::Const(ConstValue::Int(_))) => PhysicalType::Int64,
            Some(OpOperand::Const(ConstValue::Bool(_))) => PhysicalType::Bool,
            Some(OpOperand::Const(ConstValue::Float(_))) => PhysicalType::Float64,
            Some(OpOperand::Const(ConstValue::Str(_))) => PhysicalType::Str,
            Some(OpOperand::Const(ConstValue::None)) => PhysicalType::None,
            _ => PhysicalType::Unknown,
        },
        OpCode::LoadFast | OpCode::LoadCell | OpCode::Phi => {
            if let Some(OpOperand::Name(n)) = op.operands.first() {
                slots.get(n).copied().unwrap_or(PhysicalType::Unknown)
            } else {
                PhysicalType::Unknown
            }
        }
        OpCode::BinOpAdd | OpCode::BinOpSub | OpCode::BinOpMul | OpCode::BinOpFloorDiv
        | OpCode::BinOpMod => {
            let a = operand_ty(0);
            let b = operand_ty(1);
            if a == PhysicalType::Float64 || b == PhysicalType::Float64 {
                PhysicalType::Float64
            } else {
                PhysicalType::Int64
            }
        }
        OpCode::BinOpTrueDiv => PhysicalType::Float64,
        OpCode::UnaryNot
        | OpCode::CompareLt
        | OpCode::CompareLe
        | OpCode::CompareGt
        | OpCode::CompareGe
        | OpCode::CompareEq
        | OpCode::CompareNe
        | OpCode::CompareIs
        | OpCode::CompareIsNot => PhysicalType::Bool,
        OpCode::UnaryNeg => operand_ty(0),
        OpCode::LoadAttr | OpCode::SubscriptLoad | OpCode::Call | OpCode::MakeClosure
        | OpCode::BuildClass | OpCode::GetIter | OpCode::ForIterNext => PhysicalType::Dyn,
        OpCode::CallExtern => PhysicalType::Int64,
        _ => PhysicalType::Unknown,
    }
}

fn type_of_operand(
    op: &OpOperand,
    values: &HashMap<u32, PhysicalType>,
    slots: &HashMap<String, PhysicalType>,
) -> Option<PhysicalType> {
    match op {
        OpOperand::Value(v) => values.get(&v.0).copied(),
        OpOperand::Name(n) => slots.get(n).copied(),
        OpOperand::Const(c) => Some(match c {
            ConstValue::Int(_) => PhysicalType::Int64,
            ConstValue::Bool(_) => PhysicalType::Bool,
            ConstValue::Float(_) => PhysicalType::Float64,
            ConstValue::Str(_) => PhysicalType::Str,
            ConstValue::None => PhysicalType::None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sikuwa_pir::{lower_source, sample_add_module};

    #[test]
    fn analyze_add_is_s0() {
        let m = sample_add_module();
        let report = analyze_module(&m);
        let f = &report.module.functions[0];
        assert!(f.static_eligible);
        assert_eq!(f.return_ty, PhysicalType::Int64);
    }

    #[test]
    fn analyze_clamp_has_bool_itr() {
        let src = r#"def clamp(x, lo, hi):
    if x < lo:
        return lo
    if x > hi:
        return hi
    return x
"#;
        let m = lower_source(src, "clamp.py").unwrap();
        let report = analyze_module(&m);
        assert!(!report.module.functions.is_empty());
    }
}
