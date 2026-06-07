//! Plan 6c golden compile pipeline: PIR O1 → HPGI → PIR O2.

use std::collections::HashMap;

use sikuwa_core::{Result, SikuwaError};
use sikuwa_pir::{optimize_module, verify_module, Module, OptLevel, OptReport};
use sikuwa_pystat::{
    analyze_module_with_options, analyze_module_with_peers, PystatOptions, PystatReport,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineMode {
    /// No PIR optimization; PyStat only.
    None,
    /// Single `optimize_module` pass (legacy).
    SinglePass(OptLevel),
    /// O1 → analyze (HPGI) → O2.
    Golden,
}

impl PipelineMode {
    pub fn from_opt_flags(opt: bool, single_pass: bool, level: OptLevel) -> Self {
        if !opt {
            return Self::None;
        }
        if single_pass {
            Self::SinglePass(level)
        } else {
            Self::Golden
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompilePipelineReport {
    pub mode: PipelineModeLabel,
    pub opt_o1: Option<OptReport>,
    pub opt_single: Option<OptReport>,
    pub opt_o2: Option<OptReport>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PipelineModeLabel {
    #[default]
    None,
    SinglePass,
    Golden,
}

impl CompilePipelineReport {
    pub fn total_pass_changes(&self) -> usize {
        self.opt_o1.as_ref().map(|r| r.changed_passes()).unwrap_or(0)
            + self
                .opt_single
                .as_ref()
                .map(|r| r.changed_passes())
                .unwrap_or(0)
            + self.opt_o2.as_ref().map(|r| r.changed_passes()).unwrap_or(0)
    }
}

/// Run the selected compile pipeline and return PyStat analysis of the final IR.
pub fn run_compile_pipeline(
    module: &mut Module,
    mode: PipelineMode,
) -> Result<(PystatReport, CompilePipelineReport)> {
    run_compile_pipeline_with_options(module, mode, &PystatOptions::default())
}

pub fn run_compile_pipeline_with_options(
    module: &mut Module,
    mode: PipelineMode,
    pystat_opts: &PystatOptions,
) -> Result<(PystatReport, CompilePipelineReport)> {
    run_compile_pipeline_with_peers(module, mode, pystat_opts, &HashMap::new())
}

pub fn run_compile_pipeline_with_peers(
    module: &mut Module,
    mode: PipelineMode,
    pystat_opts: &PystatOptions,
    peer_summaries: &HashMap<String, sikuwa_pystat::FuncSummary>,
) -> Result<(PystatReport, CompilePipelineReport)> {
    let analyze = |m: &Module| analyze_module_with_peers(m, pystat_opts, peer_summaries);
    match mode {
        PipelineMode::None => {
            let pystat = analyze(module);
            Ok((
                pystat,
                CompilePipelineReport {
                    mode: PipelineModeLabel::None,
                    ..Default::default()
                },
            ))
        }
        PipelineMode::SinglePass(level) => {
            let opt_single = optimize_module(module, level);
            verify_after_opt(module, "single-pass")?;
            let pystat = analyze(module);
            Ok((
                pystat,
                CompilePipelineReport {
                    mode: PipelineModeLabel::SinglePass,
                    opt_single: Some(opt_single),
                    ..Default::default()
                },
            ))
        }
        PipelineMode::Golden => run_golden_pipeline_with_peers(module, pystat_opts, peer_summaries),
    }
}

pub fn run_golden_pipeline(module: &mut Module) -> Result<(PystatReport, CompilePipelineReport)> {
    run_golden_pipeline_with_options(module, &PystatOptions::default())
}

pub fn run_golden_pipeline_with_options(
    module: &mut Module,
    pystat_opts: &PystatOptions,
) -> Result<(PystatReport, CompilePipelineReport)> {
    run_golden_pipeline_with_peers(module, pystat_opts, &HashMap::new())
}

pub fn run_golden_pipeline_with_peers(
    module: &mut Module,
    pystat_opts: &PystatOptions,
    peer_summaries: &HashMap<String, sikuwa_pystat::FuncSummary>,
) -> Result<(PystatReport, CompilePipelineReport)> {
    let opt_o1 = optimize_module(module, OptLevel::O1);
    verify_after_opt(module, "O1")?;

    let pystat = analyze_module_with_peers(module, pystat_opts, peer_summaries);

    let opt_o2 = optimize_module(module, OptLevel::O2);
    verify_after_opt(module, "O2")?;

    Ok((
        pystat,
        CompilePipelineReport {
            mode: PipelineModeLabel::Golden,
            opt_o1: Some(opt_o1),
            opt_o2: Some(opt_o2),
            ..Default::default()
        },
    ))
}

fn verify_after_opt(module: &Module, stage: &str) -> Result<()> {
    let v = verify_module(module);
    if v.ok() {
        return Ok(());
    }
    Err(SikuwaError::pir(format!(
        "PIR verify failed after {stage}: {}",
        v.errors.join("; ")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sikuwa_pir::lower_source;
    use sikuwa_pir::opcode::OpCode;

    #[test]
    fn golden_pipeline_folds_const_if() {
        let src = r#"def const_if():
    if True:
        return 10
    return 0
"#;
        let mut m = lower_source(src, "opt_const.py").unwrap();
        let (_, pipe) = run_golden_pipeline(&mut m).unwrap();
        assert_eq!(pipe.mode, PipelineModeLabel::Golden);
        assert!(pipe.total_pass_changes() > 0);
        let f = &m.functions[0];
        assert!(!f.blocks.iter().flat_map(|b| &b.ops).any(|o| {
            matches!(
                o.opcode,
                OpCode::CompareEq | OpCode::CompareLt | OpCode::CompareIs
            )
        }));
    }

    #[test]
    fn golden_inlines_add_into_main() {
        let src = r#"def add(a, b):
    return a + b

def main():
    return add(1, 2)
"#;
        let mut m = lower_source(src, "t.py").unwrap();
        run_golden_pipeline(&mut m).unwrap();
        let main_fn = m
            .functions
            .iter()
            .find(|f| f.symbol.0.ends_with(".main"))
            .unwrap();
        assert!(!main_fn
            .blocks
            .iter()
            .flat_map(|b| &b.ops)
            .any(|o| o.opcode == OpCode::Call));
    }
}
