//! Lower `FunctionDef` to PIR CFG with SSA phi nodes and local environment.

use std::collections::{HashMap, HashSet};

use rustpython_ast::{self as ast, CmpOp, Ranged};

use sikuwa_core::{Result, SikuwaError};

use crate::ids::{BlockId, SymbolRef, ValueId};
use crate::module::{Block, ExternDecl, FuncDef, ExceptionRegion, ModuleImport, Op, OpOperand, Phi, PhiIncoming, Terminator};
use crate::opcode::OpCode;
use crate::span::Span;

use super::expr::{lower_expr, LowerExprResult};

/// Visible names in an enclosing scope (for closure capture).
#[derive(Debug, Clone)]
pub struct EnclosingScope {
    pub params: HashSet<String>,
    pub locals: HashSet<String>,
}

/// Import / extern context for a module or nested function.
#[derive(Debug, Clone, Default)]
pub struct LowerContext {
    pub externs: HashMap<String, ExternDecl>,
    pub import_syms: HashMap<String, String>,
    pub module_locals: HashSet<String>,
    pub module_resolver: HashMap<String, String>,
}

impl LowerContext {
    pub fn from_module(externs: &[ExternDecl], imports: &[ModuleImport]) -> Self {
        let externs = externs
            .iter()
            .cloned()
            .map(|e| (e.name.clone(), e))
            .collect();
        let import_syms = super::import::import_map(imports);
        let module_locals = super::import::module_locals(imports);
        let module_resolver = imports
            .iter()
            .filter(|i| i.symbol.ends_with(".*"))
            .map(|i| (i.local.clone(), i.module.clone()))
            .collect();
        Self {
            externs,
            import_syms,
            module_locals,
            module_resolver,
        }
    }
}

pub struct FunctionLowerer {
    pub(crate) file_path: String,
    source: String,
    symbol: SymbolRef,
    params: Vec<String>,
    span: Span,
    next_value: u32,
    next_block: u32,
    blocks: Vec<Block>,
    current_id: BlockId,
    current_phis: Vec<Phi>,
    current_ops: Vec<Op>,
    block_sealed: bool,
    /// Current SSA bindings for Python local names (LogicalSlot).
    locals: HashMap<String, ValueId>,
    local_names: HashSet<String>,
    /// Free variables captured from enclosing scope (inner function only).
    free_cells: HashSet<String>,
    /// Names promoted to cells in this function (for nested closures).
    cellvars: HashSet<String>,
    nested: Vec<FuncDef>,
    module_name: String,
    ctx: LowerContext,
    exception_regions: Vec<ExceptionRegion>,
}

impl FunctionLowerer {
    pub(crate) fn fresh_value(&mut self) -> ValueId {
        let v = ValueId(self.next_value);
        self.next_value += 1;
        v
    }

    pub(crate) fn emit(&mut self, op: Op) {
        self.current_ops.push(op);
    }

    fn fresh_block_id(&mut self, hint: &str) -> BlockId {
        self.next_block += 1;
        BlockId::new(format!("{hint}_{}", self.next_block))
    }

    fn track_local(&mut self, name: &str) {
        self.local_names.insert(name.to_string());
    }

    pub(crate) fn span_from(&self, node: &impl Ranged) -> Span {
        let range = node.range();
        Span {
            file: self.file_path.clone(),
            start_line: line_at_offset(&self.source, range.start()),
            start_col: col_at_offset(&self.source, range.start()),
            end_line: line_at_offset(&self.source, range.end()),
            end_col: col_at_offset(&self.source, range.end()),
        }
    }

    fn start_block(&mut self, id: BlockId) {
        self.current_id = id;
        self.current_phis.clear();
        self.current_ops.clear();
        self.block_sealed = false;
    }

    fn seal(&mut self, term: Terminator) -> Result<()> {
        if self.block_sealed {
            return Ok(());
        }
        self.blocks.push(Block {
            id: self.current_id.clone(),
            phis: std::mem::take(&mut self.current_phis),
            ops: std::mem::take(&mut self.current_ops),
            term,
        });
        self.block_sealed = true;
        Ok(())
    }

