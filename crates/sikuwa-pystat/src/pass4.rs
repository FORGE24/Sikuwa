//! Pass4 — interprocedural constraint checks (Call arity vs callee signature).

use std::collections::HashMap;

use sikuwa_pir::module::{FuncDef, Module, Op, OpOperand};
use sikuwa_pir::opcode::OpCode;

use crate::diagnostic::PystatDiagnostic;

pub fn pass4_module_diagnostics(module: &Module) -> Vec<PystatDiagnostic> {
    let funcs = module_func_map(module);
    let mut diags = Vec::new();
    for func in module
        .functions
        .iter()
        .chain(module.classes.iter().flat_map(|c| c.methods.iter()))
    {
        for block in &func.blocks {
            for op in &block.ops {
                if op.opcode != OpCode::Call {
                    continue;
                }
                let Some(sym) = resolve_call(func, op) else {
                    continue;
                };
                let Some(callee) = funcs.get(&sym.0) else {
                    continue;
                };
                let arg_count = op.operands.len().saturating_sub(1);
                let param_count = callee.params.len();
                if arg_count != param_count {
                    diags.push(PystatDiagnostic::t005(
                        format!(
                            "call `{}`: {} arg(s) vs callee {} param(s)",
                            sym.0, arg_count, param_count
                        ),
                        Some(func.symbol.0.clone()),
                    ));
                }
            }
        }
        for nested in &func.nested {
            diags.extend(pass4_nested_calls(func, nested, &funcs));
        }
    }
    diags
}

fn pass4_nested_calls(
    parent: &FuncDef,
    nested: &FuncDef,
    funcs: &HashMap<String, &FuncDef>,
) -> Vec<PystatDiagnostic> {
    let mut diags = Vec::new();
    for block in &nested.blocks {
        for op in &block.ops {
            if op.opcode != OpCode::Call {
                continue;
            }
            let Some(sym) = resolve_call(parent, op) else {
                continue;
            };
            let Some(callee) = funcs.get(&sym.0) else {
                continue;
            };
            let arg_count = op.operands.len().saturating_sub(1);
            if arg_count != callee.params.len() {
                diags.push(PystatDiagnostic::t005(
                    format!(
                        "nested call `{}`: arity mismatch ({} vs {})",
                        sym.0,
                        arg_count,
                        callee.params.len()
                    ),
                    Some(nested.symbol.0.clone()),
                ));
            }
        }
    }
    diags
}

fn module_func_map(module: &Module) -> HashMap<String, &FuncDef> {
    let mut map = HashMap::new();
    for f in &module.functions {
        map.insert(f.symbol.0.clone(), f);
    }
    for class in &module.classes {
        for m in &class.methods {
            map.insert(m.symbol.0.clone(), m);
        }
    }
    for f in &module.functions {
        for n in &f.nested {
            map.insert(n.symbol.0.clone(), n);
        }
    }
    map
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
    use sikuwa_pir::lower_source;

    #[test]
    fn add_module_has_no_pass4_violations() {
        let m = lower_source("def add(a, b):\n    return a + b\n", "add.py").unwrap();
        assert!(pass4_module_diagnostics(&m).is_empty());
    }
}
