//! Python 3.11 keyword → PIR optimization pass registry (35 passes).
//!
//! Each pass operates purely on lowered SSA IR — no Python AST required.

/// One of Python 3.11's 35 reserved keywords.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PythonKeyword {
    False = 0,
    NoneKw = 1,
    True = 2,
    And = 3,
    As = 4,
    Assert = 5,
    Async = 6,
    Await = 7,
    Break = 8,
    Class = 9,
    Continue = 10,
    Def = 11,
    Del = 12,
    Elif = 13,
    Else = 14,
    Except = 15,
    Finally = 16,
    For = 17,
    From = 18,
    Global = 19,
    If = 20,
    Import = 21,
    In = 22,
    Is = 23,
    Lambda = 24,
    Nonlocal = 25,
    Not = 26,
    Or = 27,
    Pass = 28,
    Raise = 29,
    Return = 30,
    Try = 31,
    While = 32,
    With = 33,
    Yield = 34,
}

impl PythonKeyword {
    pub const ALL: [PythonKeyword; 35] = [
        Self::False,
        Self::NoneKw,
        Self::True,
        Self::And,
        Self::As,
        Self::Assert,
        Self::Async,
        Self::Await,
        Self::Break,
        Self::Class,
        Self::Continue,
        Self::Def,
        Self::Del,
        Self::Elif,
        Self::Else,
        Self::Except,
        Self::Finally,
        Self::For,
        Self::From,
        Self::Global,
        Self::If,
        Self::Import,
        Self::In,
        Self::Is,
        Self::Lambda,
        Self::Nonlocal,
        Self::Not,
        Self::Or,
        Self::Pass,
        Self::Raise,
        Self::Return,
        Self::Try,
        Self::While,
        Self::With,
        Self::Yield,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::False => "False",
            Self::NoneKw => "None",
            Self::True => "True",
            Self::And => "and",
            Self::As => "as",
            Self::Assert => "assert",
            Self::Async => "async",
            Self::Await => "await",
            Self::Break => "break",
            Self::Class => "class",
            Self::Continue => "continue",
            Self::Def => "def",
            Self::Del => "del",
            Self::Elif => "elif",
            Self::Else => "else",
            Self::Except => "except",
            Self::Finally => "finally",
            Self::For => "for",
            Self::From => "from",
            Self::Global => "global",
            Self::If => "if",
            Self::Import => "import",
            Self::In => "in",
            Self::Is => "is",
            Self::Lambda => "lambda",
            Self::Nonlocal => "nonlocal",
            Self::Not => "not",
            Self::Or => "or",
            Self::Pass => "pass",
            Self::Raise => "raise",
            Self::Return => "return",
            Self::Try => "try",
            Self::While => "while",
            Self::With => "with",
            Self::Yield => "yield",
        }
    }

    /// LLVM-style pass analogue for documentation / `--list-passes`.
    pub fn llvm_analog(self) -> &'static str {
        match self {
            Self::False | Self::NoneKw | Self::True => "constant-folding",
            Self::Not => "instcombine",
            Self::And | Self::Or => "instcombine (short-circuit)",
            Self::If | Self::Elif | Self::Else => "simplifycfg",
            Self::While | Self::For => "loop-simplify",
            Self::Break | Self::Continue => "loop-simplify (exit)",
            Self::Return => "tail-call-demotion / simplifycfg",
            Self::Pass => "dce (lowered no-op)",
            Self::Del => "dce",
            Self::Is | Self::In => "instcombine (compare)",
            Self::Def | Self::Lambda => "inline (analysis)",
            Self::Class => "globalopt",
            Self::Import | Self::From => "globaldce",
            Self::Global | Self::Nonlocal => "mem2reg / promote",
            Self::Assert | Self::Raise => "simplifycfg (exception)",
            Self::Try | Self::Except | Self::Finally => "simplifycfg (exception)",
            Self::Async | Self::Await => "coro-elide",
            Self::With | Self::Yield => "coro-split",
            Self::As => "instcombine (binding)",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::False | Self::NoneKw | Self::True => {
                "Fold literal comparisons and propagate const None/True/False"
            }
            Self::Not => "Fold `not` on constant booleans; eliminate double-negation",
            Self::And | Self::Or => "Short-circuit branch simplification on known conditions",
            Self::If | Self::Elif | Self::Else => {
                "Constant-fold conditional branches; remove unreachable blocks"
            }
            Self::While | Self::For => "Remove empty loops; fold constant loop conditions",
            Self::Break | Self::Continue => "Simplify loop exit edges",
            Self::Return => "Merge duplicate return blocks with identical values",
            Self::Pass => "No-op at IR level (already eliminated by lowering)",
            Self::Del => "Dead code elimination on unused SSA values",
            Self::Is | Self::In => "Fold identity / membership compares on constants",
            Self::Def | Self::Lambda => "Mark small functions for inlining (analysis only)",
            Self::Class => "Simplify class method dispatch metadata",
            Self::Import | Self::From => "Remove unused import metadata",
            Self::Global | Self::Nonlocal => "Promote known globals to fast locals where safe",
            Self::Assert | Self::Raise => "Remove provably-dead assert/raise paths",
            Self::Try | Self::Except | Self::Finally => "Prune unreachable exception edges",
            Self::Async | Self::Await => "Coroutine frame elision (planned)",
            Self::With | Self::Yield => "Generator/with-state cleanup (planned)",
            Self::As => "Remove redundant alias bindings in IR",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.name().eq_ignore_ascii_case(name))
    }
}

#[derive(Debug, Clone)]
pub struct KeywordPassInfo {
    pub keyword: PythonKeyword,
    pub name: &'static str,
    pub llvm_analog: &'static str,
    pub description: &'static str,
}

impl PythonKeyword {
    pub fn info(self) -> KeywordPassInfo {
        KeywordPassInfo {
            keyword: self,
            name: self.name(),
            llvm_analog: self.llvm_analog(),
            description: self.description(),
        }
    }
}
