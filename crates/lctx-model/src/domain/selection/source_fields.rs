//! Source initialization associations do not describe a receiver's runtime value.
use super::{classification::ClassificationData, build::need, FieldTarget};
use crate::domain::{*, catalog::{CatalogOption, CatalogOptionSubject}, normalized::callables::Knowledge};

/// Unknown is preserved on absent or nonunique correspondence; absence of one C1 row is not closure.
pub fn declared_parameter(
    d: &ClassificationData,
    option: Id<CatalogOption>,
    target: &FieldTarget,
    context: Id<attribution::AnalysisContext>,
) -> Result<Option<bool>, ModelError> {
    let (slot, formal)=match target {
        FieldTarget::Parameter {slot} => {
            let Some(slot)=d.source.core.slots.get(*slot) else {return Ok(None)};
            let variant=need(&d.source.core.variants,slot.variant)?;
            if variant.context!=context {return Ok(None)}
            (Some(slot),None)
        }
        FieldTarget::Declaration {entity} => match d.source.core.refs.get(*entity) {
            Some(crate::domain::normalized::entities::EntityRef::Parameter {parameter}) => (None,Some(*parameter)),
            _ => return Ok(None),
        },
    };
    let mut found = false;
    for link in d.evidence.source_field_links.iter().filter(|l| l.field_option == option) {
        let association = need(&d.source.facts.symbolic_associations, link.association)?;
        let q = need(&d.source.core.qualifications, association.qualification)?;
        if q.context != context || link.source_association != Knowledge::Known
            || link.runtime_value != Knowledge::Unknown { continue }
        let parameter_option = need(&d.source.catalog.options, link.parameter_option)?;
        let selected_option = need(&d.source.catalog.options, option)?;
        if parameter_option.member != selected_option.member { return Err(ModelError::Invalid("foreign source-field parameter option".into())) }
        let exact = match need(&d.source.catalog.subjects, parameter_option.subject)? {
            CatalogOptionSubject::Parameter {slot: other} => {
                let other=need(&d.source.core.slots,*other)?;
                slot.is_some_and(|s|s.id()==other.id()) || formal.is_some_and(|f|d.source.core.parameter_links.iter().any(|p|p.entity==f && p.parameter==other.parameter))
            }
            CatalogOptionSubject::SourceParameter {parameter} => formal==Some(*parameter) || slot.is_some_and(|s|d.source.core.parameter_links.iter().any(|p|p.entity==*parameter && p.parameter==s.parameter)),
            CatalogOptionSubject::Field {..} => false,
        };
        let same_parameter=slot.is_some_and(|s|association.parameter==s.parameter) || formal.is_some_and(|f|d.source.core.parameter_links.iter().any(|p|p.entity==f && p.parameter==association.parameter));
        if exact && same_parameter {found=true}
    }
    Ok(found.then_some(true))
}