    /// Load a local: use env binding, cell, or materialize parameter.
    pub(crate) fn load_local(&mut self, name: &str, span: Span) -> Result<ValueId> {
        if self.free_cells.contains(name) {
            let v = self.fresh_value();
            self.emit(Op {
                opcode: OpCode::LoadCell,
                result: Some(v),
                operands: vec![OpOperand::Name(name.to_string())],
                span: span.clone(),
            });
            return Ok(v);
        }
        if let Some(v) = self.locals.get(name) {
            return Ok(*v);
        }
        if self.params.iter().any(|p| p == name) {
            let v = self.fresh_value();
            self.emit(Op {
                opcode: OpCode::LoadFast,
                result: Some(v),
                operands: vec![OpOperand::Name(name.to_string())],
                span: span.clone(),
            });
            self.locals.insert(name.to_string(), v);
            self.track_local(name);
            return Ok(v);
        }
        Err(SikuwaError::pir(format!(
            "undefined name `{name}` in {}",
            self.symbol
        )))
    }

    pub(crate) fn store_local(&mut self, name: &str, value: ValueId, span: Span) {
        if self.cellvars.contains(name) {
            self.emit(Op {
                opcode: OpCode::StoreCell,
                result: None,
                operands: vec![OpOperand::Name(name.to_string()), OpOperand::Value(value)],
                span,
            });
        } else {
            self.emit(Op {
                opcode: OpCode::StoreFast,
                result: None,
                operands: vec![OpOperand::Name(name.to_string()), OpOperand::Value(value)],
                span,
            });
        }
        self.locals.insert(name.to_string(), value);
        self.track_local(name);
    }

    fn emit_phi(
        &mut self,
        name: &str,
        then_id: &BlockId,
        then_val: ValueId,
        else_id: &BlockId,
        else_val: ValueId,
        span: Span,
    ) -> ValueId {
        let result = self.fresh_value();
        self.current_phis.push(Phi {
            result,
            name: name.to_string(),
            incoming: vec![
                PhiIncoming {
                    block: then_id.clone(),
                    value: then_val,
                },
                PhiIncoming {
                    block: else_id.clone(),
                    value: else_val,
                },
            ],
        });
        self.emit(Op {
            opcode: OpCode::Phi,
            result: Some(result),
            operands: vec![
                OpOperand::Name(name.to_string()),
                OpOperand::Value(then_val),
                OpOperand::Block(then_id.clone()),
                OpOperand::Value(else_val),
                OpOperand::Block(else_id.clone()),
            ],
            span,
        });
        self.track_local(name);
        result
    }

    fn merge_locals(
        &mut self,
        pre: &HashMap<String, ValueId>,
        then_end: &HashMap<String, ValueId>,
        else_end: &HashMap<String, ValueId>,
        then_id: &BlockId,
        else_id: &BlockId,
        span: Span,
    ) -> HashMap<String, ValueId> {
        let mut keys: HashSet<String> = pre.keys().cloned().collect();
        keys.extend(then_end.keys().cloned());
        keys.extend(else_end.keys().cloned());

        let mut merged = HashMap::new();
        for name in keys {
            let tv = then_end
                .get(&name)
                .copied()
                .or_else(|| pre.get(&name).copied());
            let ev = else_end
                .get(&name)
                .copied()
                .or_else(|| pre.get(&name).copied());
            match (tv, ev) {
                (Some(tv), Some(ev)) if tv == ev => {
                    merged.insert(name, tv);
                }
                (Some(tv), Some(ev)) => {
                    let phi = self.emit_phi(&name, then_id, tv, else_id, ev, span.clone());
                    merged.insert(name, phi);
                }
                (Some(tv), None) | (None, Some(tv)) => {
                    merged.insert(name, tv);
                }
                (None, None) => {}
            }
        }
        merged
    }

