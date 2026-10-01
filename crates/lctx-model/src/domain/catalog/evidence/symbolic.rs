//! C1 consumes normalized association identity directly, independently of behavioral Summary.
use super::{build::{EvidenceData,EvidenceOutput,need,invalid},*};
use crate::Domain;
use crate::domain::normalized::symbolic_fields::*;

#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="catalog_source_field_links",rule="exact_source_field_reader")]
pub struct SourceFieldLink {
    #[model(key)] pub field_option: Id<CatalogOption>,
    #[model(key)] pub parameter_option: Id<CatalogOption>,
    #[model(key,premise)] pub association: Id<SourceFieldAssociation>,
    #[model(key,premise)] pub reader: Id<SourceFieldReaderLink>,
    pub reader_owner: Id<EntityRef>,
    pub source_association: Knowledge,
    pub runtime_value: Knowledge,
}
pub(super) fn derive(d:&EvidenceData,out:&mut EvidenceOutput)->Result<(),ModelError> {
    for link in d.facts.symbolic_links.iter() {
        let association=need(&d.facts.symbolic_associations,link.association)?;
        let reader=need(&d.facts.symbolic_readers,link.reader)?;
        let class=need(&d.facts.symbolic_classes,association.class)?;
        if !class.supported_record{continue}
        let q=need(&d.core.qualifications,association.qualification)?;
        let r=need(&d.core.occurrences,reader.access)?;
        let mut owners=d.core.ownership.iter().filter(|o|d.core.occurrences.get(o.occurrence)
            .is_some_and(|a|(a.source,a.start,a.end,a.syntax_kind)==(r.source,r.start,r.end,r.syntax_kind)));
        let Some(owner)=owners.next()else{return Err(invalid("C1 source reader lacks normalized owner"))};
        if owners.any(|o|o.entity!=owner.entity){return Err(invalid("C1 source reader owner is ambiguous"))}
        for field_option in d.catalog.options.iter() {
            let CatalogOptionSubject::Field{field}=need(&d.catalog.subjects,field_option.subject)? else{continue};
            if !d.core.field_links.iter().any(|l|l.field==*field&&l.observation==association.field){continue}
            let member=need(&d.catalog.members,field_option.member)?;
            if !d.facts.core_links.iter().any(|l|l.member==member.id()&&d.facts.core_invocations.get(l.invocation)
                .is_some_and(|i|i.context==q.context&&i.input==member.input)){continue}
            for parameter_option in d.catalog.options.iter().filter(|o|o.member==field_option.member) {
                let exact=match need(&d.catalog.subjects,parameter_option.subject)? {
                    CatalogOptionSubject::Parameter{slot}=>need(&d.core.slots,*slot)?.parameter==association.parameter,
                    CatalogOptionSubject::SourceParameter{parameter}=>d.core.parameter_links.iter().any(|l|l.entity==*parameter&&l.parameter==association.parameter),
                    _=>false,
                };
                if exact {out.source_field_links.insert(SourceFieldLink{field_option:field_option.id(),parameter_option:parameter_option.id(),association:association.id(),reader:link.id(),reader_owner:owner.entity,source_association:Knowledge::Known,runtime_value:Knowledge::Unknown})?;}
            }
        }
    }
    Ok(())
}
