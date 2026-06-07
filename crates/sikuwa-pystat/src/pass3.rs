//! Pass3 — flow-sensitive narrow + dynamic-op diagnostics (SKW-T004).

use std::collections::{HashMap, VecDeque};

use sikuwa_pir::ids::{BlockId, ValueId};
use sikuwa_pir::module::{ConstValue, ExternDecl, FuncDef, Op, OpOperand, Terminator};
use sikuwa_pir::opcode::OpCode;

use crate::config::{MinSlot, PystatMode, PystatOptions};
use crate::diagnostic::PystatDiagnostic;
use crate::infer::{
    join, meet, normalize_union, FuncSummary, LogicalType, SparseEnvironment, UNION_CAP,
};

/// Branch guard extracted from `CompareEq` / `CompareIs` feeding a conditional.
#[derive(Debug, Clone)]
enum BranchGuard {
    EqName { name: String, ty: LogicalType },
    IsNone { name: String },
}

#[derive(Clone)]
struct FrameState {
    slots: SparseEnvironment,
    values: HashMap<u32, LogicalType>,
}

pub fn pass3_dyn_diagnostics(
    func: &FuncDef,
    opts: &PystatOptions,
    has_dyn_ops: bool,
) -> Vec<PystatDiagnostic> {
    if !has_dyn_ops {
        return Vec::new();
    }
    if opts.mode != PystatMode::Strict && opts.min_slot != MinSlot::Static {
        return Vec::new();
    }
    vec![PystatDiagnostic::t004(
        "dynamic IR opcode forces dyn slot (attribute/subscript/unknown call/etc.)",
        Some(func.symbol.0.clone()),
    )]
}

/// CFG walk with edge narrowing; mutates `slots` and `value_types`.
pub fn run_func_body_cfg(
    func: &FuncDef,
    slots: &mut SparseEnvironment,
    value_types: &mut HashMap<u32, LogicalType>,
    summaries: &HashMap<String, FuncSummary>,
    externs: &HashMap<String, ExternDecl>,
    infer_op: impl Fn(
        &FuncDef,
        &Op,
        &HashMap<u32, LogicalType>,
        &mut SparseEnvironment,
        &HashMap<String, FuncSummary>,
        &HashMap<String, ExternDecl>,
    ) -> LogicalType,
) {
    let block_idx: HashMap<BlockId, usize> = func
        .blocks
        .iter()
        .enumerate()
        .map(|(i, b)| (b.id.clone(), i))
        .collect();

    let mut in_states: HashMap<BlockId, FrameState> = HashMap::new();
    let mut finished: HashMap<BlockId, FrameState> = HashMap::new();
    let mut work: VecDeque<BlockId> = VecDeque::new();

    in_states.insert(
        BlockId::entry(),
        FrameState {
            slots: slots.clone(),
            values: value_types.clone(),
        },
    );
    work.push_back(BlockId::entry());

    while let Some(bid) = work.pop_front() {
        let idx = match block_idx.get(&bid) {
            Some(i) => *i,
            None => continue,
        };
        let block = &func.blocks[idx];
        // Keep `in_states` entries for loop headers; removing them causes back-edges
        // to re-insert the same block forever (e.g. `while` in sum_range.py).
        let mut state = in_states.get(&bid).cloned().unwrap_or_else(|| FrameState {
            slots: slots.clone(),
            values: value_types.clone(),
        });

        for phi in &block.phis {
            let incoming: Vec<LogicalType> = phi
                .incoming
                .iter()
                .filter_map(|inc| state.values.get(&inc.value.0).cloned())
                .collect();
            state.slots.merge_phi(&phi.name, incoming);
            state
                .values
                .insert(phi.result.0, state.slots.get(&phi.name));
        }

        for op in &block.ops {
            let ty = infer_op(
                func,
                op,
                &state.values,
                &mut state.slots,
                summaries,
                externs,
            );
            if let Some(result) = op.result {
                state.values.insert(result.0, ty);
            }
            if matches!(op.opcode, OpCode::StoreFast | OpCode::StoreCell) {
                if let Some(OpOperand::Name(name)) = op.operands.first() {
                    if let Some(OpOperand::Value(v)) = op.operands.get(1) {
                        if let Some(t) = state.values.get(&v.0) {
                            state.slots.join_slot(name.clone(), t.clone());
                        }
                    }
                }
            }
        }

        match &block.term {
            Terminator::Branch { target } => {
                enqueue(&mut in_states, &mut work, target, state);
            }
            Terminator::CondBranch {
                cond,
                then_block,
                else_block,
            } => {
                let (guard, then_polarity) = decode_branch_guard(block, *cond, &state.values);
                let mut then_state = state.clone();
                let mut else_state = state;
                if let Some(g) = guard {
                    apply_guard_narrow(&mut then_state.slots, &g, then_polarity);
                    apply_guard_narrow(&mut else_state.slots, &g, !then_polarity);
                }
                enqueue(&mut in_states, &mut work, then_block, then_state);
                enqueue(&mut in_states, &mut work, else_block, else_state);
            }
            Terminator::Return { .. } | Terminator::Unreachable => {
                merge_finished(&mut finished, bid, state);
            }
        }
    }

    let mut merged_slots = SparseEnvironment::new();
    let mut merged_values = HashMap::new();
    for state in finished.values().chain(in_states.values()) {
        for (name, ty) in state.slots.iter() {
            merged_slots.join_slot(name.to_string(), ty.clone());
        }
        for (vid, ty) in &state.values {
            merged_values
                .entry(*vid)
                .and_modify(|e: &mut LogicalType| *e = join(e.clone(), ty.clone()))
                .or_insert_with(|| ty.clone());
        }
    }
    if merged_slots.iter().next().is_some() || !merged_values.is_empty() {
        *slots = merged_slots;
        *value_types = merged_values;
    }
}

