//! Structural access-path composition. A derived value does not inherit its source's subfields.
use std::collections::BTreeMap;
use super::{*, value::*, transfer::TransferKind, attribution::ObligationKind};

#[derive(Debug,Clone,PartialEq,Eq)]
pub enum PathRelation { Rest(AccessPath), Shorter(AccessPath), Disjoint, Unknown }
/// Nominal lookup inputs must retain their recorded identities. This also refuses a forged map
/// instead of trusting a caller-supplied key for a different attribute/literal.
pub struct PathCatalog<'a> {
    pub segments: &'a BTreeMap<Id<PathSegment>,PathSegment>,
    pub literals: &'a BTreeMap<Id<Literal>,Literal>,
}
impl PathCatalog<'_> {
    fn segment(&self,id: Id<PathSegment>) -> Result<&PathSegment,ModelError> {
        let row = self.segments.get(&id).ok_or_else(|| invalid("path segment absent"))?;
        row.validate()?;
        if row.id() != id { return Err(invalid("path segment lookup identity mismatch")); } Ok(row)
    }
    fn literal(&self,id: Id<Literal>) -> Result<&Literal,ModelError> {
        let row = self.literals.get(&id).ok_or_else(|| invalid("path item literal absent"))?;
        row.validate()?;
        if row.id() != id { return Err(invalid("path literal lookup identity mismatch")); } Ok(row)
    }
    fn compare(&self,left: Id<PathSegment>,right: Id<PathSegment>) -> Result<SegmentRelation,ModelError> {
        use SegmentRelation::*;
        Ok(match (self.segment(left)?,self.segment(right)?) {
            (PathSegment::AnyItem,_) | (_,PathSegment::AnyItem) => Unknown,
            (PathSegment::Attribute { name: a },PathSegment::Attribute { name: b }) => if a == b { Same } else { Disjoint },
            (PathSegment::Item { key: a },PathSegment::Item { key: b }) => literal_relation(self.literal(*a)?,self.literal(*b)?),
            (PathSegment::Attribute { .. },PathSegment::Item { .. }) | (PathSegment::Item { .. },PathSegment::Attribute { .. }) => Disjoint,
        })
    }
}
fn invalid(message: &str) -> ModelError { ModelError::Invalid(message.into()) }
#[derive(Clone,Copy)]
enum SegmentRelation { Same,Disjoint,Unknown }
fn literal_relation(left: &Literal,right: &Literal) -> SegmentRelation {
    use SegmentRelation::*;
    // Equal NaN bit patterns do not prove equal keys; separate evaluations may create distinct
    // NaN objects. Numeric cross-kind equality must not become a false disjointness claim.
    if matches!(left,Literal::Float { bits } if f64::from_bits(*bits as u64).is_nan())
        || matches!(right,Literal::Float { bits } if f64::from_bits(*bits as u64).is_nan()) { return Unknown; }
    if left == right { return Same; }
    let numeric = |literal: &Literal| matches!(literal,Literal::Bool { .. }|Literal::Integer { .. }|Literal::Float { .. });
    if numeric(left) && numeric(right) {
        return match (left,right) {
            (Literal::Bool { value },Literal::Integer { decimal }) | (Literal::Integer { decimal },Literal::Bool { value }) =>
                if decimal == if *value { "1" } else { "0" } { Same } else { Disjoint },
            (Literal::Float { bits: a },Literal::Float { bits: b }) =>
                if f64::from_bits(*a as u64) == f64::from_bits(*b as u64) { Same } else { Disjoint },
            (Literal::Integer { .. },Literal::Integer { .. }) | (Literal::Bool { .. },Literal::Bool { .. }) => Disjoint,
            _ => Unknown,
        };
    }
    Disjoint
}
fn segments(path: &AccessPath) -> Vec<Id<PathSegment>> { [path.first,path.second].into_iter().flatten().collect() }
fn tail(values: &[Id<PathSegment>],unknown_suffix: bool) -> AccessPath {
    AccessPath { first: values.first().copied(),second: values.get(1).copied(),unknown_suffix }
}
/// Compare structural places in the declared bounded-path model; this is not a proof about
/// arbitrary overloaded Python attribute/item access or object aliasing.
pub fn relation(path: &AccessPath,prefix: &AccessPath,catalog: &PathCatalog<'_>) -> Result<PathRelation,ModelError> {
    path.validate()?; prefix.validate()?;
    let own = segments(path); let other = segments(prefix);
    // Check the entire supplied paths, including tails not reached by an early difference.
    for id in own.iter().chain(&other) {
        if let PathSegment::Item { key } = catalog.segment(*id)? { catalog.literal(*key)?; }
    }
    for (index,segment) in other.iter().enumerate() {
        match own.get(index) {
            Some(own) => match catalog.compare(*own,*segment)? {
                SegmentRelation::Same => {}, SegmentRelation::Disjoint => return Ok(PathRelation::Disjoint),
                SegmentRelation::Unknown => return Ok(PathRelation::Unknown),
            },
            None if path.unknown_suffix => return Ok(PathRelation::Unknown),
            None => return Ok(PathRelation::Shorter(tail(&other[index..],prefix.unknown_suffix))),
        }
    }
    if prefix.unknown_suffix { return Ok(PathRelation::Unknown); }
    Ok(PathRelation::Rest(tail(&own[other.len()..],path.unknown_suffix)))
}

#[derive(Debug,Clone,PartialEq,Eq)]
pub enum ComposedPaths {
    Flow { input: AccessPath,output: AccessPath,kind: TransferKind }, Disjoint,
    Obligation(ObligationKind),
}
/// Caller delivers its input at an actual's path; callee reads a formal's path and delivers its
/// output. Extend the caller's input only through caller identity, and the callee's output only
/// through callee identity. This kernel does not bind/map roots or certify a whole cross-call proof.
pub fn compose(
    caller_input: &AccessPath, actual: &AccessPath, formal_read: &AccessPath, callee_output: &AccessPath,
    caller_kind: TransferKind, callee_kind: TransferKind, catalog: &PathCatalog<'_>,
) -> Result<ComposedPaths,ModelError> {
    caller_input.validate()?; callee_output.validate()?;
    for path in [caller_input,callee_output] { for id in segments(path) {
        if let PathSegment::Item { key } = catalog.segment(id)? { catalog.literal(*key)?; }
    } }
    let (input,output) = match relation(formal_read,actual,catalog)? {
        PathRelation::Rest(rest) => (if caller_kind == TransferKind::Identity { caller_input.append(&rest) } else { caller_input.clone() },callee_output.clone()),
        PathRelation::Shorter(rest) => (caller_input.clone(),if callee_kind == TransferKind::Identity { callee_output.append(&rest) } else { callee_output.clone() }),
        PathRelation::Disjoint => return Ok(ComposedPaths::Disjoint),
        PathRelation::Unknown => return Ok(ComposedPaths::Obligation(ObligationKind::AmbiguousBinding)),
    };
    Ok(ComposedPaths::Flow { input,output,kind: super::transfer::compose_kinds(caller_kind,callee_kind) })
}
