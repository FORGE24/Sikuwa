use serde::{Deserialize, Serialize};

use crate::ids::{BlockId, SymbolRef, ValueId};
use crate::opcode::OpCode;
use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub source_hash: [u8; 32],
    pub python_lang: String,
    /// Exported function symbols (`@module.func`).
    pub exports: Vec<SymbolRef>,
    pub functions: Vec<FuncDef>,
    pub classes: Vec<ClassDef>,
    /// `# skw @c_extern` declarations.
    #[serde(default)]
    pub externs: Vec<ExternDecl>,
    /// `import` / `from … import` metadata for cross-module FFI.
    #[serde(default)]
    pub imports: Vec<ModuleImport>,
    /// `# skw @c_include` headers required by externs.
    #[serde(default)]
    pub c_includes: Vec<String>,
    /// `# skw @type` hints keyed by full symbol (`module.func`).
    #[serde(default)]
    pub type_hints: std::collections::HashMap<String, FuncTypeHint>,
}

/// Pass1 type evidence from `# skw @type` directives.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FuncTypeHint {
    #[serde(default)]
    pub param_by_name: std::collections::HashMap<String, String>,
    /// Positional param types (aligned with `def` parameter order).
    #[serde(default)]
    pub param_types_pos: Vec<String>,
    #[serde(default)]
    pub return_ty: Option<String>,
}

impl FuncTypeHint {
    pub fn is_empty(&self) -> bool {
        self.param_by_name.is_empty() && self.param_types_pos.is_empty() && self.return_ty.is_none()
    }

    pub fn merge(&mut self, other: Self) {
        self.param_by_name.extend(other.param_by_name);
        if !other.param_types_pos.is_empty() {
            self.param_types_pos = other.param_types_pos;
        }
        if other.return_ty.is_some() {
            self.return_ty = other.return_ty;
        }
    }

    pub fn apply_positional_params(&mut self, types: &[String]) -> &mut Self {
        self.param_types_pos = types.to_vec();
        self
    }

    /// Positional types override name hints for the same parameter.
    pub fn bind_params(&self, param_names: &[String]) -> std::collections::HashMap<String, String> {
        let mut out = self.param_by_name.clone();
        for (name, ty) in param_names.iter().zip(self.param_types_pos.iter()) {
            out.insert(name.clone(), ty.clone());
        }
        out
    }
}

impl Module {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source_hash: [0u8; 32],
            python_lang: "3.11".into(),
            exports: Vec::new(),
            functions: Vec::new(),
            classes: Vec::new(),
            externs: Vec::new(),
            imports: Vec::new(),
            c_includes: Vec::new(),
            type_hints: std::collections::HashMap::new(),
        }
    }

    pub fn with_source_hash(mut self, hash: [u8; 32]) -> Self {
        self.source_hash = hash;
        self
    }

    pub fn hash_source(source: &[u8]) -> [u8; 32] {
        *blake3::hash(source).as_bytes()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionRegion {
    /// Basic blocks covered by `try` (normal control-flow).
    pub protected: Vec<BlockId>,
    /// Handler blocks (exceptional edges only).
    pub handlers: Vec<BlockId>,
    #[serde(default)]
    pub finally: Option<BlockId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuncDef {
    pub symbol: SymbolRef,
    pub params: Vec<String>,
    /// All local names (params + assigned), for PGTE / LogicalSlot.
    pub locals: Vec<String>,
    /// Free variables captured by nested closures in this function.
    pub cellvars: Vec<String>,
    /// Nested closure/function IR (lowered separately).
    pub nested: Vec<FuncDef>,
    pub return_value: Option<ValueId>,
    pub blocks: Vec<Block>,
    pub span: Span,
    /// `try` / `except` / `finally` regions for exceptional-edge pruning.
    #[serde(default)]
    pub exception_regions: Vec<ExceptionRegion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassDef {
    pub symbol: SymbolRef,
    pub name: String,
    pub bases: Vec<String>,
    pub methods: Vec<FuncDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternDecl {
    pub library: String,
    pub c_symbol: String,
    /// Python/local name used at call sites.
    pub name: String,
    pub return_ty: String,
    pub params: Vec<String>,
    pub param_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleImport {
    pub module: String,
    /// Full PIR symbol e.g. `add.add`.
    pub symbol: String,
    /// Local name in this module.
    pub local: String,
}

/// SSA phi node at block entry (merge point).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Phi {
    pub result: ValueId,
    /// Python local name (LogicalSlot key).
    pub name: String,
    pub incoming: Vec<PhiIncoming>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhiIncoming {
    pub block: BlockId,
    pub value: ValueId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub phis: Vec<Phi>,
    pub ops: Vec<Op>,
    pub term: Terminator,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Op {
    pub opcode: OpCode,
    pub result: Option<ValueId>,
    pub operands: Vec<OpOperand>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OpOperand {
    Value(ValueId),
    Symbol(SymbolRef),
    Const(ConstValue),
    Name(String),
    Block(BlockId),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstValue {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Terminator {
    Branch { target: BlockId },
    CondBranch {
        cond: ValueId,
        then_block: BlockId,
        else_block: BlockId,
    },
    Return { value: Option<ValueId> },
    Unreachable,
}

/// Minimal hand-built IR for tests and golden files.
pub fn sample_add_module() -> Module {
    let entry = BlockId::entry();
    let func = FuncDef {
        symbol: SymbolRef::new("sample.add"),
        params: vec!["a".into(), "b".into()],
        locals: vec!["a".into(), "b".into()],
        cellvars: Vec::new(),
        nested: Vec::new(),
        return_value: Some(ValueId(0)),
        blocks: vec![Block {
            id: entry.clone(),
            phis: Vec::new(),
            ops: vec![
                Op {
                    opcode: OpCode::BinOpAdd,
                    result: Some(ValueId(0)),
                    operands: vec![
                        OpOperand::Name("a".into()),
                        OpOperand::Name("b".into()),
                    ],
                    span: Span::single_line("sample.py", 1),
                },
            ],
            term: Terminator::Return {
                value: Some(ValueId(0)),
            },
        }],
        span: Span::single_line("sample.py", 1),
        exception_regions: Vec::new(),
    };

    Module {
        name: "sample".into(),
        source_hash: Module::hash_source(b"def add(a, b): return a + b\n"),
        python_lang: "3.11".into(),
        exports: vec![SymbolRef::new("sample.add")],
        functions: vec![func],
        classes: Vec::new(),
        externs: Vec::new(),
        imports: Vec::new(),
        c_includes: Vec::new(),
        type_hints: std::collections::HashMap::new(),
    }
}
