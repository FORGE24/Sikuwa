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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tagged_slots: Vec<SkwTaggedSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkwTaggedSlot {
    pub function: String,
    pub name: String,
    pub level: String,
    pub physical: String,
    pub tagged_arms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkwExport {
    pub symbol: String,
    pub c_symbol: String,
    pub slot: String,
    pub signature: SkwSignature,
    pub static_eligible: bool,
    /// Intentional ABI break vs prior manifest (RFC native-c-ffi).
    #[serde(default, skip_serializing_if = "is_false")]
    pub abi_breaking: bool,
}

fn is_false(v: &bool) -> bool {
    !*v
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
    let mut tagged_slots = Vec::new();
    let mut seen_itr = std::collections::HashSet::new();
    let mut seen_tagged = std::collections::HashSet::new();

    for f in &report.module.functions {
        if f.static_eligible {
            exports.push(func_to_export(f));
        }
        collect_itr(f, &mut itr_slots, &mut seen_itr);
        collect_tagged(f, &mut tagged_slots, &mut seen_tagged);
    }

    SkwManifest {
        manifest_version: MANIFEST_VERSION.to_string(),
        abi: crate::emit::SKW_ABI_STRING.to_string(),
        module: pir.name.clone(),
        source_hash: hex32(pir.source_hash),
        exports,
        imports: collect_manifest_imports(pir),
        itr_slots,
        tagged_slots,
    }
}

pub fn manifest_to_json(manifest: &SkwManifest) -> String {
    serde_json::to_string_pretty(manifest).expect("manifest json")
}

pub fn load_manifest(path: &std::path::Path) -> Result<SkwManifest, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// Load golden baseline manifest (alias for CI preset).
pub fn load_baseline_manifest(path: &std::path::Path) -> Result<SkwManifest, String> {
    load_manifest(path)
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
        abi_breaking: false,
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

fn collect_tagged(
    f: &FuncStat,
    out: &mut Vec<SkwTaggedSlot>,
    seen: &mut std::collections::HashSet<String>,
) {
    for slot in f.params.iter().chain(f.locals.iter()) {
        if let Some(tagged) = &slot.tagged {
            let key = format!("{}:{}", f.symbol.0, slot.name);
            if seen.insert(key) {
                out.push(SkwTaggedSlot {
                    function: f.symbol.0.clone(),
                    name: slot.name.clone(),
                    level: slot_level_label(slot.level).to_string(),
                    physical: physical_type_str(slot.ty),
                    tagged_arms: tagged.arms.clone(),
                });
            }
        }
    }
}

fn slot_level_label(level: SlotLevel) -> &'static str {
    match level {
        SlotLevel::S0 => "S0",
        SlotLevel::S1 => "S1",
        SlotLevel::S2 => "S2",
        SlotLevel::S3 => "S3",
    }
}

pub fn slot_level_str(f: &FuncStat) -> &'static str {
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
        assert!(!m.exports[0].abi_breaking);
        let json = manifest_to_json(&m);
        assert!(json.contains("itr"));
        assert!(!json.contains("abi_breaking"));
    }
}
