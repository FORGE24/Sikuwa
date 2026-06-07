//! Merge Pass1 evidence: `# skw @type` > PEP 484 > `.pyi`.

use std::collections::HashMap;
use std::path::Path;

use sikuwa_core::Result;

use crate::module::FuncTypeHint;
use crate::lower::pep484::{extract_pep484_hints, load_pyi_hints};
use crate::lower::type_directive::resolve_type_hints;

/// Collect merged hints keyed by full symbol (`module.func`).
pub fn collect_type_hints(
    source: &str,
    file_path: &str,
    body: &[rustpython_ast::Stmt],
) -> Result<HashMap<String, FuncTypeHint>> {
    let module_name = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module");

    let skw = resolve_type_hints(source)?;
    let pep484 = extract_pep484_hints(body);
    let pyi = load_pyi_hints(file_path)?;

    let mut out = HashMap::new();
    for (name, hint) in merge_layers(&pyi, &pep484) {
        out.insert(format!("{module_name}.{name}"), hint);
    }
    for (name, hint) in skw {
        out
            .entry(format!("{module_name}.{name}"))
            .and_modify(|existing| existing.merge(hint.clone()))
            .or_insert(hint);
    }
    Ok(out)
}

fn merge_layers(
    pyi: &HashMap<String, FuncTypeHint>,
    pep484: &HashMap<String, FuncTypeHint>,
) -> HashMap<String, FuncTypeHint> {
    let mut out = pyi.clone();
    for (name, hint) in pep484 {
        out
            .entry(name.clone())
            .and_modify(|existing| existing.merge(hint.clone()))
            .or_insert_with(|| hint.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_ast as ast;
    use rustpython_parser::Parse;

    #[test]
    fn skw_overrides_pep484() {
        let src = "# skw @type add int int -> int\ndef add(a: str, b: str) -> str:\n    return a\n";
        let body = ast::ModModule::parse(src, "m.py").unwrap().body;
        let hints = collect_type_hints(src, "m.py", &body).unwrap();
        let h = hints.get("m.add").unwrap();
        let bound = h.bind_params(&["a".into(), "b".into()]);
        assert_eq!(bound.get("a").map(String::as_str), Some("int"));
        assert_eq!(h.return_ty.as_deref(), Some("int"));
    }
}
