//! Module-scoped PIR optimization passes.

use std::collections::HashSet;

use crate::module::{Module, OpOperand};
use crate::opcode::OpCode;

use super::inline::pass_def_inline;

/// Remove `ModuleImport` entries never referenced by a `call` in the module.
pub fn pass_import_dce(module: &mut Module) -> bool {
    let used = collect_used_import_symbols(module);
    let before = module.imports.len();
    module.imports.retain(|imp| used.contains(&imp.symbol) || used.contains(&imp.local));
    module.imports.len() != before
}

fn collect_used_import_symbols(module: &Module) -> HashSet<String> {
    let mut used = HashSet::new();
    for func in &module.functions {
        scan_func(&func, &mut used);
        for nested in &func.nested {
            scan_func(nested, &mut used);
        }
    }
    for class in &module.classes {
        for method in &class.methods {
            scan_func(method, &mut used);
        }
    }
    used
}

fn scan_func(func: &crate::module::FuncDef, used: &mut HashSet<String>) {
    for block in &func.blocks {
        for op in &block.ops {
            if !matches!(op.opcode, OpCode::Call | OpCode::CallExtern) {
                continue;
            }
            match op.operands.first() {
                Some(OpOperand::Symbol(sym)) => {
                    used.insert(sym.0.clone());
                    if let Some((mod_name, name)) = sym.0.rsplit_once('.') {
                        used.insert(mod_name.to_string());
                        used.insert(name.to_string());
                    }
                }
                Some(OpOperand::Name(n)) => {
                    used.insert(n.clone());
                }
                _ => {}
            }
        }
    }
}

/// O2 module pass bundle: def inline then import DCE.
pub fn run_module_passes(module: &mut Module) -> bool {
    let mut changed = pass_def_inline(module);
    if pass_import_dce(module) {
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::lower_source;

    #[test]
    fn removes_unused_import() {
        let src = r#"import os
from add import add

def twice(a, b):
    return add(a, b)
"#;
        let mut module = lower_source(src, "caller.py").unwrap();
        assert_eq!(module.imports.len(), 2);
        assert!(pass_import_dce(&mut module));
        assert_eq!(module.imports.len(), 1);
        assert!(module.imports[0].symbol.contains("add"));
    }
}
