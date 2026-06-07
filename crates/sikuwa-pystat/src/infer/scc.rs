//! Tarjan SCC + inter-procedural fixpoint with `MAX_SCC_ITER` widen guard.

use sikuwa_pir::ids::SymbolRef;
use sikuwa_pir::module::Module;

use super::cg::CallGraph;
use super::logical::{join, LogicalType};

pub const MAX_SCC_ITER: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryCertainty {
    Inferred,
    Assumed,
    Widened,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncSummary {
    pub return_ty: LogicalType,
    pub certainty: SummaryCertainty,
}

impl FuncSummary {
    pub fn assumed_numeric() -> Self {
        Self {
            return_ty: LogicalType::Int,
            certainty: SummaryCertainty::Assumed,
        }
    }
}

pub fn tarjan_scc(n: usize, adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut index = 0usize;
    let mut stack = Vec::new();
    let mut on_stack = vec![false; n];
    let mut indices = vec![None; n];
    let mut lowlink = vec![0usize; n];
    let mut sccs = Vec::new();

    for start in 0..n {
        if indices[start].is_none() {
            strong_connect(
                start,
                adj,
                &mut index,
                &mut stack,
                &mut on_stack,
                &mut indices,
                &mut lowlink,
                &mut sccs,
            );
        }
    }
    sccs
}

fn strong_connect(
    v: usize,
    adj: &[Vec<usize>],
    index: &mut usize,
    stack: &mut Vec<usize>,
    on_stack: &mut [bool],
    indices: &mut [Option<usize>],
    lowlink: &mut [usize],
    sccs: &mut Vec<Vec<usize>>,
) {
    indices[v] = Some(*index);
    lowlink[v] = *index;
    *index += 1;
    stack.push(v);
    on_stack[v] = true;

    for &w in &adj[v] {
        if indices[w].is_none() {
            strong_connect(w, adj, index, stack, on_stack, indices, lowlink, sccs);
            lowlink[v] = lowlink[v].min(lowlink[w]);
        } else if on_stack[w] {
            lowlink[v] = lowlink[v].min(indices[w].unwrap());
        }
    }

    if lowlink[v] == indices[v].unwrap() {
        let mut comp = Vec::new();
        loop {
            let w = stack.pop().unwrap();
            on_stack[w] = false;
            comp.push(w);
            if w == v {
                break;
            }
        }
        sccs.push(comp);
    }
}

pub fn is_nontrivial_scc(comp: &[usize], adj: &[Vec<usize>]) -> bool {
    if comp.len() > 1 {
        return true;
    }
    let i = comp[0];
    adj[i].contains(&i)
}

/// Seed summaries; recursive SCCs start as `Assumed` numeric.
pub fn seed_summaries(module: &Module, graph: &CallGraph) -> Vec<FuncSummary> {
    let n = graph.symbols.len();
    let sccs = tarjan_scc(n, &graph.edges);
    let mut node_scc = vec![0usize; n];
    for (id, comp) in sccs.iter().enumerate() {
        for &i in comp {
            node_scc[i] = id;
        }
    }

    let mut out = vec![FuncSummary::assumed_numeric(); n];
    for (id, comp) in sccs.iter().enumerate() {
        if is_nontrivial_scc(comp, &graph.edges) {
            for &i in comp {
                out[i] = FuncSummary::assumed_numeric();
            }
        } else {
            for &i in comp {
                let _ = id;
                out[i] = FuncSummary {
                    return_ty: LogicalType::Top,
                    certainty: SummaryCertainty::Inferred,
                };
            }
        }
    }
    let _ = module;
    out
}

pub fn apply_fixpoint<F>(mut summaries: Vec<FuncSummary>, mut analyze: F) -> Vec<FuncSummary>
where
    F: FnMut(&[FuncSummary]) -> Vec<LogicalType>,
{
    for iter in 0..MAX_SCC_ITER {
        let returns = analyze(&summaries);
        let mut changed = false;
        for (sum, new_ret) in summaries.iter_mut().zip(returns) {
            let merged = join(sum.return_ty.clone(), new_ret);
            if merged != sum.return_ty {
                sum.return_ty = merged;
                sum.certainty = SummaryCertainty::Inferred;
                changed = true;
            }
        }
        if !changed {
            return summaries;
        }
        if iter + 1 == MAX_SCC_ITER {
            for sum in &mut summaries {
                sum.return_ty = LogicalType::Dyn;
                sum.certainty = SummaryCertainty::Widened;
            }
        }
    }
    summaries
}

pub fn summaries_map(
    graph: &CallGraph,
    summaries: &[FuncSummary],
) -> std::collections::HashMap<String, FuncSummary> {
    graph
        .symbols
        .iter()
        .zip(summaries.iter())
        .map(|(s, sum)| (s.0.clone(), sum.clone()))
        .collect()
}

pub fn lookup_summary<'a>(
    map: &'a std::collections::HashMap<String, FuncSummary>,
    sym: &SymbolRef,
) -> Option<&'a FuncSummary> {
    map.get(&sym.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infer::cg::build_call_graph;
    use sikuwa_pir::lower_source;

    #[test]
    fn tarjan_finds_single_self_loop_scc() {
        let sccs = tarjan_scc(1, &[vec![0]]);
        assert_eq!(sccs.len(), 1);
        assert_eq!(sccs[0], vec![0]);
    }

    #[test]
    fn fib_seed_is_assumed() {
        let src = "def fib(n):\n    if n <= 1:\n        return n\n    return fib(n - 1) + fib(n - 2)\n";
        let m = lower_source(src, "fib.py").unwrap();
        let g = build_call_graph(&m);
        let sums = seed_summaries(&m, &g);
        assert_eq!(sums[0].certainty, SummaryCertainty::Assumed);
        assert_eq!(sums[0].return_ty, LogicalType::Int);
    }

    #[test]
    fn fixpoint_converges_without_widen_for_add() {
        let mut sums = vec![FuncSummary {
            return_ty: LogicalType::Top,
            certainty: SummaryCertainty::Inferred,
        }];
        let out = apply_fixpoint(sums, |_| vec![LogicalType::Int]);
        assert_eq!(out[0].return_ty, LogicalType::Int);
        assert_ne!(out[0].certainty, SummaryCertainty::Widened);
    }
}