    fn lower_stmts(&mut self, stmts: &[ast::Stmt]) -> Result<()> {
        for stmt in stmts {
            if self.block_sealed {
                break;
            }
            self.lower_stmt(stmt)?;
        }
        Ok(())
    }

    fn lower_stmt(&mut self, stmt: &ast::Stmt) -> Result<()> {
        match stmt {
            ast::Stmt::Return(ret) => {
                let _span = self.span_from(ret);
                let value = match &ret.value {
                    Some(expr) => Some(lower_expr(self, expr)?.into_value(self)?),
                    None => None,
                };
                self.seal(Terminator::Return { value })?;
                Ok(())
            }
            ast::Stmt::If(if_stmt) => self.lower_if(if_stmt),
            ast::Stmt::While(while_stmt) => self.lower_while(while_stmt),
            ast::Stmt::For(for_stmt) => self.lower_for(for_stmt),
            ast::Stmt::Assign(assign) => self.lower_assign(assign),
            ast::Stmt::AugAssign(aug) => self.lower_aug_assign(aug),
            ast::Stmt::FunctionDef(fd) => self.lower_nested_function(fd),
            ast::Stmt::Try(try_stmt) => self.lower_try(try_stmt),
            ast::Stmt::Pass(_) => Ok(()),
            ast::Stmt::Expr(expr) => {
                if matches!(&*expr.value, ast::Expr::Constant(_)) {
                    Ok(())
                } else {
                    let _ = lower_expr(self, &expr.value)?;
                    Ok(())
                }
            }
            other => Err(SikuwaError::pir(format!(
                "unsupported statement in {}: {other:?}",
                self.file_path
            ))),
        }
    }

    /// Bare `try` / `except` with `return` bodies (exceptional-edge metadata).
    fn lower_try(&mut self, try_stmt: &ast::StmtTry) -> Result<()> {
        if !try_stmt.orelse.is_empty() || !try_stmt.finalbody.is_empty() {
            return Err(SikuwaError::pir(
                "try/else and try/finally not supported yet",
            ));
        }
        if try_stmt.handlers.len() != 1 {
            return Err(SikuwaError::pir("only single except handler supported yet"));
        }
        let handler = match &try_stmt.handlers[0] {
            ast::ExceptHandler::ExceptHandler(h) => h,
        };
        if handler.type_.is_some() || handler.name.is_some() {
            return Err(SikuwaError::pir("only bare except supported yet"));
        }

        let protected_id = self.current_id.clone();
        self.lower_stmts(&try_stmt.body)?;

        let handler_id = self.fresh_block_id("except");
        self.exception_regions.push(ExceptionRegion {
            protected: vec![protected_id],
            handlers: vec![handler_id.clone()],
            finally: None,
        });

        if !self.block_sealed {
            return Err(SikuwaError::pir(
                "try body must end with return (no fallthrough yet)",
            ));
        }

        self.start_block(handler_id);
        self.lower_stmts(&handler.body)?;
        if !self.block_sealed {
            return Err(SikuwaError::pir(
                "except body must end with return (no fallthrough yet)",
            ));
        }
        Ok(())
    }

    fn lower_assign(&mut self, assign: &ast::StmtAssign) -> Result<()> {
        let span = self.span_from(assign);
        if assign.targets.len() != 1 {
            return Err(SikuwaError::pir("multi-target assign not supported yet"));
        }
        let value = lower_expr(self, &assign.value)?.into_value(self)?;
        match &assign.targets[0] {
            ast::Expr::Name(name) => {
                self.store_local(&name.id.to_string(), value, span);
                Ok(())
            }
            ast::Expr::Attribute(attr) => {
                let obj = lower_expr(self, &attr.value)?.into_value(self)?;
                self.emit(Op {
                    opcode: OpCode::StoreAttr,
                    result: None,
                    operands: vec![
                        OpOperand::Value(obj),
                        OpOperand::Name(attr.attr.to_string()),
                        OpOperand::Value(value),
                    ],
                    span,
                });
                Ok(())
            }
            ast::Expr::Subscript(sub) => {
                let obj = lower_expr(self, &sub.value)?.into_value(self)?;
                let key = lower_expr(self, &sub.slice)?.into_value(self)?;
                self.emit(Op {
                    opcode: OpCode::SubscriptStore,
                    result: None,
                    operands: vec![
                        OpOperand::Value(obj),
                        OpOperand::Value(key),
                        OpOperand::Value(value),
                    ],
                    span,
                });
                Ok(())
            }
            _ => Err(SikuwaError::pir(
                "unsupported assignment target",
            )),
        }
    }

