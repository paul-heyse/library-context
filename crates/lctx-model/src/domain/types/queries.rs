//! Availability of one selected native type query, independent of unrelated omitted queries.
use super::{TypeObservation, TypeRole};
use crate::{Assertion, Domain, DomainCode};
use crate::domain::{*, assertion::AssertionQualification, attribution::FactFamily, source::Occurrence, obligation::ObligationKind};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum TypeQueryStatus { Available = 0, Unavailable = 1, Partial = 2 }
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name="native_type_query_observations", validate=validate, invariants=invariants)]
#[assertion(support=TypeQuerySupport, name="native_type_query_supports", family=FactFamily::Types, subjects(subject))]
pub struct TypeQueryObservation {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub subject: Id<Occurrence>,
    #[model(key)] pub role: TypeRole,
    #[model(key)] pub declared: bool,
    #[model(key)] pub observation: Option<Id<TypeObservation>>,
    #[model(key)] pub status: TypeQueryStatus,
    #[model(key)] pub reason: Option<ObligationKind>,
}
fn invalid(message:&str)->ModelError {ModelError::Invalid(message.into())}
fn validate(row:&TypeQueryObservation)->Result<(),ModelError> {
    match row.status {
        TypeQueryStatus::Available if row.observation.is_some() && row.reason.is_none()=>Ok(()),
        TypeQueryStatus::Unavailable if row.observation.is_none() && row.reason.is_some()=>Ok(()),
        TypeQueryStatus::Partial if row.observation.is_some() && row.reason.is_some()=>Ok(()),
        _=>Err(invalid("type query availability contradicts its result")),
    }
}
fn invariants()->Vec<Invariant> {vec![Invariant {name:"located_type_query_availability", inputs:vec![ValidationInput::of::<TypeQueryObservation>(&["id"]),ValidationInput::of::<TypeObservation>(&["id"])],create:std::sync::Arc::new(|budget|Box::new(Check {queries:crate::domain::normalized::Rows::new(budget),observations:crate::domain::normalized::Rows::new(budget)}))}]}
struct Check {queries:crate::domain::normalized::Rows<TypeQueryObservation>,observations:crate::domain::normalized::Rows<TypeObservation>}
impl InvariantCheck for Check {
    fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError> {
        if name==TypeQueryObservation::NAME {self.queries.decode(batch)?;Ok(())}
        else if name==TypeObservation::NAME {self.observations.decode(batch)?;Ok(())}
        else {Err(invalid("undeclared type query input"))}
    }
    fn finish(self:Box<Self>)->Result<(),ModelError> {
        let mut seen=std::collections::BTreeSet::new();
        for query in self.queries.iter() {
            // A native query may expose several overload candidates. Availability concerns this
            // selected answer, not completeness of the set; distinct answers remain distinct.
            if !seen.insert((query.qualification,query.subject,query.role as i16,query.declared,query.observation)) {return Err(invalid("native type query has conflicting availability for one result"));}
            query.validate()?;
            if let Some(id)=query.observation {
                let observation=self.observations.get(id).ok_or_else(||invalid("native type query result missing"))?;
                if (observation.qualification,observation.subject,observation.role,observation.declared)!=(query.qualification,query.subject,query.role,query.declared) {return Err(invalid("type query changes located role/frame"));}
            }
        }
        Ok(())
    }
}
