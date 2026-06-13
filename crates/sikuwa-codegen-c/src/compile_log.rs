//! Build-time compile summary — distinguish native codegen from dyn stubs.

use std::collections::HashMap;
use std::io::{self, Write};

use sikuwa_pir::module::{FuncDef, Module};
use sikuwa_pystat::{FuncStat, PystatReport, SlotLevel};

use crate::closure::{is_class_init_method, is_closure_factory};
use crate::emit::func_has_unsupported_dyn_ops;
use crate::slots::{collect_func_defs, max_slot_level, tier_for, CodegenTier};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenMode {
    /// Full IR lowered to C (S0/S1/Closure/ClassMethod).
    Native,
    /// `emit_func_s3_stub` — links but body is placeholder.
    DynStub,
}

#[derive(Debug, Clone)]
pub struct FunctionCompileRecord {
    pub symbol: String,
    pub static_eligible: bool,
    pub max_slot: SlotLevel,
    pub tier: CodegenTier,
    pub mode: CodegenMode,
    pub slot_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct TierCounts {
    pub s0: usize,
    pub s1: usize,
    pub s3: usize,
    pub closure: usize,
    pub class_method: usize,
}

#[derive(Debug, Clone)]
pub struct ModuleCompileLog {
    pub module: String,
    pub functions: Vec<FunctionCompileRecord>,
}

#[derive(Debug, Clone, Default)]
pub struct CompileLog {
    pub modules: Vec<ModuleCompileLog>,
}

impl CompileLog {
    pub fn push_module(&mut self, module: ModuleCompileLog) {
        self.modules.push(module);
    }

    pub fn function_count(&self) -> usize {
        self.modules.iter().map(|m| m.functions.len()).sum()
    }

    pub fn tier_counts(&self) -> TierCounts {
        let mut c = TierCounts::default();
        for m in &self.modules {
            for f in &m.functions {
                match f.tier {
                    CodegenTier::S0 => c.s0 += 1,
                    CodegenTier::S1 => c.s1 += 1,
                    CodegenTier::S3 => c.s3 += 1,
                    CodegenTier::Closure => c.closure += 1,
                    CodegenTier::ClassMethod => c.class_method += 1,
                }
            }
        }
        c
    }

    pub fn mode_counts(&self) -> (usize, usize) {
        let mut native = 0usize;
        let mut stub = 0usize;
        for m in &self.modules {
            for f in &m.functions {
                match f.mode {
                    CodegenMode::Native => native += 1,
                    CodegenMode::DynStub => stub += 1,
                }
            }
        }
        (native, stub)
    }

    pub fn static_eligible_count(&self) -> usize {
        self.modules
            .iter()
            .flat_map(|m| &m.functions)
            .filter(|f| f.static_eligible)
            .count()
    }
}

pub fn uses_dyn_stub(func: &FuncDef) -> bool {
    func_has_unsupported_dyn_ops(func)
        || (is_closure_factory(func)
            && func
                .nested
                .iter()
                .any(|n| func_has_unsupported_dyn_ops(n)))
}

pub fn module_compile_log(pir: &Module, report: &PystatReport) -> ModuleCompileLog {
    let stat_map: HashMap<_, _> = report
        .module
        .functions
        .iter()
        .map(|f| (f.symbol.0.clone(), f))
        .collect();

    let mut functions = Vec::new();
    for func in collect_func_defs(pir) {
        let Some(stat) = stat_map.get(&func.symbol.0) else {
            continue;
        };
        functions.push(record_function(func, stat));
    }

    ModuleCompileLog {
        module: pir.name.clone(),
        functions,
    }
}

fn record_function(func: &FuncDef, stat: &FuncStat) -> FunctionCompileRecord {
    let mode = if uses_dyn_stub(func) {
        CodegenMode::DynStub
    } else {
        CodegenMode::Native
    };
    let tier = tier_for(stat, func).unwrap_or(CodegenTier::S3);
    let slot_count = stat.params.len() + stat.locals.len();
    FunctionCompileRecord {
        symbol: func.symbol.0.clone(),
        static_eligible: stat.static_eligible,
        max_slot: max_slot_level(stat),
        tier,
        mode,
        slot_count,
    }
}

fn pct(part: usize, total: usize) -> u32 {
    if total == 0 {
        0
    } else {
        ((part as f64 / total as f64) * 100.0).round() as u32
    }
}

fn tier_label(t: CodegenTier) -> &'static str {
    match t {
        CodegenTier::S0 => "S0",
        CodegenTier::S1 => "S1",
        CodegenTier::S3 => "S3",
        CodegenTier::Closure => "closure",
        CodegenTier::ClassMethod => "class",
    }
}