    fn lower_nested_function(&mut self, fd: &ast::StmtFunctionDef) -> Result<()> {
        let span = self.span_from(fd);
        let inner_params: HashSet<String> = fd
            .args
            .args
            .iter()
            .map(|a| a.def.arg.to_string())
            .collect();
        let enclosing = EnclosingScope {
            params: self.params.iter().cloned().collect(),
            locals: self.local_names.clone(),
        };
        let free = collect_free_vars(fd, &enclosing, &inner_params);
        for name in &free {
            self.cellvars.insert(name.clone());
        }

        let nested = lower_function(
            &self.module_name,
            &self.file_path,
            &self.source,
            fd,
            Some(EnclosingScope {
                params: enclosing.params,
                locals: enclosing
                    .locals
                    .union(&self.cellvars)
                    .cloned()
                    .collect(),
            }),
            &self.ctx,
        )?;
        let sym = nested.symbol.clone();
        self.nested.push(nested);

        let mut operands = vec![OpOperand::Symbol(sym)];
        for name in &free {
            let cell_val = self.load_local(name, span.clone())?;
            operands.push(OpOperand::Value(cell_val));
        }
        let closure = self.fresh_value();
        self.emit(Op {
            opcode: OpCode::MakeClosure,
            result: Some(closure),
            operands,
            span: span.clone(),
        });
        self.store_local(&fd.name.to_string(), closure, span);
        Ok(())
    }

    fn lower_aug_assign(&mut self, aug: &ast::StmtAugAssign) -> Result<()> {
        let span = self.span_from(aug);
        let name = match &*aug.target {
            ast::Expr::Name(n) => n.id.to_string(),
            _ => {
                return Err(SikuwaError::pir(
                    "augassign only supported on simple names",
                ))
            }
        };
        let lhs = self.load_local(&name, span.clone())?;
        let rhs = lower_expr(self, &aug.value)?.into_value(self)?;
        let result = self.fresh_value();
        let opcode = aug_op_to_opcode(aug.op)?;
        self.emit(Op {
            opcode,
            result: Some(result),
            operands: vec![OpOperand::Value(lhs), OpOperand::Value(rhs)],
            span: span.clone(),
        });
        self.store_local(&name, result, span);
        Ok(())
    }

    fn lower_if(&mut self, if_stmt: &ast::StmtIf) -> Result<()> {
        let span = self.span_from(if_stmt);
        let pre_locals = self.locals.clone();
        let cond_val = lower_expr(self, &if_stmt.test)?.into_value(self)?;
        let then_id = self.fresh_block_id("then");
        let else_id = self.fresh_block_id("else");
        let merge_id = self.fresh_block_id("merge");

        self.seal(Terminator::CondBranch {
            cond: cond_val,
            then_block: then_id.clone(),
            else_block: else_id.clone(),
        })?;

        self.start_block(then_id.clone());
        self.lower_stmts(&if_stmt.body)?;
        let then_locals = self.locals.clone();
        if !self.block_sealed {
            self.seal(Terminator::Branch {
                target: merge_id.clone(),
            })?;
        }

        self.locals = pre_locals.clone();
        self.start_block(else_id.clone());
        if if_stmt.orelse.is_empty() {
            self.seal(Terminator::Branch {
                target: merge_id.clone(),
            })?;
        } else {
            self.lower_stmts(&if_stmt.orelse)?;
            if !self.block_sealed {
                self.seal(Terminator::Branch {
                    target: merge_id.clone(),
                })?;
            }
        }
        let else_locals = self.locals.clone();

        self.start_block(merge_id);
        self.locals = self.merge_locals(
            &pre_locals,
            &then_locals,
            &else_locals,
            &then_id,
            &else_id,
            span,
        );
        Ok(())
    }