fn merge_finished(finished: &mut HashMap<BlockId, FrameState>, bid: BlockId, state: FrameState) {
    match finished.get_mut(&bid) {
        Some(existing) => merge_frame(existing, state),
        None => {
            finished.insert(bid, state);
        }
    }
}

fn enqueue(
    in_states: &mut HashMap<BlockId, FrameState>,
    work: &mut VecDeque<BlockId>,
    target: &BlockId,
    state: FrameState,
) {
    match in_states.get_mut(target) {
        Some(existing) => {
            let before = existing.clone();
            merge_frame(existing, state);
            if !frame_equal(&before, existing) {
                work.push_back(target.clone());
            }
        }
        None => {
            in_states.insert(target.clone(), state);
            work.push_back(target.clone());
        }
    }
}

fn frame_equal(a: &FrameState, b: &FrameState) -> bool {
    a.slots.iter().eq(b.slots.iter()) && a.values == b.values
}

fn merge_frame(dst: &mut FrameState, src: FrameState) {
    for (name, ty) in src.slots.iter() {
        dst.slots.join_slot(name.to_string(), ty.clone());
    }
    for (vid, ty) in src.values {
        dst.values
            .entry(vid)
            .and_modify(|e| *e = join(e.clone(), ty.clone()))
            .or_insert(ty);
    }
}

fn decode_branch_guard(
    block: &sikuwa_pir::module::Block,
    cond: ValueId,
    _values: &HashMap<u32, LogicalType>,
) -> (Option<BranchGuard>, bool) {
    let mut polarity = true;
    let mut cid = cond;
    if let Some(op) = block.ops.iter().find(|o| o.result == Some(cid)) {
        if op.opcode == OpCode::UnaryNot {
            polarity = false;
            cid = match op.operands.first() {
                Some(OpOperand::Value(v)) => *v,
                _ => return (None, polarity),
            };
        }
    }
    let cmp = match block.ops.iter().find(|o| o.result == Some(cid)) {
        Some(o) => o,
        None => return (None, polarity),
    };
    match cmp.opcode {
        OpCode::CompareEq => {
            let (name, ty) = match eq_operands(&cmp.operands) {
                Some(v) => v,
                None => return (None, polarity),
            };
            (Some(BranchGuard::EqName { name, ty }), polarity)
        }
        OpCode::CompareIs => {
            let name = match is_none_operands(&cmp.operands) {
                Some(n) => n,
                None => return (None, polarity),
            };
            (Some(BranchGuard::IsNone { name }), polarity)
        }
        _ => (None, polarity),
    }
}

fn eq_operands(ops: &[OpOperand]) -> Option<(String, LogicalType)> {
    let (left, right) = (ops.first()?, ops.get(1)?);
    match (left, right) {
        (OpOperand::Name(n), OpOperand::Const(c)) => Some((n.clone(), const_lt(c))),
        (OpOperand::Const(c), OpOperand::Name(n)) => Some((n.clone(), const_lt(c))),
        _ => None,
    }
}

fn is_none_operands(ops: &[OpOperand]) -> Option<String> {
    let none_side = |o: &OpOperand| matches!(o, OpOperand::Const(ConstValue::None));
    match (ops.first()?, ops.get(1)?) {
        (OpOperand::Name(n), rhs) if none_side(rhs) => Some(n.clone()),
        (lhs, OpOperand::Name(n)) if none_side(lhs) => Some(n.clone()),
        _ => None,
    }
}

