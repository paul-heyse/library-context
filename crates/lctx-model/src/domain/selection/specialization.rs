//! Located native typing query. Site evidence specializes an observed port, never a public alias.
use super::{evaluate::Prepared, build::{invalid, need}};
use crate::domain::{normalized::generic_specialization as generic, resources::ResourceBudget, types::*, *};
pub struct LocatedTypedPort {
    pub observation: Id<SignatureTypeObservation>,
    pub specialized: generic::Specialized,
}
impl Prepared {
    pub fn specialize_port(
        &self,
        site: Id<source::Occurrence>,
        declaration: Id<NativeSignatureObservation>,
        subject: Id<SignatureTypeSubject>,
        context: Id<attribution::AnalysisContext>,
        budget: &ResourceBudget,
    ) -> Result<LocatedTypedPort, ModelError> {
        let data = self.data();
        let native = need(&data.source.core.native_signatures, declaration)?;
        let declaration_q = need(&data.source.core.qualifications, native.qualification)?;
        if declaration_q.context != context {return Err(invalid("specialized port crosses analysis context"));}
        if !data.facts.call_syntax.iter().any(|call|call.callee==site && data.source.core.qualifications.get(call.qualification).is_some_and(|q|q.context==context)) {return Err(invalid("specialized port site is not a callee in this context"));}
        let source = need(&data.facts.signature_subjects, subject)?;
        let signature = match source {
            SignatureTypeSubject::Parameter {parameter} => need(&data.facts.signature_parameters, *parameter)?.signature,
            SignatureTypeSubject::Return {signature} => *signature,
        };
        if signature != native.signature {return Err(invalid("specialized port has foreign native signature subject"));}
        let mut ports = data.source.core.signature_types.iter().filter(|p|p.subject==subject && p.qualification==native.qualification);
        let port=ports.next().ok_or_else(||invalid("specialized port must have one native typing observation"))?;
        if ports.next().is_some() {return Err(invalid("specialized port must have one native typing observation"));}
        let binding_count=data.facts.generic_specializations.iter().filter(|r|r.site==site && r.declaration==declaration).count();
        let _bindings=budget.reserve("located-native-bindings",binding_count.saturating_mul(size_of::<GenericSpecializationObservation>()))?;
        let bindings: Vec<_> = data.facts.generic_specializations.iter().filter(|r|r.site==site && r.declaration==declaration).cloned().collect();
        let qualification = bindings.first().map_or(native.qualification, |r|r.qualification);
        let q = need(&data.source.core.qualifications, qualification)?;
        if q.context != context {return Err(invalid("specialized site crosses analysis context"));}
        let mut graph = generic::Graph::new(budget);
        macro_rules! copy {($field:ident,$source:ident)=>{for row in data.facts.$source.iter() {let _copy=budget.reserve("located-type-copy",size_of_val(row).saturating_add(row.heap_bytes()))?;graph.$field.insert(row.clone())?;}};}
        copy!(terms,type_terms); copy!(sequences,type_sequence_headers); copy!(members,type_sequences);
        copy!(lists,type_callable_lists); copy!(slots,type_callable_slots); copy!(dict_lists,type_dict_lists); copy!(fields,type_dict_fields); copy!(variables,type_variables);
        let environment = generic::Environment {qualification,declaration,site,bindings};
        let mut specialized = generic::substitute(&graph,port.term,&environment,budget)?;
        if q.approximation != assertion::Approximation::Exact || declaration_q.approximation != assertion::Approximation::Exact
            || q.modality != attribution::Modality::Definite || declaration_q.modality != attribution::Modality::Definite
            || q.condition != conditions::Diagram::always().id() || declaration_q.condition != conditions::Diagram::always().id() {
            specialized.boundaries.push(generic::Boundary::Qualification);
            specialized.status = generic::SpecializationStatus::Unknown;
        }
        Ok(LocatedTypedPort {observation:port.id(),specialized})
    }
}
