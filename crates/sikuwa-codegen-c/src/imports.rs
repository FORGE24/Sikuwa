//! Collect cross-module / extern imports for manifest.

use std::collections::HashSet;

use sikuwa_pir::module::OpOperand;
use sikuwa_pir::OpCode;

use crate::emit::skw_c_symbol;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SkwImport {
    pub module: String,
    pub symbol: String,
    pub c_symbol: String,
    pub kind: String,
}

pub fn collect_manifest_imports(pir: &sikuwa_pir::Module) -> Vec<SkwImport> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for imp in &pir.imports {
        if imp.symbol.ends_with(".*") {
            let key = format!("module:{}", imp.module);
            if seen.insert(key) {
                out.push(SkwImport {
                    module: imp.module.clone(),
                    symbol: imp.symbol.clone(),
                    c_symbol: String::new(),
                    kind: "module".into(),
                });
            }
        } else {
            let key = format!("sym:{}", imp.symbol);
            if seen.insert(key) {
                out.push(SkwImport {
                    module: imp.module.clone(),
                    symbol: imp.symbol.clone(),
                    c_symbol: skw_c_symbol(&imp.symbol),
                    kind: "symbol".into(),
                });
            }
        }
    }

    for ext in &pir.externs {
        let key = format!("extern:{}", ext.c_symbol);
        if seen.insert(key) {
            out.push(SkwImport {
                module: ext.library.clone(),
                symbol: ext.name.clone(),
                c_symbol: ext.c_symbol.clone(),
                kind: "extern".into(),
            });
        }
    }

    for func in &pir.functions {
        scan_func_calls(func, &mut out, &mut seen);
        for nested in &func.nested {
            scan_func_calls(nested, &mut out, &mut seen);
        }
    }

    out.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    out
}

fn scan_func_calls(
    func: &sikuwa_pir::FuncDef,
    out: &mut Vec<SkwImport>,
    seen: &mut HashSet<String>,
) {
    for block in &func.blocks {
        for op in &block.ops {
            if matches!(op.opcode, OpCode::Call | OpCode::CallExtern) {
                if let Some(OpOperand::Symbol(sym)) = op.operands.first() {
                    let key = format!("call:{}", sym.0);
                    if seen.insert(key) {
                        out.push(SkwImport {
                            module: sym.0.split('.').next().unwrap_or("").into(),
                            symbol: sym.0.clone(),
                            c_symbol: skw_c_symbol(&sym.0),
                            kind: "call".into(),
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sikuwa_pir::lower_source;

    #[test]
    fn collects_from_import() {
        let src = r#"from add import add

def twice(a, b):
    return add(a, b)
"#;
        let pir = lower_source(src, "caller.py").unwrap();
        let imports = collect_manifest_imports(&pir);
        assert!(imports.iter().any(|i| i.symbol == "add.add"));
    }
}
