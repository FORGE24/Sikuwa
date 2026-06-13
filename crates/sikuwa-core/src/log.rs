//! Compiler logging — tier-aware build diagnostics (default: Verbose).

use std::sync::OnceLock;

/// Minimum level for compiler diagnostics on stderr/stdout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Quiet = 0,
    Normal = 1,
    Verbose = 2,
    Trace = 3,
}

impl LogLevel {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "quiet" | "q" | "0" => Self::Quiet,
            "normal" | "info" | "1" => Self::Normal,
            "verbose" | "v" | "2" => Self::Verbose,
            "trace" | "debug" | "3" => Self::Trace,
            _ => return None,
        })
    }
}

static LOG_LEVEL: OnceLock<LogLevel> = OnceLock::new();

/// Default compiler log level (proof-driven tier reports need Verbose).
pub fn default_log_level() -> LogLevel {
    LogLevel::Verbose
}

/// Resolve level: explicit CLI > `SIKUWA_LOG` env > default Verbose.
pub fn resolve_log_level(cli: Option<LogLevel>) -> LogLevel {
    if let Some(l) = cli {
        return l;
    }
    if let Ok(env) = std::env::var("SIKUWA_LOG") {
        if let Some(l) = LogLevel::parse(&env) {
            return l;
        }
    }
    default_log_level()
}

pub fn init_log_level(level: LogLevel) {
    let _ = LOG_LEVEL.set(level);
}

pub fn log_level() -> LogLevel {
    *LOG_LEVEL.get().unwrap_or(&default_log_level())
}

fn enabled(min: LogLevel) -> bool {
    log_level() >= min
}

/// Always shown unless Quiet (build errors still use eprintln separately).
pub fn info(msg: impl AsRef<str>) {
    if enabled(LogLevel::Normal) {
        println!("{}", msg.as_ref());
    }
}

pub fn verbose(msg: impl AsRef<str>) {
    if enabled(LogLevel::Verbose) {
        println!("{}", msg.as_ref());
    }
}

pub fn trace(msg: impl AsRef<str>) {
    if enabled(LogLevel::Trace) {
        println!("{}", msg.as_ref());
    }
}

pub fn verbose_block(title: &str, lines: impl IntoIterator<Item = impl AsRef<str>>) {
    if !enabled(LogLevel::Verbose) {
        return;
    }
    println!("{title}");
    for line in lines {
        println!("{}", line.as_ref());
    }
}
