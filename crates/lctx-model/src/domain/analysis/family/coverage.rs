use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name=owner_table!("analysis_coverage"), validate = validate_coverage, invariants = coverage_invariants)]
pub struct AnalysisCoverage {
    #[model(key)]
    pub invocation: Id<AnalysisInvocation>,
    #[model(key)]
    pub capability: AnalysisCapability,
    #[model(key)]
    pub scope: Id<CoverageScope>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    /// Exact lower membership is identity-bearing; dropping a premise changes the receipt.
    #[model(key)]
    pub premises: ContentHash,
    pub availability: EvidenceAvailability,
    pub reason: Option<ObligationKind>,
}
impl crate::domain::analysis::coverage::sealed::CoverageEvidence for AnalysisCoverage {}
impl crate::domain::analysis::coverage::CoverageEvidence for AnalysisCoverage {
    fn coverage_frame(&self)->(Id<CoverageScope>,Id<AnalysisContext>,EvidenceAvailability) {(self.scope,self.context,self.availability)}
}
fn validate_coverage(row: &AnalysisCoverage) -> Result<(), ModelError> {
    match row.availability {
        EvidenceAvailability::Complete | EvidenceAvailability::NoScope if row.reason.is_some() => {
            Err(invalid("complete/empty analysis coverage has a boundary"))
        }
        EvidenceAvailability::NotRequested if row.reason != Some(ObligationKind::NotRequested) => {
            Err(invalid(
                "unrequested analysis coverage lacks its explicit reason",
            ))
        }
        EvidenceAvailability::Partial | EvidenceAvailability::Unavailable
            if row.reason.is_none() =>
        {
            Err(invalid("incomplete analysis coverage lacks its boundary"))
        }
        _ => Ok(()),
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name=owner_table!("analysis_coverage_premises"), rule = "analysis_coverage", conclusion = coverage)]
pub struct AnalysisCoveragePremise {
    #[model(key)]
    pub coverage: Id<AnalysisCoverage>,
    #[model(key)]
    #[model(premise)]
    pub source: Id<CoverageSource>,
}
/// A semantic dependency observation, retaining its nominal lower outcome. Construction is from
/// canonical typed rows; store authorization must additionally bind their completed source receipt.
#[derive(Debug, Clone)]
pub struct CoverageObservation {
    source: CoverageSource,
    scope: Id<CoverageScope>,
    context: Id<AnalysisContext>,
    availability: EvidenceAvailability,
}
impl CoverageObservation {
    pub fn native(row: &ProviderCoverage) -> Result<Self, ModelError> {
        row.validate()?;
        let availability = match row.status {
            CoverageStatus::CompleteUnderStatedModel => EvidenceAvailability::Complete,
            CoverageStatus::Partial => EvidenceAvailability::Partial,
            CoverageStatus::Unavailable => EvidenceAvailability::Unavailable,
            CoverageStatus::NotRequested => EvidenceAvailability::NotRequested,
            CoverageStatus::Failed => {
                return Err(invalid(
                    "failed native coverage cannot be an analysis premise",
                ));
            }
        };
        Ok(Self {
            source: CoverageSource::Native { coverage: row.id() },
            scope: row.scope,
            context: row.context,
            availability,
        })
    }
    pub fn normalized(row: &NormalizationCoverage) -> Result<Self,ModelError> {
        Ok(Self {
            source: normalized_source(row)?,
            scope: row.scope,
            context: row.context,
            availability: row.availability,
        })
    }
    pub fn analysis(row: &AnalysisCoverage) -> Result<Self, ModelError> {
        row.validate()?;
        Ok(Self {
            source: CoverageSource::Analysis { coverage: row.id() },
            scope: row.scope,
            context: row.context,
            availability: row.availability,
        })
    }
    pub fn source(&self) -> &CoverageSource {
        &self.source
    }
    /// The finite source sum and sealed row adapter preserve the actual predecessor identity.
    pub fn predecessor<R:crate::domain::analysis::coverage::CoverageEvidence>(source:&CoverageSource,row:&R)->Result<Self,ModelError> {
        row.validate()?;
        if source.reference()!=derivation::RowRef::of(row.id()) {return Err(invalid("coverage observation changes nominal predecessor"));}
        let (scope,context,availability)=row.coverage_frame();Ok(Self {source:source.clone(),scope,context,availability})
    }
}
/// The domain comes from admitted input/capability contracts. NoScope is a declared empty domain,
/// distinct from an empty list of evidence. R0 receipt admission supplies the authority to use it.
#[derive(Debug, Clone)]
pub struct CoverageExpectation {
    pub invocation: Id<AnalysisInvocation>,
    pub capability: AnalysisCapability,
    pub scope: Id<CoverageScope>,
    pub context: Id<AnalysisContext>,
    pub requested: bool,
    pub no_scope: bool,
    pub sources: Vec<Id<CoverageSource>>,
}
/// Shared conservative coverage fold used by qualification and scoped result construction.
/// This operation does not certify membership; assess verifies the exact expected members first.
pub use crate::domain::analysis::coverage::combine_availability;
fn membership_digest(ids: &std::collections::BTreeSet<Id<CoverageSource>>) -> ContentHash {
    let mut sink = KeySink::new("analysis-coverage-membership");
    for id in ids {
        id.encode(&mut sink);
    }
    sink.finish()
}
/// Computes a total outcome without trusting a producer's desired status. Missing/extra/duplicate
/// or foreign-context inputs refuse. Empty observations never imply complete requested coverage.
pub fn assess(
    expectation: &CoverageExpectation,
    observed: &[CoverageObservation],
    status: AnalysisStatus,
    reason: Option<ObligationKind>,
    budget: &resources::ResourceBudget,
) -> Result<(AnalysisCoverage, Vec<AnalysisCoveragePremise>), ModelError> {
    let mut charge = charged::StateCharge::new(budget, "analysis_coverage");
    let mut expected = charged::ChargedSet::default();
    for id in &expectation.sources {
        if !expected.insert(&mut charge, *id)? {
            return Err(invalid("duplicate expected analysis coverage source"));
        }
    }
    let mut seen = charged::ChargedSet::default();
    for row in observed {
        if !expected.contains(&row.source.id()) || !seen.insert(&mut charge, row.source.id())? {
            return Err(invalid("extra or duplicate analysis coverage premise"));
        }
        if (row.scope, row.context) != (expectation.scope, expectation.context) {
            return Err(invalid("analysis coverage premise crosses scope/context"));
        }
    }
    if *expected != *seen {
        return Err(invalid("missing analysis coverage premise"));
    }
    let availability = if !expectation.requested {
        if status != AnalysisStatus::NotRequested || reason != Some(ObligationKind::NotRequested) {
            return Err(invalid("unrequested analysis was attempted"));
        }
        EvidenceAvailability::NotRequested
    } else if expectation.no_scope {
        if !observed.is_empty() || status != AnalysisStatus::Completed || reason.is_some() {
            return Err(invalid(
                "NoScope requires a declared empty domain and completed computation",
            ));
        }
        EvidenceAvailability::NoScope
    } else {
        if observed.is_empty() {
            return Err(invalid("requested analysis has no admitted lower coverage"));
        }
        let lower = combine_availability(observed.iter().map(|row| row.availability));
        let execution = match status {
            AnalysisStatus::Completed if reason.is_none() => EvidenceAvailability::Complete,
            AnalysisStatus::Partial if reason.is_some() => EvidenceAvailability::Partial,
            AnalysisStatus::Unavailable if reason.is_some() => EvidenceAvailability::Unavailable,
            _ => {
                return Err(invalid(
                    "analysis computation status disagrees with request/boundary",
                ));
            }
        };
        combine_availability([lower, execution])
    };
    let reason = match availability {
        EvidenceAvailability::Complete | EvidenceAvailability::NoScope => None,
        EvidenceAvailability::NotRequested => Some(ObligationKind::NotRequested),
        EvidenceAvailability::Partial | EvidenceAvailability::Unavailable => {
            reason.or(Some(ObligationKind::IncompleteCoverage))
        }
    };
    let row = AnalysisCoverage {
        invocation: expectation.invocation,
        capability: expectation.capability,
        scope: expectation.scope,
        context: expectation.context,
        premises: membership_digest(&expected),
        availability,
        reason,
    };
    row.validate()?;
    let members = seen
        .iter()
        .map(|source| AnalysisCoveragePremise {
            coverage: row.id(),
            source: *source,
        })
        .collect();
    Ok((row, members))
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<AnalysisCoverage>(),
        Relation::of::<CoverageRequirement>(),
        Relation::of::<CoverageRequiredSource>(),
        Relation::of::<CoverageSource>(),
        Relation::of::<AnalysisCoveragePremise>(),
    ]
}