    fn lower_while(&mut self, while_stmt: &ast::StmtWhile) -> Result<()> {
        let header_id = self.fresh_block_id("while_hdr");
        let body_id = self.fresh_block_id("while_body");
        let exit_id = self.fresh_block_id("while_exit");

        self.seal(Terminator::Branch {
            target: header_id.clone(),
        })?;

        self.start_block(header_id.clone());
        let cond = lower_expr(self, &while_stmt.test)?.into_value(self)?;
        self.seal(Terminator::CondBranch {
            cond,
            then_block: body_id.clone(),
            else_block: exit_id.clone(),
        })?;

        self.start_block(body_id);
        self.lower_stmts(&while_stmt.body)?;
        if !self.block_sealed {
            self.seal(Terminator::Branch {
                target: header_id,
            })?;
        }

        self.start_block(exit_id);
        Ok(())
    }

    fn lower_for(&mut self, for_stmt: &ast::StmtFor) -> Result<()> {
        let span = self.span_from(for_stmt);
        let iter_val = lower_expr(self, &for_stmt.iter)?.into_value(self)?;
        let iter_slot = self.fresh_value();
        self.emit(Op {
            opcode: OpCode::GetIter,
            result: Some(iter_slot),
            operands: vec![OpOperand::Value(iter_val)],
            span: span.clone(),
        });

        let header_id = self.fresh_block_id("for_hdr");
        let body_id = self.fresh_block_id("for_body");
        let exit_id = self.fresh_block_id("for_exit");

        self.seal(Terminator::Branch {
            target: header_id.clone(),
        })?;

        self.start_block(header_id.clone());
        let item = self.fresh_value();
        self.emit(Op {
            opcode: OpCode::ForIterNext,
            result: Some(item),
            operands: vec![OpOperand::Value(iter_slot)],
            span: span.clone(),
        });
        let none_val = self.fresh_value();
        self.emit(Op {
            opcode: OpCode::Const,
            result: Some(none_val),
            operands: vec![OpOperand::Const(crate::module::ConstValue::None)],
            span: span.clone(),
        });
        let is_none = self.fresh_value();
        self.emit(Op {
            opcode: OpCode::CompareIs,
            result: Some(is_none),
            operands: vec![OpOperand::Value(item), OpOperand::Value(none_val)],
            span: span.clone(),
        });
        let has_next = self.fresh_value();
        self.emit(Op {
            opcode: OpCode::UnaryNot,
            result: Some(has_next),
            operands: vec![OpOperand::Value(is_none)],
            span: span.clone(),
        });
        self.seal(Terminator::CondBranch {
            cond: has_next,
            then_block: body_id.clone(),
            else_block: exit_id.clone(),
        })?;

        self.start_block(body_id);
        match &*for_stmt.target {
            ast::Expr::Name(name) => {
                self.store_local(&name.id.to_string(), item, span.clone());
            }
            _ => {
                return Err(SikuwaError::pir(
                    "for-loop target must be a simple name",
                ))
            }
        }
        self.lower_stmts(&for_stmt.body)?;
        if !self.block_sealed {
            self.seal(Terminator::Branch {
                target: header_id,
            })?;
        }

        self.start_block(exit_id);
        Ok(())
    }

