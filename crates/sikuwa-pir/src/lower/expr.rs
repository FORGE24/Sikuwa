//! Lower Python expressions to PIR operands / SSA values.

use rustpython_ast as ast;

use sikuwa_core::{Result, SikuwaError};

use crate::ids::ValueId;
use crate::module::{ConstValue, Op, OpOperand};
use crate::opcode::OpCode;
use crate::span::Span;

use super::function::FunctionLowerer;

pub enum LowerExprResult {
    Value(ValueId),
    Const(ConstValue),
    Name(String),
}

impl LowerExprResult {
    pub fn into_operand(self) -> OpOperand {
        match self {
            Self::Value(v) => OpOperand::Value(v),
            Self::Const(c) => OpOperand::Const(c),
            Self::Name(n) => OpOperand::Name(n),
        }
    }

    pub fn into_value(self, lowerer: &mut FunctionLowerer) -> Result<ValueId> {
        match self {
            Self::Value(v) => Ok(v),
            Self::Const(c) => {
                let result = lowerer.fresh_value();
                lowerer.emit(Op {
                    opcode: OpCode::Const,
                    result: Some(result),
                    operands: vec![OpOperand::Const(c)],
                    span: Span::unknown(),
                });
                Ok(result)
            }
            Self::Name(name) => lowerer.load_local(&name, Span::unknown()),
        }
    }
}

pub fn lower_expr(lowerer: &mut FunctionLowerer, expr: &ast::Expr) -> Result<LowerExprResult> {
    let span = lowerer.span_from(expr);

    match expr {
        ast::Expr::Name(name) => {
            let n = name.id.to_string();
            if lowerer.locals.contains_key(&n)
                || lowerer.params.iter().any(|p| p == &n)
                || lowerer.free_cells.contains(&n)
            {
                let v = lowerer.load_local(&n, span)?;
                Ok(LowerExprResult::Value(v))
            } else {
                let result = lowerer.fresh_value();
                lowerer.emit(Op {
                    opcode: OpCode::LoadGlobal,
                    result: Some(result),
                    operands: vec![OpOperand::Symbol(crate::ids::SymbolRef::new(format!(
                        "{}.{n}",
                        lowerer.module_name
                    )))],
                    span,
                });
                Ok(LowerExprResult::Value(result))
            }
        }
        ast::Expr::Constant(c) => Ok(LowerExprResult::Const(constant_to_pir(&c.value)?)),
        ast::Expr::BinOp(binop) => {
            lowerer.lower_binop(binop.op, &binop.left, &binop.right, span)
        }
        ast::Expr::Compare(cmp) => {
            lowerer.lower_compare(&cmp.left, &cmp.ops, &cmp.comparators, span)
        }
        ast::Expr::BoolOp(bo) => {
            if bo.values.is_empty() {
                return Err(SikuwaError::pir("empty boolop"));
            }
            let mut acc = lower_expr(lowerer, &bo.values[0])?.into_value(lowerer)?;
            for val in &bo.values[1..] {
                let next = lower_expr(lowerer, val)?.into_value(lowerer)?;
                let result = lowerer.fresh_value();
                let opcode = match bo.op {
                    ast::BoolOp::And => OpCode::BinOpBitAnd,
                    ast::BoolOp::Or => {
                        return Err(SikuwaError::pir("bool or not supported yet"))
                    }
                };
                lowerer.emit(Op {
                    opcode,
                    result: Some(result),
                    operands: vec![OpOperand::Value(acc), OpOperand::Value(next)],
                    span: span.clone(),
                });
                acc = result;
            }
            Ok(LowerExprResult::Value(acc))
        }
        ast::Expr::Call(call) => lowerer.lower_call(call, span),
        ast::Expr::UnaryOp(u) => {
            let opcode = match u.op {
                ast::UnaryOp::Not => OpCode::UnaryNot,
                ast::UnaryOp::USub => OpCode::UnaryNeg,
                other => {
                    return Err(SikuwaError::pir(format!(
                        "unsupported unary op: {other:?}"
                    )))
                }
            };
            let inner = lower_expr(lowerer, &u.operand)?.into_value(lowerer)?;
            let result = lowerer.fresh_value();
            lowerer.emit(Op {
                opcode,
                result: Some(result),
                operands: vec![OpOperand::Value(inner)],
                span,
            });
            Ok(LowerExprResult::Value(result))
        }
        ast::Expr::Attribute(attr) => {
            let obj = lower_expr(lowerer, &attr.value)?.into_value(lowerer)?;
            let result = lowerer.fresh_value();
            lowerer.emit(Op {
                opcode: OpCode::LoadAttr,
                result: Some(result),
                operands: vec![
                    OpOperand::Value(obj),
                    OpOperand::Name(attr.attr.to_string()),
                ],
                span,
            });
            Ok(LowerExprResult::Value(result))
        }
        ast::Expr::Subscript(sub) => {
            let obj = lower_expr(lowerer, &sub.value)?.into_value(lowerer)?;
            let key = lower_expr(lowerer, &sub.slice)?.into_value(lowerer)?;
            let result = lowerer.fresh_value();
            lowerer.emit(Op {
                opcode: OpCode::SubscriptLoad,
                result: Some(result),
                operands: vec![OpOperand::Value(obj), OpOperand::Value(key)],
                span,
            });
            Ok(LowerExprResult::Value(result))
        }
        ast::Expr::Tuple(tup) => {
            let mut operands = Vec::with_capacity(tup.elts.len());
            for elt in &tup.elts {
                let v = lower_expr(lowerer, elt)?.into_value(lowerer)?;
                operands.push(OpOperand::Value(v));
            }
            let result = lowerer.fresh_value();
            lowerer.emit(Op {
                opcode: OpCode::BuildTuple,
                result: Some(result),
                operands,
                span,
            });
            Ok(LowerExprResult::Value(result))
        }
        ast::Expr::List(lst) => {
            let mut operands = Vec::with_capacity(lst.elts.len());
            for elt in &lst.elts {
                let v = lower_expr(lowerer, elt)?.into_value(lowerer)?;
                operands.push(OpOperand::Value(v));
            }
            let result = lowerer.fresh_value();
            lowerer.emit(Op {
                opcode: OpCode::BuildList,
                result: Some(result),
                operands,
                span,
            });
            Ok(LowerExprResult::Value(result))
        }
        ast::Expr::Dict(dict) => {
            let mut operands = Vec::with_capacity(dict.keys.len() * 2);
            for (key, val) in dict.keys.iter().zip(dict.values.iter()) {
                let key_expr = key
                    .as_ref()
                    .ok_or_else(|| SikuwaError::pir("dict unpack not supported yet"))?;
                let k = lower_expr(lowerer, key_expr)?.into_value(lowerer)?;
                let v = lower_expr(lowerer, val)?.into_value(lowerer)?;
                operands.push(OpOperand::Value(k));
                operands.push(OpOperand::Value(v));
            }
            let result = lowerer.fresh_value();
            lowerer.emit(Op {
                opcode: OpCode::BuildMap,
                result: Some(result),
                operands,
                span,
            });
            Ok(LowerExprResult::Value(result))
        }
        ast::Expr::JoinedStr(js) => lower_joined_str(lowerer, js, span),
        other => Err(SikuwaError::pir(format!(
            "unsupported expression: {other:?}"
        ))),
    }
}

