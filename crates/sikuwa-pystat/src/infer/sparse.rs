//! Sparse slot environment — only track `load_fast` / `store_fast` / `phi` names.

use std::collections::HashMap;

use super::logical::{join, LogicalType};

/// Tracks logical types for named slots (params, phi names, stored locals).
#[derive(Debug, Clone, Default)]
pub struct SparseEnvironment {
    slots: HashMap<String, LogicalType>,
}

impl SparseEnvironment {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed a parameter or previously unseen slot as `Top` (unknown).
    pub fn seed(&mut self, name: impl Into<String>) {
        self.slots.entry(name.into()).or_insert(LogicalType::Top);
    }

    pub fn get(&self, name: &str) -> LogicalType {
        self.slots.get(name).cloned().unwrap_or(LogicalType::Top)
    }

    /// Join incoming type at `store_fast` / assignment.
    pub fn join_slot(&mut self, name: impl Into<String>, ty: LogicalType) {
        let name = name.into();
        self.slots
            .entry(name)
            .and_modify(|existing| *existing = join(existing.clone(), ty.clone()))
            .or_insert(ty);
    }

    /// Merge phi incoming values into the phi's logical slot name.
    pub fn merge_phi(&mut self, name: impl Into<String>, incoming: impl IntoIterator<Item = LogicalType>) {
        let mut merged = LogicalType::Bottom;
        for ty in incoming {
            merged = join(merged, ty);
        }
        if merged.is_bottom() {
            merged = LogicalType::Top;
        }
        self.join_slot(name, merged);
    }

    pub fn set_exact(&mut self, name: impl Into<String>, ty: LogicalType) {
        self.slots.insert(name.into(), ty);
    }

    pub fn contains(&self, name: &str) -> bool {
        self.slots.contains_key(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, LogicalType)> {
        self.slots
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
    }

    /// Materialize only slots that were touched or appear in `locals`.
    pub fn snapshot_for_locals(&self, locals: &[String]) -> HashMap<String, LogicalType> {
        let mut out = HashMap::new();
        for name in locals {
            if let Some(ty) = self.slots.get(name) {
                out.insert(name.clone(), ty.clone());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infer::logical::LiteralValue;

    #[test]
    fn sparse_only_tracks_touched_slots() {
        let mut env = SparseEnvironment::new();
        env.seed("a");
        env.join_slot("a", LogicalType::Literal(LiteralValue::Int(1)));
        env.join_slot("a", LogicalType::Literal(LiteralValue::Int(2)));
        assert_eq!(env.get("a"), LogicalType::Int);
        assert_eq!(env.get("never_loaded"), LogicalType::Top);
        assert!(!env.contains("never_loaded"));
    }

    #[test]
    fn phi_merge_joins_incoming() {
        let mut env = SparseEnvironment::new();
        env.merge_phi(
            "x",
            [
                LogicalType::Literal(LiteralValue::Int(1)),
                LogicalType::Literal(LiteralValue::Int(2)),
            ],
        );
        assert_eq!(env.get("x"), LogicalType::Int);
    }
}
