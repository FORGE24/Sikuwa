//! Closure codegen (Plan 8c — `MakeClosure` / `LoadCell` / nested `FuncDef`).

use std::collections::HashMap;
use std::fmt::Write;

use sikuwa_pir::module::{FuncDef, Op, OpOperand, Terminator};
use sikuwa_pir::opcode::OpCode;

use crate::emit::{block_label, c_type_for_op, emit_op_expr_s0, sanitize};

pub fn closure_base_name(parent_sym: &str, nested_sym: &str) -> String {
    let parent = crate::emit::skw_c_symbol(parent_sym);
    let nested_name = nested_sym.rsplit('.').next().unwrap_or("inner");
    format!("{parent}_{nested_name}")
}

pub fn env_type_name(base: &str) -> String {
    format!("{base}_env_t")
}

pub fn closure_type_name(base: &str) -> String {
    format!("{base}_closure_t")
}

pub fn impl_symbol(base: &str) -> String {
    format!("{base}_impl")
}

pub fn is_closure_factory(func: &FuncDef) -> bool {
    !func.nested.is_empty()
        && func
            .blocks
            .iter()
            .flat_map(|b| &b.ops)
            .any(|op| op.opcode == OpCode::MakeClosure)
}

pub fn capture_names(parent: &FuncDef, nested_sym: &str) -> Vec<String> {
    for block in &parent.blocks {
        for op in &block.ops {
            if op.opcode != OpCode::MakeClosure {
                continue;
            }
            if let Some(OpOperand::Symbol(s)) = op.operands.first() {
                if s.0 == nested_sym {
                    return parent.cellvars.clone();
                }
            }
        }
    }
    parent.cellvars.clone()
}

pub fn emit_closure_structs(parent: &FuncDef, out: &mut String) {
    for nested in &parent.nested {
        let captured = capture_names(parent, &nested.symbol.0);
        if captured.is_empty() {
            continue;
        }
        let base = closure_base_name(&parent.symbol.0, &nested.symbol.0);
        let env_name = format!("{base}_env");
        let _ = writeln!(out, "typedef struct {env_name} {{");
        for cell in &captured {
            let _ = writeln!(out, "    int64_t {cell};");
        }
        let _ = writeln!(out, "}} {env_name}_t;");
        let _ = writeln!(
            out,
            "typedef int64_t (SKW_CALL *{base}_fn_t)({env_name}_t *env, int64_t x);"
        );
        let _ = writeln!(
            out,
            "typedef struct {base}_closure {{\n    {env_name}_t env;\n    {base}_fn_t fn;\n}} {base}_closure_t;\n"
        );
    }
}

pub fn emit_nested_impls(parent: &FuncDef, out: &mut String) {
    for nested in &parent.nested {
        emit_nested_impl(parent, nested, out);
    }
}

fn emit_nested_impl(parent: &FuncDef, nested: &FuncDef, out: &mut String) {
    let base = closure_base_name(&parent.symbol.0, &nested.symbol.0);
    let env_ty = env_type_name(&base);
    let impl_fn = impl_symbol(&base);
    let mut params = format!("{env_ty} *env");
    for p in &nested.params {
        let _ = write!(params, ", int64_t {}", sanitize(p));
    }
    let _ = writeln!(
        out,
        "static int64_t SKW_CALL {impl_fn}({params}) {{"
    );

    let mut env: HashMap<String, String> = HashMap::new();
    for cell in capture_names(parent, &nested.symbol.0) {
        env.insert(cell.clone(), format!("env->{cell}"));
    }
    for p in &nested.params {
        env.insert(p.clone(), sanitize(p));
    }

    let mut temps: HashMap<u32, String> = HashMap::new();
    let mut next_temp = 0u32;

    for block in &nested.blocks {
        let _ = writeln!(out, "  {}:", block_label(&block.id));
        emit_simple_block_body(block, out, &mut env, &mut temps, &mut next_temp);
        emit_simple_terminator(block, out, &temps);
    }
    let _ = writeln!(out, "}}\n");
}

