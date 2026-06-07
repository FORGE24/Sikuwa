//! Pass 5 materialization: LogicalType → slot level + `TaggedLayout`.

use crate::infer::logical::{normalize_type, LogicalType, LiteralValue, UNION_CAP};
use crate::types::{PhysicalType, SlotLevel, SlotStrategy, TaggedLayout};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotMaterialization {
    pub physical: PhysicalType,
    pub strategy: SlotStrategy,
    pub level: SlotLevel,
    pub tagged: Option<TaggedLayout>,
}

pub fn materialize_slot(lt: LogicalType) -> SlotMaterialization {
    let lt = normalize_type(lt);
    if let Some(layout) = tagged_layout_from_type(&lt) {
        return SlotMaterialization {
            physical: PhysicalType::Dyn,
            strategy: SlotStrategy::Dyn,
            level: SlotLevel::S1,
            tagged: Some(layout),
        };
    }

    let physical = project_single(&lt);
    let (strategy, level) = plan_physical(physical);
    SlotMaterialization {
        physical,
        strategy,
        level,
        tagged: None,
    }
}

fn tagged_layout_from_type(lt: &LogicalType) -> Option<TaggedLayout> {
    let arms = match lt {
        LogicalType::Union(v) => v.clone(),
        LogicalType::Optional(inner) => {
            vec![LogicalType::None, *inner.clone()]
        }
        _ => return None,
    };
    if arms.is_empty() || arms.len() > UNION_CAP {
        return None;
    }
    let mut names = Vec::with_capacity(arms.len());
    for arm in &arms {
        names.push(logical_arm_name(arm)?);
    }
    if names.iter().any(|n| n == "none") {
        names.retain(|n| n != "none");
        names.push("none".into());
    }
    Some(TaggedLayout { arms: names })
}

pub fn logical_arm_name(lt: &LogicalType) -> Option<String> {
    match normalize_type(lt.clone()) {
        LogicalType::None | LogicalType::Literal(LiteralValue::None) => Some("none".into()),
        LogicalType::Bool | LogicalType::Literal(LiteralValue::Bool(_)) => Some("bool".into()),
        LogicalType::Int | LogicalType::Literal(LiteralValue::Int(_)) => Some("int64".into()),
        LogicalType::Float | LogicalType::Literal(LiteralValue::Float(_)) => Some("float64".into()),
        LogicalType::Str => Some("str".into()),
        _ => None,
    }
}

fn project_single(lt: &LogicalType) -> PhysicalType {
    use crate::infer::logical::project_to_physical;
    project_to_physical(lt.clone())
}

fn plan_physical(ty: PhysicalType) -> (SlotStrategy, SlotLevel) {
    match ty {
        PhysicalType::Int64 | PhysicalType::Bool | PhysicalType::Float64 | PhysicalType::None => (
            SlotStrategy::Itr {
                primary: if ty == PhysicalType::Bool {
                    PhysicalType::Int64
                } else {
                    ty
                },
            },
            SlotLevel::S0,
        ),
        PhysicalType::Str => (SlotStrategy::Alloc { ty }, SlotLevel::S0),
        PhysicalType::Unknown => (
            SlotStrategy::Itr {
                primary: PhysicalType::Int64,
            },
            SlotLevel::S0,
        ),
        _ => (SlotStrategy::Dyn, SlotLevel::S3),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infer::logical::normalize_union;

    #[test]
    fn optional_materializes_s1() {
        let lt = LogicalType::Optional(Box::new(LogicalType::Int));
        let m = materialize_slot(lt);
        assert_eq!(m.level, SlotLevel::S1);
        assert_eq!(
            m.tagged.as_ref().map(|t| t.arms.clone()),
            Some(vec!["int64".into(), "none".into()])
        );
    }

    #[test]
    fn int_materializes_s0() {
        let m = materialize_slot(LogicalType::Int);
        assert_eq!(m.level, SlotLevel::S0);
        assert!(m.tagged.is_none());
    }
}
