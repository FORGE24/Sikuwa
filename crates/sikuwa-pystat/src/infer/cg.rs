//! Call graph construction + local callee resolution (Plan 6b).

use sikuwa_pir::ids::SymbolRef;
use sikuwa_pir::module::{FuncDef, Module, Op, OpOperand};
use sikuwa_pir::opcode::OpCode;

#[derive(Debug, Clone, Default)]
pub struct CallGraph {
    pub symbols: Vec<SymbolRef>,
    pub index: std::collections::HashMap<String, usize>,
    /// caller index → callee indices (may contain duplicates).
    pub edges: Vec<Vec<usize>>,
}

impl CallGraph {
    pub fn callee_indices(&self, caller: usize) -> &[usize] {
        &self.edges[caller]
    }
}

pub fn build_call_graph(module: &Module) -> CallGraph {
    let mut symbols = Vec::new();
    let mut index = std::collections::HashMap::new();
    for f in &module.functions {
        let i = symbols.len();
        index.insert(f.symbol.0.clone(), i);
        symbols.push(f.symbol.clone());
    }

    let mut edges = vec![Vec::new(); symbols.len()];
    for (i, f) in module.functions.iter().enumerate() {
        for callee in collect_callees(module, f) {
            if let Some(&j) = index.get(&callee.0) {
                edges[i].push(j);
            }
        }
    }

    CallGraph {
        symbols,
        index,
        edges,
    }
}

pub fn collect_callees(module: &Module, func: &FuncDef) -> Vec<SymbolRef> {
    let mut out = Vec::new();
    for block in &func.blocks {
        for op in &block.ops {
            if !matches!(op.opcode, OpCode::Call) {
                continue;
            }
            if let Some(sym) = resolve_callee(module, func, op) {
                out.push(sym);
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.dedup();
    out
}

pub fn resolve_callee(module: &Module, _caller: &FuncDef, op: &Op) -> Option<SymbolRef> {
    match op.operands.first()? {
        OpOperand::Symbol(s) => Some(s.clone()),
        OpOperand::Name(n) => {
            let sym = format!("{}.{}", module.name, n);
            module
                .functions
                .iter()
                .find(|f| f.symbol.0 == sym)
                .map(|f| f.symbol.clone())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sikuwa_pir::lower_source;

    #[test]
    fn graph_finds_self_recursive_edge() {
        let src = "def fib(n):\n    if n <= 1:\n        return n\n    return fib(n - 1) + fib(n - 2)\n";
        let m = lower_source(src, "fib.py").unwrap();
        let g = build_call_graph(&m);
        assert_eq!(g.symbols.len(), 1);
        assert_eq!(g.edges[0], vec![0]);
    }
}
