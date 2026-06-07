//! Parse `# skw @type` directives (Plan 7 Pass1).

use std::collections::HashMap;

use sikuwa_core::{Result, SikuwaError};

use crate::module::FuncTypeHint;

const TYPE_KEYWORDS: &[&str] = &[
    "int", "int64", "float", "float64", "double", "bool", "str", "string", "none", "dyn",
];

fn is_type_keyword(s: &str) -> bool {
    TYPE_KEYWORDS.contains(&s.to_ascii_lowercase().as_str())
}

/// Resolve `@type` directives keyed by bare Python function name (`add`, not `mod.add`).
pub fn resolve_type_hints(source: &str) -> Result<HashMap<String, FuncTypeHint>> {
    let mut resolved: HashMap<String, FuncTypeHint> = HashMap::new();
    let mut pending: FuncTypeHint = FuncTypeHint::default();

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(name) = parse_def_name(trimmed) {
            if !pending.is_empty() {
                resolved
                    .entry(name.clone())
                    .or_default()
                    .merge(pending.clone());
            }
            pending = FuncTypeHint::default();
            continue;
        }

        let Some(rest) = parse_skw_line(trimmed) else {
            continue;
        };
        let Some(rest) = rest.strip_prefix("@type") else {
            continue;
        };
        let rest = rest.trim();
        if rest.is_empty() {
            return Err(SikuwaError::pir("empty @type directive"));
        }

        if let Some((left, ret)) = rest.split_once("->") {
            let left = left.trim();
            let return_ty = ret.trim().to_string();
            if return_ty.is_empty() {
                return Err(SikuwaError::pir(format!("invalid @type return: `{rest}`")));
            }
            if left.is_empty() || left.eq_ignore_ascii_case("return") {
                pending.return_ty = Some(return_ty);
            } else {
                let parts: Vec<&str> = left.split_whitespace().collect();
                if parts.len() < 2 {
                    return Err(SikuwaError::pir(format!(
                        "invalid @type signature: `{rest}` (expected: NAME TYPE ... -> RET)"
                    )));
                }
                let func = parts[0].to_string();
                let param_types: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
                resolved
                    .entry(func)
                    .or_default()
                    .apply_positional_params(&param_types)
                    .return_ty = Some(return_ty);
            }
            continue;
        }

        let parts: Vec<&str> = rest.split_whitespace().collect();
        match parts.as_slice() {
            [ty] if is_type_keyword(ty) => {
                pending.return_ty = Some((*ty).to_string());
            }
            [name, ty] if is_type_keyword(ty) => {
                pending
                    .param_by_name
                    .insert(name.to_string(), (*ty).to_string());
            }
            [func, param_types @ ..]
                if param_types.len() >= 2
                    && param_types
                        .last()
                        .is_some_and(|s| is_type_keyword(s)) => {
                let func = (*func).to_string();
                let return_ty = param_types.last().unwrap().to_string();
                let param_types: Vec<String> = param_types[..param_types.len() - 1]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                let hint = resolved.entry(func).or_default();
                hint.apply_positional_params(&param_types);
                hint.return_ty = Some(return_ty);
            }
            _ => {
                return Err(SikuwaError::pir(format!(
                    "invalid @type directive: `{rest}`"
                )));
            }
        }
    }

    Ok(resolved)
}

fn parse_skw_line(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix('#')?.trim();
    Some(rest.strip_prefix("skw")?.trim())
}

fn parse_def_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let rest = trimmed
        .strip_prefix("async")
        .map(|s| s.trim())
        .unwrap_or(trimmed);
    if !rest.starts_with("def ") {
        return None;
    }
    let name = rest.strip_prefix("def ")?.split('(').next()?.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_function_signature_arrow() {
        let src = "# skw @type add int int -> int\ndef add(a, b):\n    return a + b\n";
        let hints = resolve_type_hints(src).unwrap();
        let h = hints.get("add").unwrap();
        assert_eq!(h.param_types_pos, vec!["int", "int"]);
        assert_eq!(h.return_ty.as_deref(), Some("int"));
    }

    #[test]
    fn parse_pending_param_and_return() {
        let src = "# skw @type a int\n# skw @type b int\n# skw @type -> int\ndef add(a, b):\n    pass\n";
        let hints = resolve_type_hints(src).unwrap();
        let h = hints.get("add").unwrap();
        assert_eq!(h.param_by_name.get("a").map(String::as_str), Some("int"));
        assert_eq!(h.param_by_name.get("b").map(String::as_str), Some("int"));
        assert_eq!(h.return_ty.as_deref(), Some("int"));
    }

    #[test]
    fn parse_shorthand_return_only() {
        let src = "# skw @type int\ndef f():\n    return 1\n";
        let hints = resolve_type_hints(src).unwrap();
        assert_eq!(hints.get("f").unwrap().return_ty.as_deref(), Some("int"));
    }
}
