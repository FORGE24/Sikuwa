//! Lower `class` definitions to PIR `ClassDef` + `BuildClass` ops.

use rustpython_ast as ast;
use rustpython_ast::Ranged;

use sikuwa_core::{Result, SikuwaError};

use crate::ids::SymbolRef;
use crate::module::{ClassDef, Op, OpOperand};
use crate::opcode::OpCode;
use crate::span::Span;

use super::function::{lower_function_in_class, FunctionLowerer, LowerContext};

pub fn lower_class(
    module_name: &str,
    file_path: &str,
    source: &str,
    cd: &ast::StmtClassDef,
    ctx: &LowerContext,
) -> Result<ClassDef> {
    let name = cd.name.to_string();
    let symbol = SymbolRef::new(format!("{module_name}.{name}"));
    let span = Span {
        file: file_path.to_string(),
        start_line: super::function::line_at_offset(source, cd.start()),
        start_col: super::function::col_at_offset(source, cd.start()),
        end_line: super::function::line_at_offset(source, cd.end()),
        end_col: super::function::col_at_offset(source, cd.end()),
    };

    let mut bases = Vec::new();
    for base in &cd.bases {
        match base {
            ast::Expr::Name(n) => bases.push(n.id.to_string()),
            _ => {
                return Err(SikuwaError::pir(
                    "class bases must be simple names for now",
                ))
            }
        }
    }

    let mut methods = Vec::new();
    for stmt in &cd.body {
        match stmt {
            ast::Stmt::FunctionDef(fd) => {
                methods.push(lower_function_in_class(
                    module_name,
                    &name,
                    file_path,
                    source,
                    fd,
                    ctx,
                )?);
            }
            ast::Stmt::Pass(_) => {}
            other => {
                return Err(SikuwaError::pir(format!(
                    "unsupported statement in class `{name}`: {other:?}"
                )));
            }
        }
    }

    Ok(ClassDef {
        symbol,
        name,
        bases,
        methods,
        span,
    })
}

/// Emit `BuildClass` in the enclosing function (module init stub uses entry block).
pub fn emit_build_class(
    lowerer: &mut FunctionLowerer,
    class: &ClassDef,
    span: Span,
) -> Result<crate::ids::ValueId> {
    let mut operands = vec![
        OpOperand::Name(class.name.clone()),
        OpOperand::Const(crate::module::ConstValue::Str(class.bases.join(","))),
    ];
    for method in &class.methods {
        operands.push(OpOperand::Symbol(method.symbol.clone()));
    }
    let result = lowerer.fresh_value();
    lowerer.emit(Op {
        opcode: OpCode::BuildClass,
        result: Some(result),
        operands,
        span,
    });
    Ok(result)
}
