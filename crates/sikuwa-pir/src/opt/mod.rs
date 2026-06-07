//! PIR mid-end optimization — 35 Python keyword passes (AST-free, LLVM-style).

mod analysis;
mod inline;
mod keyword;
mod module_passes;
mod passes;

pub use analysis::{const_map, count_uses, reachable_blocks, ConstInfo};
pub use inline::pass_def_inline;
pub use keyword::{KeywordPassInfo, PythonKeyword};
pub use module_passes::{pass_import_dce, run_module_passes};
pub use passes::run_keyword_pass;

/// Optimization aggressiveness (mirrors LLVM `-O1`/`-O2` naming).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptLevel {
    #[default]
    O0,
    O1,
    O2,
}

impl OptLevel {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "0" | "o0" | "none" => Some(Self::O0),
            "1" | "o1" => Some(Self::O1),
            "2" | "o2" => Some(Self::O2),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PassRunReport {
    pub keyword: &'static str,
    pub llvm_analog: &'static str,
    pub changed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct OptReport {
    pub passes: Vec<PassRunReport>,
}

impl OptReport {
    pub fn changed_passes(&self) -> usize {
        self.passes.iter().filter(|p| p.changed).count()
    }

    pub fn any_changed(&self) -> bool {
        self.changed_passes() > 0
    }
}

/// Run the default keyword pass pipeline on every function in the module.
pub fn optimize_module(module: &mut crate::module::Module, level: OptLevel) -> OptReport {
    let mut report = OptReport::default();
    if level == OptLevel::O0 {
        return report;
    }

    let pipeline = pipeline_for_level(level);
    let rounds = if level == OptLevel::O2 { 3 } else { 1 };

    for _ in 0..rounds {
        for kw in &pipeline {
            for func in &mut module.functions {
                optimize_func_impl(func, *kw, &mut report);
            }
            for class in &mut module.classes {
                for method in &mut class.methods {
                    optimize_func_impl(method, *kw, &mut report);
                }
            }
        }
    }

    if level == OptLevel::O2 {
        if pass_def_inline(module) {
            push_pass_report(&mut report, PythonKeyword::Def);
        }
        if pass_import_dce(module) {
            push_pass_report(&mut report, PythonKeyword::Import);
            push_pass_report(&mut report, PythonKeyword::From);
        }
    }
    report
}

fn push_pass_report(report: &mut OptReport, kw: PythonKeyword) {
    if let Some(entry) = report.passes.iter_mut().find(|p| p.keyword == kw.name()) {
        entry.changed = true;
    } else {
        report.passes.push(PassRunReport {
            keyword: kw.name(),
            llvm_analog: kw.llvm_analog(),
            changed: true,
        });
    }
}

/// Run all keyword passes on a single function.
pub fn optimize_func(func: &mut crate::module::FuncDef, level: OptLevel) -> OptReport {
    let mut report = OptReport::default();
    if level == OptLevel::O0 {
        return report;
    }
    let pipeline = pipeline_for_level(level);
    let rounds = if level == OptLevel::O2 { 3 } else { 1 };
    for _ in 0..rounds {
        for kw in &pipeline {
            optimize_func_impl(func, *kw, &mut report);
        }
    }
    report
}

fn optimize_func_impl(
    func: &mut crate::module::FuncDef,
    kw: PythonKeyword,
    report: &mut OptReport,
) {
    let changed = run_keyword_pass(kw, func);
    if changed {
        if let Some(entry) = report.passes.iter_mut().find(|p| p.keyword == kw.name()) {
            entry.changed = true;
        } else {
            report.passes.push(PassRunReport {
                keyword: kw.name(),
                llvm_analog: kw.llvm_analog(),
                changed: true,
            });
        }
    }
    for nested in &mut func.nested {
        optimize_func_impl(nested, kw, report);
    }
}

/// Canonical pass order for keyword pipeline.
pub fn pipeline_for_level(level: OptLevel) -> Vec<PythonKeyword> {
    let mut pipe = vec![
        // Literals & const folding
        PythonKeyword::False,
        PythonKeyword::NoneKw,
        PythonKeyword::True,
        PythonKeyword::Not,
        PythonKeyword::Is,
        PythonKeyword::In,
        // Control flow
        PythonKeyword::If,
        PythonKeyword::Elif,
        PythonKeyword::Else,
        PythonKeyword::While,
        PythonKeyword::For,
        PythonKeyword::Break,
        PythonKeyword::Continue,
        PythonKeyword::Return,
        // Cleanup
        PythonKeyword::Pass,
        PythonKeyword::Del,
        PythonKeyword::As,
    ];

    if level == OptLevel::O2 {
        pipe.extend([
            PythonKeyword::And,
            PythonKeyword::Or,
            PythonKeyword::Lambda,
            PythonKeyword::Class,
            PythonKeyword::Global,
            PythonKeyword::Nonlocal,
            PythonKeyword::Assert,
            PythonKeyword::Raise,
            PythonKeyword::Try,
            PythonKeyword::Except,
            PythonKeyword::Finally,
            PythonKeyword::Async,
            PythonKeyword::Await,
            PythonKeyword::With,
            PythonKeyword::Yield,
        ]);
    }
    pipe
}

/// Metadata for all 35 keyword passes.
pub fn all_keyword_passes() -> Vec<KeywordPassInfo> {
    PythonKeyword::ALL.iter().map(|k| k.info()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exactly_35_keywords() {
        assert_eq!(PythonKeyword::ALL.len(), 35);
        assert_eq!(all_keyword_passes().len(), 35);
    }

    #[test]
    fn keyword_names_unique() {
        let names: std::collections::HashSet<_> =
            PythonKeyword::ALL.iter().map(|k| k.name()).collect();
        assert_eq!(names.len(), 35);
    }

    #[test]
    fn o2_pipeline_inlines_and_folds() {
        use crate::lower::lower_source;
        use crate::opcode::OpCode;

        let src = r#"def add(a, b):
    return a + b

def main():
    return add(1, 2)
"#;
        let mut module = lower_source(src, "t.py").unwrap();
        let report = optimize_module(&mut module, OptLevel::O2);
        assert!(report.changed_passes() > 0);
        let main_fn = module
            .functions
            .iter()
            .find(|f| f.symbol.0.ends_with("main"))
            .unwrap();
        assert!(!main_fn
            .blocks
            .iter()
            .flat_map(|b| &b.ops)
            .any(|o| o.opcode == OpCode::Call));
    }
}
