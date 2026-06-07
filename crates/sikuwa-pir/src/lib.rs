//! Sikuwa PythonIR (PIR) - version 1.0 alpha.


pub mod opt;

pub mod ids;

pub mod lower;

pub mod module;

pub mod opcode;

pub mod pirb;

pub mod span;

pub mod text;

pub mod verify;



pub use ids::{BlockId, SymbolRef, ValueId};

pub use lower::{lower_file, lower_source};

pub use module::{
    sample_add_module, Block, ClassDef, ExceptionRegion, ExternDecl, FuncDef, FuncTypeHint, Module,
    ModuleImport, Phi, PhiIncoming, Terminator,
};

pub use opcode::OpCode;

pub use pirb::{decode_module, encode_module, PirHeader, PIR_MAGIC, PIR_VERSION};

pub use span::Span;

pub use text::module_to_text;

pub use verify::{ensure_valid_module, verify_func, verify_module, VerifyReport};

pub use opt::{
    all_keyword_passes, optimize_func, optimize_module, pass_def_inline, pass_import_dce,
    run_module_passes, OptLevel, OptReport, PassRunReport, PythonKeyword,
};


