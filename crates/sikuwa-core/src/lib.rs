//! Sikuwa 2.0 core library.
//!
//! Codename: **Sikuwa 2026/6/6 Ver.A2**

pub mod error;
pub mod log;
pub mod version;

pub use error::{Result, SikuwaError};
pub use log::{default_log_level, info, init_log_level, log_level, resolve_log_level, trace, verbose, verbose_block, LogLevel};
pub use version::{Codename, Version, VERSION};
