//! Sikuwa configuration schema v2 (Plan 1 subset).

use serde::{Deserialize, Serialize};
use sikuwa_core::{Result, SikuwaError};

pub const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootConfig {
    pub sikuwa: SikuwaSection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SikuwaSection {
    pub project_name: String,
    pub version: String,
    #[serde(default = "default_schema")]
    pub schema: u32,
    #[serde(default = "default_engine")]
    pub engine: String,
    pub main_script: Option<String>,
    #[serde(default = "default_compiler_mode")]
    pub compiler_mode: String,
    pub output_dir: Option<String>,
    pub build_dir: Option<String>,
    #[serde(default)]
    pub pir: PirSection,
    #[serde(default)]
    pub pystat: PystatSection,
    #[serde(default)]
    pub ffi: FfiSection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FfiSection {
    #[serde(default = "default_abi")]
    pub abi: String,
    #[serde(default = "default_true")]
    pub export_dll: bool,
    #[serde(default = "default_true")]
    pub export_module_desc: bool,
    #[serde(default = "default_true")]
    pub link_runtime: bool,
    #[serde(default = "default_false")]
    pub python_shim: bool,
    #[serde(default = "default_visibility")]
    pub visibility: String,
}

fn default_false() -> bool {
    false
}

impl Default for FfiSection {
    fn default() -> Self {
        Self {
            abi: default_abi(),
            export_dll: true,
            export_module_desc: true,
            link_runtime: true,
            python_shim: false,
            visibility: default_visibility(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PirSection {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PystatSection {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_pystat_mode")]
    pub mode: String,
    #[serde(default = "default_min_slot")]
    pub min_slot: String,
    #[serde(default = "default_true")]
    pub allow_dyn_fallback: bool,
}

impl Default for PirSection {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_dir: default_cache_dir(),
        }
    }
}

impl Default for PystatSection {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: default_pystat_mode(),
            min_slot: default_min_slot(),
            allow_dyn_fallback: true,
        }
    }
}

fn default_schema() -> u32 {
    SCHEMA_VERSION
}
fn default_engine() -> String {
    "a2".into()
}
fn default_compiler_mode() -> String {
    "native".into()
}
fn default_true() -> bool {
    true
}
fn default_cache_dir() -> String {
    ".sikuwa/pir".into()
}
fn default_pystat_mode() -> String {
    "progressive".into()
}
fn default_min_slot() -> String {
    "tagged".into()
}
fn default_abi() -> String {
    "1.0".into()
}
fn default_visibility() -> String {
    "hidden".into()
}

pub fn load_from_str(content: &str) -> Result<RootConfig> {
    toml::from_str(content).map_err(|e| SikuwaError::config(format!("TOML parse error: {e}")))
}

pub fn validate(config: &RootConfig) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    if config.sikuwa.schema != SCHEMA_VERSION {
        warnings.push(format!(
            "schema version {} != expected {}",
            config.sikuwa.schema, SCHEMA_VERSION
        ));
    }
    if config.sikuwa.engine != "a2" {
        warnings.push(format!(
            "engine '{}' is not Ver.A2 ('a2')",
            config.sikuwa.engine
        ));
    }
    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_v2() {
        let toml = r#"
[sikuwa]
project_name = "demo"
version = "0.1.0"
schema = 2
engine = "a2"
compiler_mode = "native"
"#;
        let cfg = load_from_str(toml).unwrap();
        assert_eq!(cfg.sikuwa.project_name, "demo");
        assert_eq!(cfg.sikuwa.schema, 2);
    }
}
