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
        use crate::infer::{from_physical, join, project_to_physical};
        project_to_physical(join(from_physical(self), from_physical(other)))
    }

    pub fn c_type(self) -> &'static str {
        match self {
            Self::None => "void",
            Self::Bool => "int64_t",
            Self::Int64 => "int64_t",
            Self::Float64 => "double",
            Self::Str => "const char*",
            Self::Object | Self::Dyn | Self::Unknown => "skw_value_t *",
        }
    }

    /// C type for a slot at the given DTSS tier.
    pub fn c_type_for_slot(self, level: SlotLevel) -> &'static str {
        match level {
            SlotLevel::S1 => "skw_tagged_t",
            SlotLevel::S2 => "skw_value_t *",
            SlotLevel::S3 => "skw_value_t *",
            SlotLevel::S0 => self.c_type(),
        }
    }
}

/// DTSS slot tier for codegen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SlotLevel {
    S0,
    S1,
    S2,
    S3,
}

/// S1 tagged union arms for codegen (`skw_tagged_t`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaggedLayout {
    pub arms: Vec<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tagged: Option<TaggedLayout>,
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
