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
    }
}
