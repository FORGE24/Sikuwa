use std::fs;

use sikuwa_config::{load_from_str, validate};

pub fn run(config_path: &str) -> i32 {
    let content = match fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: cannot read {config_path}: {e}");
            return 1;
        }
    };

    let config = match load_from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    match validate(&config) {
        Ok(warnings) => {
            println!("[ok] config valid: {}", config.sikuwa.project_name);
            for w in warnings {
                println!("  warn: {w}");
            }
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}