fn emit_simple_block_body(
    block: &sikuwa_pir::module::Block,
    out: &mut String,
    env: &mut HashMap<String, String>,
    temps: &mut HashMap<u32, String>,
    next_temp: &mut u32,
) {
    for op in &block.ops {
        if matches!(
            op.opcode,
            OpCode::LoadFast | OpCode::LoadCell | OpCode::Phi
        ) {
            if let Some(result) = op.result {
                if let Some(OpOperand::Name(n)) = op.operands.first() {
                    let src = env.get(n).cloned().unwrap_or_else(|| sanitize(n));
                    temps.insert(result.0, src);
                }
            }
        } else if matches!(op.opcode, OpCode::StoreFast | OpCode::StoreCell) {
            if let (Some(OpOperand::Name(n)), Some(OpOperand::Value(v))) =
                (op.operands.first(), op.operands.get(1))
            {
                if let Some(src) = temps.get(&v.0) {
                    env.insert(n.clone(), src.clone());
                }
            }
        } else if let Some(result) = op.result {
            let name = format!("t{}", next_temp);
            *next_temp += 1;
            let expr = emit_op_expr_s0(op, env, temps);
            let _ = writeln!(out, "  {} {} = {};", c_type_for_op(op), name, expr);
            temps.insert(result.0, name);
        }
    }
}

fn emit_simple_terminator(
    block: &sikuwa_pir::module::Block,
    out: &mut String,
    temps: &HashMap<u32, String>,
) {
    match &block.term {
        Terminator::Branch { target } => {
            let _ = writeln!(out, "  goto {};", block_label(target));
        }
        Terminator::CondBranch {
            cond,
            then_block,
            else_block,
        } => {
            let c = temps.get(&cond.0).cloned().unwrap_or_else(|| "0".into());
            let _ = writeln!(out, "  if ({c}) goto {};", block_label(then_block));
            let _ = writeln!(out, "  goto {};", block_label(else_block));
        }
        Terminator::Return { value: Some(v) } => {
            let ret = temps.get(&v.0).cloned().unwrap_or_else(|| "0".into());
            let _ = writeln!(out, "  return {};", ret);
        }
        Terminator::Return { value: None } => {
            let _ = writeln!(out, "  return 0;");
        }
        Terminator::Unreachable => {
            let _ = writeln!(out, "  /* unreachable */");
        }
    }
}

pub fn emit_make_closure_op(
    op: &Op,
    parent: &FuncDef,
    out: &mut String,
    temps: &mut HashMap<u32, String>,
    next_temp: &mut u32,
    captured_vals: &[String],
) {
    let Some(OpOperand::Symbol(sym)) = op.operands.first() else {
        return;
    };
    let base = closure_base_name(&parent.symbol.0, &sym.0);
    let captured = capture_names(parent, &sym.0);
    let struct_var = format!("t{}", next_temp);
    *next_temp += 1;
    let clo_ty = closure_type_name(&base);
    let _ = writeln!(out, "  {clo_ty} {struct_var};");
    for (cell, v) in captured.iter().zip(captured_vals.iter()) {
        let _ = writeln!(out, "  {struct_var}.env.{cell} = {v};");
    }
    let _ = writeln!(
        out,
        "  {struct_var}.fn = {};",
        impl_symbol(&base)
    );
    if let Some(result) = op.result {
        temps.insert(result.0, struct_var);
    }
}

pub fn class_struct_type(method_symbol: &str) -> Option<String> {
    let mut parts: Vec<&str> = method_symbol.split('.').collect();
    if parts.len() < 3 {
        return None;
    }
    if parts.last()? != &"__init__" {
        return None;
    }
    parts.pop();
    Some(format!("skw_{}", parts.join("_")))
}

pub fn is_class_init_method(func: &FuncDef) -> bool {
    func.symbol.0.contains(".__init__")
        && func.blocks.iter().flat_map(|b| &b.ops).all(|op| {
            !matches!(
                op.opcode,
                OpCode::LoadAttr
                    | OpCode::SubscriptLoad
                    | OpCode::MakeClosure
                    | OpCode::BuildClass
                    | OpCode::GetIter
            )
        })
}

pub fn closure_return_type(func: &FuncDef) -> Option<String> {
    let nested = func.nested.first()?;
    let base = closure_base_name(&func.symbol.0, &nested.symbol.0);
    Some(closure_type_name(&base))
}
