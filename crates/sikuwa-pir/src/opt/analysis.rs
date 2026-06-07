//! PIR analysis helpers for optimization passes (no AST dependency).

use std::collections::{HashMap, HashSet};

use crate::ids::{BlockId, ValueId};
use crate::module::{Block, ConstValue, FuncDef, Op, OpOperand, Terminator};

/// Known constant value for an SSA virtual register.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstInfo {
    pub value: ConstValue,
}

/// Use-count of each SSA value within a function.
pub fn count_uses(func: &FuncDef) -> HashMap<ValueId, usize> {
    let mut counts = HashMap::new();
    let bump = |counts: &mut HashMap<ValueId, usize>, v: ValueId| {
        *counts.entry(v).or_insert(0) += 1;
    };

    for block in &func.blocks {
        for phi in &block.phis {
            for inc in &phi.incoming {
                bump(&mut counts, inc.value);
            }
        }
        for op in &block.ops {
            for opnd in &op.operands {
                if let OpOperand::Value(v) = opnd {
                    bump(&mut counts, *v);
                }
            }
        }
        match &block.term {
            Terminator::CondBranch { cond, .. } => bump(&mut counts, *cond),
            Terminator::Return { value: Some(v) } => bump(&mut counts, *v),
            _ => {}
        }
    }
    counts
}

/// Forward constant propagation over SSA (single function, fixed-point on phis).
pub fn const_map(func: &FuncDef) -> HashMap<ValueId, ConstInfo> {
    let mut map = HashMap::new();

    loop {
        let before = map.len();
        for block in &func.blocks {
            for phi in &block.phis {
                if map.contains_key(&phi.result) {
                    continue;
                }
                if phi.incoming.is_empty() {
                    continue;
                }
                let first: Option<&ConstInfo> = map.get(&phi.incoming[0].value);
                if first.is_some()
                    && phi
                        .incoming
                        .iter()
                        .all(|inc| map.get(&inc.value) == first)
                {
                    if let Some(info) = first {
                        map.insert(phi.result, info.clone());
                    }
                }
            }

            for op in &block.ops {
                if let Some(result) = op.result {
                    if map.contains_key(&result) {
                        continue;
                    }
                    if let Some(info) = eval_const_op(op, &map) {
                        map.insert(result, info);
                    }
                }
            }
        }
        if map.len() == before {
            break;
        }
    }
    map
}

/// Evaluate whether an op produces a known constant given current const map.
pub(crate) fn eval_const_op_for_fold(
    op: &Op,
    map: &HashMap<ValueId, ConstInfo>,
) -> Option<ConstInfo> {
    eval_const_op(op, map)
}

fn eval_const_op(op: &Op, map: &HashMap<ValueId, ConstInfo>) -> Option<ConstInfo> {
    match op.opcode {
        crate::opcode::OpCode::Const => {
            if let Some(OpOperand::Const(c)) = op.operands.first() {
                Some(ConstInfo { value: c.clone() })
            } else {
                None
            }
        }
        crate::opcode::OpCode::UnaryNot => {
            let v = value_const(&op.operands, 0, map)?;
            match v {
                ConstValue::Bool(b) => Some(ConstInfo {
                    value: ConstValue::Bool(!b),
                }),
                _ => None,
            }
        }
        crate::opcode::OpCode::BinOpAdd => fold_binop_i64(&op.operands, map, |a, b| a.wrapping_add(b)),
        crate::opcode::OpCode::BinOpSub => fold_binop_i64(&op.operands, map, |a, b| a.wrapping_sub(b)),
        crate::opcode::OpCode::BinOpMul => fold_binop_i64(&op.operands, map, |a, b| a.wrapping_mul(b)),
        crate::opcode::OpCode::CompareEq => fold_cmp(&op.operands, map, |a, b| a == b),
        crate::opcode::OpCode::CompareNe => fold_cmp(&op.operands, map, |a, b| a != b),
        crate::opcode::OpCode::CompareLt => fold_cmp(&op.operands, map, |a, b| a < b),
        crate::opcode::OpCode::CompareLe => fold_cmp(&op.operands, map, |a, b| a <= b),
        crate::opcode::OpCode::CompareGt => fold_cmp(&op.operands, map, |a, b| a > b),
        crate::opcode::OpCode::CompareGe => fold_cmp(&op.operands, map, |a, b| a >= b),
        crate::opcode::OpCode::CompareIs | crate::opcode::OpCode::CompareIsNot => {
            fold_is(&op.operands, map, op.opcode)
        }
        _ => None,
    }
}

