//! Parse `# skw @c_extern` / `# skw @c_include` directives.

use sikuwa_core::{Result, SikuwaError};

use crate::module::ExternDecl;

const C_TYPE_KEYWORDS: &[&str] = &[
    "int", "int64", "float", "float64", "double", "bool", "str", "string", "void", "none", "dyn",
    "char", "size_t", "void_ptr",
];

fn is_c_type_keyword(s: &str) -> bool {
    C_TYPE_KEYWORDS.contains(&s.to_ascii_lowercase().as_str())
}

/// Parse extern/include directives from Python source comments.
pub fn parse_directives(source: &str) -> Result<(Vec<ExternDecl>, Vec<String>)> {
    let mut externs = Vec::new();
    let mut includes = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("#") else {
            continue;
        };
        let rest = rest.trim();
        let Some(rest) = rest.strip_prefix("skw") else {
            continue;
        };
        let rest = rest.trim();
        if let Some(inc) = rest.strip_prefix("@c_include") {
            let header = inc.trim();
            if !header.is_empty() {
                includes.push(header.to_string());
            }
            continue;
        }
        if let Some(spec) = rest.strip_prefix("@c_extern") {
            externs.push(parse_extern_line(spec.trim())?);
        }
    }

    Ok((externs, includes))
}

/// `# skw @c_extern libc strlen int64 s:str`
/// or `# skw @c_extern libc.strlen(s: str) -> int64`
fn parse_extern_line(spec: &str) -> Result<ExternDecl> {
    if spec.contains("->") {
        return parse_extern_arrow(spec);
    }
    let parts: Vec<&str> = spec.split_whitespace().collect();
    if parts.len() < 4 {
        return Err(SikuwaError::pir(format!(
            "invalid @c_extern directive: `{spec}` (expected: LIB CNAME RET PARAM[:TYPE]...)"
        )));
    }
    let library = parts[0].to_string();
    let c_symbol = parts[1].to_string();
    let return_ty = parts[2].to_string();
    let (params, param_types) = parse_param_tokens(&parts[3..])?;
    Ok(ExternDecl {
        library,
        c_symbol: c_symbol.clone(),
        name: c_symbol,
        return_ty,
        params,
        param_types,
    })
}

fn parse_extern_arrow(spec: &str) -> Result<ExternDecl> {
    let (left, ret) = spec
        .split_once("->")
        .ok_or_else(|| SikuwaError::pir(format!("invalid @c_extern: `{spec}`")))?;
    let return_ty = ret.trim().to_string();
    let left = left.trim();
    let (lib_sym, params_part) = left
        .split_once('(')
        .ok_or_else(|| SikuwaError::pir(format!("invalid @c_extern: `{spec}`")))?;
    let params_part = params_part.trim_end_matches(')').trim();
    let (library, c_symbol) = lib_sym
        .split_once('.')
        .ok_or_else(|| SikuwaError::pir(format!("invalid @c_extern lib.sym: `{lib_sym}`")))?;
    let tokens: Vec<&str> = if params_part.is_empty() {
        Vec::new()
    } else {
        params_part.split(',').map(|s| s.trim()).collect()
    };
    let (params, param_types) = parse_param_tokens(&tokens)?;
    Ok(ExternDecl {
        library: library.to_string(),
        c_symbol: c_symbol.to_string(),
        name: c_symbol.to_string(),
        return_ty,
        params,
        param_types,
    })
}

/// Parse `name`, `name:type`, or `type name` tokens.
fn parse_param_tokens(tokens: &[&str]) -> Result<(Vec<String>, Vec<String>)> {
    let mut params = Vec::new();
    let mut param_types = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        if let Some((name, ty)) = token.split_once(':') {
            params.push(name.trim().to_string());
            param_types.push(ty.trim().to_string());
            i += 1;
            continue;
        }
        if is_c_type_keyword(token) && i + 1 < tokens.len() && !tokens[i + 1].contains(':') {
            params.push(tokens[i + 1].to_string());
            param_types.push(token.to_ascii_lowercase());
            i += 2;
            continue;
        }
        params.push(token.to_string());
        param_types.push(default_param_type(token));
        i += 1;
    }
    Ok((params, param_types))
}

fn default_param_type(name: &str) -> String {
    match name.to_ascii_lowercase().as_str() {
        "s" | "text" | "msg" => "str".into(),
        _ => "int64".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_extern_space_form() {
        let (ext, inc) = parse_directives("# skw @c_extern libc strlen int64 s\n").unwrap();
        assert_eq!(ext[0].c_symbol, "strlen");
        assert_eq!(ext[0].param_types, vec!["str"]);
        assert!(inc.is_empty());
    }

    #[test]
    fn parse_extern_typed_param() {
        let (ext, _) = parse_directives("# skw @c_extern libc memcpy int64 dst:int64 src:int64 n:size_t\n").unwrap();
        assert_eq!(ext[0].params, vec!["dst", "src", "n"]);
        assert_eq!(
            ext[0].param_types,
            vec!["int64", "int64", "size_t"]
        );
    }

    #[test]
    fn parse_extern_arrow_typed() {
        let (ext, _) =
            parse_directives("# skw @c_extern libc.strlen(s: str) -> int64\n").unwrap();
        assert_eq!(ext[0].params, vec!["s"]);
        assert_eq!(ext[0].param_types, vec!["str"]);
        assert_eq!(ext[0].return_ty, "int64");
    }

    #[test]
    fn parse_include() {
        let (_, inc) = parse_directives("# skw @c_include string.h\n").unwrap();
        assert_eq!(inc[0], "string.h");
    }
}