fn lower_joined_str(
    lowerer: &mut FunctionLowerer,
    js: &ast::ExprJoinedStr,
    span: Span,
) -> Result<LowerExprResult> {
    let mut operands = vec![OpOperand::Name("skw_py_joined_str".into())];
    for val in &js.values {
        match val {
            ast::Expr::Constant(c) => {
                operands.push(OpOperand::Const(constant_to_pir(&c.value)?));
            }
            ast::Expr::FormattedValue(fv) => {
                let v = lower_expr(lowerer, &fv.value)?.into_value(lowerer)?;
                operands.push(OpOperand::Value(v));
            }
            other => {
                return Err(SikuwaError::pir(format!(
                    "unsupported f-string part: {other:?}"
                )))
            }
        }
    }
    let result = lowerer.fresh_value();
    lowerer.emit(Op {
        opcode: OpCode::CallBuiltin,
        result: Some(result),
        operands,
        span,
    });
    Ok(LowerExprResult::Value(result))
}

fn constant_to_pir(value: &ast::Constant) -> Result<ConstValue> {
    use num_traits::ToPrimitive;
    Ok(match value {
        ast::Constant::None => ConstValue::None,
        ast::Constant::Bool(b) => ConstValue::Bool(*b),
        ast::Constant::Int(i) => {
            let n = i
                .to_i64()
                .ok_or_else(|| SikuwaError::pir("integer literal out of range for i64"))?;
            ConstValue::Int(n)
        }
        ast::Constant::Float(f) => ConstValue::Float(*f),
        ast::Constant::Str(s) => ConstValue::Str(s.clone()),
        other => {
            return Err(SikuwaError::pir(format!(
                "unsupported constant: {other:?}"
            )))
        }
    })
}
