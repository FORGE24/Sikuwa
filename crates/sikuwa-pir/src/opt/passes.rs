//! Keyword-scoped PIR optimization passes (AST-free).

use crate::ids::{BlockId, ValueId};
use crate::module::{ConstValue, FuncDef, Op, OpOperand, Terminator};
use crate::opcode::OpCode;

use super::analysis::{const_map, count_uses, reachable_blocks, replace_value_uses, ConstInfo};
use super::keyword::PythonKeyword;

pub fn run_keyword_pass(kw: PythonKeyword, func: &mut FuncDef) -> bool {
    match kw {
        PythonKeyword::False | PythonKeyword::NoneKw | PythonKeyword::True => pass_const_lit(func),
        PythonKeyword::Not => pass_not(func),
        PythonKeyword::And | PythonKeyword::Or => pass_short_circuit(func),
        PythonKeyword::If | PythonKeyword::Elif | PythonKeyword::Else => pass_cfg_simplify(func),
        PythonKeyword::While | PythonKeyword::For => pass_loop_simplify(func),
        PythonKeyword::Break | PythonKeyword::Continue => pass_loop_exit(func),
        PythonKeyword::Return => pass_return_merge(func),
        PythonKeyword::Pass => false,
        PythonKeyword::Del => pass_dce(func),
        PythonKeyword::Is | PythonKeyword::In => pass_compare_fold(func),
        PythonKeyword::Def | PythonKeyword::Lambda => false, // module-level inline
        PythonKeyword::Class => pass_class_simplify(func),
        PythonKeyword::Import | PythonKeyword::From => false, // module-level import DCE
        PythonKeyword::Global | PythonKeyword::Nonlocal => pass_global_promote(func),
        PythonKeyword::Assert | PythonKeyword::Raise => pass_exception_prune(func),
        PythonKeyword::Try | PythonKeyword::Except | PythonKeyword::Finally => {
            pass_exception_prune(func)
        }
        PythonKeyword::Async | PythonKeyword::Await => false,
        PythonKeyword::With | PythonKeyword::Yield => false,
        PythonKeyword::As => pass_as_cleanup(func),
    }
}

// --- Implemented passes ---

fn pass_const_lit(func: &mut FuncDef) -> bool {
    pass_const_fold_ops(func)
}

fn pass_not(func: &mut FuncDef) -> bool {
    let mut changed = pass_const_fold_ops(func);
    changed |= eliminate_double_not(func);
    changed
}

fn pass_short_circuit(func: &mut FuncDef) -> bool {
    pass_cfg_simplify(func)
}

fn pass_compare_fold(func: &mut FuncDef) -> bool {
    pass_const_fold_ops(func)
}

fn pass_const_fold_ops(func: &mut FuncDef) -> bool {
    let mut changed = false;
    loop {
        let cmap = const_map(func);
        let mut round = false;
        for block in &mut func.blocks {
            for op in &mut block.ops {
                let Some(result) = op.result else { continue };
                if matches!(op.opcode, OpCode::Const) {
                    continue;
                }
                if let Some(info) = super::analysis::eval_const_op_for_fold(op, &cmap) {
                    *op = Op {
                        opcode: OpCode::Const,
                        result: Some(result),
                        operands: vec![OpOperand::Const(info.value)],
                        span: op.span.clone(),
                    };
                    round = true;
                }
            }
        }
        if !round {
            break;
        }
        changed = true;
    }
    changed
}

fn eliminate_double_not(func: &mut FuncDef) -> bool {
    let mut changed = false;
    let mut replacements = Vec::new();

    for block in &func.blocks {
        for op in &block.ops {
            if op.opcode != OpCode::UnaryNot {
                continue;
            }
            let Some(inner) = op.operands.first() else { continue };
            let OpOperand::Value(v) = inner else { continue };
            if let Some(def_block) = find_def_op(func, *v) {
                if def_block.opcode == OpCode::UnaryNot {
                    if let Some(OpOperand::Value(orig)) = def_block.operands.first() {
                        if let Some(result) = op.result {
                            replacements.push((result, *orig));
                        }
                    }
                }
            }
        }
    }

    for (old, new) in replacements {
        replace_value_uses(func, old, new);
        changed = true;
    }
    changed
}

