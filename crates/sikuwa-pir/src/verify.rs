use std::collections::HashSet;

use sikuwa_core::{Result, SikuwaError};

use crate::ids::{BlockId, ValueId};
use crate::module::{Block, FuncDef, Module, Terminator};
use crate::opcode::OpCode;

#[derive(Debug, Default, Clone)]
pub struct VerifyReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl VerifyReport {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }

    fn err(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
    }

    fn warn(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }
}

pub fn verify_module(module: &Module) -> VerifyReport {
    let mut report = VerifyReport::default();
    if module.name.is_empty() {
        report.err("module name must not be empty");
    }
    if module.functions.is_empty() && module.classes.is_empty() {
        report.warn("module has no functions or classes");
    }
    if module.exports.len() != module.functions.len() + module.classes.len() {
        report.warn(format!(
            "exports count ({}) != functions ({}) + classes ({})",
            module.exports.len(),
            module.functions.len(),
            module.classes.len()
        ));
    }
    for func in &module.functions {
        let fr = verify_func(func);
        report.errors.extend(fr.errors);
        report.warnings.extend(fr.warnings);
        for nested in &func.nested {
            let nr = verify_func(nested);
            report.errors.extend(nr.errors);
            report.warnings.extend(nr.warnings);
        }
    }
    for class in &module.classes {
        for method in &class.methods {
            let mr = verify_func(method);
            report.errors.extend(mr.errors);
            report.warnings.extend(mr.warnings);
        }
    }
    report
}

pub fn verify_func(func: &FuncDef) -> VerifyReport {
    let mut report = VerifyReport::default();
    if func.blocks.is_empty() {
        report.err(format!("{} has no basic blocks", func.symbol));
        return report;
    }

    let block_ids: HashSet<&BlockId> = func.blocks.iter().map(|b| &b.id).collect();
    if block_ids.len() != func.blocks.len() {
        report.err(format!("{} has duplicate block ids", func.symbol));
    }

    if func.blocks.first().map(|b| b.id.0.as_str()) != Some("entry") {
        report.warn(format!("{}: first block is not ^entry", func.symbol));
    }

    let mut defined_values: HashSet<ValueId> = HashSet::new();
    let mut next_expected = 0u32;

    for block in &func.blocks {
        verify_block(
            func,
            block,
            &block_ids,
            &mut defined_values,
            &mut next_expected,
            &mut report,
        );
    }

    report
}

fn verify_block(
    func: &FuncDef,
    block: &Block,
    block_ids: &HashSet<&BlockId>,
    defined_values: &mut HashSet<ValueId>,
    next_expected: &mut u32,
    report: &mut VerifyReport,
) {
    for phi in &block.phis {
        if defined_values.contains(&phi.result) {
            report.err(format!(
                "{}: phi result {} already defined",
                block.id, phi.result
            ));
        }
        if phi.incoming.len() < 2 {
            report.warn(format!(
                "{}: phi `{}` has fewer than 2 incoming edges",
                block.id, phi.name
            ));
        }
        for inc in &phi.incoming {
            if !block_ids.contains(&inc.block) {
                report.err(format!(
                    "{}: phi `{}` incoming from unknown block {}",
                    block.id, phi.name, inc.block
                ));
            }
            if !defined_values.contains(&inc.value) {
                report.warn(format!(
                    "{}: phi `{}` incoming value {} not yet defined",
                    block.id, phi.name, inc.value
                ));
            }
        }
        defined_values.insert(phi.result);
        if phi.result.0 != *next_expected {
            report.warn(format!(
                "{}: SSA gap at phi — expected %{} got {}",
                block.id, next_expected, phi.result
            ));
        }
        *next_expected = phi.result.0 + 1;
    }

    for op in &block.ops {
        if let Some(result) = op.result {
            if defined_values.contains(&result) {
                report.err(format!(
                    "{}: SSA value {} defined twice",
                    block.id, result
                ));
            }
            if result.0 != *next_expected {
                report.warn(format!(
                    "{}: SSA gap — expected %{} got {}",
                    block.id, next_expected, result
                ));
            }
            defined_values.insert(result);
            *next_expected = result.0 + 1;
        }

        if matches!(op.opcode, OpCode::StoreFast) && op.result.is_some() {
            report.warn(format!(
                "{}: store_fast should not define a result",
                block.id
            ));
        }
    }

    match &block.term {
        Terminator::Branch { target } => {
            if !block_ids.contains(target) {
                report.err(format!("{}: branch to unknown block {}", block.id, target));
            }
        }
        Terminator::CondBranch {
            cond,
            then_block,
            else_block,
        } => {
            if !defined_values.contains(cond) {
                report.warn(format!(
                    "{}: condition {} used before definition",
                    block.id, cond
                ));
            }
            if !block_ids.contains(then_block) {
                report.err(format!(
                    "{}: branch to unknown block {}",
                    block.id, then_block
                ));
            }
            if !block_ids.contains(else_block) {
                report.err(format!(
                    "{}: branch to unknown block {}",
                    block.id, else_block
                ));
            }
        }
        Terminator::Return { value: Some(v) } => {
            if !defined_values.contains(v) {
                report.warn(format!(
                    "{}: return value {} used before definition",
                    block.id, v
                ));
            }
        }
        Terminator::Return { value: None } | Terminator::Unreachable => {}
    }

    let _ = func; // reserved for param checks
}

pub fn ensure_valid_module(module: &Module) -> Result<()> {
    let report = verify_module(module);
    if report.ok() {
        Ok(())
    } else {
        Err(SikuwaError::pir(report.errors.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::sample_add_module;

    #[test]
    fn sample_module_valid() {
        let module = sample_add_module();
        let report = verify_module(&module);
        assert!(report.ok(), "{:?}", report.errors);
    }
}