fn slot_level_label(l: SlotLevel) -> &'static str {
    match l {
        SlotLevel::S0 => "S0",
        SlotLevel::S1 => "S1",
        SlotLevel::S2 => "S2",
        SlotLevel::S3 => "S3",
    }
}

/// Print human-readable compile summary to stdout.
pub fn print_compile_log(log: &CompileLog) {
    let _ = print_compile_log_to(io::stdout(), log);
}

pub fn print_compile_log_to(mut out: impl Write, log: &CompileLog) -> io::Result<()> {
    let total = log.function_count();
    if total == 0 {
        writeln!(out, "[compile] (no functions)")?;
        return Ok(());
    }

    let tiers = log.tier_counts();
    let (native, stub) = log.mode_counts();
    let static_n = log.static_eligible_count();

    writeln!(out, "[compile] ────────────────────────────────────────")?;
    writeln!(
        out,
        "[compile] functions: {total}  static_eligible: {static_n}/{total} ({}%)",
        pct(static_n, total)
    )?;
    writeln!(
        out,
        "[compile] slot tier:  S0={}  S1={}  S3={}  closure={}  class={}",
        tiers.s0, tiers.s1, tiers.s3, tiers.closure, tiers.class_method
    )?;
    writeln!(
        out,
        "[compile] codegen:    native={} ({}%)  dyn_stub={} ({}%)",
        native,
        pct(native, total),
        stub,
        pct(stub, total)
    )?;

    if stub > 0 {
        writeln!(out, "[compile] dyn stub (linked placeholder, not real IR):")?;
        for m in &log.modules {
            for f in &m.functions {
                if f.mode == CodegenMode::DynStub {
                    writeln!(out, "[compile]   {}", f.symbol)?;
                }
            }
        }
    }

    if native > 0 {
        writeln!(out, "[compile] native codegen:")?;
        for m in &log.modules {
            for f in &m.functions {
                if f.mode == CodegenMode::Native {
                    writeln!(
                        out,
                        "[compile]   {}  [{} tier={}, slots={}, max_slot={}]",
                        f.symbol,
                        if f.static_eligible {
                            "static"
                        } else {
                            "non-static"
                        },
                        tier_label(f.tier),
                        f.slot_count,
                        slot_level_label(f.max_slot),
                    )?;
                }
            }
        }
    }

    for m in &log.modules {
        if log.modules.len() > 1 {
            writeln!(out, "[compile] module `{}`:", m.module)?;
        }
    }

    writeln!(out, "[compile] ────────────────────────────────────────")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sikuwa_pir::lower_file;
    use sikuwa_pystat::analyze_module;

    #[test]
    fn feb_compile_log_shows_stub_vs_native() {
        let path = format!(
            "{}/../../tests/feb/feb.py",
            env!("CARGO_MANIFEST_DIR")
        );
        let pir = lower_file(std::path::Path::new(&path)).unwrap();
        let report = analyze_module(&pir);
        let log = module_compile_log(&pir, &report);
        assert!(log.functions.len() >= 6);
        let (native, stub) = {
            let mut n = 0;
            let mut s = 0;
            for f in &log.functions {
                match f.mode {
                    CodegenMode::Native => n += 1,
                    CodegenMode::DynStub => s += 1,
                }
            }
            (n, s)
        };
        assert!(native >= 1, "expected at least fib_recursive native");
        assert!(stub >= 1, "expected dyn stub functions in feb");
        assert!(
            log.functions
                .iter()
                .any(|f| f.symbol.ends_with("fib_recursive") && f.mode == CodegenMode::Native)
        );
    }
}
