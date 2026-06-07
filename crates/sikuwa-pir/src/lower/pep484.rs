//! PEP 484 annotations and `.pyi` stub collection (Plan 7 Pass1).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rustpython_ast as ast;
use rustpython_parser::Parse;
use sikuwa_core::{Result, SikuwaError};

use crate::module::FuncTypeHint;

/// Extract type hints from function `def` annotations in an AST body.
pub fn extract_pep484_hints(body: &[ast::Stmt]) -> HashMap<String, FuncTypeHint> {
    let mut out = HashMap::new();
    for stmt in body {
        match stmt {
            ast::Stmt::FunctionDef(fd) => {
                let hint = hint_from_function(fd);
                if !hint.is_empty() {
                    out.insert(fd.name.to_string(), hint);
                }
            }
            ast::Stmt::ClassDef(cd) => {
                for inner in &cd.body {
                    if let ast::Stmt::FunctionDef(fd) = inner {
                        let hint = hint_from_function(fd);
                        if !hint.is_empty() {
                            out.insert(fd.name.to_string(), hint);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Load `{stem}.pyi` adjacent to a `.py` path, if present.
pub fn load_pyi_hints(py_path: &str) -> Result<HashMap<String, FuncTypeHint>> {
    let path = Path::new(py_path);
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return Ok(HashMap::new());
    };
    let pyi_path: PathBuf = path.with_file_name(format!("{stem}.pyi"));
    if !pyi_path.is_file() {
        return Ok(HashMap::new());
    }
    let source = std::fs::read_to_string(&pyi_path).map_err(SikuwaError::from)?;
    let pyi_display = pyi_path.to_string_lossy();
    let mod_module = ast::ModModule::parse(&source, &pyi_display)
        .map_err(|e| SikuwaError::pir(format!("parse error in {pyi_display}: {e}")))?;
    Ok(extract_pep484_hints(&mod_module.body))
}

fn hint_from_function(fd: &ast::StmtFunctionDef) -> FuncTypeHint {
    let mut hint = FuncTypeHint::default();
    for arg in fd
        .args
        .posonlyargs
        .iter()
        .chain(&fd.args.args)
        .chain(&fd.args.kwonlyargs)
    {
        collect_arg_hint(&arg.def, &mut hint);
    }
    if let Some(ret) = &fd.returns {
        if let Some(ty) = expr_to_type_str(ret) {
            hint.return_ty = Some(ty);
        }
    }
    hint
}

fn collect_arg_hint(arg: &ast::Arg, hint: &mut FuncTypeHint) {
    if let Some(ann) = &arg.annotation {
        if let Some(ty) = expr_to_type_str(ann) {
            hint.param_by_name.insert(arg.arg.to_string(), ty);
        }
    }
}

fn expr_to_type_str(expr: &ast::Expr) -> Option<String> {
    match expr {
        ast::Expr::Name(n) => Some(normalize_type_token(&n.id)),
        ast::Expr::Constant(c) => match &c.value {
            ast::Constant::None => Some("none".into()),
            ast::Constant::Str(s) => Some(s.to_string()),
            _ => None,
        },
        ast::Expr::Subscript(sub) => {
            if let ast::Expr::Name(base) = &*sub.value {
                let inner = expr_to_type_str(&sub.slice);
                match base.id.as_str() {
                    "Optional" => inner.map(|t| format!("Optional[{t}]")),
                    "Union" => Some("dyn".into()),
                    "List" | "Dict" | "Tuple" | "Set" => Some("dyn".into()),
                    other => Some(normalize_type_token(other)),
                }
            } else {
                None
            }
        }
        ast::Expr::BinOp(b) if matches!(b.op, ast::Operator::BitOr) => {
            let _ = (expr_to_type_str(&b.left), expr_to_type_str(&b.right));
            Some("dyn".into())
        }
        ast::Expr::Tuple(t) => {
            if t.elts.is_empty() {
                None
            } else {
                Some("dyn".into())
            }
        }
        _ => None,
    }
}

fn normalize_type_token(name: &str) -> String {
    match name {
        "None" => "none".into(),
        other => other.to_ascii_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_parser::Parse;

    #[test]
    fn pep484_param_and_return() {
        let src = "def add(a: int, b: int) -> int:\n    return a + b\n";
        let m = ast::ModModule::parse(src, "t.py").unwrap();
        let hints = extract_pep484_hints(&m.body);
        let h = hints.get("add").unwrap();
        assert_eq!(h.param_by_name.get("a").map(String::as_str), Some("int"));
        assert_eq!(h.return_ty.as_deref(), Some("int"));
    }

    #[test]
    fn pep484_optional_union() {
        let src = "def f(x: int | None) -> None:\n    pass\n";
        let m = ast::ModModule::parse(src, "t.py").unwrap();
        let h = extract_pep484_hints(&m.body).remove("f").unwrap();
        assert_eq!(h.param_by_name.get("x").map(String::as_str), Some("dyn"));
        assert_eq!(h.return_ty.as_deref(), Some("none"));
    }
}
