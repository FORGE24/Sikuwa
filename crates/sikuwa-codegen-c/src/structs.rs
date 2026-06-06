//! Emit C struct layouts for classes and closures.

use std::collections::HashSet;
use std::fmt::Write;

use sikuwa_pir::module::{ClassDef, FuncDef, Module, OpOperand};
use sikuwa_pir::OpCode;

use crate::emit::{module_c_name, skw_c_symbol};

pub fn emit_structs_h(pir: &Module, out: &mut String) {
    for class in &pir.classes {
        emit_class_struct(class, out);
    }
    for func in &pir.functions {
        emit_nested_closure_structs(func, out);
    }
}

fn emit_class_struct(class: &ClassDef, out: &mut String) {
    let mut fields = HashSet::new();
    for method in &class.methods {
        if method.symbol.0.ends_with(".__init__") || method.symbol.0.contains(".__init__") {
            collect_self_fields(method, &mut fields);
        }
    }
    if fields.is_empty() {
        return;
    }
    let struct_name = class_struct_name(&class.symbol.0);
    let _ = writeln!(out, "typedef struct {struct_name} {{");
    let mut sorted: Vec<_> = fields.into_iter().collect();
    sorted.sort();
    for f in sorted {
        let _ = writeln!(out, "    int64_t {f};");
    }
    let _ = writeln!(out, "}} {struct_name}_t;", struct_name = struct_name);
    let _ = writeln!(out);
}

fn collect_self_fields(func: &FuncDef, fields: &mut HashSet<String>) {
    for block in &func.blocks {
        for op in &block.ops {
            if op.opcode == OpCode::StoreAttr {
                if let (Some(OpOperand::Name(attr)), Some(OpOperand::Value(_))) =
                    (op.operands.get(1), op.operands.get(2))
                {
                    fields.insert(attr.clone());
                }
            }
        }
    }
}

fn emit_nested_closure_structs(parent: &FuncDef, out: &mut String) {
    for nested in &parent.nested {
        if nested.cellvars.is_empty() {
            continue;
        }
        let base = closure_base_name(&parent.symbol.0, &nested.symbol.0);
        let env_name = format!("{base}_env");
        let _ = writeln!(out, "typedef struct {env_name} {{");
        for cell in &nested.cellvars {
            let _ = writeln!(out, "    int64_t {cell};");
        }
        let _ = writeln!(out, "}} {env_name}_t;");
        let fn_sym = skw_c_symbol(&nested.symbol.0);
        let _ = writeln!(
            out,
            "typedef int64_t (SKW_CALL *{base}_fn_t)({env_name}_t *env, int64_t x);"
        );
        let _ = writeln!(
            out,
            "typedef struct {base}_closure {{\n    {env_name}_t env;\n    {base}_fn_t fn;\n}} {base}_closure_t;\n"
        );
        let _ = fn_sym; // nested fn symbol reserved for future codegen wiring
    }
}

fn class_struct_name(class_symbol: &str) -> String {
    format!("skw_{}", class_symbol.replace('.', "_"))
}

fn closure_base_name(parent_sym: &str, nested_sym: &str) -> String {
    let parent = skw_c_symbol(parent_sym);
    let nested_name = nested_sym.rsplit('.').next().unwrap_or("inner");
    format!("{parent}_{nested_name}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sikuwa_pir::lower_source;

    #[test]
    fn emits_point_struct() {
        let src = include_str!("../../../tests/fixtures/plan3.py");
        let pir = lower_source(src, "plan3.py").unwrap();
        let mut out = String::new();
        emit_structs_h(&pir, &mut out);
        assert!(out.contains("skw_plan3_Point"));
        assert!(out.contains("int64_t x"));
    }
}
