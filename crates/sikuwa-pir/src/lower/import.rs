//! Lower `import` statements to `ModuleImport` metadata.

use rustpython_ast as ast;

use crate::module::ModuleImport;

pub fn lower_import(stmt: &ast::Stmt) -> Option<Vec<ModuleImport>> {
    match stmt {
        ast::Stmt::Import(imp) => {
            let mut out = Vec::new();
            for alias in &imp.names {
                let module = alias.name.to_string();
                let local = alias
                    .asname
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| module.clone());
                out.push(ModuleImport {
                    module: module.clone(),
                    symbol: format!("{module}.*"),
                    local,
                });
            }
            Some(out)
        }
        ast::Stmt::ImportFrom(imp) => {
            let module = imp.module.as_ref().map(|m| m.to_string()).unwrap_or_default();
            let mut out = Vec::new();
            for alias in &imp.names {
                let name = alias.name.to_string();
                let local = alias
                    .asname
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| name.clone());
                out.push(ModuleImport {
                    module: module.clone(),
                    symbol: if module.is_empty() {
                        name.clone()
                    } else {
                        format!("{module}.{name}")
                    },
                    local,
                });
            }
            Some(out)
        }
        _ => None,
    }
}

pub fn import_map(imports: &[ModuleImport]) -> std::collections::HashMap<String, String> {
    imports
        .iter()
        .filter(|i| !i.symbol.ends_with(".*"))
        .map(|i| (i.local.clone(), i.symbol.clone()))
        .collect()
}

pub fn module_locals(imports: &[ModuleImport]) -> std::collections::HashSet<String> {
    imports
        .iter()
        .filter(|i| i.symbol.ends_with(".*"))
        .map(|i| i.local.clone())
        .collect()
}
