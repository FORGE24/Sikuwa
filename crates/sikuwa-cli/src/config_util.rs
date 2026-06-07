//! Load `[sikuwa.pystat]` from TOML for CLI commands.

use std::path::Path;

use sikuwa_config::{load_from_str, SCHEMA_VERSION};
use sikuwa_pystat::PystatOptions;

pub fn load_pystat_options(config: Option<&Path>) -> PystatOptions {
    let Some(path) = config else {
        return PystatOptions::default();
    };
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(cfg) = load_from_str(&content) {
            if cfg.sikuwa.schema == SCHEMA_VERSION && cfg.sikuwa.engine == "a2" {
                let ps = &cfg.sikuwa.pystat;
                return PystatOptions::from_config_section(
                    &ps.mode,
                    &ps.min_slot,
                    ps.allow_dyn_fallback,
                );
            }
        }
    }
    PystatOptions::default()
}
