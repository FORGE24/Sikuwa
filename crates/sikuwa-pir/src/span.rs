use serde::{Deserialize, Serialize};

/// Source location for diagnostics and debug info.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Span {
    pub file: String,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl Span {
    pub fn unknown() -> Self {
        Self::default()
    }

    pub fn single_line(file: impl Into<String>, line: u32) -> Self {
        Self {
            file: file.into(),
            start_line: line,
            start_col: 0,
            end_line: line,
            end_col: 0,
        }
    }
}
