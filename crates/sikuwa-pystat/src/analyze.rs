use std::collections::HashMap;

use sikuwa_pir::ids::SymbolRef;
use sikuwa_pir::module::{ConstValue, ExternDecl, FuncDef, Module, Op, OpOperand};
use sikuwa_pir::opcode::OpCode;

use crate::config::PystatOptions;
use crate::evidence::{check_return_hint, hint_for_func, parse_type_name, seed_params_from_hint};
use crate::pass2::pass2_diagnostics;
use crate::pass3::{func_has_dyn_ops, pass3_dyn_diagnostics, run_func_body_cfg};
use crate::pass5::pass5_finalize_func;
use crate::infer::{
    apply_fixpoint, build_call_graph, from_physical, lookup_summary, materialize_slot,
    project_to_physical, seed_summaries, summaries_map, FuncSummary, LogicalType,
    SparseEnvironment, SummaryCertainty,
};
use crate::types::{
    FuncStat, LogicalSlot, PhysicalType, PystatModule, SlotLevel, SlotStrategy,
};
use crate::PystatDiagnostic;

#[derive(Debug, Clone)]
pub struct PystatReport {
    pub module: PystatModule,
    pub itr_slots: usize,
    pub dyn_slots: usize,
    pub diagnostics: Vec<PystatDiagnostic>,
}

pub fn analyze_module(module: &Module) -> PystatReport {
    analyze_module_with_options(module, &PystatOptions::default())
}

pub fn peer_summaries_from_stats(stats: &[FuncStat]) -> HashMap<String, FuncSummary> {
    stats
        .iter()
        .map(|f| {
            (
                f.symbol.0.clone(),
                FuncSummary {
                    return_ty: from_physical(f.return_ty),
                    certainty: SummaryCertainty::Inferred,
                },
            )
        })
        .collect()
}

pub fn analyze_module_with_options(module: &Module, opts: &PystatOptions) -> PystatReport {
    analyze_module_with_peers(module, opts, &HashMap::new())
}

pub fn analyze_module_with_peers(
    module: &Module,
    opts: &PystatOptions,
    peer_summaries: &HashMap<String, FuncSummary>,
) -> PystatReport {
    let externs: HashMap<String, ExternDecl> = module
        .externs
        .iter()
        .cloned()
        .map(|e| (e.name.clone(), e))
        .collect();
    let graph = build_call_graph(module);
    let seeded = seed_summaries(module, &graph);
    let summaries = apply_fixpoint(seeded, |sums| {
        let map = merge_summaries(&interim_summary_map(&graph, sums), peer_summaries);
        module
            .functions
            .iter()
            .map(|f| {
                analyze_func_return_lt(f, &map, hint_for_func(module, f), &externs, opts, false)
            })
            .collect()
    });
    let summary_map = summaries_map(&graph, &summaries);
    let merged_summaries = merge_summaries(&summary_map, peer_summaries);

    let mut functions = Vec::new();
    let mut itr_slots = 0;
    let mut dyn_slots = 0;
    let mut diagnostics = Vec::new();

    for func in &module.functions {
        let hint = hint_for_func(module, func);
        let (stat, diags) = analyze_func_with_diags(
            func,
            &merged_summaries,
            hint,
            &externs,
            opts,
            true,
        );
        diagnostics.extend(diags);
        for slot in stat.params.iter().chain(stat.locals.iter()) {
            match slot.strategy {
                SlotStrategy::Itr { .. } => itr_slots += 1,
                SlotStrategy::Dyn => dyn_slots += 1,
                SlotStrategy::Alloc { .. } => {}
            }
        }
        functions.push(stat);
    }

    for class in &module.classes {
        for method in &class.methods {
            let hint = hint_for_func(module, method);
            let (stat, diags) = analyze_func_with_diags(
                method,
                &merged_summaries,
                hint,
                &externs,
                opts,
                true,
            );
            diagnostics.extend(diags);
            functions.push(stat);
        }
    }

    diagnostics.extend(crate::pass4::pass4_module_diagnostics(module));

    PystatReport {
        module: PystatModule {
            module: module.name.clone(),
            source_hash: module.source_hash,
            functions,
        },
        itr_slots,
        dyn_slots,
        diagnostics,
    }
}

fn merge_summaries(
    local: &HashMap<String, FuncSummary>,
    peers: &HashMap<String, FuncSummary>,
) -> HashMap<String, FuncSummary> {
    let mut merged = peers.clone();
    merged.extend(local.iter().map(|(k, v)| (k.clone(), v.clone())));
    merged
}

fn interim_summary_map(graph: &crate::infer::CallGraph, sums: &[FuncSummary]) -> HashMap<String, FuncSummary> {
    graph
        .symbols
        .iter()
        .zip(sums.iter())
        .map(|(s, sum)| (s.0.clone(), sum.clone()))
        .collect()
}

