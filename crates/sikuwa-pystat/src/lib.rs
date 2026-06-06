//! PyStat — PGTE (physical type graph) + ITR (in-place type replacement) analysis.

mod analyze;
mod pstat;
mod types;

pub use analyze::{analyze_func, analyze_module, PystatReport};
pub use pstat::{pstat_from_reader, pstat_to_json, pstat_to_writer, read_pstat, write_pstat, PSTAT_MAGIC};
pub use types::{FuncStat, LogicalSlot, PhysicalType, PystatModule, SlotLevel, SlotStrategy};
