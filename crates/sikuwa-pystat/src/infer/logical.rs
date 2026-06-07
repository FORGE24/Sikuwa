//! HPGI logical type lattice (Plan 6a): join/meet + `normalize_union`.

use crate::types::PhysicalType;

/// Max distinct union arms before widening to `Dyn`.
pub const UNION_CAP: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LiteralValue {
    Int(i64),
    Bool(bool),
    Float(u64),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LogicalType {
    Bottom,
    Top,
    Dyn,
    None,
    Bool,
    Int,
    Float,
    Str,
    Literal(LiteralValue),
    Union(Vec<LogicalType>),
    Optional(Box<LogicalType>),
}

impl LogicalType {
    pub fn is_bottom(&self) -> bool {
        matches!(self, Self::Bottom)
    }

    pub fn is_dyn(&self) -> bool {
        matches!(self, Self::Dyn)
    }
}

/// Flatten, deduplicate, sort `None` last; widen to `Dyn` when `|arms| > cap`.
pub fn normalize_union(mut arms: Vec<LogicalType>, cap: usize) -> LogicalType {
    arms = flatten_union_arms(arms);
    dedup_sort_union(&mut arms);
    match arms.len() {
        0 => LogicalType::Bottom,
        1 => arms.into_iter().next().unwrap(),
        n if n <= cap => collapse_union_vec(arms),
        _ => LogicalType::Dyn,
    }
}

fn collapse_union_vec(arms: Vec<LogicalType>) -> LogicalType {
    debug_assert!(!arms.is_empty());
    if arms.len() == 1 {
        arms.into_iter().next().unwrap()
    } else {
        LogicalType::Union(arms)
    }
}

fn flatten_union_arms(arms: Vec<LogicalType>) -> Vec<LogicalType> {
    let mut out = Vec::new();
    for ty in arms {
        match ty {
            LogicalType::Bottom => {}
            LogicalType::Union(inner) => out.extend(flatten_union_arms(inner)),
            other => out.push(other),
        }
    }
    out
}

fn dedup_sort_union(arms: &mut Vec<LogicalType>) {
    arms.sort_by(cmp_logical_type);
    arms.dedup();
    let none_pos = arms.iter().position(|t| *t == LogicalType::None);
    if let Some(i) = none_pos {
        let none = arms.remove(i);
        arms.push(none);
    }
}

fn cmp_logical_type(a: &LogicalType, b: &LogicalType) -> std::cmp::Ordering {
    format!("{a:?}").cmp(&format!("{b:?}"))
}

/// Least upper bound (join, ⊔).
pub fn join(a: LogicalType, b: LogicalType) -> LogicalType {
    let a = normalize_type(a);
    let b = normalize_type(b);
    if a == b {
        return a;
    }
    if a == LogicalType::Bottom {
        return b;
    }
    if b == LogicalType::Bottom {
        return a;
    }
    if a == LogicalType::Top {
        return b;
    }
    if b == LogicalType::Top {
        return a;
    }
    if a == LogicalType::Dyn || b == LogicalType::Dyn {
        return LogicalType::Dyn;
    }

    match (&a, &b) {
        (LogicalType::Literal(la), LogicalType::Literal(lb)) => join_literals(*la, *lb),
        (LogicalType::Literal(LiteralValue::Int(_)), LogicalType::Int)
        | (LogicalType::Int, LogicalType::Literal(LiteralValue::Int(_))) => LogicalType::Int,
        (LogicalType::Literal(LiteralValue::Bool(_)), LogicalType::Bool)
        | (LogicalType::Bool, LogicalType::Literal(LiteralValue::Bool(_))) => LogicalType::Bool,
        (LogicalType::Literal(LiteralValue::Float(_)), LogicalType::Float)
        | (LogicalType::Float, LogicalType::Literal(LiteralValue::Float(_))) => LogicalType::Float,
        (LogicalType::Literal(LiteralValue::None), LogicalType::None)
        | (LogicalType::None, LogicalType::Literal(LiteralValue::None)) => LogicalType::None,
        (LogicalType::Int, LogicalType::Bool) | (LogicalType::Bool, LogicalType::Int) => {
            LogicalType::Int
        }
        (LogicalType::Int, LogicalType::Float) | (LogicalType::Float, LogicalType::Int) => {
            normalize_union(vec![LogicalType::Int, LogicalType::Float], UNION_CAP)
        }
        (LogicalType::Optional(ta), LogicalType::Optional(tb)) => {
            LogicalType::Optional(Box::new(join(*ta.clone(), *tb.clone())))
        }
        (LogicalType::Optional(t), LogicalType::None) | (LogicalType::None, LogicalType::Optional(t)) => {
            LogicalType::Optional(Box::new(join(*t.clone(), LogicalType::None)))
        }
        (LogicalType::Union(_), _) | (_, LogicalType::Union(_)) => {
            let mut arms = union_arms(a);
            arms.extend(union_arms(b));
            normalize_union(arms, UNION_CAP)
        }
        _ => normalize_union(vec![a, b], UNION_CAP),
    }
}

fn join_literals(a: LiteralValue, b: LiteralValue) -> LogicalType {
    if a == b {
        return LogicalType::Literal(a);
    }
    match (a, b) {
        (LiteralValue::Int(_), LiteralValue::Int(_)) => LogicalType::Int,
        (LiteralValue::Bool(_), LiteralValue::Bool(_)) => LogicalType::Bool,
        (LiteralValue::Float(_), LiteralValue::Float(_)) => LogicalType::Float,
        _ => LogicalType::Dyn,
    }
}

fn union_arms(ty: LogicalType) -> Vec<LogicalType> {
    match ty {
        LogicalType::Union(v) => v,
        other => vec![other],
    }
}

/// Greatest lower bound (meet, ⊓).
pub fn meet(a: LogicalType, b: LogicalType) -> LogicalType {
    let a = normalize_type(a);
    let b = normalize_type(b);
    if a == b {
        return a;
    }
    if a == LogicalType::Bottom || b == LogicalType::Bottom {
        return LogicalType::Bottom;
    }
    if a == LogicalType::Top {
        return b;
    }
    if b == LogicalType::Top {
        return a;
    }
    if a == LogicalType::Dyn || b == LogicalType::Dyn {
        return LogicalType::Dyn;
    }

    match (&a, &b) {
        (LogicalType::Literal(la), LogicalType::Literal(lb)) => {
            if la == lb {
                LogicalType::Literal(*la)
            } else {
                LogicalType::Bottom
            }
        }
        (LogicalType::Literal(LiteralValue::Int(v)), LogicalType::Int)
        | (LogicalType::Int, LogicalType::Literal(LiteralValue::Int(v))) => {
            LogicalType::Literal(LiteralValue::Int(*v))
        }
        (LogicalType::Literal(LiteralValue::Bool(v)), LogicalType::Bool)
        | (LogicalType::Bool, LogicalType::Literal(LiteralValue::Bool(v))) => {
            LogicalType::Literal(LiteralValue::Bool(*v))
        }
        (LogicalType::Literal(LiteralValue::None), LogicalType::None)
        | (LogicalType::None, LogicalType::Literal(LiteralValue::None)) => LogicalType::None,
        (LogicalType::Optional(ta), LogicalType::Optional(tb)) => {
            LogicalType::Optional(Box::new(meet(*ta.clone(), *tb.clone())))
        }
        (LogicalType::Optional(t), LogicalType::None) => {
            LogicalType::Optional(Box::new(meet(*t.clone(), LogicalType::None)))
        }
        (LogicalType::None, LogicalType::Optional(t)) => {
            LogicalType::Optional(Box::new(meet(LogicalType::None, *t.clone())))
        }
        (LogicalType::Int, LogicalType::Bool) | (LogicalType::Bool, LogicalType::Int) => {
            LogicalType::Bottom
        }
        _ => LogicalType::Bottom,
    }
}

pub fn normalize_type(ty: LogicalType) -> LogicalType {
    match ty {
        LogicalType::Union(arms) => normalize_union(arms, UNION_CAP),
        LogicalType::Optional(inner) => {
            let inner = normalize_type(*inner);
            if inner == LogicalType::None {
                LogicalType::None
            } else if inner == LogicalType::Bottom {
                LogicalType::Bottom
            } else {
                LogicalType::Optional(Box::new(inner))
            }
        }
        other => other,
    }
}

pub fn from_physical(pt: PhysicalType) -> LogicalType {
    match pt {
        PhysicalType::None => LogicalType::None,
        PhysicalType::Bool => LogicalType::Bool,
        PhysicalType::Int64 => LogicalType::Int,
        PhysicalType::Float64 => LogicalType::Float,
        PhysicalType::Str => LogicalType::Str,
        PhysicalType::Object => LogicalType::Dyn,
        PhysicalType::Dyn => LogicalType::Dyn,
        PhysicalType::Unknown => LogicalType::Top,
    }
}

pub fn project_to_physical(ty: LogicalType) -> PhysicalType {
    match normalize_type(ty) {
        LogicalType::Bottom | LogicalType::Top => PhysicalType::Unknown,
        LogicalType::Dyn | LogicalType::Union(_) | LogicalType::Optional(_) => PhysicalType::Dyn,
        LogicalType::None => PhysicalType::None,
        LogicalType::Bool => PhysicalType::Bool,
        LogicalType::Int | LogicalType::Literal(LiteralValue::Int(_)) => PhysicalType::Int64,
        LogicalType::Float | LogicalType::Literal(LiteralValue::Float(_)) => PhysicalType::Float64,
        LogicalType::Str => PhysicalType::Str,
        LogicalType::Literal(LiteralValue::Bool(_)) => PhysicalType::Bool,
        LogicalType::Literal(LiteralValue::None) => PhysicalType::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit_int(n: i64) -> LogicalType {
        LogicalType::Literal(LiteralValue::Int(n))
    }

    #[test]
    fn join_literal_ints_widen_to_int() {
        assert_eq!(join(lit_int(3), lit_int(5)), LogicalType::Int);
    }

    #[test]
    fn join_same_literal_stays() {
        assert_eq!(join(lit_int(3), lit_int(3)), lit_int(3));
    }

    #[test]
    fn meet_distinct_literal_ints_is_bottom() {
        assert_eq!(meet(lit_int(3), lit_int(5)), LogicalType::Bottom);
    }

    #[test]
    fn meet_literal_with_int() {
        assert_eq!(meet(lit_int(7), LogicalType::Int), lit_int(7));
    }

    #[test]
    fn union_over_cap_collapses_to_dyn() {
        let arms: Vec<_> = (0..5).map(|i| lit_int(i)).collect();
        assert_eq!(normalize_union(arms, UNION_CAP), LogicalType::Dyn);
    }

    #[test]
    fn union_at_cap_stays_union() {
        let arms = vec![
            LogicalType::None,
            LogicalType::Int,
            LogicalType::Bool,
            LogicalType::Str,
        ];
        match normalize_union(arms, UNION_CAP) {
            LogicalType::Union(v) => assert_eq!(v.len(), 4),
            other => panic!("expected Union, got {other:?}"),
        }
    }

    #[test]
    fn join_union_nested_flatten() {
        let u1 = normalize_union(vec![LogicalType::Int, LogicalType::Bool], UNION_CAP);
        let u2 = normalize_union(vec![LogicalType::Str, LogicalType::None], UNION_CAP);
        let j = join(u1, u2);
        match j {
            LogicalType::Union(v) => assert_eq!(v.len(), 4),
            other => panic!("expected 4-arm Union at cap, got {other:?}"),
        }
        let u5 = normalize_union(
            vec![
                lit_int(0),
                lit_int(1),
                lit_int(2),
                lit_int(3),
                lit_int(4),
            ],
            UNION_CAP,
        );
        assert_eq!(u5, LogicalType::Dyn);
    }

    #[test]
    fn join_is_commutative_and_stable() {
        let a = join(LogicalType::Int, LogicalType::Bool);
        let b = join(LogicalType::Bool, LogicalType::Int);
        assert_eq!(a, b);
        let j = join(join(a, LogicalType::Str), LogicalType::None);
        match &j {
            LogicalType::Union(v) => assert_eq!(v.len(), 3),
            other => panic!("expected 3-arm Union, got {other:?}"),
        }
        let five = join(
            j,
            normalize_union(
                vec![LogicalType::Float, LogicalType::Bool],
                UNION_CAP,
            ),
        );
        assert_eq!(five, LogicalType::Dyn);
    }

    #[test]
    fn project_s0_types() {
        assert_eq!(project_to_physical(LogicalType::Int), PhysicalType::Int64);
        assert_eq!(project_to_physical(lit_int(1)), PhysicalType::Int64);
    }

    #[test]
    fn project_small_union_is_dyn() {
        let u = normalize_union(vec![LogicalType::Int, LogicalType::None], UNION_CAP);
        assert_eq!(project_to_physical(u), PhysicalType::Dyn);
    }
}
