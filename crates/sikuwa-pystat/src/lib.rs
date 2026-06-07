//! PyStat — PGTE (physical type graph) + ITR (in-place type replacement) analysis.

mod analyze;
mod config;
mod diagnostic;
mod evidence;
mod infer;
mod pass2;
mod pass3;
mod pass4;
mod pass5;
mod pstat;
mod types;

pub use analyze::{
    analyze_func, analyze_module, analyze_module_with_options, analyze_module_with_peers,
    peer_summaries_from_stats, PystatReport,
};
pub use config::{MinSlot, PystatMode, PystatOptions};
pub use diagnostic::PystatDiagnostic;
pub use evidence::{hint_for_func, parse_type_name, seed_params_from_hint};
pub use infer::{
    apply_fixpoint, build_call_graph, from_physical, join, lookup_summary, materialize_slot,
    meet, normalize_type, normalize_union, project_to_physical, seed_summaries, summaries_map,
    FuncSummary, LiteralValue, LogicalType, SparseEnvironment, SummaryCertainty, UNION_CAP,
    MAX_SCC_ITER,
};
pub use pstat::{pstat_from_reader, pstat_to_json, pstat_to_writer, read_pstat, write_pstat, PSTAT_MAGIC};
pub use types::{FuncStat, LogicalSlot, PhysicalType, PystatModule, SlotLevel, SlotStrategy, TaggedLayout};
