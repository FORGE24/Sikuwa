//! PyStat diagnostic codes (SKW-Txxx).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PystatDiagnostic {
    pub code: &'static str,
    pub message: String,
    pub symbol: Option<String>,
}

impl PystatDiagnostic {
    pub fn t001(message: impl Into<String>, symbol: Option<String>) -> Self {
        Self {
            code: "SKW-T001",
            message: message.into(),
            symbol,
        }
    }

    pub fn t003(message: impl Into<String>, symbol: Option<String>) -> Self {
        Self {
            code: "SKW-T003",
            message: message.into(),
            symbol,
        }
    }

    pub fn t002(message: impl Into<String>, symbol: Option<String>) -> Self {
        Self {
            code: "SKW-T002",
            message: message.into(),
            symbol,
        }
    }

    pub fn t004(message: impl Into<String>, symbol: Option<String>) -> Self {
        Self {
            code: "SKW-T004",
            message: message.into(),
            symbol,
        }
    }

    pub fn t005(message: impl Into<String>, symbol: Option<String>) -> Self {
        Self {
            code: "SKW-T005",
            message: message.into(),
            symbol,
        }
    }

    pub fn format_line(&self) -> String {
        match &self.symbol {
            Some(sym) => format!("{} ({}): {}", self.code, sym, self.message),
            None => format!("{}: {}", self.code, self.message),
        }
    }
}
