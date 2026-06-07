//! PyStat analysis options (Plan 8 — maps to `[sikuwa.pystat]` in TOML).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PystatMode {
    /// Require S0 static eligibility; emit SKW-T002 otherwise.
    Strict,
    /// Allow downgrade when `allow_dyn_fallback` is set.
    Progressive,
    /// Like progressive; reserved for legacy Python 1.x compat heuristics.
    Compat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MinSlot {
    Static,
    Tagged,
    Dyn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PystatOptions {
    pub mode: PystatMode,
    pub min_slot: MinSlot,
    pub allow_dyn_fallback: bool,
}

impl Default for PystatOptions {
    fn default() -> Self {
        Self {
            mode: PystatMode::Progressive,
            min_slot: MinSlot::Static,
            allow_dyn_fallback: true,
        }
    }
}

impl PystatOptions {
    pub fn strict() -> Self {
        Self {
            mode: PystatMode::Strict,
            min_slot: MinSlot::Static,
            allow_dyn_fallback: false,
        }
    }

    pub fn from_mode_str(mode: &str) -> PystatMode {
        match mode.to_ascii_lowercase().as_str() {
            "strict" => PystatMode::Strict,
            "compat" => PystatMode::Compat,
            _ => PystatMode::Progressive,
        }
    }

    pub fn from_strings(mode: &str, min_slot: &str, allow_dyn_fallback: bool) -> Self {
        Self {
            mode: Self::from_mode_str(mode),
            min_slot: Self::from_min_slot_str(min_slot),
            allow_dyn_fallback,
        }
    }

    pub fn from_min_slot_str(s: &str) -> MinSlot {
        match s.to_ascii_lowercase().as_str() {
            "static" | "s0" => MinSlot::Static,
            "dyn" | "s3" => MinSlot::Dyn,
            _ => MinSlot::Tagged,
        }
    }

    pub fn from_config_section(mode: &str, min_slot: &str, allow_dyn_fallback: bool) -> Self {
        Self::from_strings(mode, min_slot, allow_dyn_fallback)
    }
}

impl MinSlot {
    pub fn floor_level(self) -> crate::types::SlotLevel {
        use crate::types::SlotLevel;
        match self {
            MinSlot::Static => SlotLevel::S0,
            MinSlot::Tagged => SlotLevel::S1,
            MinSlot::Dyn => SlotLevel::S3,
        }
    }
}