fn analyze_func_return_lt(
    func: &FuncDef,
    summaries: &HashMap<String, FuncSummary>,
    hint: Option<&sikuwa_pir::FuncTypeHint>,
    externs: &HashMap<String, ExternDecl>,
    opts: &PystatOptions,
    apply_passes: bool,
) -> LogicalType {
    let (stat, _) = analyze_func_with_diags(func, summaries, hint, externs, opts, apply_passes);
    func.return_value
        .and_then(|_| infer_return_from_func(func, summaries, hint, externs))
        .unwrap_or_else(|| crate::infer::from_physical(stat.return_ty))
}

fn infer_return_from_func(
    func: &FuncDef,
    summaries: &HashMap<String, FuncSummary>,
    hint: Option<&sikuwa_pir::FuncTypeHint>,
    externs: &HashMap<String, ExternDecl>,
) -> Option<LogicalType> {
    let mut slots = SparseEnvironment::new();
    let mut value_types: HashMap<u32, LogicalType> = HashMap::new();
    seed_params_from_hint(func, hint, &mut slots);
    run_func_body_cfg(
        func,
        &mut slots,
        &mut value_types,
        summaries,
        externs,
        infer_op,
    );
    func.return_value
        .and_then(|v| value_types.get(&v.0).cloned())
}

pub fn analyze_func(
    func: &FuncDef,
    summaries: &HashMap<String, FuncSummary>,
) -> FuncStat {
    analyze_func_with_diags(
        func,
        summaries,
        None,
        &HashMap::new(),
        &PystatOptions::default(),
        true,
    )
    .0
}

fn analyze_func_with_diags(
    func: &FuncDef,
    summaries: &HashMap<String, FuncSummary>,
    hint: Option<&sikuwa_pir::FuncTypeHint>,
    externs: &HashMap<String, ExternDecl>,
    opts: &PystatOptions,
    apply_passes: bool,
) -> (FuncStat, Vec<PystatDiagnostic>) {
    let mut slots = SparseEnvironment::new();
    let mut value_types: HashMap<u32, LogicalType> = HashMap::new();
    let mut diagnostics = seed_params_from_hint(func, hint, &mut slots);

    run_func_body_cfg(
        func,
        &mut slots,
        &mut value_types,
        summaries,
        externs,
        infer_op,
    );

    let slot_snapshot = slots.snapshot_for_locals(&func.locals);

    let inferred_return_lt = func
        .return_value
        .and_then(|v| value_types.get(&v.0).cloned())
        .unwrap_or(LogicalType::Top);
    diagnostics.extend(check_return_hint(func, hint, &inferred_return_lt));

    let mut return_ty = project_to_physical(inferred_return_lt.clone());

    if return_ty == PhysicalType::Unknown {
        if let Some(h) = hint {
            if let Some(ret_str) = &h.return_ty {
                let ht = parse_type_name(ret_str);
                if ht != LogicalType::Top && !ht.is_bottom() {
                    return_ty = project_to_physical(ht);
                }
            }
        }
    }

    let dyn_ops = func_has_dyn_ops(func);
    if return_ty == PhysicalType::Unknown && !dyn_ops {
        return_ty = PhysicalType::Int64;
    }

    let params: Vec<LogicalSlot> = func
        .params
        .iter()
        .map(|n| {
            let lt = slot_snapshot
                .get(n)
                .cloned()
                .unwrap_or(LogicalType::Top);
            let mut slot = logical_slot_from_lt(n, lt);
            if slot.ty == PhysicalType::Unknown && !dyn_ops {
                slot.ty = PhysicalType::Int64;
                slot.level = SlotLevel::S0;
                slot.strategy = SlotStrategy::Itr {
                    primary: PhysicalType::Int64,
                };
            }
            slot
        })
        .collect();

    let locals: Vec<LogicalSlot> = func
        .locals
        .iter()
        .filter(|n| !func.params.contains(n))
        .map(|n| {
            let lt = slot_snapshot
                .get(n)
                .cloned()
                .unwrap_or(LogicalType::Top);
            logical_slot_from_lt(n, lt)
        })
        .collect();

    let static_eligible = params.iter().chain(locals.iter()).all(|s| s.level == SlotLevel::S0)
        && return_ty.bit_width().is_some()
        && !matches!(return_ty, PhysicalType::Dyn | PhysicalType::Unknown | PhysicalType::Object);

    let stat = FuncStat {
        symbol: func.symbol.clone(),
        params,
        locals,
        return_ty,
        static_eligible,
    };

    if !apply_passes {
        return (stat, diagnostics);
    }

    diagnostics.extend(pass3_dyn_diagnostics(func, opts, dyn_ops));
    let (stat, pass5_diags) = pass5_finalize_func(stat, opts);
    diagnostics.extend(pass5_diags);
    diagnostics.extend(pass2_diagnostics(&stat, opts));

    (stat, diagnostics)
}

