use std::path::PathBuf;

use clap::Subcommand;
use sikuwa_pir::{
    decode_module, encode_module, lower_file, lower_source, module_to_text,
    sample_add_module, verify_module,
};

#[derive(Subcommand)]
pub enum PirCommands {
    /// Lower `.py` source to `.pirb`
    Build {
        /// Python source file
        input: PathBuf,
        /// Output `.pirb` path (default: <stem>.pirb)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Skip verify after lowering
        #[arg(long)]
        no_verify: bool,
    },
    /// Verify a `.pirb` module or built-in sample
    Verify {
        /// Path to `.pirb` file (omit to verify built-in sample)
        path: Option<String>,
    },
    /// Write built-in sample module to `.pirb`
    Sample {
        #[arg(short, long, default_value = "sample.pirb")]
        output: String,
    },
    /// Dump `.pirb` module summary
    Dump {
        path: String,
    },
    /// Print human-readable `.pir` text from `.py` or `.pirb`
    Text {
        input: PathBuf,
    },
}

pub fn run(cmd: PirCommands) -> i32 {
    match cmd {
        PirCommands::Build {
            input,
            output,
            no_verify,
        } => build_cmd(&input, output.as_ref(), no_verify),
        PirCommands::Verify { path } => verify_cmd(path.as_deref()),
        PirCommands::Sample { output } => sample_cmd(&output),
        PirCommands::Dump { path } => dump_cmd(&path),
        PirCommands::Text { input } => text_cmd(&input),
    }
}

fn build_cmd(input: &PathBuf, output: Option<&PathBuf>, no_verify: bool) -> i32 {
    let module = match lower_file(input) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    if !no_verify {
        let report = verify_module(&module);
        if !report.ok() {
            eprintln!("[fail] PIR verify after lowering");
            for e in &report.errors {
                eprintln!("  error: {e}");
            }
            return 1;
        }
        for w in &report.warnings {
            println!("  warn: {w}");
        }
    }

    let out_path = output.cloned().unwrap_or_else(|| {
        input.with_extension("pirb")
    });

    match encode_module(&module) {
        Ok(bytes) => {
            let size = bytes.len();
            match std::fs::write(&out_path, &bytes) {
            Ok(()) => {
                println!(
                    "[ok] {} → {} ({} functions, {} bytes)",
                    input.display(),
                    out_path.display(),
                    module.functions.len(),
                    size
                );
                0
            }
            Err(e) => {
                eprintln!("error: {e}");
                1
            }
        }},
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn text_cmd(input: &PathBuf) -> i32 {
    let text = if input.extension().and_then(|s| s.to_str()) == Some("pirb") {
        match std::fs::read(input) {
            Ok(bytes) => match decode_module(&bytes) {
                Ok(m) => module_to_text(&m),
                Err(e) => {
                    eprintln!("error: {e}");
                    return 1;
                }
            },
            Err(e) => {
                eprintln!("error: {e}");
                return 1;
            }
        }
    } else {
        match std::fs::read_to_string(input) {
            Ok(src) => {
                let path = input.to_str().unwrap_or("input.py");
                match lower_source(&src, path) {
                    Ok(m) => module_to_text(&m),
                    Err(e) => {
                        eprintln!("error: {e}");
                        return 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                return 1;
            }
        }
    };
    print!("{text}");
    0
}

fn verify_cmd(path: Option<&str>) -> i32 {
    let module = match path {
        Some(p) => match load_pirb(p) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: {e}");
                return 1;
            }
        },
        None => sample_add_module(),
    };

    let report = verify_module(&module);
    if report.ok() {
        println!("[ok] PIR verify passed: {}", module.name);
        for w in &report.warnings {
            println!("  warn: {w}");
        }
        0
    } else {
        eprintln!("[fail] PIR verify: {}", module.name);
        for e in &report.errors {
            eprintln!("  error: {e}");
        }
        1
    }
}

fn sample_cmd(output: &str) -> i32 {
    let module = sample_add_module();
    match encode_module(&module) {
        Ok(bytes) => match std::fs::write(output, bytes) {
            Ok(()) => {
                println!("[ok] wrote {output}");
                0
            }
            Err(e) => {
                eprintln!("error: {e}");
                1
            }
        },
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn dump_cmd(path: &str) -> i32 {
    match load_pirb(path) {
        Ok(module) => {
            println!("module:    {}", module.name);
            println!("exports:   {}", module.exports.len());
            println!("lang:      {}", module.python_lang);
            println!("hash:      {}", hex32(module.source_hash));
            println!("functions: {}", module.functions.len());
            for f in &module.functions {
                println!(
                    "  {} ({} blocks, {} locals)",
                    f.symbol,
                    f.blocks.len(),
                    f.locals.len()
                );
            }
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn load_pirb(path: &str) -> sikuwa_core::Result<sikuwa_pir::Module> {
    let bytes = std::fs::read(path)?;
    decode_module(&bytes)
}

fn hex32(bytes: [u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
