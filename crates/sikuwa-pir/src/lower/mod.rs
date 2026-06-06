//! AST → PythonIR lowering (Plan 2 prototype).

mod c_extern;
mod class;
mod expr;
mod function;
mod import;

use std::path::Path;

use rustpython_ast as ast;
use rustpython_parser::Parse;
use sikuwa_core::{Result, SikuwaError};

use crate::ids::SymbolRef;
use crate::module::{Module, ModuleImport};
use class::lower_class;
use c_extern::parse_directives;
use function::lower_function;
use import::{import_map, lower_import, module_locals};
use function::LowerContext;

/// Lower Python source to a PIR `Module`.
pub fn lower_source(source: &str, file_path: &str) -> Result<Module> {
    let mod_module = ast::ModModule::parse(source, file_path)
        .map_err(|e| SikuwaError::pir(format!("parse error in {file_path}: {e}")))?;

    let body = mod_module.body;

    let module_name = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module")
        .to_string();

    let (externs, c_includes) = parse_directives(source)?;
    let mut imports: Vec<ModuleImport> = Vec::new();
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut exports = Vec::new();
    for stmt in &body {
        if let Some(imps) = lower_import(stmt) {
            imports.extend(imps);
        }
    }
    let _sym_map = import_map(&imports);
    let _mod_locals = module_locals(&imports);
    let ctx = LowerContext::from_module(&externs, &imports);

    for stmt in body {
        match stmt {
            ast::Stmt::FunctionDef(fd) => {
                let sym = SymbolRef::new(format!("{module_name}.{}", fd.name));
                exports.push(sym.clone());
                functions.push(lower_function(
                    &module_name,
                    file_path,
                    source,
                    &fd,
                    None,
                    &ctx,
                )?);
            }
            ast::Stmt::ClassDef(cd) => {
                let class = lower_class(&module_name, file_path, source, &cd, &ctx)?;
                exports.push(class.symbol.clone());
                classes.push(class);
            }
            ast::Stmt::Import(_) | ast::Stmt::ImportFrom(_) => {}
            ast::Stmt::AsyncFunctionDef(_) => {
                return Err(SikuwaError::pir(format!(
                    "async def not supported yet ({file_path})"
                )));
            }
            _ => {}
        }
    }

    if functions.is_empty() && classes.is_empty() {
        return Err(SikuwaError::pir(format!(
            "no top-level `def` or `class` found in {file_path}"
        )));
    }

    Ok(Module {
        name: module_name,
        source_hash: Module::hash_source(source.as_bytes()),
        python_lang: "3.11".into(),
        exports,
        functions,
        classes,
        externs,
        imports,
        c_includes,
    })
}

/// Read a `.py` file and lower it to PIR.
pub fn lower_file(path: &Path) -> Result<Module> {
    let source = std::fs::read_to_string(path).map_err(SikuwaError::from)?;
    let file_path = path
        .to_str()
        .ok_or_else(|| SikuwaError::pir("non-utf8 path"))?;
    lower_source(&source, file_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify_module;

    #[test]
    fn lower_add_function() {
        let src = "def add(a, b):\n    return a + b\n";
        let module = lower_source(src, "add.py").unwrap();
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].symbol.0, "add.add");
        let report = verify_module(&module);
        assert!(report.ok(), "{:?}", report.errors);
    }

    #[test]
    fn lower_clamp_function() {
        let src = r#"def clamp(x, lo, hi):
    if x < lo:
        return lo
    if x > hi:
        return hi
    return x
"#;
        let module = lower_source(src, "clamp.py").unwrap();
        assert_eq!(module.functions.len(), 1);
        assert!(module.functions[0].blocks.len() >= 5);
        let report = verify_module(&module);
        assert!(report.ok(), "{:?}", report.errors);
    }

    #[test]
    fn lower_assign_and_while() {
        let src = include_str!("../../../../tests/fixtures/sum_range.py");
        let module = lower_source(src, "sum_range.py").unwrap();
        assert!(module.exports.iter().any(|e| e.0.contains("sum_range")));
        assert!(module.functions[0].locals.contains(&"total".to_string()));
        let report = verify_module(&module);
        assert!(report.ok(), "{:?}", report.errors);
    }

    #[test]
    fn lower_for_loop() {
        let src = include_str!("../../../../tests/fixtures/total.py");
        let module = lower_source(src, "total.py").unwrap();
        let f = &module.functions[0];
        assert!(f.blocks.iter().any(|b| b.id.0.starts_with("for_")));
        let report = verify_module(&module);
        assert!(report.ok(), "{:?}", report.errors);
    }

    #[test]
    fn lower_attr_and_subscript() {
        let src = r#"def get_item(d, k):
    return d[k]

def set_attr(obj, v):
    obj.x = v
    return obj.x
"#;
        let module = lower_source(src, "attrs.py").unwrap();
        let ops: Vec<_> = module.functions[0]
            .blocks
            .iter()
            .flat_map(|b| b.ops.iter())
            .map(|o| o.opcode)
            .collect();
        assert!(ops.contains(&crate::opcode::OpCode::SubscriptLoad));
        let f2 = &module.functions[1];
        let ops2: Vec<_> = f2
            .blocks
            .iter()
            .flat_map(|b| b.ops.iter())
            .map(|o| o.opcode)
            .collect();
        assert!(ops2.contains(&crate::opcode::OpCode::StoreAttr));
        assert!(ops2.contains(&crate::opcode::OpCode::LoadAttr));
    }

    #[test]
    fn lower_class_and_closure() {
        let src = r#"class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y

def make_adder(n):
    def add(x):
        return x + n
    return add
"#;
        let module = lower_source(src, "plan3.py").unwrap();
        assert_eq!(module.classes.len(), 1);
        assert_eq!(module.classes[0].methods.len(), 1);
        let make_adder = module.functions.iter().find(|f| f.symbol.0.ends_with("make_adder")).unwrap();
        assert!(!make_adder.nested.is_empty());
        assert!(make_adder.nested[0].blocks.iter().flat_map(|b| &b.ops).any(|o| {
            o.opcode == crate::opcode::OpCode::LoadCell
        }));
        let report = verify_module(&module);
        assert!(report.ok(), "{:?}", report.errors);
    }

    #[test]
    fn lower_c_extern_and_import() {
        let ext = include_str!("../../../../tests/fixtures/plan5_extern.py");
        let m = lower_source(ext, "plan5_extern.py").unwrap();
        assert_eq!(m.externs.len(), 1);
        assert!(m.c_includes.contains(&"string.h".to_string()));
        assert!(m.functions[0].blocks.iter().flat_map(|b| &b.ops).any(|o| {
            o.opcode == crate::opcode::OpCode::CallExtern
        }));

        let caller = include_str!("../../../../tests/fixtures/plan5_caller.py");
        let m2 = lower_source(caller, "plan5_caller.py").unwrap();
        assert!(m2.imports.iter().any(|i| i.symbol == "add.add"));
        assert!(m2.functions[0].blocks.iter().flat_map(|b| &b.ops).any(|o| {
            matches!(
                (&o.opcode, o.operands.first()),
                (crate::opcode::OpCode::Call, Some(crate::module::OpOperand::Symbol(s))) if s.0 == "add.add"
            )
        }));
    }
}