    pub(crate) fn lower_binop(
        &mut self,
        op: ast::Operator,
        left: &ast::Expr,
        right: &ast::Expr,
        span: Span,
    ) -> Result<LowerExprResult> {
        let lhs = lower_expr(self, left)?.into_value(self)?;
        let rhs = lower_expr(self, right)?.into_value(self)?;
        let opcode = match op {
            ast::Operator::Add => OpCode::BinOpAdd,
            ast::Operator::Sub => OpCode::BinOpSub,
            ast::Operator::Mult => OpCode::BinOpMul,
            ast::Operator::Div => OpCode::BinOpTrueDiv,
            ast::Operator::FloorDiv => OpCode::BinOpFloorDiv,
            ast::Operator::Mod => OpCode::BinOpMod,
            other => {
                return Err(SikuwaError::pir(format!(
                    "unsupported binop: {other:?}"
                )))
            }
        };
        let result = self.fresh_value();
        self.emit(Op {
            opcode,
            result: Some(result),
            operands: vec![OpOperand::Value(lhs), OpOperand::Value(rhs)],
            span,
        });
        Ok(LowerExprResult::Value(result))
    }

    pub(crate) fn lower_compare(
        &mut self,
        left: &ast::Expr,
        ops: &[CmpOp],
        comparators: &[ast::Expr],
        span: Span,
    ) -> Result<LowerExprResult> {
        if ops.len() != 1 || comparators.len() != 1 {
            return Err(SikuwaError::pir(
                "chained comparisons not supported yet",
            ));
        }
        let lhs = lower_expr(self, left)?.into_value(self)?;
        let rhs = lower_expr(self, &comparators[0])?.into_value(self)?;
        let opcode = match ops[0] {
            CmpOp::Lt => OpCode::CompareLt,
            CmpOp::LtE => OpCode::CompareLe,
            CmpOp::Gt => OpCode::CompareGt,
            CmpOp::GtE => OpCode::CompareGe,
            CmpOp::Eq => OpCode::CompareEq,
            CmpOp::NotEq => OpCode::CompareNe,
            CmpOp::Is => OpCode::CompareIs,
            CmpOp::IsNot => OpCode::CompareIsNot,
            other => {
                return Err(SikuwaError::pir(format!(
                    "unsupported compare op: {other:?}"
                )))
            }
        };
        let result = self.fresh_value();
        self.emit(Op {
            opcode,
            result: Some(result),
            operands: vec![OpOperand::Value(lhs), OpOperand::Value(rhs)],
            span,
        });
        Ok(LowerExprResult::Value(result))
    }

    pub(crate) fn lower_call(
        &mut self,
        call: &ast::ExprCall,
        span: Span,
    ) -> Result<LowerExprResult> {
        if !call.keywords.is_empty() {
            return Err(SikuwaError::pir("keyword arguments not supported yet"));
        }
        let (opcode, callee) = match &*call.func {
            ast::Expr::Name(name) => {
                let n = name.id.to_string();
                if self.ctx.externs.contains_key(&n) {
                    (
                        OpCode::CallExtern,
                        OpOperand::Name(n),
                    )
                } else if let Some(sym) = self.ctx.import_syms.get(&n) {
                    (
                        OpCode::Call,
                        OpOperand::Symbol(SymbolRef::new(sym.clone())),
                    )
                } else {
                    (OpCode::Call, OpOperand::Name(n))
                }
            }
            ast::Expr::Attribute(attr) => {
                if let ast::Expr::Name(mod_name) = &*attr.value {
                    let local = mod_name.id.to_string();
                    if self.ctx.module_locals.contains(&local) {
                        let mod_path = self
                            .ctx
                            .module_resolver
                            .get(&local)
                            .ok_or_else(|| SikuwaError::pir(format!("unknown module `{local}`")))?;
                        let sym = format!("{mod_path}.{}", attr.attr);
                        (
                            OpCode::Call,
                            OpOperand::Symbol(SymbolRef::new(sym)),
                        )
                    } else {
                        return Err(SikuwaError::pir(
                            "only imported-module attribute calls supported",
                        ));
                    }
                } else {
                    return Err(SikuwaError::pir(
                        "only direct name or module.attr calls supported yet",
                    ));
                }
            }
            _ => {
                return Err(SikuwaError::pir(
                    "only direct name calls supported yet",
                ))
            }
        };
        let mut operands = vec![callee];
        for arg in &call.args {
            let v = lower_expr(self, arg)?.into_value(self)?;
            operands.push(OpOperand::Value(v));
        }
        let result = self.fresh_value();
        self.emit(Op {
            opcode,
            result: Some(result),
            operands,
            span,
        });
        Ok(LowerExprResult::Value(result))
    }
}

