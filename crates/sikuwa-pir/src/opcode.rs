use serde::{Deserialize, Serialize};

/// PIR instruction opcodes (v1.0 subset — Plan 1 / A2-alpha).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum OpCode {
    // Arithmetic
    BinOpAdd = 1,
    BinOpSub = 2,
    BinOpMul = 3,
    BinOpTrueDiv = 4,
    BinOpFloorDiv = 5,
    BinOpMod = 6,
    BinOpBitAnd = 7,
    BinOpRShift = 8,
    UnaryNeg = 10,
    UnaryNot = 11,

    // Comparison
    CompareLt = 20,
    CompareLe = 21,
    CompareGt = 22,
    CompareGe = 23,
    CompareEq = 24,
    CompareNe = 25,
    CompareIs = 26,
    CompareIsNot = 27,

    // Memory / globals
    LoadGlobal = 40,
    StoreGlobal = 41,
    LoadFast = 42,
    StoreFast = 43,

    // Object / container
    LoadAttr = 51,
    StoreAttr = 52,
    SubscriptLoad = 53,
    SubscriptStore = 54,
    LoadCell = 55,
    StoreCell = 56,

    // Calls / closures
    Call = 60,
    CallBuiltin = 61,
    CallExtern = 62,
    MakeClosure = 63,
    CallIndirect = 64,

    // Constants / building
    Const = 80,
    BuildTuple = 81,
    BuildList = 82,
    BuildMap = 83,
    BuildClass = 84,

    // Control / iteration
    GetIter = 90,
    ForIterNext = 91,

    // SSA
    Phi = 45,

    // Intrinsics (Sikuwa extensions)
    IntrinsicTypeOf = 200,
    DebugSloc = 201,
}

impl OpCode {
    pub fn name(self) -> &'static str {
        match self {
            Self::BinOpAdd => "binop_add",
            Self::BinOpSub => "binop_sub",
            Self::BinOpMul => "binop_mul",
            Self::BinOpTrueDiv => "binop_truediv",
            Self::BinOpFloorDiv => "binop_floordiv",
            Self::BinOpMod => "binop_mod",
            Self::BinOpBitAnd => "binop_bitand",
            Self::BinOpRShift => "binop_rshift",
            Self::UnaryNeg => "unary_neg",
            Self::UnaryNot => "unary_not",
            Self::CompareLt => "compare_lt",
            Self::CompareLe => "compare_le",
            Self::CompareGt => "compare_gt",
            Self::CompareGe => "compare_ge",
            Self::CompareEq => "compare_eq",
            Self::CompareNe => "compare_ne",
            Self::CompareIs => "compare_is",
            Self::CompareIsNot => "compare_is_not",
            Self::LoadGlobal => "load_global",
            Self::StoreGlobal => "store_global",
            Self::LoadFast => "load_fast",
            Self::StoreFast => "store_fast",
            Self::LoadAttr => "load_attr",
            Self::StoreAttr => "store_attr",
            Self::SubscriptLoad => "subscript_load",
            Self::SubscriptStore => "subscript_store",
            Self::LoadCell => "load_cell",
            Self::StoreCell => "store_cell",
            Self::Call => "call",
            Self::CallBuiltin => "call_builtin",
            Self::CallExtern => "call_extern",
            Self::MakeClosure => "make_closure",
            Self::CallIndirect => "call_indirect",
            Self::Const => "const",
            Self::BuildTuple => "build_tuple",
            Self::BuildList => "build_list",
            Self::BuildMap => "build_map",
            Self::BuildClass => "build_class",
            Self::GetIter => "get_iter",
            Self::ForIterNext => "for_iter_next",
            Self::Phi => "phi",
            Self::IntrinsicTypeOf => "intrinsic_typeof",
            Self::DebugSloc => "debug_sloc",
        }
    }
}