fn find_def_op<'a>(func: &'a FuncDef, value: ValueId) -> Option<&'a Op> {
    for block in &func.blocks {
        for op in &block.ops {
            if op.result == Some(value) {
                return Some(op);
            }
        }
    }
    None
}

fn pass_cfg_simplify(func: &mut FuncDef) -> bool {
    let mut changed = false;
    loop {
        let cmap = const_map(func);
        let mut folded = false;

        for block in &mut func.blocks {
            if let Terminator::CondBranch {
                cond,
                then_block,
                else_block,
            } = &block.term
            {
                if let Some(ConstInfo {
                    value: ConstValue::Bool(b),
                }) = cmap.get(cond)
                {
                    block.term = Terminator::Branch {
                        target: if *b {
                            then_block.clone()
                        } else {
                            else_block.clone()
                        },
                    };
                    folded = true;
                }
            }
        }

        if !folded {
            break;
        }
        changed = true;
    }

    if remove_unreachable_blocks(func) {
        changed = true;
    }
    if simplify_phi(func) {
        changed = true;
    }
    changed
}

fn pass_loop_simplify(func: &mut FuncDef) -> bool {
    pass_cfg_simplify(func)
}

fn pass_loop_exit(_func: &mut FuncDef) -> bool {
    false
}

fn pass_return_merge(func: &mut FuncDef) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < func.blocks.len() {
        let block = &func.blocks[i];
        if block.ops.is_empty()
            && block.phis.is_empty()
            && matches!(block.term, Terminator::Return { .. })
        {
            let ret = block.term.clone();
            let dupes: Vec<usize> = func
                .blocks
                .iter()
                .enumerate()
                .filter(|(j, b)| *j != i && b.ops.is_empty() && b.phis.is_empty() && b.term == ret)
                .map(|(j, _)| j)
                .collect();

            if !dupes.is_empty() {
                let id = block.id.clone();
                for j in dupes.into_iter().rev() {
                    let dead_id = func.blocks[j].id.clone();
                    redirect_branches(func, &dead_id, &id);
                    func.blocks.remove(j);
                    if j < i {
                        i -= 1;
                    }
                }
                changed = true;
            }
        }
        i += 1;
    }
    changed
}

fn redirect_branches(func: &mut FuncDef, from: &BlockId, to: &BlockId) {
    for block in &mut func.blocks {
        match &mut block.term {
            Terminator::Branch { target } if target == from => *target = to.clone(),
            Terminator::CondBranch {
                then_block,
                else_block,
                ..
            } => {
                if then_block == from {
                    *then_block = to.clone();
                }
                if else_block == from {
                    *else_block = to.clone();
                }
            }
            _ => {}
        }
        for phi in &mut block.phis {
            for inc in &mut phi.incoming {
                if inc.block == *from {
                    inc.block = to.clone();
                }
            }
        }
    }
}

fn remove_unreachable_blocks(func: &mut FuncDef) -> bool {
    let reachable = reachable_blocks(func);
    let before = func.blocks.len();
    func.blocks.retain(|b| reachable.contains(&b.id));
    if func.blocks.len() == before {
        return false;
    }
    let live: std::collections::HashSet<_> = func.blocks.iter().map(|b| b.id.clone()).collect();
    for block in &mut func.blocks {
        block.phis.retain_mut(|phi| {
            phi.incoming.retain(|inc| live.contains(&inc.block));
            !phi.incoming.is_empty()
        });
    }
    true
}

fn simplify_phi(func: &mut FuncDef) -> bool {
    let mut changed = false;
    let mut replacements = Vec::new();

    for block in &func.blocks {
        for phi in &block.phis {
            if phi.incoming.len() == 1 {
                replacements.push((phi.result, phi.incoming[0].value));
                continue;
            }
            if phi.incoming.len() >= 2 {
                let first = phi.incoming[0].value;
                if phi.incoming.iter().all(|inc| inc.value == first) {
                    replacements.push((phi.result, first));
                }
            }
        }
    }

    for (old, new) in replacements {
        replace_value_uses(func, old, new);
        changed = true;
    }

    if changed {
        for block in &mut func.blocks {
            block.phis.retain(|phi| {
                if phi.incoming.len() == 1 {
                    return false;
                }
                if phi.incoming.len() >= 2 {
                    let first = phi.incoming[0].value;
                    if phi.incoming.iter().all(|inc| inc.value == first) {
                        return false;
                    }
                }
                true
            });
        }
    }
    changed
}

