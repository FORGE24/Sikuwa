//! Pass1 type evidence — apply `# skw @type` hints into HPGI.

use sikuwa_pir::module::{FuncDef, FuncTypeHint, Module};

use crate::diagnostic::PystatDiagnostic;
use crate::infer::{join, meet, LogicalType, SparseEnvironment};

pub fn parse_type_name(s: &str) -> LogicalType {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix("Optional[") {
        if let Some(rest) = inner.strip_suffix(']') {
            let inner_ty = parse_type_name(rest);
            if inner_ty.is_bottom() || inner_ty == LogicalType::Top {
                return LogicalType::Optional(Box::new(LogicalType::Top));
            }
            return LogicalType::Optional(Box::new(inner_ty));
        }
    }
    match s.to_ascii_lowercase().as_str() {
        "int" | "int64" => LogicalType::Int,
        "float" | "float64" | "double" => LogicalType::Float,
        "bool" => LogicalType::Bool,
        "str" | "string" | "char" | "void_ptr" => LogicalType::Str,
        "none" | "void" => LogicalType::None,
        "dyn" | "size_t" | "object" => LogicalType::Dyn,
        _ => LogicalType::Top,
    }
}

pub fn hint_for_func<'a>(module: &'a Module, func: &FuncDef) -> Option<&'a FuncTypeHint> {
    module.type_hints.get(&func.symbol.0)
}

pub fn seed_params_from_hint(
    func: &FuncDef,
    hint: Option<&FuncTypeHint>,
    slots: &mut SparseEnvironment,
) -> Vec<PystatDiagnostic> {
    let bound = hint.map(|h| h.bind_params(&func.params));

    for param in &func.params {
        if let Some(map) = &bound {
            if let Some(ty_str) = map.get(param) {
                let hinted = parse_type_name(ty_str);
                if !hinted.is_bottom() && hinted != LogicalType::Top {
                    slots.set_exact(param, hinted);
                    continue;
                }
            }
        }
        slots.seed(param);
    }
    Vec::new()
}

pub fn check_return_hint(
    func: &FuncDef,
    hint: Option<&FuncTypeHint>,
    inferred: &LogicalType,
) -> Vec<PystatDiagnostic> {
    let Some(hint) = hint else {
        return Vec::new();
    };
    let Some(ret_str) = hint.return_ty.as_ref() else {
        return Vec::new();
    };
    let hinted = parse_type_name(ret_str);
    if hinted.is_bottom() || hinted == LogicalType::Top {
        return Vec::new();
    }
    if meet(hinted.clone(), inferred.clone()).is_bottom() {
        return vec![PystatDiagnostic::t001(
            format!(
                "return type `{ret_str}` conflicts with inferred `{}`",
                logical_type_label(inferred)
            ),
            Some(func.symbol.0.clone()),
        )];
    }
    if join(hinted.clone(), inferred.clone()) == LogicalType::Dyn && hinted != LogicalType::Dyn {
        return vec![PystatDiagnostic::t001(
            format!(
                "return widened to dyn; annotation `{ret_str}` is narrower than inferred `{}`",
                logical_type_label(inferred)
            ),
            Some(func.symbol.0.clone()),
        )];
    }
    Vec::new()
}

fn logical_type_label(ty: &LogicalType) -> &'static str {
    match ty {
        LogicalType::Bottom => "bottom",
        LogicalType::Top => "top",
        LogicalType::Dyn => "dyn",
        LogicalType::None => "none",
        LogicalType::Bool => "bool",
        LogicalType::Int => "int",
        LogicalType::Float => "float",
        LogicalType::Str => "str",
        LogicalType::Literal(_) => "literal",
        LogicalType::Union(_) => "union",
        LogicalType::Optional(_) => "optional",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sikuwa_pir::lower_source;

    #[test]
    fn type_name_parsing() {
        assert_eq!(parse_type_name("int64"), LogicalType::Int);
        assert_eq!(parse_type_name("float"), LogicalType::Float);
    }

    #[test]
    fn seed_params_from_module_hint() {
        let src = "# skw @type add int int -> int\ndef add(a, b):\n    return a + b\n";
        let m = lower_source(src, "t.py").unwrap();
        let f = &m.functions[0];
        let hint = hint_for_func(&m, f).unwrap();
        let mut slots = SparseEnvironment::new();
        seed_params_from_hint(f, Some(hint), &mut slots);
        assert_eq!(slots.get("a"), LogicalType::Int);
        assert_eq!(slots.get("b"), LogicalType::Int);
    }
}
