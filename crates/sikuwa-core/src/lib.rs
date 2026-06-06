//! Sikuwa 2.0 core library.
//!
//! Codename: **Sikuwa 2026/6/6 Ver.A2**

pub mod error;
pub mod version;

pub use error::{Result, SikuwaError};
pub use version::{Codename, Version, VERSION};