fn pass_dce(func: &mut FuncDef) -> bool {
    let uses = count_uses(func);
    let mut changed = false;

    for block in &mut func.blocks {
        block.ops.retain(|op| {
            if op_has_side_effect(op) {
                return true;
            }
            match op.result {
                Some(r) => uses.get(&r).copied().unwrap_or(0) > 0,
                None => true,
            }
        });
    }

    changed |= simplify_phi(func);
    changed
}

fn op_has_side_effect(op: &Op) -> bool {
    matches!(
        op.opcode,
        OpCode::StoreFast
            | OpCode::StoreGlobal
            | OpCode::StoreAttr
            | OpCode::SubscriptStore
            | OpCode::StoreCell
            | OpCode::Call
            | OpCode::CallBuiltin
            | OpCode::CallExtern
            | OpCode::MakeClosure
            | OpCode::BuildClass
            | OpCode::DebugSloc
    )
}

// --- O2 passes ---

fn pass_class_simplify(_func: &mut FuncDef) -> bool {
    false
}

fn pass_global_promote(_func: &mut FuncDef) -> bool {
    false
}

fn pass_exception_prune(func: &mut FuncDef) -> bool {
    if func.exception_regions.is_empty() {
        return false;
    }
    let mut remove_regions = Vec::new();
    let mut remove_blocks = std::collections::HashSet::new();

    for (idx, region) in func.exception_regions.iter().enumerate() {
        if region_cannot_raise(func, region) {
            for h in &region.handlers {
                remove_blocks.insert(h.clone());
            }
            if let Some(f) = &region.finally {
                remove_blocks.insert(f.clone());
            }
            remove_regions.push(idx);
        }
    }

    if remove_regions.is_empty() {
        return false;
    }

    for idx in remove_regions.into_iter().rev() {
        func.exception_regions.remove(idx);
    }
    func.blocks.retain(|b| !remove_blocks.contains(&b.id));
    true
}

fn region_cannot_raise(func: &FuncDef, region: &crate::module::ExceptionRegion) -> bool {
    for bid in &region.protected {
        let Some(block) = func.blocks.iter().find(|b| &b.id == bid) else {
            continue;
        };
        for op in &block.ops {
            if op_may_raise(op) {
                return false;
            }
        }
    }
    true
}

fn op_may_raise(op: &Op) -> bool {
    matches!(
        op.opcode,
        OpCode::Call
            | OpCode::CallBuiltin
            | OpCode::CallExtern
            | OpCode::LoadAttr
            | OpCode::StoreAttr
            | OpCode::SubscriptLoad
            | OpCode::SubscriptStore
            | OpCode::GetIter
            | OpCode::ForIterNext
            | OpCode::BuildClass
            | OpCode::MakeClosure
    )
}