fn value_const(
    operands: &[OpOperand],
    idx: usize,
    map: &HashMap<ValueId, ConstInfo>,
) -> Option<ConstValue> {
    match operands.get(idx)? {
        OpOperand::Const(c) => Some(c.clone()),
        OpOperand::Value(v) => map.get(v).map(|i| i.value.clone()),
        _ => None,
    }
}

fn fold_binop_i64(
    operands: &[OpOperand],
    map: &HashMap<ValueId, ConstInfo>,
    f: fn(i64, i64) -> i64,
) -> Option<ConstInfo> {
    let a = value_const(operands, 0, map)?;
    let b = value_const(operands, 1, map)?;
    match (a, b) {
        (ConstValue::Int(x), ConstValue::Int(y)) => Some(ConstInfo {
            value: ConstValue::Int(f(x, y)),
        }),
        _ => None,
    }
}

fn fold_cmp(
    operands: &[OpOperand],
    map: &HashMap<ValueId, ConstInfo>,
    f: fn(i64, i64) -> bool,
) -> Option<ConstInfo> {
    let a = value_const(operands, 0, map)?;
    let b = value_const(operands, 1, map)?;
    match (a, b) {
        (ConstValue::Int(x), ConstValue::Int(y)) => Some(ConstInfo {
            value: ConstValue::Bool(f(x, y)),
        }),
        (ConstValue::Bool(x), ConstValue::Bool(y)) => Some(ConstInfo {
            value: ConstValue::Bool(f(x as i64, y as i64)),
        }),
        _ => None,
    }
}

fn fold_is(
    operands: &[OpOperand],
    map: &HashMap<ValueId, ConstInfo>,
    opcode: crate::opcode::OpCode,
) -> Option<ConstInfo> {
    let a = value_const(operands, 0, map)?;
    let b = value_const(operands, 1, map)?;
    let eq = a == b;
    let result = match opcode {
        crate::opcode::OpCode::CompareIs => eq,
        crate::opcode::OpCode::CompareIsNot => !eq,
        _ => return None,
    };
    Some(ConstInfo {
        value: ConstValue::Bool(result),
    })
}

/// Blocks reachable from `^entry` via terminators.
pub fn reachable_blocks(func: &FuncDef) -> HashSet<BlockId> {
    let mut seen = HashSet::new();
    let mut stack = vec![BlockId::entry()];
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some(block) = func.blocks.iter().find(|b| b.id == id) {
            for succ in successor_blocks(&block.term) {
                if !seen.contains(&succ) {
                    stack.push(succ);
                }
            }
        }
    }
    seen
}

pub fn successor_blocks(term: &Terminator) -> Vec<BlockId> {
    match term {
        Terminator::Branch { target } => vec![target.clone()],
        Terminator::CondBranch {
            then_block,
            else_block,
            ..
        } => vec![then_block.clone(), else_block.clone()],
        _ => Vec::new(),
    }
}

pub fn replace_value_uses(func: &mut FuncDef, old: ValueId, new: ValueId) {
    if old == new {
        return;
    }
    for block in &mut func.blocks {
        for phi in &mut block.phis {
            if phi.result == old {
                continue;
            }
            for inc in &mut phi.incoming {
                if inc.value == old {
                    inc.value = new;
                }
            }
        }
        for op in &mut block.ops {
            if op.result == Some(old) {
                continue;
            }
            for opnd in &mut op.operands {
                if let OpOperand::Value(v) = opnd {
                    if *v == old {
                        *v = new;
                    }
                }
            }
        }
        match &mut block.term {
            Terminator::CondBranch { cond, .. } if *cond == old => *cond = new,
            Terminator::Return { value: Some(v) } if *v == old => *v = new,
            _ => {}
        }
    }
}

// Reserved for future CFG queries.
#[allow(dead_code)]
pub fn block_by_id<'a>(func: &'a FuncDef, id: &BlockId) -> Option<&'a Block> {
    func.blocks.iter().find(|b| &b.id == id)
}