fn const_lt(c: &ConstValue) -> LogicalType {
    match c {
        ConstValue::Int(v) => LogicalType::Literal(crate::infer::LiteralValue::Int(*v)),
        ConstValue::Bool(v) => LogicalType::Literal(crate::infer::LiteralValue::Bool(*v)),
        ConstValue::Float(v) => {
            LogicalType::Literal(crate::infer::LiteralValue::Float(v.to_bits()))
        }
        ConstValue::Str(_) => LogicalType::Str,
        ConstValue::None => LogicalType::None,
    }
}

fn apply_guard_narrow(slots: &mut SparseEnvironment, guard: &BranchGuard, when_true: bool) {
    match guard {
        BranchGuard::EqName { name, ty } => {
            if when_true {
                let prev = slots.get(name);
                slots.set_exact(name.clone(), meet(prev, ty.clone()));
            }
        }
        BranchGuard::IsNone { name } => {
            let prev = slots.get(name);
            if when_true {
                slots.set_exact(name.clone(), meet(prev, LogicalType::None));
            } else {
                slots.set_exact(name.clone(), narrow_not_none(prev));
            }
        }
    }
}

fn narrow_not_none(ty: LogicalType) -> LogicalType {
    match ty {
        LogicalType::Optional(inner) => *inner,
        LogicalType::Union(arms) => {
            let filtered: Vec<_> = arms
                .into_iter()
                .filter(|a| *a != LogicalType::None)
                .collect();
            normalize_union(filtered, UNION_CAP)
        }
        LogicalType::None => LogicalType::Bottom,
        other => other,
    }
}

pub fn func_has_dyn_ops(func: &FuncDef) -> bool {
    func.blocks.iter().flat_map(|b| &b.ops).any(|op| {
        matches!(
            op.opcode,
            OpCode::LoadGlobal
                | OpCode::LoadAttr
                | OpCode::StoreAttr
                | OpCode::SubscriptLoad
                | OpCode::SubscriptStore
                | OpCode::BuildTuple
                | OpCode::BuildList
                | OpCode::BuildMap
                | OpCode::CallIndirect
                | OpCode::CallBuiltin
                | OpCode::MakeClosure
                | OpCode::BuildClass
                | OpCode::GetIter
                | OpCode::ForIterNext
        ) || (op.opcode == OpCode::Call && resolve_call(func, op).is_none())
    })
}

fn resolve_call(func: &FuncDef, op: &Op) -> Option<sikuwa_pir::ids::SymbolRef> {
    if op.opcode != OpCode::Call {
        return None;
    }
    match op.operands.first()? {
        OpOperand::Symbol(s) => Some(s.clone()),
        OpOperand::Name(n) => {
            let prefix = func.symbol.0.rsplit_once('.').map(|(m, _)| m).unwrap_or("");
            Some(sikuwa_pir::ids::SymbolRef::new(format!("{prefix}.{n}")))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze_module;
    use std::collections::HashMap;

    #[test]
    fn sum_range_while_loop_converges() {
        let path = format!(
            "{}/../../tests/fixtures/sum_range.py",
            env!("CARGO_MANIFEST_DIR")
        );
        let m = sikuwa_pir::lower_file(std::path::Path::new(&path)).unwrap();
        let report = analyze_module(&m);
        assert_eq!(report.module.functions.len(), 1);
    }

    #[test]
    fn cfg_worklist_terminates_on_self_branch() {
        use sikuwa_pir::ids::{BlockId, SymbolRef, ValueId};
        use sikuwa_pir::module::{Block, FuncDef, Terminator};
        use sikuwa_pir::span::Span;

        let func = FuncDef {
            symbol: SymbolRef::new("t.loop"),
            params: vec!["i".into()],
            locals: vec![],
            cellvars: vec![],
            nested: vec![],
            return_value: None,
            blocks: vec![Block {
                id: BlockId::entry(),
                phis: vec![],
                ops: vec![],
                term: Terminator::Branch {
                    target: BlockId::entry(),
                },
            }],
            span: Span::single_line("t.py", 1),
            exception_regions: vec![],
        };
        let mut slots = SparseEnvironment::new();
        slots.seed("i");
        let mut values = HashMap::new();
        run_func_body_cfg(
            &func,
            &mut slots,
            &mut values,
            &HashMap::new(),
            &HashMap::new(),
            |_, _, _, _, _, _| LogicalType::Int,
        );
    }
}
