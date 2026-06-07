//! Same-module `def` inlining on PIR (AST-free).

use std::collections::HashMap;

use crate::ids::{SymbolRef, ValueId};
use crate::module::{FuncDef, Module, Op, OpOperand, Terminator};
use crate::opcode::OpCode;

const MAX_INLINE_BLOCKS: usize = 1;

struct InlineAction {
    block_idx: usize,
    op_idx: usize,
    inlined_ops: Vec<Op>,
    ret_replace: Option<(ValueId, ValueId)>,
}

/// Inline eligible same-module calls across all functions.
pub fn pass_def_inline(module: &mut Module) -> bool {
    let callees: HashMap<String, FuncDef> = module
        .functions
        .iter()
        .map(|f| (f.symbol.0.clone(), f.clone()))
        .collect();
    let by_short: HashMap<String, SymbolRef> = module
        .functions
        .iter()
        .map(|f| {
            let short = f.symbol.0.rsplit('.').next().unwrap_or(&f.symbol.0).to_string();
            (short, f.symbol.clone())
        })
        .collect();

    let mut changed = false;
    for func in &mut module.functions {
        if inline_in_function(func, &callees, &by_short, &module.name) {
            changed = true;
        }
        for nested in &mut func.nested {
            if inline_in_function(nested, &callees, &by_short, &module.name) {
                changed = true;
            }
        }
    }
    for class in &mut module.classes {
        for method in &mut class.methods {
            if inline_in_function(method, &callees, &by_short, &module.name) {
                changed = true;
            }
        }
    }
    changed
}

fn inline_in_function(
    func: &mut FuncDef,
    callees: &HashMap<String, FuncDef>,
    by_short: &HashMap<String, SymbolRef>,
    module_name: &str,
) -> bool {
    let mut actions = Vec::new();
    for (block_idx, block) in func.blocks.iter().enumerate() {
        for (op_idx, op) in block.ops.iter().enumerate() {
            if op.opcode != OpCode::Call {
                continue;
            }
            let Some(callee_sym) = resolve_callee(&op.operands, by_short, module_name) else {
                continue;
            };
            let Some(callee) = callees.get(&callee_sym.0) else {
                continue;
            };
            if !is_inline_candidate(callee, &func.symbol) {
                continue;
            }
            let Some(call_result) = op.result else {
                continue;
            };
            let args: Vec<ValueId> = op.operands[1..]
                .iter()
                .filter_map(|o| match o {
                    OpOperand::Value(v) => Some(*v),
                    _ => None,
                })
                .collect();
            if args.len() != callee.params.len() {
                continue;
            }
            let param_map: HashMap<String, ValueId> = callee
                .params
                .iter()
                .cloned()
                .zip(args)
                .collect();
            let next_id = max_value_id(func);
            let Some((inlined_ops, ret_val)) =
                clone_callee_body(callee, &param_map, call_result, next_id)
            else {
                continue;
            };
            let ret_replace = ret_val.map(|from| (from, call_result));
            actions.push(InlineAction {
                block_idx,
                op_idx,
                inlined_ops,
                ret_replace,
            });
        }
    }

    if actions.is_empty() {
        return false;
    }

    let mut replacements = Vec::new();
    for action in actions.into_iter().rev() {
        let block = &mut func.blocks[action.block_idx];
        block.ops.remove(action.op_idx);
        for (j, inlined) in action.inlined_ops.into_iter().enumerate() {
            block.ops.insert(action.op_idx + j, inlined);
        }
        if let Some((from, to)) = action.ret_replace {
            replacements.push((from, to));
        }
    }
    for (from, to) in replacements {
        super::analysis::replace_value_uses(func, from, to);
    }
    true
}

fn max_value_id(func: &FuncDef) -> u32 {
    let mut max = 0u32;
    for block in &func.blocks {
        for phi in &block.phis {
            max = max.max(phi.result.0);
        }
        for op in &block.ops {
            if let Some(r) = op.result {
                max = max.max(r.0);
            }
        }
    }
    max + 1
}

fn resolve_callee(
    operands: &[OpOperand],
    by_short: &HashMap<String, SymbolRef>,
    module_name: &str,
) -> Option<SymbolRef> {
    match operands.first()? {
        OpOperand::Symbol(s) => Some(s.clone()),
        OpOperand::Name(n) => {
            if by_short.contains_key(n) {
                Some(SymbolRef::new(format!("{module_name}.{n}")))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_inline_candidate(callee: &FuncDef, caller_sym: &SymbolRef) -> bool {
    if callee.symbol == *caller_sym {
        return false;
    }
    callee.blocks.len() <= MAX_INLINE_BLOCKS
        && callee.nested.is_empty()
        && callee.cellvars.is_empty()
        && !callee.params.is_empty()
}

fn clone_callee_body(
    callee: &FuncDef,
    param_map: &HashMap<String, ValueId>,
    call_result: ValueId,
    mut next_id: u32,
) -> Option<(Vec<Op>, Option<ValueId>)> {
    let entry = callee.blocks.first()?;
    let mut id_remap = HashMap::new();
    let mut out = Vec::new();

    for op in &entry.ops {
        if op.opcode == OpCode::LoadFast {
            let name = match op.operands.first()? {
                OpOperand::Name(n) => n,
                _ => return None,
            };
            if let Some(&arg) = param_map.get(name) {
                if let Some(result) = op.result {
                    id_remap.insert(result, arg);
                }
                continue;
            }
        }
        let mut cloned = op.clone();
        if let Some(result) = cloned.result {
            cloned.result = Some(remap_value(result, &mut id_remap, &mut next_id));
        }
        cloned.operands = cloned
            .operands
            .iter()
            .map(|o| remap_operand(o, param_map, &id_remap))
            .collect();
        out.push(cloned);
    }

    let ret_val = match &entry.term {
        Terminator::Return { value: Some(v) } => {
            let mapped = *id_remap.get(v).unwrap_or(v);
            if mapped != call_result {
                Some(mapped)
            } else {
                None
            }
        }
        _ => return None,
    };
    Some((out, ret_val))
}

fn remap_value(
    old: ValueId,
    remap: &mut HashMap<ValueId, ValueId>,
    next_id: &mut u32,
) -> ValueId {
    if let Some(&n) = remap.get(&old) {
        return n;
    }
    let n = ValueId(*next_id);
    *next_id += 1;
    remap.insert(old, n);
    n
}

fn remap_operand(
    op: &OpOperand,
    param_map: &HashMap<String, ValueId>,
    remap: &HashMap<ValueId, ValueId>,
) -> OpOperand {
    match op {
        OpOperand::Value(v) => OpOperand::Value(*remap.get(v).unwrap_or(v)),
        OpOperand::Name(n) => {
            if let Some(&v) = param_map.get(n) {
                OpOperand::Value(v)
            } else {
                op.clone()
            }
        }
        _ => op.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::lower_source;
    use crate::opcode::OpCode;

    #[test]
    fn inlines_same_module_add() {
        let src = r#"def add(a, b):
    return a + b

def twice(x, y):
    return add(x, y)
"#;
        let mut module = lower_source(src, "inline.py").unwrap();
        assert!(pass_def_inline(&mut module));
        let twice = module
            .functions
            .iter()
            .find(|f| f.symbol.0.ends_with("twice"))
            .unwrap();
        let has_call = twice
            .blocks
            .iter()
            .flat_map(|b| &b.ops)
            .any(|o| o.opcode == OpCode::Call);
        assert!(!has_call);
        assert!(twice
            .blocks
            .iter()
            .flat_map(|b| &b.ops)
            .any(|o| o.opcode == OpCode::BinOpAdd));
    }
}