fn aug_op_to_opcode(op: ast::Operator) -> Result<OpCode> {
    Ok(match op {
        ast::Operator::Add => OpCode::BinOpAdd,
        ast::Operator::Sub => OpCode::BinOpSub,
        ast::Operator::Mult => OpCode::BinOpMul,
        ast::Operator::Div => OpCode::BinOpTrueDiv,
        ast::Operator::FloorDiv => OpCode::BinOpFloorDiv,
        ast::Operator::Mod => OpCode::BinOpMod,
        other => {
            return Err(SikuwaError::pir(format!(
                "unsupported augassign op: {other:?}"
            )))
        }
    })
}

pub fn lower_function(
    module_name: &str,
    file_path: &str,
    source: &str,
    fd: &ast::StmtFunctionDef,
    parent: Option<EnclosingScope>,
    ctx: &LowerContext,
) -> Result<FuncDef> {
    let params: Vec<String> = fd
        .args
        .args
        .iter()
        .map(|a| a.def.arg.to_string())
        .collect();

    let inner_param_set: HashSet<String> = params.iter().cloned().collect();
    let free_cells = parent
        .as_ref()
        .map(|p| collect_free_vars(fd, p, &inner_param_set))
        .unwrap_or_default();

    let mut lowerer = FunctionLowerer {
        file_path: file_path.to_string(),
        source: source.to_string(),
        symbol: SymbolRef::new(format!("{module_name}.{}", fd.name)),
        params: params.clone(),
        span: Span {
            file: file_path.to_string(),
            start_line: line_at_offset(source, fd.start()),
            start_col: col_at_offset(source, fd.start()),
            end_line: line_at_offset(source, fd.end()),
            end_col: col_at_offset(source, fd.end()),
        },
        next_value: 0,
        next_block: 0,
        blocks: Vec::new(),
        current_id: BlockId::entry(),
        current_phis: Vec::new(),
        current_ops: Vec::new(),
        block_sealed: false,
        locals: HashMap::new(),
        local_names: params.iter().cloned().collect(),
        free_cells: free_cells.iter().cloned().collect(),
        cellvars: HashSet::new(),
        nested: Vec::new(),
        module_name: module_name.to_string(),
        ctx: ctx.clone(),
        exception_regions: Vec::new(),
    };

    for p in &params {
        let span = lowerer.span.clone();
        let _ = lowerer.load_local(p, span)?;
    }

    lowerer.lower_stmts(&fd.body)?;
    if !lowerer.block_sealed {
        lowerer.seal(Terminator::Return { value: None })?;
    }

    if lowerer.blocks.is_empty() {
        return Err(SikuwaError::pir(format!(
            "empty function body: {}",
            lowerer.symbol
        )));
    }

    let return_value = lowerer
        .blocks
        .iter()
        .rev()
        .find_map(|b| match &b.term {
            Terminator::Return { value: Some(v) } => Some(*v),
            _ => None,
        });

    let mut locals: Vec<String> = lowerer.local_names.into_iter().collect();
    locals.sort();
    let mut cellvars: Vec<String> = lowerer.cellvars.into_iter().collect();
    cellvars.sort();

    Ok(FuncDef {
        symbol: lowerer.symbol,
        params,
        locals,
        cellvars,
        nested: lowerer.nested,
        return_value,
        blocks: lowerer.blocks,
        span: lowerer.span,
        exception_regions: lowerer.exception_regions,
    })
}

