//! Sikuwa-C codegen — emit native C from PIR + PyStat analysis.

mod emit;
mod imports;
mod manifest;
mod py_shim;
mod structs;

pub use emit::{
    emit_module_c, emit_module_h, module_c_name, skw_c_symbol, CodegenOptions, SKW_ABI_STRING,
};
pub use imports::{collect_manifest_imports, SkwImport};
pub use manifest::{emit_manifest, manifest_to_json, SkwManifest};
pub use py_shim::emit_pywrap_c;
pub use structs::emit_structs_h;
