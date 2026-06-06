//! Parse `# skw @c_extern` / `# skw @c_include` directives.

use sikuwa_core::{Result, SikuwaError};

use crate::module::ExternDecl;

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

/// `# skw @c_extern libc strlen int64 s` or `# skw @c_extern libc.strlen(s) -> int64`
fn parse_extern_line(spec: &str) -> Result<ExternDecl> {
    if spec.contains("->") {
        return parse_extern_arrow(spec);
    }
    let parts: Vec<&str> = spec.split_whitespace().collect();
    if parts.len() < 4 {
        return Err(SikuwaError::pir(format!(
            "invalid @c_extern directive: `{spec}` (expected: LIB CNAME RET PARAM...)"
        )));
    }
    let library = parts[0].to_string();
    let c_symbol = parts[1].to_string();
    let return_ty = parts[2].to_string();
    let params: Vec<String> = parts[3..].iter().map(|s| s.to_string()).collect();
    let param_types: Vec<String> = params
        .iter()
        .map(|_| "int64".to_string())
        .collect();
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
    let params: Vec<String> = if params_part.is_empty() {
        Vec::new()
    } else {
        params_part.split(',').map(|s| s.trim().to_string()).collect()
    };
    let param_types: Vec<String> = params.iter().map(|_| "int64".to_string()).collect();
    Ok(ExternDecl {
        library: library.to_string(),
        c_symbol: c_symbol.to_string(),
        name: c_symbol.to_string(),
        return_ty,
        params,
        param_types,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_extern_space_form() {
        let (ext, inc) = parse_directives("# skw @c_extern libc strlen int64 s\n").unwrap();
        assert_eq!(ext[0].c_symbol, "strlen");
        assert!(inc.is_empty());
    }

    #[test]
    fn parse_include() {
        let (_, inc) = parse_directives("# skw @c_include string.h\n").unwrap();
        assert_eq!(inc[0], "string.h");
    }
}
