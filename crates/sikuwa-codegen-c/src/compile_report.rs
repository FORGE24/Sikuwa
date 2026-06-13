//! Per-function codegen classification for build diagnostics.

use std::collections::HashMap;

use sikuwa_pir::module::{FuncDef, Module};
use sikuwa_pystat::{FuncStat, PystatReport, SlotLevel};

use crate::closure::is_closure_factory;
use crate::emit::func_has_unsupported_dyn_ops;
use crate::slots::{tier_for, CodegenTier};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenMode {
    Native,
    DynStub,
}

#[derive(Debug, Clone)]
pub struct FunctionCodegenEntry {
    pub symbol: String,
    pub tier: Option<CodegenTier>,
    pub mode: CodegenMode,
    pub static_eligible: bool,
    pub max_slot: SlotLevel,
    pub stub_reason: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ModuleCompileReport {
    pub module: String,
    pub functions: Vec<FunctionCodegenEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct CompileReport {
    pub modules: Vec<ModuleCompileReport>,
}

impl CompileReport {
    pub fn all_functions(&self) -> impl Iterator<Item = &FunctionCodegenEntry> {
        self.modules.iter().flat_map(|m| m.functions.iter())
    }

    pub fn function_count(&self) -> usize {
        self.modules.iter().map(|m| m.functions.len()).sum()
    }

    pub fn native_count(&self) -> usize {
        self.all_functions()
            .filter(|f| f.mode == CodegenMode::Native)
            .count()
    }

    pub fn stub_count(&self) -> usize {
        self.all_functions()
            .filter(|f| f.mode == CodegenMode::DynStub)
            .count()
    }

    pub fn static_eligible_count(&self) -> usize {
        self.all_functions()
            .filter(|f| f.static_eligible)
            .count()
    }

    pub fn tier_counts(&self) -> HashMap<&'static str, usize> {
        let mut m = HashMap::new();
        for f in self.all_functions() {
            let label = tier_label(f.tier);
            *m.entry(label).or_insert(0) += 1;
        }
        m
    }

    pub fn format_verbose_summary(&self) -> Vec<String> {
        let total = self.function_count().max(1);
        let native = self.native_count();
        let stub = self.stub_count();
        let se = self.static_eligible_count();
        let pct = |n: usize| (n as f64 * 100.0 / total as f64).round() as u32;

        let mut lines = vec![
            format!(
                "[build] codegen summary ({total} function(s), {} module(s)):",
                self.modules.len()
            ),
            format!(
                "  native codegen:  {native:>3}  ({pct}%)  — full C body emitted",
                pct = pct(native)
            ),
            format!(
                "  dyn stub:        {stub:>3}  ({pct}%)  — returns placeholder, not optimized IR",
                pct = pct(stub)
            ),
            format!(
                "  static_eligible: {se}/{total} ({pct}%)",
                pct = pct(se)
            ),
        ];

        let tiers = self.tier_counts();
        for label in ["S0", "S1", "S3", "Closure", "ClassMethod", "?"] {
            if let Some(&n) = tiers.get(label) {
                lines.push(format!("  {label} functions: {n}"));
            }
        }

        let slot_s0 = self
            .all_functions()
            .filter(|f| f.max_slot == SlotLevel::S0)
            .count();
        lines.push(format!(
            "  max slot S0:     {slot_s0}/{total} ({pct}%)",
            pct = pct(slot_s0)
        ));

        lines
    }

    pub fn format_stub_list(&self) -> Vec<String> {
        let mut lines = vec!["[build] dyn fallback (stub, no native body):".into()];
        let mut stubs: Vec<_> = self
            .all_functions()
            .filter(|f| f.mode == CodegenMode::DynStub)
            .collect();
        stubs.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        if stubs.is_empty() {
            lines.push("  (none)".into());
        } else {
            for f in stubs {
                lines.push(format!(
                    "  {}  tier={}  reason={}",
                    f.symbol,
                    tier_label(f.tier),
                    f.stub_reason.as_deref().unwrap_or("unsupported dyn IR")
                ));
            }
        }
        lines
    }

    pub fn format_native_list(&self) -> Vec<String> {
        let mut lines = vec!["[build] native codegen:".into()];
        let mut natives: Vec<_> = self
            .all_functions()
            .filter(|f| f.mode == CodegenMode::Native)
            .collect();
        natives.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        if natives.is_empty() {
            lines.push("  (none)".into());
        } else {
            for f in natives {
                lines.push(format!(
                    "  {}  tier={}  static_eligible={}",
                    f.symbol,
                    tier_label(f.tier),
                    if f.static_eligible { "yes" } else { "no" }
                ));
            }
        }
        lines
    }
}

fn tier_label(tier: Option<CodegenTier>) -> &'static str {
    match tier {
        Some(CodegenTier::S0) => "S0",
        Some(CodegenTier::S1) => "S1",
        Some(CodegenTier::S3) => "S3",
        Some(CodegenTier::Closure) => "Closure",
        Some(CodegenTier::ClassMethod) => "ClassMethod",
        None => "?",
    }
}

