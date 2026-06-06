use std::io::{Read, Write};
use std::path::Path;

use sikuwa_core::{Result, SikuwaError};

use crate::types::PystatModule;

pub const PSTAT_MAGIC: &[u8; 6] = b"SKPST\x01";

pub fn write_pstat(path: &Path, module: &PystatModule) -> Result<()> {
    let json = serde_json::to_vec_pretty(module)
        .map_err(|e| SikuwaError::pir(e.to_string()))?;
    let mut out = Vec::with_capacity(PSTAT_MAGIC.len() + json.len());
    out.extend_from_slice(PSTAT_MAGIC);
    out.extend(json);
    std::fs::write(path, out).map_err(SikuwaError::from)
}

pub fn read_pstat(path: &Path) -> Result<PystatModule> {
    let bytes = std::fs::read(path).map_err(SikuwaError::from)?;
    if bytes.len() < PSTAT_MAGIC.len() || &bytes[..PSTAT_MAGIC.len()] != PSTAT_MAGIC {
        return Err(SikuwaError::pir("invalid .pstat magic"));
    }
    serde_json::from_slice(&bytes[PSTAT_MAGIC.len()..]).map_err(|e| SikuwaError::pir(e.to_string()))
}

pub fn pstat_to_json(module: &PystatModule) -> Result<String> {
    serde_json::to_string_pretty(module).map_err(|e| SikuwaError::pir(e.to_string()))
}

pub fn pstat_from_reader(mut r: impl Read) -> Result<PystatModule> {
    let mut bytes = Vec::new();
    r.read_to_end(&mut bytes).map_err(SikuwaError::from)?;
    if bytes.len() < PSTAT_MAGIC.len() || &bytes[..PSTAT_MAGIC.len()] != PSTAT_MAGIC {
        return Err(SikuwaError::pir("invalid .pstat magic"));
    }
    serde_json::from_slice(&bytes[PSTAT_MAGIC.len()..]).map_err(|e| SikuwaError::pir(e.to_string()))
}

pub fn pstat_to_writer(module: &PystatModule, mut w: impl Write) -> Result<()> {
    w.write_all(PSTAT_MAGIC).map_err(SikuwaError::from)?;
    let json = serde_json::to_vec(module).map_err(|e| SikuwaError::pir(e.to_string()))?;
    w.write_all(&json).map_err(SikuwaError::from)
}
