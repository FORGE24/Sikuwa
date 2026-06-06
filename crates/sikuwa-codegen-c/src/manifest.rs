//! `.skw.json` module manifest (PGTE / FFI export metadata).

use serde::{Deserialize, Serialize};
use sikuwa_pir::Module;
use sikuwa_pystat::{FuncStat, PhysicalType, PystatReport, SlotLevel, SlotStrategy};

use crate::emit::skw_c_symbol;
use crate::imports::collect_manifest_imports;

pub const MANIFEST_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkwManifest {
    pub manifest_version: String,
    pub abi: String,
    pub module: String,
    pub source_hash: String,
    pub exports: Vec<SkwExport>,
    pub imports: Vec<crate::imports::SkwImport>,
    pub itr_slots: Vec<SkwItrSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkwExport {
    pub symbol: String,
    pub c_symbol: String,
    pub slot: String,
    pub signature: SkwSignature,
    pub static_eligible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkwSignature {
    pub params: Vec<SkwParam>,
    pub return_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkwParam {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkwItrSlot {
    pub logical: String,
    pub physical: String,
    pub strategy: String,
}

pub fn emit_manifest(pir: &Module, report: &PystatReport) -> SkwManifest {
    let mut exports = Vec::new();
    let mut itr_slots = Vec::new();
    let mut seen_itr = std::collections::HashSet::new();

    for f in &report.module.functions {
        if !f.static_eligible {
            continue;
        }
        exports.push(func_to_export(f));
        collect_itr(f, &mut itr_slots, &mut seen_itr);
    }

    SkwManifest {
        manifest_version: MANIFEST_VERSION.to_string(),
        abi: crate::emit::SKW_ABI_STRING.to_string(),
        module: pir.name.clone(),
        source_hash: hex32(pir.source_hash),
        exports,
        imports: collect_manifest_imports(pir),
        itr_slots,
    }
}

pub fn manifest_to_json(manifest: &SkwManifest) -> String {
    serde_json::to_string_pretty(manifest).expect("manifest json")
}

fn func_to_export(f: &FuncStat) -> SkwExport {
    SkwExport {
        symbol: f.symbol.0.clone(),
        c_symbol: skw_c_symbol(&f.symbol.0),
        slot: slot_level_str(f).to_string(),
        signature: SkwSignature {
            params: f
                .params
                .iter()
                .map(|p| SkwParam {
                    name: p.name.clone(),
                    ty: physical_type_str(p.ty),
                })
                .collect(),
            return_type: physical_type_str(f.return_ty),
        },
        static_eligible: f.static_eligible,
    }
}

fn collect_itr(
    f: &FuncStat,
    out: &mut Vec<SkwItrSlot>,
    seen: &mut std::collections::HashSet<String>,
) {
    for slot in f.params.iter().chain(f.locals.iter()) {
        if let SlotStrategy::Itr { primary } = slot.strategy {
            let key = format!("{}:{}", f.symbol.0, slot.name);
            if seen.insert(key) {
                out.push(SkwItrSlot {
                    logical: slot.name.clone(),
                    physical: physical_type_str(primary),
                    strategy: "itr".into(),
                });
            }
        }
    }
}

fn slot_level_str(f: &FuncStat) -> &'static str {
    if f.static_eligible {
        "S0"
    } else if f.params.iter().chain(f.locals.iter()).any(|s| s.level == SlotLevel::S3) {
        "S3"
    } else {
        "S1"
    }
}

fn physical_type_str(t: PhysicalType) -> String {
    match t {
        PhysicalType::None => "none".into(),
        PhysicalType::Bool => "bool".into(),
        PhysicalType::Int64 => "int64".into(),
        PhysicalType::Float64 => "float64".into(),
        PhysicalType::Str => "str".into(),
        PhysicalType::Object => "object".into(),
        PhysicalType::Dyn => "dyn".into(),
        PhysicalType::Unknown => "unknown".into(),
    }
}

fn hex32(bytes: [u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sikuwa_pir::sample_add_module;
    use sikuwa_pystat::analyze_module;

    #[test]
    fn manifest_has_skw_symbol() {
        let pir = sample_add_module();
        let report = analyze_module(&pir);
        let m = emit_manifest(&pir, &report);
        assert_eq!(m.exports[0].c_symbol, "skw_sample_add");
        assert!(manifest_to_json(&m).contains("itr"));
    }
}