pub fn lower_function_in_class(
    module_name: &str,
    class_name: &str,
    file_path: &str,
    source: &str,
    fd: &ast::StmtFunctionDef,
    ctx: &LowerContext,
) -> Result<FuncDef> {
    lower_function(
        module_name,
        file_path,
        source,
        fd,
        None,
        ctx,
    )
    .map(|mut f| {
        f.symbol = SymbolRef::new(format!("{module_name}.{class_name}.{}", fd.name));
        f
    })
}

fn collect_free_vars(
    fd: &ast::StmtFunctionDef,
    parent: &EnclosingScope,
    inner_params: &HashSet<String>,
) -> Vec<String> {
    let mut used = HashSet::new();
    for stmt in &fd.body {
        collect_stmt_loads(stmt, &mut used);
    }
    let mut visible: HashSet<String> = parent.params.clone();
    visible.extend(parent.locals.iter().cloned());
    let mut free: Vec<String> = used
        .into_iter()
        .filter(|n| visible.contains(n) && !inner_params.contains(n))
        .collect();
    free.sort();
    free
}

fn collect_stmt_loads(stmt: &ast::Stmt, out: &mut HashSet<String>) {
    match stmt {
        ast::Stmt::Return(r) => {
            if let Some(v) = &r.value {
                collect_expr_loads(v, out);
            }
        }
        ast::Stmt::Assign(a) => collect_expr_loads(&a.value, out),
        ast::Stmt::AugAssign(a) => {
            collect_expr_loads(&a.target, out);
            collect_expr_loads(&a.value, out);
        }
        ast::Stmt::If(i) => {
            collect_expr_loads(&i.test, out);
            for s in &i.body {
                collect_stmt_loads(s, out);
            }
            for s in &i.orelse {
                collect_stmt_loads(s, out);
            }
        }
        ast::Stmt::While(w) => {
            collect_expr_loads(&w.test, out);
            for s in &w.body {
                collect_stmt_loads(s, out);
            }
        }
        ast::Stmt::For(f) => {
            collect_expr_loads(&f.iter, out);
            for s in &f.body {
                collect_stmt_loads(s, out);
            }
        }
        ast::Stmt::Expr(e) => collect_expr_loads(&e.value, out),
        ast::Stmt::FunctionDef(_) | ast::Stmt::Pass(_) => {}
        _ => {}
    }
}

fn collect_expr_loads(expr: &ast::Expr, out: &mut HashSet<String>) {
    match expr {
        ast::Expr::Name(n) => {
            out.insert(n.id.to_string());
        }
        ast::Expr::BinOp(b) => {
            collect_expr_loads(&b.left, out);
            collect_expr_loads(&b.right, out);
        }
        ast::Expr::UnaryOp(u) => collect_expr_loads(&u.operand, out),
        ast::Expr::Compare(c) => {
            collect_expr_loads(&c.left, out);
            for comp in &c.comparators {
                collect_expr_loads(comp, out);
            }
        }
        ast::Expr::Call(c) => {
            collect_expr_loads(&c.func, out);
            for arg in &c.args {
                collect_expr_loads(arg, out);
            }
        }
        ast::Expr::Attribute(a) => collect_expr_loads(&a.value, out),
        ast::Expr::Subscript(s) => {
            collect_expr_loads(&s.value, out);
            collect_expr_loads(&s.slice, out);
        }
        ast::Expr::Constant(_) => {}
        _ => {}
    }
}

pub fn line_at_offset(source: &str, offset: rustpython_ast::text_size::TextSize) -> u32 {
    let pos = offset.to_usize().min(source.len());
    (source[..pos].bytes().filter(|b| *b == b'\n').count() as u32) + 1
}

pub fn col_at_offset(source: &str, offset: rustpython_ast::text_size::TextSize) -> u32 {
    let pos = offset.to_usize().min(source.len());
    source[..pos]
        .rfind('\n')
        .map(|i| (pos - i - 1) as u32)
        .unwrap_or(pos as u32)
}