/// Canonical expected domain, populated from admitted source contracts. Stored validation
/// compares outcomes against this record, never reconstructs the domain from output rows.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name=owner_table!("coverage_requirements"))]
pub struct CoverageRequirement {
    #[model(key)] pub invocation: Id<AnalysisInvocation>,
    #[model(key)] pub capability: AnalysisCapability,
    #[model(key)] pub scope: Id<CoverageScope>,
    #[model(key)] pub context: Id<AnalysisContext>,
    #[model(key)] pub sources: ContentHash,
    #[model(key)] pub requested: bool,
    #[model(key)] pub no_scope: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name=owner_table!("coverage_required_sources"))]
pub struct CoverageRequiredSource {
    #[model(key)] pub requirement: Id<CoverageRequirement>,
    #[model(key)] pub source: Id<CoverageSource>,
}
impl CoverageExpectation {
    pub fn records(&self) -> Result<(CoverageRequirement,Vec<CoverageRequiredSource>),ModelError> {
        let sources=self.sources.iter().copied().collect::<std::collections::BTreeSet<_>>();
        if sources.len()!=self.sources.len() { return Err(invalid("duplicate expected analysis coverage source")); }
        if self.no_scope && (!sources.is_empty() || !self.requested) { return Err(invalid("NoScope is a requested empty domain")); }
        let row=CoverageRequirement { invocation:self.invocation,capability:self.capability,scope:self.scope,context:self.context,sources:membership_digest(&sources),requested:self.requested,no_scope:self.no_scope };
        let members=sources.into_iter().map(|source|CoverageRequiredSource { requirement:row.id(),source }).collect();
        Ok((row,members))
    }
}
fn coverage_invariants()->Vec<Invariant> {
    vec![Invariant { name:owner_table!("analysis_coverage_membership"),inputs: coverage_inputs(),create:std::sync::Arc::new(|budget|Box::new(CoverageCheck { charge:charged::StateCharge::new(budget,"analysis_coverage_membership"),invocations:Default::default(),outcomes:Default::default(),requirements:Default::default(),required:Default::default(),coverage:Default::default(),members:Default::default(),sources:Default::default(),native:Default::default(),normalized:Default::default(),predecessors:Default::default() })) }]
}
fn coverage_inputs()->Vec<ValidationInput> {let mut inputs=vec![
        ValidationInput::of::<AnalysisInvocation>(&["id"]),
        ValidationInput::of::<super::AnalysisOutcome>(&["id"]),
        ValidationInput::of::<CoverageRequirement>(&["id"]),
        ValidationInput::of::<CoverageRequiredSource>(&["id"]),
        ValidationInput::of::<AnalysisCoverage>(&["id"]),
        ValidationInput::of::<AnalysisCoveragePremise>(&["id"]),
        ValidationInput::of::<CoverageSource>(&["id"]),
        ValidationInput::of::<ProviderCoverage>(&["id"]),
    ]; normalized_coverage_inputs(&mut inputs); predecessor_coverage_inputs(&mut inputs); inputs
}
struct CoverageCheck {
    predecessors:charged::ChargedMap<derivation::RowRef,(Id<source::CoverageScope>,Id<attribution::AnalysisContext>,normalized::coverage::EvidenceAvailability)>,
    charge:charged::StateCharge,
    invocations:charged::ChargedMap<Id<AnalysisInvocation>,AnalysisInvocation>,
    outcomes:charged::ChargedMap<Id<AnalysisInvocation>,super::AnalysisOutcome>,
    requirements:charged::ChargedMap<Id<CoverageRequirement>,CoverageRequirement>,
    required:charged::ChargedMap<Id<CoverageRequirement>,std::collections::BTreeSet<Id<CoverageSource>>>,
    coverage:charged::ChargedMap<Id<AnalysisCoverage>,AnalysisCoverage>,
    members:charged::ChargedMap<Id<AnalysisCoverage>,std::collections::BTreeSet<Id<CoverageSource>>>,
    sources:charged::ChargedMap<Id<CoverageSource>,CoverageSource>,
    native:charged::ChargedMap<Id<ProviderCoverage>,ProviderCoverage>,
    normalized:charged::ChargedMap<Id<NormalizationCoverage>,NormalizationCoverage>,
}
impl InvariantCheck for CoverageCheck {
    fn visit(&mut self,relation:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError> {
        macro_rules! insert { ($r:ty,$field:ident,$key:expr) => { if relation==<$r>::NAME { for row in <$r>::decode(batch)? { let key=$key(&row); if self.$field.insert(&mut self.charge,key,row)?.is_some() { return Err(ModelError::Conflict(<$r>::NAME)); } } return Ok(()); } }; }
        insert!(AnalysisInvocation,invocations,|r:&AnalysisInvocation|r.id());
        insert!(super::AnalysisOutcome,outcomes,|r:&super::AnalysisOutcome|r.invocation);
        insert!(CoverageRequirement,requirements,|r:&CoverageRequirement|r.id());
        insert!(AnalysisCoverage,coverage,|r:&AnalysisCoverage|r.id());
        insert!(CoverageSource,sources,|r:&CoverageSource|r.id());
        insert!(ProviderCoverage,native,|r:&ProviderCoverage|r.id());
        if normalization_enabled() {insert!(NormalizationCoverage,normalized,|r:&NormalizationCoverage|r.id());}
        if relation==CoverageRequiredSource::NAME { for row in CoverageRequiredSource::decode(batch)? { if !self.required.update(&mut self.charge,row.requirement,|m|m.insert(row.source))? { return Err(invalid("duplicate required coverage source")); } } return Ok(()); }
        if relation==AnalysisCoveragePremise::NAME { for row in AnalysisCoveragePremise::decode(batch)? { if !self.members.update(&mut self.charge,row.coverage,|m|m.insert(row.source))? { return Err(invalid("duplicate stored coverage premise")); } } return Ok(()); }
        if visit_predecessor_coverage(relation,batch,&mut self.predecessors,&mut self.charge)? {return Ok(());}
        Err(invalid("undeclared analysis coverage input"))
    }
    fn finish(self:Box<Self>)->Result<(),ModelError> {
        let budget=self.charge.budget().ok_or_else(||invalid("coverage budget absent"))?;
        let empty=std::collections::BTreeSet::new();
        let mut matched=charged::ChargedSet::default();
        let mut charge=charged::StateCharge::new(budget,"analysis_coverage_recheck");
        for requirement in self.requirements.values() {
            let members=self.required.get(&requirement.id()).unwrap_or(&empty);
            if membership_digest(members)!=requirement.sources { return Err(invalid("required coverage membership digest differs")); }
            let invocation=self.invocations.get(&requirement.invocation).ok_or_else(||invalid("coverage invocation absent"))?;
            if invocation.context!=requirement.context { return Err(invalid("coverage requirement crosses invocation context")); }
            let outcome=self.outcomes.get(&requirement.invocation).ok_or_else(||invalid("coverage computation outcome absent"))?;
            let buffer_bytes=members.len().checked_mul(size_of::<CoverageObservation>()+size_of::<Id<CoverageSource>>()+size_of::<AnalysisCoveragePremise>()).ok_or_else(||invalid("coverage allocation overflow"))?;
            let _reservation=budget.reserve("analysis_coverage_recheck",buffer_bytes)?;
            let mut observations=Vec::with_capacity(members.len());
            for id in members {
                let source=self.sources.get(id).ok_or_else(||invalid("required coverage source absent"))?;
                observations.push(match source {
                    CoverageSource::Native { coverage }=>CoverageObservation::native(self.native.get(coverage).ok_or_else(||invalid("native coverage absent"))?)?,
                    CoverageSource::Analysis { coverage }=>CoverageObservation::analysis(self.coverage.get(coverage).ok_or_else(||invalid("lower analysis coverage absent"))?)?,
                    #[allow(unreachable_patterns,reason="Owners without final normalization coverage or predecessors have no other source variants")]
                    other=>{if let Some(id)=normalized_reference(other) {CoverageObservation::normalized(self.normalized.get(&id).ok_or_else(||invalid("normalization coverage absent"))?)?} else {let (scope,context,availability)=*self.predecessors.get(&other.reference()).ok_or_else(||invalid("predecessor coverage absent"))?;CoverageObservation {source:other.clone(),scope,context,availability}}},
                });
            }
            let expectation=CoverageExpectation { invocation:requirement.invocation,capability:requirement.capability,scope:requirement.scope,context:requirement.context,requested:requirement.requested,no_scope:requirement.no_scope,sources:members.iter().copied().collect() };
            expectation.records()?;
            let (expected,_)=assess(&expectation,&observations,outcome.status,outcome.reason,budget)?;
            let actual=self.coverage.get(&expected.id()).ok_or_else(||invalid("expected scoped analysis outcome absent"))?;
            if *actual!=expected || self.members.get(&actual.id()).unwrap_or(&empty)!=members { return Err(invalid("analysis coverage differs from exact lower evidence")); }
            if !matched.insert(&mut charge,actual.id())? { return Err(invalid("ambiguous coverage requirements")); }
        }
        if matched.len()!=self.coverage.len() { return Err(invalid("analysis coverage has no admitted expected domain")); }
        for id in self.members.keys() { if !self.coverage.contains_key(id) { return Err(invalid("orphan analysis coverage premise")); } }
        for id in self.required.keys() { if !self.requirements.contains_key(id) { return Err(invalid("orphan required analysis coverage source")); } }
        Ok(())
    }
}

/// This result retains its admission reservation while expected scopes and observations are read.
pub struct AdmittedCoverage {scopes:Vec<AdmittedScope>,_reservation:Box<dyn resources::Reservation>}
pub struct AdmittedScope {expectation:CoverageExpectation,observations:Vec<CoverageObservation>}
impl AdmittedScope {pub fn expectation(&self)->&CoverageExpectation {&self.expectation}pub fn observations(&self)->&[CoverageObservation] {&self.observations}}
impl AdmittedCoverage {pub fn scopes(&self)->&[AdmittedScope] {&self.scopes}}
fn admitted_scope(invocation:Id<AnalysisInvocation>,capability:AnalysisCapability,scope:&crate::domain::analysis::expected::ExpectedScope)->Result<AdmittedScope,ModelError> {let mut observations=Vec::with_capacity(scope.native.len()+scope.normalized.len());for row in &scope.native {observations.push(CoverageObservation::native(row)?);}for row in &scope.normalized {observations.push(CoverageObservation::normalized(row)?);}let expectation=CoverageExpectation {invocation,capability,scope:scope.scope,context:scope.context,requested:scope.requested,no_scope:scope.no_scope,sources:observations.iter().map(|r|r.source.id()).collect()};Ok(AdmittedScope {expectation,observations})}
/// Producer-side operation over captured declared inputs. The publication callback independently
/// repeats this operation against actual physical input rows and the effect owner's profile.
pub fn admit(invocation:&AnalysisInvocation,definition:&AnalysisDefinition,capability:AnalysisCapability,admission:&crate::domain::analysis::expected::CoverageAdmission<'_>,budget:&resources::ResourceBudget)->Result<AdmittedCoverage,ModelError> {
    if invocation.definition!=definition.id() {return Err(invalid("coverage changes the invocation definition"));}
    let domain=admission.domain(invocation.input,invocation.context,definition.method,capability,invocation.sources)?;
    let bytes=domain.scopes.iter().try_fold(0usize,|n,r|n.checked_add(size_of::<AdmittedScope>())?.checked_add((r.native.len()+r.normalized.len()).checked_mul(size_of::<CoverageObservation>()+size_of::<Id<CoverageSource>>())?)).ok_or_else(||invalid("admitted coverage allocation overflow"))?;
    let reservation=budget.reserve("admitted_analysis_coverage",bytes)?;
    let scopes=domain.scopes.iter().map(|scope|admitted_scope(invocation.id(),capability,scope)).collect::<Result<Vec<_>,_>>()?;Ok(AdmittedCoverage {scopes,_reservation:reservation})
}
fn bound_method()->Option<AnalysisMethod> {match AnalysisCoverage::NAME {"local_analysis_coverage"=>Some(AnalysisMethod::LocalTransfers),"catalog_core_analysis_coverage"=>Some(AnalysisMethod::Catalog),_=>None}}
pub(super) fn publication_checks()->Vec<PublicationInvariant> {
    let mut inputs=vec![ValidationInput::of::<AnalysisInvocation>(&["id"]),ValidationInput::of::<AnalysisDefinition>(&["id"]),ValidationInput::of::<super::AnalysisOutcome>(&["id"]),ValidationInput::of::<CoverageRequirement>(&["id"]),ValidationInput::of::<CoverageRequiredSource>(&["id"]),ValidationInput::of::<AnalysisCoverage>(&["id"]),ValidationInput::of::<AnalysisCoveragePremise>(&["id"]),ValidationInput::of::<CoverageSource>(&["id"])];
    if let Some(method)=bound_method() {inputs.extend(crate::domain::analysis::expected::inputs(method));}
    vec![PublicationInvariant {name:owner_table!("coverage_frontier"),inputs,create:std::sync::Arc::new(|budget|Box::new(FrontierCheck {charge:charged::StateCharge::new(budget,"analysis_coverage_frontier"),frontier:crate::domain::analysis::expected::FrontierIndex::new(stages::Profile::Catalog,budget),invocations:Default::default(),definitions:Default::default(),outcomes:Default::default(),requirements:Default::default(),required:Default::default(),coverage:Default::default(),premises:Default::default(),sources:Default::default()}))}]
}
struct FrontierCheck {charge:charged::StateCharge,frontier:crate::domain::analysis::expected::FrontierIndex,invocations:charged::ChargedMap<Id<AnalysisInvocation>,AnalysisInvocation>,definitions:charged::ChargedMap<Id<AnalysisDefinition>,AnalysisDefinition>,outcomes:charged::ChargedMap<Id<AnalysisInvocation>,super::AnalysisOutcome>,requirements:charged::ChargedMap<Id<CoverageRequirement>,CoverageRequirement>,required:charged::ChargedMap<Id<CoverageRequirement>,std::collections::BTreeSet<Id<CoverageSource>>>,coverage:charged::ChargedMap<Id<AnalysisCoverage>,AnalysisCoverage>,premises:charged::ChargedMap<Id<AnalysisCoverage>,std::collections::BTreeSet<Id<CoverageSource>>>,sources:charged::ChargedMap<Id<CoverageSource>,CoverageSource>}
impl PublicationCheck for FrontierCheck {
    fn visit(&mut self,relation:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError> {
        macro_rules! insert {($r:ty,$field:ident,$key:expr)=>{if relation==<$r>::NAME {for row in <$r>::decode(batch)? {if self.$field.insert(&mut self.charge,$key(&row),row)?.is_some() {return Err(ModelError::Conflict(<$r>::NAME));}}return Ok(());}};}
        insert!(AnalysisInvocation,invocations,|r:&AnalysisInvocation|r.id());insert!(AnalysisDefinition,definitions,|r:&AnalysisDefinition|r.id());insert!(super::AnalysisOutcome,outcomes,|r:&super::AnalysisOutcome|r.invocation);insert!(CoverageRequirement,requirements,|r:&CoverageRequirement|r.id());insert!(AnalysisCoverage,coverage,|r:&AnalysisCoverage|r.id());insert!(CoverageSource,sources,|r:&CoverageSource|r.id());
        if relation==CoverageRequiredSource::NAME {for row in CoverageRequiredSource::decode(batch)? {if !self.required.update(&mut self.charge,row.requirement,|v|v.insert(row.source))? {return Err(invalid("duplicate frontier requirement member"));}}return Ok(());}
        if relation==AnalysisCoveragePremise::NAME {for row in AnalysisCoveragePremise::decode(batch)? {if !self.premises.update(&mut self.charge,row.coverage,|v|v.insert(row.source))? {return Err(invalid("duplicate frontier coverage premise"));}}return Ok(());}
        if bound_method().is_some() && self.frontier.visit(relation,batch)? {return Ok(());}Err(invalid("undeclared analysis frontier input"))
    }
    fn finish(mut self:Box<Self>,actual:&[stages::CompletedRelation],profile:stages::Profile)->Result<(),ModelError> {
        self.frontier.set_profile(profile);let budget=self.charge.budget().ok_or_else(||invalid("frontier budget absent"))?;let mut charge=charged::StateCharge::new(budget,"analysis_expected_recheck");let mut expected_requirements=charged::ChargedSet::default();let mut expected_coverage=charged::ChargedSet::default();let mut expected_sources=charged::ChargedSet::default();let empty=std::collections::BTreeSet::new();
        for invocation in self.invocations.values() {
            let definition=self.definitions.get(&invocation.definition).ok_or_else(||invalid("admitted analysis definition absent"))?;
            if bound_method()!=Some(definition.method) {return Err(invalid("publication owner has no bound method/capability contract"));}
            let contract=crate::domain::analysis::expected::method_contract(definition.method)?;
            for input in crate::domain::analysis::expected::inputs(definition.method) {if !actual.iter().any(|r|r.relation()==input.name()) {return Err(invalid("frontier input has no declared completed source"));}}
            let outcome=self.outcomes.get(&invocation.id()).ok_or_else(||invalid("admitted analysis computation outcome absent"))?;
            let domain=self.frontier.domain(invocation.input,invocation.context,contract)?;
            for scope in &domain.scopes {
                let admitted=admitted_scope(invocation.id(),contract.capability,scope)?;let expectation=&admitted.expectation;let (requirement,members)=expectation.records()?;
                if self.requirements.get(&requirement.id())!=Some(&requirement) {return Err(invalid("expected capability scope requirement absent or forged"));}expected_requirements.insert(&mut charge,requirement.id())?;
                let expected_members=members.iter().map(|r|r.source).collect::<std::collections::BTreeSet<_>>();if self.required.get(&requirement.id()).unwrap_or(&empty)!=&expected_members {return Err(invalid("required domain was reduced or expanded"));}
                for observation in &admitted.observations {if self.sources.get(&observation.source.id())!=Some(&observation.source) {return Err(invalid("expected lower source mapping absent or forged"));}expected_sources.insert(&mut charge,observation.source.id())?;}
                let (coverage,premises)=assess(expectation,&admitted.observations,outcome.status,outcome.reason,budget)?;
                if self.coverage.get(&coverage.id())!=Some(&coverage) {return Err(invalid("scoped coverage differs from admitted expected domain"));}expected_coverage.insert(&mut charge,coverage.id())?;
                let expected_premises=premises.iter().map(|r|r.source).collect::<std::collections::BTreeSet<_>>();if self.premises.get(&coverage.id()).unwrap_or(&empty)!=&expected_premises {return Err(invalid("coverage premise domain was reduced or expanded"));}
            }
        }
        if expected_requirements.len()!=self.requirements.len() || expected_coverage.len()!=self.coverage.len() || expected_sources.len()!=self.sources.len() || self.outcomes.len()!=self.invocations.len() {return Err(invalid("analysis frontier has unexpected rows"));}
        for id in self.required.keys() {if !expected_requirements.contains(id) {return Err(invalid("orphan expected-domain member"));}}
        for id in self.premises.keys() {if !expected_coverage.contains(id) {return Err(invalid("orphan admitted coverage premise"));}}Ok(())
    }
}