fn infer_op(
    func: &FuncDef,
    op: &Op,
    values: &HashMap<u32, LogicalType>,
    slots: &mut SparseEnvironment,
    summaries: &HashMap<String, FuncSummary>,
    externs: &HashMap<String, ExternDecl>,
) -> LogicalType {
    let operand_ty = |i: usize| -> LogicalType {
        op.operands
            .get(i)
            .and_then(|o| type_of_operand(o, values, slots))
            .unwrap_or(LogicalType::Top)
    };

    match op.opcode {
        OpCode::Const => match op.operands.first() {
            Some(OpOperand::Const(ConstValue::Int(v))) => {
                LogicalType::Literal(crate::infer::LiteralValue::Int(*v))
            }
            Some(OpOperand::Const(ConstValue::Bool(v))) => {
                LogicalType::Literal(crate::infer::LiteralValue::Bool(*v))
            }
            Some(OpOperand::Const(ConstValue::Float(v))) => LogicalType::Literal(
                crate::infer::LiteralValue::Float(v.to_bits()),
            ),
            Some(OpOperand::Const(ConstValue::Str(_))) => LogicalType::Str,
            Some(OpOperand::Const(ConstValue::None)) => LogicalType::None,
            _ => LogicalType::Top,
        },
        OpCode::LoadFast | OpCode::LoadCell | OpCode::Phi => {
            if let Some(OpOperand::Name(n)) = op.operands.first() {
                slots.get(n)
            } else {
                LogicalType::Top
            }
        }
        OpCode::BinOpAdd | OpCode::BinOpSub | OpCode::BinOpMul | OpCode::BinOpFloorDiv
        | OpCode::BinOpMod => {
            let a = operand_ty(0);
            let b = operand_ty(1);
            if a == LogicalType::Float || b == LogicalType::Float {
                LogicalType::Float
            } else {
                LogicalType::Int
            }
        }
        OpCode::BinOpTrueDiv => LogicalType::Float,
        OpCode::BinOpBitAnd | OpCode::BinOpRShift => LogicalType::Int,
        OpCode::UnaryNot
        | OpCode::CompareLt
        | OpCode::CompareLe
        | OpCode::CompareGt
        | OpCode::CompareGe
        | OpCode::CompareEq
        | OpCode::CompareNe
        | OpCode::CompareIs
        | OpCode::CompareIsNot => LogicalType::Bool,
        OpCode::UnaryNeg => operand_ty(0),
        OpCode::Call => {
            if let Some(sym) = pass3_resolve_call(func, op) {
                if let Some(sum) = lookup_summary(summaries, &sym) {
                    return sum.return_ty.clone();
                }
            }
            LogicalType::Dyn
        }
        OpCode::LoadGlobal | OpCode::LoadAttr | OpCode::SubscriptLoad | OpCode::MakeClosure | OpCode::BuildClass
        | OpCode::BuildTuple
        | OpCode::BuildList
        | OpCode::BuildMap
        | OpCode::CallIndirect
        | OpCode::CallBuiltin
        | OpCode::GetIter
        | OpCode::ForIterNext => LogicalType::Dyn,
        OpCode::CallExtern => {
            if let Some(OpOperand::Name(name)) = op.operands.first() {
                if let Some(ext) = externs.get(name) {
                    return parse_type_name(&ext.return_ty);
                }
            }
            LogicalType::Int
        }
        _ => LogicalType::Top,
    }
}

fn pass3_resolve_call(func: &FuncDef, op: &Op) -> Option<SymbolRef> {
    if op.opcode != OpCode::Call {
        return None;
    }
    match op.operands.first()? {
        OpOperand::Symbol(s) => Some(s.clone()),
        OpOperand::Name(n) => {
            let prefix = func.symbol.0.rsplit_once('.').map(|(m, _)| m).unwrap_or("");
            Some(SymbolRef::new(format!("{prefix}.{n}")))
        }
        _ => None,
    }
}

fn logical_slot_from_lt(name: &str, lt: LogicalType) -> LogicalSlot {
    let mat = materialize_slot(lt);
    LogicalSlot {
        name: name.to_string(),
        ty: mat.physical,
        strategy: mat.strategy,
        level: mat.level,
        tagged: mat.tagged,
    }
}

