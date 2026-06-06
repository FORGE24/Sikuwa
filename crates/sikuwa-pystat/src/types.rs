use serde::{Deserialize, Serialize};

use sikuwa_pir::ids::SymbolRef;

/// Physical representation width / kind (PGTE node).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhysicalType {
    None,
    Bool,
    Int64,
    Float64,
    Str,
    Object,
    Dyn,
    Unknown,
}

impl PhysicalType {
    pub fn bit_width(self) -> Option<u32> {
        match self {
            Self::Bool | Self::Int64 => Some(64),
            Self::Float64 => Some(64),
            Self::None => Some(0),
            Self::Str | Self::Object | Self::Dyn | Self::Unknown => None,
        }
    }

    pub fn merge(self, other: Self) -> Self {
        if self == other {
            return self;
        }
        if self == Self::Unknown {
            return other;
        }
        if other == Self::Unknown {
            return self;
        }
        if self.bit_width().is_some()
            && self.bit_width() == other.bit_width()
            && matches!(
                (self, other),
                (Self::Int64, Self::Bool)
                    | (Self::Bool, Self::Int64)
                    | (Self::Int64, Self::Int64)
                    | (Self::Bool, Self::Bool)
            )
        {
            // Same 64-bit slot — ITR candidate; keep wider int for codegen default.
            if self == Self::Int64 || other == Self::Int64 {
                Self::Int64
            } else {
                Self::Bool
            }
        } else {
            Self::Dyn
        }
    }

    pub fn c_type(self) -> &'static str {
        match self {
            Self::None => "void",
            Self::Bool => "int64_t",
            Self::Int64 => "int64_t",
            Self::Float64 => "double",
            Self::Str => "const char*",
            Self::Object | Self::Dyn | Self::Unknown => "sikuwa_value_t",
        }
    }
}

/// DTSS slot tier for codegen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotLevel {
    S0,
    S1,
    S2,
    S3,
}

/// How a LogicalSlot is materialized in the native frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotStrategy {
    /// In-place type replacement — reuse same stack slot across compatible types.
    Itr { primary: PhysicalType },
    /// Allocate a dedicated typed slot.
    Alloc { ty: PhysicalType },
    /// Dynamic fallback.
    Dyn,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogicalSlot {
    pub name: String,
    pub ty: PhysicalType,
    pub strategy: SlotStrategy,
    pub level: SlotLevel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuncStat {
    pub symbol: SymbolRef,
    pub params: Vec<LogicalSlot>,
    pub locals: Vec<LogicalSlot>,
    pub return_ty: PhysicalType,
    pub static_eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PystatModule {
    pub module: String,
    pub source_hash: [u8; 32],
    pub functions: Vec<FuncStat>,
}
