//! Native C codegen CLI.

use std::fs;
use std::path::PathBuf;

use clap::Subcommand;
use sikuwa_codegen_c::{
    emit_manifest, emit_module_c, emit_module_h, emit_pywrap_c, manifest_to_json, CodegenOptions,
};
use sikuwa_core::Result;
use sikuwa_pir::lower_file;
use sikuwa_pystat::analyze_module;

#[derive(Subcommand)]
pub enum CodegenCommands {
    /// Emit Sikuwa-C from Python source
    C {
        /// Input `.py` file
        input: PathBuf,
        /// Output directory
        #[arg(short, long, default_value = ".")]
        out_dir: PathBuf,
        /// Skip `.skw.json` manifest
        #[arg(long)]
        no_manifest: bool,
        /// Skip `skw_module_t` descriptor
        #[arg(long)]
        no_module_desc: bool,
        /// Skip class/closure struct typedefs
        #[arg(long)]
        no_structs: bool,
        /// Emit `{module}_pywrap.c` (CPython extension)
        #[arg(long)]
        python_shim: bool,
    },
}

pub fn run(cmd: CodegenCommands) -> i32 {
    match run_inner(cmd) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn run_inner(cmd: CodegenCommands) -> Result<()> {
    match cmd {
        CodegenCommands::C {
            input,
            out_dir,
            no_manifest,
            no_module_desc,
            no_structs,
            python_shim,
        } => {
            let pir = lower_file(&input)?;
            let report = analyze_module(&pir);
            let opts = CodegenOptions {
                emit_module_desc: !no_module_desc,
                emit_structs: !no_structs,
                python_shim,
                ..Default::default()
            };
            let h = emit_module_h(&pir, &report, &opts);
            let c = emit_module_c(&pir, &report, &opts);
            fs::create_dir_all(&out_dir)?;
            let stem = input
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module");
            let h_path = out_dir.join(format!("{stem}.h"));
            let c_path = out_dir.join(format!("{stem}.c"));
            fs::write(&h_path, h)?;
            fs::write(&c_path, c)?;
            println!("wrote {}", h_path.display());
            println!("wrote {}", c_path.display());

            if !no_manifest {
                let manifest = emit_manifest(&pir, &report);
                let json_path = out_dir.join(format!("{stem}.skw.json"));
                fs::write(&json_path, manifest_to_json(&manifest))?;
                println!("wrote {}", json_path.display());
            }

            if python_shim {
                let wrap = emit_pywrap_c(stem, &report);
                let wrap_path = out_dir.join(format!("{stem}_pywrap.c"));
                fs::write(&wrap_path, wrap)?;
                println!("wrote {}", wrap_path.display());
            }

            if cfg!(windows) && !no_module_desc {
                let def_path = out_dir.join(format!("{stem}.def"));
                fs::write(&def_path, emit_def_file(&report))?;
                println!("wrote {}", def_path.display());
            }
        }
    }
    Ok(())
}

fn emit_def_file(report: &sikuwa_pystat::PystatReport) -> String {
    use std::collections::HashSet;
    let mut out = String::from("EXPORTS\n");
    let mut seen = HashSet::new();
    for f in &report.module.functions {
        if f.static_eligible {
            let sym = sikuwa_codegen_c::skw_c_symbol(&f.symbol.0);
            if seen.insert(sym.clone()) {
                out.push_str(&format!("    {sym}\n"));
            }
        }
    }
    let mod_sym = format!(
        "skw_module_{}",
        sikuwa_codegen_c::module_c_name(&report.module.module)
    );
    if seen.insert(mod_sym.clone()) {
        out.push_str(&format!("    {mod_sym}\n"));
    }
    out
}