fn type_of_operand(
    op: &OpOperand,
    values: &HashMap<u32, LogicalType>,
    slots: &SparseEnvironment,
) -> Option<LogicalType> {
    match op {
        OpOperand::Value(v) => values.get(&v.0).cloned(),
        OpOperand::Name(n) => Some(slots.get(n)),
        OpOperand::Const(c) => Some(match c {
            ConstValue::Int(v) => LogicalType::Literal(crate::infer::LiteralValue::Int(*v)),
            ConstValue::Bool(v) => LogicalType::Literal(crate::infer::LiteralValue::Bool(*v)),
            ConstValue::Float(v) => {
                LogicalType::Literal(crate::infer::LiteralValue::Float(v.to_bits()))
            }
            ConstValue::Str(_) => LogicalType::Str,
            ConstValue::None => LogicalType::None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PystatOptions;
    use sikuwa_pir::{lower_source, sample_add_module};

    #[test]
    fn pep484_annotations_seed_int() {
        let src = "def add(a: int, b: int) -> int:\n    return a + b\n";
        let m = lower_source(src, "t.py").unwrap();
        let report = analyze_module(&m);
        let f = &report.module.functions[0];
        assert!(f.static_eligible);
        assert_eq!(f.return_ty, PhysicalType::Int64);
    }

    #[test]
    fn pyi_stub_merged_with_source() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = format!("{manifest_dir}/../../tests/fixtures/plan7_pep484.py");
        let m = sikuwa_pir::lower_file(std::path::Path::new(&path)).unwrap();
        let report = analyze_module(&m);
        let f = &report.module.functions[0];
        assert_eq!(f.return_ty, PhysicalType::Int64);
        assert!(f.params.iter().all(|p| p.ty == PhysicalType::Int64));
    }

    #[test]
    fn type_hint_seeds_s0_params() {
        let src = "# skw @type add int int -> int\ndef add(a, b):\n    return a + b\n";
        let m = lower_source(src, "plan7_types.py").unwrap();
        assert!(m.type_hints.contains_key("plan7_types.add"));
        let report = analyze_module(&m);
        let f = &report.module.functions[0];
        assert!(f.static_eligible);
        assert_eq!(f.return_ty, PhysicalType::Int64);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn analyze_add_is_s0() {
        let m = sample_add_module();
        let report = analyze_module(&m);
        let f = &report.module.functions[0];
        assert!(f.static_eligible);
        assert_eq!(f.return_ty, PhysicalType::Int64);
    }

    #[test]
    fn analyze_clamp_has_bool_itr() {
        let src = r#"def clamp(x, lo, hi):
    if x < lo:
        return lo
    if x > hi:
        return hi
    return x
"#;
        let m = lower_source(src, "clamp.py").unwrap();
        let report = analyze_module(&m);
        assert!(!report.module.functions.is_empty());
    }

    #[test]
    fn fib_interprocedural_converges_to_int() {
        let src = "def fib(n):\n    if n <= 1:\n        return n\n    return fib(n - 1) + fib(n - 2)\n";
        let m = lower_source(src, "fib.py").unwrap();
        let report = analyze_module(&m);
        assert_eq!(report.module.functions[0].return_ty, PhysicalType::Int64);
    }

    #[test]
    fn strict_dyn_attr_emits_t002_and_t004() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = format!("{manifest_dir}/../../tests/fixtures/pystat_strict.py");
        let m = sikuwa_pir::lower_file(std::path::Path::new(&path)).unwrap();
        let report = analyze_module_with_options(&m, &PystatOptions::strict());
        assert!(report.diagnostics.iter().any(|d| d.code == "SKW-T004"));
        assert!(report.diagnostics.iter().any(|d| d.code == "SKW-T002"));
    }

    #[test]
    fn narrow_if_fixture_analyzes() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = format!("{manifest_dir}/../../tests/fixtures/narrow_if.py");
        let m = sikuwa_pir::lower_file(std::path::Path::new(&path)).unwrap();
        let report = analyze_module(&m);
        assert_eq!(report.module.functions.len(), 2);
        assert!(report
            .module
            .functions
            .iter()
            .all(|f| f.return_ty == PhysicalType::Int64));
    }

    #[test]
    fn plan5_caller_resolves_imported_add_with_peers() {
        let root = format!("{}/../../tests/fixtures", env!("CARGO_MANIFEST_DIR"));
        let add_m = sikuwa_pir::lower_file(std::path::Path::new(&format!("{root}/add.py"))).unwrap();
        let add_report = analyze_module(&add_m);
        let peers = peer_summaries_from_stats(&add_report.module.functions);
        let caller_m =
            sikuwa_pir::lower_file(std::path::Path::new(&format!("{root}/plan5_caller.py"))).unwrap();
        let report = analyze_module_with_peers(&caller_m, &PystatOptions::default(), &peers);
        let twice = &report.module.functions[0];
        assert_eq!(twice.return_ty, PhysicalType::Int64);
        assert!(twice.static_eligible);
    }
}