fn pass_as_cleanup(func: &mut FuncDef) -> bool {
    let mut block_removals: Vec<Vec<usize>> = Vec::new();
    let mut replacements = Vec::new();

    for block in &func.blocks {
        let mut bindings: std::collections::HashMap<String, ValueId> =
            std::collections::HashMap::new();
        let mut remove = Vec::new();

        for (i, op) in block.ops.iter().enumerate() {
            match op.opcode {
                OpCode::StoreFast => {
                    if let (Some(OpOperand::Name(n)), Some(OpOperand::Value(v))) =
                        (op.operands.first(), op.operands.get(1))
                    {
                        bindings.insert(n.clone(), *v);
                    }
                }
                OpCode::LoadFast => {
                    let Some(OpOperand::Name(n)) = op.operands.first() else {
                        continue;
                    };
                    let Some(result) = op.result else {
                        continue;
                    };
                    if let Some(&src) = bindings.get(n) {
                        if src != result {
                            replacements.push((result, src));
                        }
                        remove.push(i);
                    }
                }
                _ => {}
            }
        }
        block_removals.push(remove);
    }

    if replacements.is_empty() && !block_removals.iter().any(|r| !r.is_empty()) {
        return false;
    }

    for (old, new) in replacements {
        replace_value_uses(func, old, new);
    }
    for (block, remove) in func.blocks.iter_mut().zip(block_removals) {
        for idx in remove.into_iter().rev() {
            block.ops.remove(idx);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{BlockId, SymbolRef, ValueId};
    use crate::lower::lower_source;
    use crate::module::{Block, FuncDef, Op, OpOperand, Terminator};
    use crate::opcode::OpCode;
    use crate::span::Span;
    use crate::verify::verify_func;

    fn fold_true_branch() -> FuncDef {
        let then_b = BlockId::new("then_1");
        let else_b = BlockId::new("else_2");
        FuncDef {
            symbol: SymbolRef::new("t.f"),
            params: vec![],
            locals: vec![],
            cellvars: vec![],
            nested: vec![],
            return_value: Some(ValueId(2)),
            blocks: vec![
                Block {
                    id: BlockId::entry(),
                    phis: vec![],
                    ops: vec![
                        Op {
                            opcode: OpCode::Const,
                            result: Some(ValueId(0)),
                            operands: vec![OpOperand::Const(ConstValue::Bool(true))],
                            span: Span::unknown(),
                        },
                        Op {
                            opcode: OpCode::Const,
                            result: Some(ValueId(1)),
                            operands: vec![OpOperand::Const(ConstValue::Int(42))],
                            span: Span::unknown(),
                        },
                        Op {
                            opcode: OpCode::Const,
                            result: Some(ValueId(2)),
                            operands: vec![OpOperand::Const(ConstValue::Int(0))],
                            span: Span::unknown(),
                        },
                    ],
                    term: Terminator::CondBranch {
                        cond: ValueId(0),
                        then_block: then_b.clone(),
                        else_block: else_b,
                    },
                },
                Block {
                    id: then_b,
                    phis: vec![],
                    ops: vec![],
                    term: Terminator::Return {
                        value: Some(ValueId(1)),
                    },
                },
            ],
            span: Span::unknown(),
            exception_regions: Vec::new(),
        }
    }

    #[test]
    fn fold_const_if_removes_dead_else() {
        let mut func = fold_true_branch();
        assert!(pass_cfg_simplify(&mut func));
        assert_eq!(func.blocks.len(), 2);
        assert!(matches!(
            func.blocks[0].term,
            Terminator::Branch { .. }
        ));
        let report = verify_func(&func);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
    }

    #[test]
    fn const_fold_add_in_place() {
        let src = "def add(a, b):\n    return 1 + 2\n";
        let module = lower_source(src, "f.py").unwrap();
        let mut func = module.functions[0].clone();
        assert!(pass_const_lit(&mut func));
        let entry = &func.blocks[0];
        assert!(entry.ops.iter().any(|op| {
            op.opcode == OpCode::Const
                && op.operands
                    == vec![OpOperand::Const(ConstValue::Int(3))]
        }));
    }

    #[test]
    fn prune_bare_except_when_try_cannot_raise() {
        let src = r#"def safe():
    try:
        return 1 + 2
    except:
        return 0
"#;
        let module = lower_source(src, "safe.py").unwrap();
        let mut func = module.functions[0].clone();
        assert_eq!(func.blocks.len(), 2);
        assert!(!func.exception_regions.is_empty());
        assert!(pass_exception_prune(&mut func));
        assert_eq!(func.blocks.len(), 1);
        assert!(func.exception_regions.is_empty());
    }

    #[test]
    fn optimize_module_pipeline() {
        let src = "def f():\n    if True:\n        return 10\n    return 0\n";
        let mut module = lower_source(src, "f.py").unwrap();
        let report = crate::opt::optimize_module(&mut module, crate::opt::OptLevel::O1);
        assert!(report.changed_passes() > 0);
        let func = &module.functions[0];
        assert!(func.blocks.len() <= 2);
    }
}
