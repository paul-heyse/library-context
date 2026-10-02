//! Charged local lookup handles and integrity; canonical C2 replay stays in build.
use super::{*, build::{invalid,need,Output}, classification::ClassificationData};
use crate::domain::{*, charged::{ChargedMap,ChargedSet,StateCharge}, resources::ResourceBudget};
type Frame = (Id<catalog::CatalogMember>,Id<attribution::AnalysisContext>,DomainKind);
pub(super) struct Index {
    domains: ChargedMap<Frame,Id<SelectionDomain>>,
    members: ChargedMap<Id<SelectionDomain>,Vec<Id<DomainContext>>>,
    evidence: ChargedMap<(Id<SelectionDomain>,Id<Context>),Vec<Id<DomainEvidence>>>,
    closure: ChargedMap<Id<SelectionDomain>,Vec<Id<DomainClosure>>>,
    _charge: StateCharge,
}
impl Index {
    pub fn new(d:&ClassificationData,o:&Output,b:&ResourceBudget)->Result<Self,ModelError> {
        d.validate_references()?;
        let mut charge=StateCharge::new(b,"selection-prepared-indexes");
        charge.grow(size_of::<super::evaluate::Prepared>())?;
        let mut domains=ChargedMap::default();let mut members=ChargedMap::default();let mut evidence=ChargedMap::default();let mut closure=ChargedMap::default();
        for r in o.domains.iter() {
            need(&d.source.catalog.members,r.member)?;
            if domains.insert(&mut charge,(r.member,r.analysis,r.kind),r.id())?.is_some() {return Err(invalid("duplicate finite selection domain"));}
        }
        for r in o.members.iter() {
            let domain=need(&o.domains,r.domain)?;let c=need(&o.contexts,r.context)?;
            if c.analysis()!=domain.analysis || c.member().is_some_and(|m|m!=domain.member) {return Err(invalid("selection context crosses domain member or analysis"));}
            d.validate_context(c)?;
            members.update(&mut charge,r.domain,|v:&mut Vec<_>|v.push(r.id()))?;
        }
        for r in o.evidence.iter() {
            let c=need(&o.contexts,r.context)?;let w=need(&o.witnesses,r.witness)?;
            need(&o.domains,r.domain)?;
            if o.members.get(DomainContext{domain:r.domain,context:r.context,complete:false}.id()).is_none() {return Err(invalid("selection evidence has no domain membership"));}
            d.validate_witness(c,w)?;
            evidence.update(&mut charge,(r.domain,r.context),|v:&mut Vec<_>|v.push(r.id()))?;
        }
        for r in o.closure.iter() {
            need(&o.domains,r.domain)?;let w=need(&o.witnesses,r.evidence)?;
            if let Witness::Domain {domain}=w {need(&o.domains,*domain)?;}
            closure.update(&mut charge,r.domain,|v:&mut Vec<_>|v.push(r.id()))?;
        }
        for r in o.domains.iter() {
            let mut pairs=ChargedSet::default();let mut witnesses=ChargedSet::default();let mut check=StateCharge::new(b,"selection-local-digests");
            if let Some(links)=members.get(&r.id()) {for id in links {let member=need(&o.members,*id)?;
                let rows=evidence.get(&(r.id(),member.context)).ok_or_else(||invalid("selection membership has no evidence"))?;
                for id in rows {let row=need(&o.evidence,*id)?;pairs.insert(&mut check,(row.context,row.witness))?;}
            }}
            if let Some(rows)=closure.get(&r.id()) {for id in rows {witnesses.insert(&mut check,need(&o.closure,*id)?.evidence)?;}}
            let mut contexts=KeySink::new("selection-domain-contexts/v1");for (c,w) in pairs.iter() {c.encode(&mut contexts);w.encode(&mut contexts);}
            let mut closed=KeySink::new("selection-domain-closure/v1");for w in witnesses.iter() {w.encode(&mut closed);}
            if contexts.finish()!=r.contexts || closed.finish()!=r.closure {return Err(invalid("selection local membership or closure digest differs"));}
        }
        Ok(Self{domains,members,evidence,closure,_charge:charge})
    }
    pub fn domain(&self,m:Id<catalog::CatalogMember>,a:Id<attribution::AnalysisContext>,k:DomainKind)->Result<Id<SelectionDomain>,ModelError> {self.domains.get(&(m,a,k)).copied().ok_or_else(||invalid("selection has no admitted member/context domain"))}
    pub fn members(&self,d:Id<SelectionDomain>)->&[Id<DomainContext>] {self.members.get(&d).map_or(&[],Vec::as_slice)}
    pub fn evidence(&self,d:Id<SelectionDomain>,c:Id<Context>)->&[Id<DomainEvidence>] {self.evidence.get(&(d,c)).map_or(&[],Vec::as_slice)}
    pub fn closure(&self,d:Id<SelectionDomain>)->&[Id<DomainClosure>] {self.closure.get(&d).map_or(&[],Vec::as_slice)}
}
