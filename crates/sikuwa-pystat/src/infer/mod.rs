//! HPGI inference core (Plan 6a+).

mod cg;
mod layout;
mod logical;
mod scc;
mod sparse;

pub use cg::{build_call_graph, collect_callees, resolve_callee, CallGraph};
pub use layout::{logical_arm_name, materialize_slot, SlotMaterialization};
pub use logical::{
    from_physical, join, meet, normalize_type, normalize_union, project_to_physical, LiteralValue,
    LogicalType, UNION_CAP,
};
pub use scc::{
    apply_fixpoint, is_nontrivial_scc, lookup_summary, seed_summaries, summaries_map,
    tarjan_scc, FuncSummary, SummaryCertainty, MAX_SCC_ITER,
};
pub use sparse::SparseEnvironment;
