//! Human-readable `.pir` text dump.

use std::fmt::Write;

use crate::module::{FuncDef, Module, OpOperand, Terminator};

pub fn module_to_text(module: &Module) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "; sikuwa-pir v1");
    let _ = writeln!(out, "; module: {}", module.name);
    let _ = writeln!(out, "; hash: {}", hex32(module.source_hash));
    let _ = writeln!(out);

    for func in &module.functions {
        func_to_text(func, &mut out);
        let _ = writeln!(out);
    }
    for class in &module.classes {
        let _ = writeln!(out, "@class {}:", class.symbol);
        for method in &class.methods {
            func_to_text(method, &mut out);
        }
        let _ = writeln!(out);
    }
    out
}

fn func_to_text(func: &FuncDef, out: &mut String) {
    let _ = writeln!(out, "@export func {}({}):", func.symbol, func.params.join(", "));
    for block in &func.blocks {
        let _ = writeln!(out, "^{}:", block.id.0);
        for phi in &block.phis {
            let mut inc = String::new();
            for (i, edge) in phi.incoming.iter().enumerate() {
                if i > 0 {
                    inc.push_str(", ");
                }
                let _ = write!(inc, "[{}] {}", edge.block, edge.value);
            }
            let _ = writeln!(
                out,
                "  {} = phi {} ({})",
                phi.result, phi.name, inc
            );
        }
        for op in &block.ops {
            let mut line = String::new();
            if let Some(r) = op.result {
                let _ = write!(line, "{r} = ");
            }
            let _ = write!(line, "{}", op.opcode.name());
            for opd in &op.operands {
                let _ = write!(line, " {}", operand(opd));
            }
            let _ = writeln!(out, "  {line}");
        }
        match &block.term {
            Terminator::Branch { target } => {
                let _ = writeln!(out, "  br {target}");
            }
            Terminator::CondBranch {
                cond,
                then_block,
                else_block,
            } => {
                let _ = writeln!(out, "  cond_br {cond}, {then_block}, {else_block}");
            }
            Terminator::Return { value: Some(v) } => {
                let _ = writeln!(out, "  ret {v}");
            }
            Terminator::Return { value: None } => {
                let _ = writeln!(out, "  ret void");
            }
            Terminator::Unreachable => {
                let _ = writeln!(out, "  unreachable");
            }
        }
    }
}

fn operand(op: &OpOperand) -> String {
    match op {
        OpOperand::Value(v) => v.to_string(),
        OpOperand::Symbol(s) => s.to_string(),
        OpOperand::Const(c) => format!("{c:?}"),
        OpOperand::Name(n) => n.clone(),
        OpOperand::Block(b) => b.to_string(),
    }
}

fn hex32(bytes: [u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lower_source, sample_add_module};

    #[test]
    fn dump_sample() {
        let text = module_to_text(&sample_add_module());
        assert!(text.contains("binop_add"));
    }

    #[test]
    fn dump_lowered_add() {
        let m = lower_source("def add(a, b):\n    return a + b\n", "add.py").unwrap();
        let text = module_to_text(&m);
        assert!(text.contains("load_fast"));
        assert!(text.contains("binop_add"));
    }
}
