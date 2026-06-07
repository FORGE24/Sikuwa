//! Emit C struct layouts for classes and closures.

use std::collections::HashSet;
use std::fmt::Write;

use sikuwa_pir::module::{ClassDef, FuncDef, Module, OpOperand};
use sikuwa_pir::OpCode;

use crate::closure::emit_closure_structs;

pub fn emit_structs_h(pir: &Module, out: &mut String) {
    for class in &pir.classes {
        emit_class_struct(class, out);
    }
    for func in &pir.functions {
        emit_closure_structs(func, out);
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

fn class_struct_name(class_symbol: &str) -> String {
    format!("skw_{}", class_symbol.replace('.', "_"))
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

    #[test]
    fn emits_closure_struct() {
        let src = include_str!("../../../tests/fixtures/plan3.py");
        let pir = lower_source(src, "plan3.py").unwrap();
        let make_adder = pir
            .functions
            .iter()
            .find(|f| f.symbol.0.ends_with("make_adder"))
            .unwrap();
        let mut out = String::new();
        emit_closure_structs(make_adder, &mut out);
        assert!(out.contains("skw_plan3_make_adder_add_env_t"));
        assert!(out.contains("skw_plan3_make_adder_add_closure_t"));
    }
}