pub fn compile_report_from_module(
    pir: &Module,
    pystat: &PystatReport,
) -> ModuleCompileReport {
    let stat_map: HashMap<_, _> = pystat
        .module
        .functions
        .iter()
        .map(|f| (f.symbol.0.clone(), f))
        .collect();

    let mut functions = Vec::new();
    for func in collect_all_func_defs(pir) {
        let Some(stat) = stat_map.get(&func.symbol.0) else {
            continue;
        };
        functions.push(classify_function(func, stat));
    }

    ModuleCompileReport {
        module: pir.name.clone(),
        functions,
    }
}

fn collect_all_func_defs<'a>(pir: &'a Module) -> Vec<&'a FuncDef> {
    let mut out = Vec::new();
    for f in &pir.functions {
        push_func_tree(f, &mut out);
    }
    for class in &pir.classes {
        for m in &class.methods {
            push_func_tree(m, &mut out);
        }
    }
    out
}

fn push_func_tree<'a>(func: &'a FuncDef, out: &mut Vec<&'a FuncDef>) {
    out.push(func);
    for nested in &func.nested {
        push_func_tree(nested, out);
    }
}

fn classify_function(func: &FuncDef, stat: &FuncStat) -> FunctionCodegenEntry {
    let stub = will_emit_dyn_stub(func);
    let tier = tier_for(stat, func);
    let max_slot = stat
        .params
        .iter()
        .chain(stat.locals.iter())
        .map(|s| s.level)
        .max_by_key(|l| slot_ord(*l))
        .unwrap_or(SlotLevel::S0);
    FunctionCodegenEntry {
        symbol: func.symbol.0.clone(),
        tier,
        mode: if stub {
            CodegenMode::DynStub
        } else {
            CodegenMode::Native
        },
        static_eligible: stat.static_eligible,
        max_slot,
        stub_reason: stub.then(|| stub_reason(func)),
    }
}

fn slot_ord(l: SlotLevel) -> u8 {
    match l {
        SlotLevel::S0 => 0,
        SlotLevel::S1 => 1,
        SlotLevel::S2 => 2,
        SlotLevel::S3 => 3,
    }
}

fn will_emit_dyn_stub(func: &FuncDef) -> bool {
    func_has_unsupported_dyn_ops(func)
        || (is_closure_factory(func)
            && func.nested.iter().any(|n| func_has_unsupported_dyn_ops(n)))
}

fn stub_reason(func: &FuncDef) -> String {
    if is_closure_factory(func)
        && func
            .nested
            .iter()
            .any(|n| func_has_unsupported_dyn_ops(n))
    {
        return "nested function contains unsupported dyn ops".into();
    }
    let mut ops = Vec::new();
    for block in &func.blocks {
        for op in &block.ops {
            if is_unsupported_dyn_op(op.opcode) {
                let name = op.opcode.name();
                if !ops.contains(&name) {
                    ops.push(name);
                }
            }
        }
    }
    if ops.is_empty() {
        "unsupported dyn IR".into()
    } else {
        ops.join(", ")
    }
}

fn is_unsupported_dyn_op(op: sikuwa_pir::opcode::OpCode) -> bool {
    use sikuwa_pir::opcode::OpCode;
    matches!(
        op,
        OpCode::LoadGlobal
            | OpCode::LoadAttr
            | OpCode::SubscriptLoad
            | OpCode::SubscriptStore
            | OpCode::BuildTuple
            | OpCode::BuildList
            | OpCode::BuildMap
            | OpCode::CallIndirect
            | OpCode::CallBuiltin
            | OpCode::BuildClass
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sikuwa_pir::lower_file;
    use sikuwa_pystat::analyze_module;

    #[test]
    fn feb_report_shows_stub_majority() {
        let path = format!(
            "{}/../../tests/feb/feb.py",
            env!("CARGO_MANIFEST_DIR")
        );
        let pir = lower_file(std::path::Path::new(&path)).unwrap();
        let report = analyze_module(&pir);
        let module_cr = compile_report_from_module(&pir, &report);
        let cr = CompileReport {
            modules: vec![module_cr],
        };
        assert!(cr.function_count() >= 6);
        assert!(cr.stub_count() >= 1);
        assert!(cr.native_count() >= 1);
        let summary = cr.format_verbose_summary().join("\n");
        assert!(summary.contains("dyn stub"));
        assert!(summary.contains("static_eligible"));
    }
}
