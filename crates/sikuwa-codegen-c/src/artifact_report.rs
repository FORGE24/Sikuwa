//! Build artifact diagnostics — DLL vs EXE, entry `main` eligibility.

use std::path::PathBuf;

use sikuwa_pir::module::Module;

use crate::compile_report::{CodegenMode, FunctionCodegenEntry, ModuleCompileReport};
use crate::emit::{skw_c_symbol, skw_c_symbol_dyn};
use crate::slots::CodegenTier;

#[derive(Debug, Clone)]
pub struct EntryMainInfo {
    pub pir_symbol: String,
    pub c_symbol: String,
    pub mode: CodegenMode,
    pub tier: Option<CodegenTier>,
}

#[derive(Debug, Clone)]
pub enum ExeBuildStatus {
    NotRequested,
    NoEntryMain,
    Linked(PathBuf),
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct ArtifactReport {
    pub dll_path: PathBuf,
    pub dll_built: bool,
    pub exe_requested: bool,
    pub exe_status: ExeBuildStatus,
    pub entry_main: Option<EntryMainInfo>,
}

impl ArtifactReport {
    pub fn format_verbose(&self) -> Vec<String> {
        let mut lines = vec!["[build] artifacts:".into()];

        if self.dll_built {
            lines.push(format!("  dll:  ok → {}", self.dll_path.display()));
        } else {
            lines.push("  dll:  (not linked)".into());
        }

        match &self.exe_status {
            ExeBuildStatus::NotRequested => {
                if let Some(m) = &self.entry_main {
                    let run_note = if m.mode == CodegenMode::Native {
                        "native entry — real logic if all callees are native"
                    } else {
                        "dyn stub entry — process exits but main body is placeholder IR"
                    };
                    lines.push(format!(
                        "  exe:  not built (pass --exe; entry `{}` is {})",
                        m.pir_symbol,
                        if m.mode == CodegenMode::Native {
                            "native"
                        } else {
                            "dyn stub"
                        }
                    ));
                    lines.push(format!("        hint: {run_note}"));
                } else {
                    lines.push(
                        "  exe:  not built (no `main()` in entry module — pass --exe only when entry defines main)"
                            .into(),
                    );
                }
            }
            ExeBuildStatus::NoEntryMain => {
                lines.push("  exe:  skipped — entry module has no `main()` function".into());
            }
            ExeBuildStatus::Linked(path) => {
                let note = self.entry_main.as_ref().map(|m| {
                    if m.mode == CodegenMode::DynStub {
                        format!(
                            " via skw_{}_main_native (IR main is dyn stub; runtime runner)",
                            path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("module")
                        )
                    } else {
                        format!(" via `{}` (native)", m.c_symbol)
                    }
                }).unwrap_or_default();
                lines.push(format!("  exe:  ok → {}{note}", path.display()));
            }
            ExeBuildStatus::Failed(err) => {
                lines.push(format!("  exe:  failed — {err}"));
            }
        }

        lines
    }
}

pub fn find_entry_main(pir: &Module, entry_compile: &ModuleCompileReport) -> Option<EntryMainInfo> {
    let target = format!("{}.main", pir.name);
    if !pir.functions.iter().any(|f| f.symbol.0 == target) {
        return None;
    }
    let entry = entry_compile.functions.iter().find(|f| f.symbol == target)?;
    Some(EntryMainInfo {
        pir_symbol: target,
        c_symbol: c_symbol_for_entry(entry),
        mode: entry.mode,
        tier: entry.tier,
    })
}

fn c_symbol_for_entry(entry: &FunctionCodegenEntry) -> String {
    match entry.tier {
        Some(CodegenTier::S3) => skw_c_symbol_dyn(&entry.symbol),
        _ => skw_c_symbol(&entry.symbol),
    }
}

/// C source for `main()` that dispatches to the module entry function.
pub fn emit_entry_main_c(entry: &EntryMainInfo, header_stem: &str) -> String {
    let mut out = String::new();
    out.push_str("#include \"sikuwa/runtime.h\"\n");
    if entry.mode == CodegenMode::DynStub {
        out.push_str(&format!(
            "extern void skw_{header_stem}_main_native(void);\n\n"
        ));
        out.push_str("int main(int argc, char **argv) {\n");
        out.push_str("  (void)argc;\n");
        out.push_str("  (void)argv;\n");
        out.push_str(&format!("  skw_{header_stem}_main_native();\n"));
        out.push_str("  return 0;\n");
        out.push_str("}\n");
        return out;
    }
    out.push_str(&format!("#include \"{header_stem}.h\"\n\n"));
    out.push_str("int main(int argc, char **argv) {\n");
    out.push_str("  (void)argc;\n");
    out.push_str("  (void)argv;\n");
    match entry.tier {
        Some(CodegenTier::S3) => {
            out.push_str(&format!(
                "  skw_value_t *r = {}();\n",
                entry.c_symbol
            ));
            out.push_str("  if (r) skw_value_release(r);\n");
        }
        _ => {
            out.push_str(&format!("  (void){}();\n", entry.c_symbol));
        }
    }
    out.push_str("  return 0;\n");
    out.push_str("}\n");
    out
}

pub fn default_exe_path(out_dir: &std::path::Path, entry_stem: &str) -> PathBuf {
    if cfg!(windows) {
        out_dir.join(format!("{entry_stem}.exe"))
    } else {
        out_dir.join(entry_stem)
    }
}
