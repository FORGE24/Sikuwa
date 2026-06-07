//! Slot tier → C type mapping (Plan 8b/8c).

use sikuwa_pir::module::{ExternDecl, FuncDef};
use sikuwa_pystat::{FuncStat, LogicalSlot, PhysicalType, SlotLevel};

use crate::closure::{class_struct_type, closure_return_type, is_class_init_method, is_closure_factory};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenTier {
    S0,
    S1,
    S3,
    Closure,
    ClassMethod,
}

pub fn tier_for(stat: &FuncStat, func: &FuncDef) -> Option<CodegenTier> {
    if crate::emit::func_has_unsupported_dyn_ops(func) {
        return Some(CodegenTier::S3);
    }
    if is_closure_factory(func) {
        return Some(CodegenTier::Closure);
    }
    if is_class_init_method(func) {
        return Some(CodegenTier::ClassMethod);
    }
    if stat.static_eligible {
        return Some(CodegenTier::S0);
    }
    let max = max_slot_level(stat);
    if max >= SlotLevel::S3
        || matches!(
            stat.return_ty,
            PhysicalType::Dyn | PhysicalType::Object | PhysicalType::Unknown
        )
    {
        return Some(CodegenTier::S3);
    }
    Some(CodegenTier::S1)
}

pub fn max_slot_level(stat: &FuncStat) -> SlotLevel {
    let mut max = SlotLevel::S0;
    for slot in stat.params.iter().chain(stat.locals.iter()) {
        if slot.level > max {
            max = slot.level;
        }
    }
    max
}

pub fn slot_c_type(slot: &LogicalSlot) -> &'static str {
    slot.ty.c_type_for_slot(slot.level)
}

pub fn return_c_type(stat: &FuncStat, tier: CodegenTier, func: &FuncDef) -> String {
    match tier {
        CodegenTier::S0 => stat.return_ty.c_type().to_string(),
        CodegenTier::S3 => "skw_value_t *".into(),
        CodegenTier::S1 => {
            if stat.return_ty.bit_width().is_some() {
                stat.return_ty.c_type().to_string()
            } else {
                "skw_tagged_t".into()
            }
        }
        CodegenTier::Closure => closure_return_type(func).unwrap_or_else(|| "skw_value_t *".into()),
        CodegenTier::ClassMethod => "void".into(),
    }
}

pub fn emit_params(stat: &FuncStat, func: &FuncDef, tier: CodegenTier) -> Vec<String> {
    match tier {
        CodegenTier::ClassMethod => {
            let struct_ty = class_struct_type(&func.symbol.0).unwrap_or_else(|| "skw_object".into());
            let mut out = vec![format!("{struct_ty}_t *self")];
            for p in func.params.iter().skip(1) {
                out.push(format!("int64_t {}", sanitize_param(p)));
            }
            out
        }
        CodegenTier::Closure => func
            .params
            .iter()
            .map(|p| format!("int64_t {}", sanitize_param(p)))
            .collect(),
        _ => stat
            .params
            .iter()
            .map(|p| format!("{} {}", slot_c_type(p), sanitize_param(&p.name)))
            .collect(),
    }
}

fn sanitize_param(name: &str) -> String {
    if name == "type" {
        "typ".into()
    } else {
        name.replace('.', "_")
    }
}

pub fn module_slot_label(tier: CodegenTier) -> &'static str {
    match tier {
        CodegenTier::S0 | CodegenTier::Closure | CodegenTier::ClassMethod => "SKW_SLOT_S0",
        CodegenTier::S1 => "SKW_SLOT_S1",
        CodegenTier::S3 => "SKW_SLOT_S3",
    }
}

pub fn extern_type_to_c(ty: &str) -> &'static str {
    match ty.trim().to_ascii_lowercase().as_str() {
        "int" | "int64" | "i64" => "int64_t",
        "bool" => "int64_t",
        "float" | "float64" | "double" => "double",
        "str" | "string" => "const char*",
        "void" | "none" => "void",
        _ => "int64_t",
    }
}

pub fn needs_hotpath_header(report: &sikuwa_pystat::PystatReport) -> bool {
    report
        .module
        .functions
        .iter()
        .any(|f| f.params.iter().any(|p| p.level == SlotLevel::S1))
}

pub fn collect_func_defs(pir: &sikuwa_pir::Module) -> Vec<&sikuwa_pir::module::FuncDef> {
    let mut out: Vec<_> = pir.functions.iter().collect();
    for class in &pir.classes {
        for method in &class.methods {
            out.push(method);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sikuwa_pir::ids::SymbolRef;
    use sikuwa_pystat::{LogicalSlot, SlotStrategy};

    fn slot(level: SlotLevel, ty: PhysicalType) -> LogicalSlot {
        LogicalSlot {
            name: "x".into(),
            ty,
            strategy: SlotStrategy::Itr { primary: ty },
            level,
            tagged: None,
        }
    }

    fn bare_func() -> FuncDef {
        FuncDef {
            symbol: SymbolRef::new("m.f"),
            params: vec![],
            locals: vec![],
            cellvars: vec![],
            nested: vec![],
            return_value: None,
            blocks: vec![],
            span: sikuwa_pir::span::Span::single_line("t.py", 1),
            exception_regions: vec![],
        }
    }

    #[test]
    fn s0_when_static_eligible() {
        let f = FuncStat {
            symbol: SymbolRef::new("m.f"),
            params: vec![slot(SlotLevel::S0, PhysicalType::Int64)],
            locals: vec![],
            return_ty: PhysicalType::Int64,
            static_eligible: true,
        };
        assert_eq!(tier_for(&f, &bare_func()), Some(CodegenTier::S0));
    }

    #[test]
    fn s3_when_dyn_slot() {
        let f = FuncStat {
            symbol: SymbolRef::new("m.f"),
            params: vec![slot(SlotLevel::S3, PhysicalType::Dyn)],
            locals: vec![],
            return_ty: PhysicalType::Dyn,
            static_eligible: false,
        };
        assert_eq!(tier_for(&f, &bare_func()), Some(CodegenTier::S3));
    }
}
