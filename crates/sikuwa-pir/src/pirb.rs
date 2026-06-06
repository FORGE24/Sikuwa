use sikuwa_core::{Result, SikuwaError};

use crate::module::Module;

/// Magic bytes: `SIPIR` + version byte 1
pub const PIR_MAGIC: [u8; 6] = *b"SIPIR\x01";
pub const PIR_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PirHeader {
    pub magic: [u8; 6],
    pub version: u16,
    pub module_name_len: u32,
    pub payload_len: u32,
}

impl PirHeader {
    pub fn new(module_name_len: u32, payload_len: u32) -> Self {
        Self {
            magic: PIR_MAGIC,
            version: PIR_VERSION,
            module_name_len,
            payload_len,
        }
    }

    pub fn encode(&self) -> [u8; 16] {
        let mut buf = [0u8; 16];
        buf[0..6].copy_from_slice(&self.magic);
        buf[6..8].copy_from_slice(&self.version.to_le_bytes());
        buf[8..12].copy_from_slice(&self.module_name_len.to_le_bytes());
        buf[12..16].copy_from_slice(&self.payload_len.to_le_bytes());
        buf
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 16 {
            return Err(SikuwaError::pir("PIR header too short"));
        }
        let mut magic = [0u8; 6];
        magic.copy_from_slice(&bytes[0..6]);
        if magic != PIR_MAGIC {
            return Err(SikuwaError::pir("invalid PIR magic"));
        }
        let version = u16::from_le_bytes([bytes[6], bytes[7]]);
        let module_name_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let payload_len = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        Ok(Self {
            magic,
            version,
            module_name_len,
            payload_len,
        })
    }
}

/// Serialize module to `.pirb` bytes (header + JSON payload for Plan 1).
/// Binary sections will replace JSON in A2-beta.
pub fn encode_module(module: &Module) -> Result<Vec<u8>> {
    let payload = serde_json::to_vec(module)
        .map_err(|e| SikuwaError::pir(format!("encode payload: {e}")))?;
    let name_bytes = module.name.as_bytes();
    let header = PirHeader::new(name_bytes.len() as u32, payload.len() as u32);

    let mut out = Vec::with_capacity(16 + name_bytes.len() + payload.len());
    out.extend_from_slice(&header.encode());
    out.extend_from_slice(name_bytes);
    out.extend_from_slice(&payload);
    Ok(out)
}

pub fn decode_module(bytes: &[u8]) -> Result<Module> {
    let header = PirHeader::decode(bytes)?;
    let name_start = 16;
    let name_end = name_start + header.module_name_len as usize;
    let payload_end = name_end + header.payload_len as usize;
    if bytes.len() < payload_end {
        return Err(SikuwaError::pir("truncated PIR payload"));
    }
    let _name = std::str::from_utf8(&bytes[name_start..name_end])
        .map_err(|e| SikuwaError::pir(format!("module name utf8: {e}")))?;
    let payload = &bytes[name_end..payload_end];
    serde_json::from_slice(payload).map_err(|e| SikuwaError::pir(format!("decode payload: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::sample_add_module;

    #[test]
    fn roundtrip_sample_module() {
        let module = sample_add_module();
        let bytes = encode_module(&module).unwrap();
        let decoded = decode_module(&bytes).unwrap();
        assert_eq!(decoded.name, module.name);
        assert_eq!(decoded.functions.len(), 1);
    }
}
