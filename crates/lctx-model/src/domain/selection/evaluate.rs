//! Predicate evaluation follows typed earlier identities; unknown evidence stays unknown.
use super::{
    algebra::{self, ClaimContext, Classified, Domain, Observation},
    build::{Data, Output, need},
    *,
};
use crate::domain::{
    catalog::{self, evidence as c1},
    normalized::{callables::*, entities::*},
    resources::ResourceBudget,
};
pub struct Prepared {
    data: classification::ClassificationData,
    output: Output,
    index: preparation::Index,
}
impl Prepared {
    pub fn new(data: &Data, output: &Output, b: &ResourceBudget) -> Result<Self, ModelError> {
        output.matches(&build::build(data, b)?)?;
        Self::from_local_rows(
            classification::ClassificationData::project(data, b)?,
            classification::copy_output(output, b)?,
            b,
        )
    }
    /// Checks local finite integrity only. This grants no publication or repository admission.
    pub fn from_local_rows(
        data: classification::ClassificationData,
        output: Output,
        b: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let index = preparation::Index::new(&data, &output, b)?;
        Ok(Self {
            data,
            output,
            index,
        })
    }
    pub fn data(&self) -> &classification::ClassificationData {
        &self.data
    }
    pub fn output(&self) -> &Output {
        &self.output
    }
    pub fn classify(
        &self,
        member: Id<catalog::CatalogMember>,
        analysis: Id<attribution::AnalysisContext>,
        requirement: &Requirement,
        b: &ResourceBudget,
    ) -> Result<Classified, ModelError> {
        requirement.predicate.validate()?;
        if let Predicate::FacetMembership { value, .. } = &requirement.predicate {
            let _lowering = b.reserve(
                "typed-facet-lowering",
                size_of::<Requirement>().saturating_add(value.heap_bytes()),
            )?;
            if let Some(predicate) = value.predicate() {
                let mut result = self.classify(
                    member,
                    analysis,
                    &Requirement {
                        predicate,
                        quantifier: requirement.quantifier,
                    },
                    b,
                )?;
                // The equivalent evaluation never substitutes the caller's canonical query identity.
                result.requirement = requirement.clone();
                return Ok(result);
            }
        }
        if matches!(&requirement.predicate, Predicate::SpecializedType { .. }) {
            return self.classify_specialized(member, analysis, requirement, b);
        }
        let id = self
            .index
            .domain(member, analysis, requirement.predicate.domain())?;
        let stored = need(&self.output.domains, id)?;
        let members = self.index.members(id);
        let count = members.len();
        let evidence_count = members
            .iter()
            .map(|link| {
                self.index
                    .evidence(
                        id,
                        need(&self.output.members, *link)
                            .expect("indexed membership")
                            .context,
                    )
                    .len()
            })
            .sum::<usize>();
        let closure_count = self.index.closure(id).len();
        let expanded = count.saturating_add(evidence_count);
        let retained = expanded
            .saturating_mul(
                size_of::<Observation>() + size_of::<Context>() + size_of::<ClaimContext>(),
            )
            .saturating_add((evidence_count + closure_count).saturating_mul(size_of::<Witness>()));
        let _reservation = b.reserve("selection-domain-observations", retained)?;
        let mut observations = Vec::new();
        let mut dynamic_evidence =
            charged::StateCharge::new(b, "selection-characterization-clones");
        for link in members {
            let link = need(&self.output.members, *link)?;
            let c = need(&self.output.contexts, link.context)?;
            if matches!(
                requirement.predicate,
                Predicate::PublicPath { .. } | Predicate::PublicModule { .. }
            ) && !matches!(c, Context::Member { .. })
            {
                continue;
            }
            if matches!(
                requirement.predicate,
                Predicate::BehavioralRaises { .. }
                    | Predicate::MemberKind { .. }
                    | Predicate::InvocationForm { .. }
                    | Predicate::ClassOwner { .. }
            ) && matches!(c, Context::Member { .. })
            {
                continue;
            }
            let selected_role = match &requirement.predicate {
                Predicate::ParameterType { .. } => None,
                Predicate::VariantParameterType { role, .. }
                | Predicate::VariantReturnType { role, .. } => Some(*role),
                _ => None,
            };
            if matches!(&requirement.predicate, Predicate::ParameterType { .. })
                && let Context::Signature { invocation, .. } = c
            {
                let invocation = need(&self.data.source.catalog.invocations, *invocation)?;
                if !need(&self.data.source.core.variants, invocation.variant)?
                    .role
                    .runtime_source()
                {
                    continue;
                }
            }
            if let Some(role) = selected_role
                && let Context::Signature { invocation, .. } = c
            {
                let invocation = need(&self.data.source.catalog.invocations, *invocation)?;
                if need(&self.data.source.core.variants, invocation.variant)?.role != role {
                    continue;
                }
            }
            if let Predicate::FacetMembership { value, .. } = &requirement.predicate
                && !structural_facets::applicable(&self.data, value, c)?
            {
                continue;
            }
            let mut evidence = Vec::new();
            for id in self.index.evidence(stored.id(), link.context) {
                let r = need(&self.output.evidence, *id)?;
                evidence.push(need(&self.output.witnesses, r.witness)?.clone());
            }
            if let Predicate::ParameterType { name, r#type } = &requirement.predicate {
                let mut emitted = false;
                for witness in &evidence {
                    let Witness::TypeObservation { observation } = witness else {
                        continue;
                    };
                    let raw = need(&self.data.facts.type_observations, *observation)?;
                    if !type_applies(&self.data, c, name, raw.subject)? {
                        continue;
                    }
                    emitted = true;
                    let value = type_match(&self.data, raw.term, r#type, c.analysis())?;
                    let context = ClaimContext::Declaration(c.clone());
                    observations.push(Observation {
                        context: context.clone(),
                        basis: EvidenceBasis::ProviderDeclaration,
                        value,
                        evidence: vec![witness.clone()],
                        admissible: if value == Some(true) {
                            vec![context]
                        } else {
                            vec![]
                        },
                    });
                }
                if emitted {
                    continue;
                }
            }
            if matches!(
                &requirement.predicate,
                Predicate::VariantReturnType {
                    role: calls::SignatureRole::Source,
                    ..
                }
            ) && let Context::Signature { invocation, .. } = c
            {
                let variant = need(
                    &self.data.source.core.variants,
                    need(&self.data.source.catalog.invocations, *invocation)?.variant,
                )?;
                if let Some(CallableEntity::Source { declaration, .. }) = variant
                    .callable
                    .and_then(|id| self.data.source.core.source_callables.get(id))
                {
                    for row in self.data.facts.type_observations.iter().filter(|r| {
                        r.subject == *declaration
                            && r.role == types::TypeRole::Return
                            && r.declared
                            && self
                                .data
                                .source
                                .core
                                .qualifications
                                .get(r.qualification)
                                .is_some_and(|q| q.context == c.analysis())
                    }) {
                        dynamic_evidence.grow(2 * size_of::<Witness>())?;
                        let witness = Witness::TypeObservation {
                            observation: row.id(),
                        };
                        need(&self.output.witnesses, witness.id())?;
                        evidence.push(witness);
                    }
                }
            }
            let facet_answer =
                if let Predicate::FacetMembership { value, .. } = &requirement.predicate {
                    Some(structural_facets::answer(&self.data, value, c, b)?)
                } else {
                    None
                };
            let value = if let Some(answer) = &facet_answer {
                for witness in answer.evidence.iter() {
                    need(&self.output.witnesses, witness.id())?;
                    self.data.validate_witness(c, witness)?;
                    dynamic_evidence.grow(2 * size_of::<Witness>() + witness.heap_bytes())?;
                    evidence.push(witness.clone());
                }
                answer.value
            } else {
                evaluate(
                    &self.data,
                    &requirement.predicate,
                    c,
                    &evidence,
                    link.complete,
                )?
            };
            let context = ClaimContext::Declaration(c.clone());
            let admissible = if value == Some(true) {
                vec![context.clone()]
            } else {
                vec![]
            };
            let basis = if let Some(answer) = facet_answer {
                answer.basis
            } else if matches!(requirement.predicate, Predicate::BehavioralRaises { .. }) {
                EvidenceBasis::BoundedModel
            } else if matches!(c, Context::Scenario { .. }) {
                EvidenceBasis::ObservedScenario
            } else if matches!(&requirement.predicate, Predicate::VariantParameterType {role,..} | Predicate::VariantReturnType {role,..} if !role.runtime_source())
            {
                EvidenceBasis::ProviderDeclaration
            } else {
                EvidenceBasis::SourceDeclaration
            };
            observations.push(Observation {
                context,
                basis,
                value,
                evidence,
                admissible,
            });
        }
        let mut closure = Vec::new();
        for id in self.index.closure(stored.id()) {
            let r = need(&self.output.closure, *id)?;
            closure.push(need(&self.output.witnesses, r.evidence)?.clone());
        }
        let role_requested = matches!(
            &requirement.predicate,
            Predicate::VariantParameterType { .. }
                | Predicate::VariantReturnType { .. }
                | Predicate::FacetMembership { .. }
        );
        let role_observed = !observations.is_empty();
        let domain = Domain {
            corpus_complete: stored.corpus_complete,
            analyzer_complete: if matches!(
                requirement.predicate,
                Predicate::BehavioralRaises { .. }
            ) {
                !observations.is_empty() && observations.iter().all(|row| row.value.is_some())
            } else {
                stored.analyzer_complete && (!role_requested || role_observed)
                    || matches!(
                        requirement.predicate,
                        Predicate::PublicPath { .. } | Predicate::PublicModule { .. }
                    )
            },
            closure,
            observations,
        };
        algebra::classify(requirement, &domain, b)
    }
    fn classify_specialized(
        &self,
        member: Id<catalog::CatalogMember>,
        analysis: Id<attribution::AnalysisContext>,
        requirement: &Requirement,
        b: &ResourceBudget,
    ) -> Result<Classified, ModelError> {
        let Predicate::SpecializedType {
            site,
            declaration,
            subject,
            r#type,
        } = &requirement.predicate
        else {
            return Err(ModelError::Invalid(
                "located specialization requires its typed predicate".into(),
            ));
        };
        let domain_id = self
            .index
            .domain(member, analysis, DomainKind::SignatureVariants)?;
        let stored = need(&self.output.domains, domain_id)?;
        // The native declaration must be a signature of the requested catalog member.
        let mut applicable = None;
        for id in self.index.members(domain_id) {
            let row = need(&self.output.members, *id)?;
            let context = need(&self.output.contexts, row.context)?;
            if let Context::Signature { invocation, .. } = context {
                let variant = need(
                    &self.data.source.core.variants,
                    need(&self.data.source.catalog.invocations, *invocation)?.variant,
                )?;
                if variant.native == Some(*declaration) {
                    if applicable.is_some() {
                        return Err(ModelError::Invalid(
                            "located specialization has ambiguous catalog invocation".into(),
                        ));
                    }
                    applicable = Some(context);
                }
            }
        }
        let context = applicable.ok_or_else(|| {
            ModelError::Invalid(
                "located specialization crosses catalog member or native role".into(),
            )
        })?;
        let result = self.specialize_port(*site, *declaration, *subject, analysis, b)?;
        let resolved = result.specialized.status
            == crate::domain::normalized::generic_specialization::SpecializationStatus::Resolved;
        let value = if resolved {
            type_match_overlay(
                &self.data,
                Some(&result.specialized.structure),
                result.specialized.term,
                r#type,
                analysis,
            )?
        } else {
            None
        };
        let _scratch = b.reserve(
            "located-selection-evidence",
            result
                .specialized
                .support
                .len()
                .saturating_add(2)
                .saturating_mul(size_of::<Witness>())
                .saturating_add(size_of::<Observation>()),
        )?;
        let mut evidence = vec![
            Witness::SignatureTypeObservation {
                observation: result.observation,
            },
            Witness::Qualification {
                qualification: result.specialized.qualification,
            },
        ];
        evidence.extend(result.specialized.support.iter().map(|observation| {
            Witness::GenericSpecialization {
                observation: *observation,
            }
        }));
        for witness in &evidence {
            need(&self.output.witnesses, witness.id())?;
        }
        let claim = ClaimContext::Declaration(context.clone());
        let observations = vec![Observation {
            context: claim.clone(),
            basis: EvidenceBasis::ProviderDeclaration,
            value,
            evidence: evidence.clone(),
            admissible: if value == Some(true) {
                vec![claim]
            } else {
                vec![]
            },
        }];
        // This closure concerns one located native answer, never all runtime invocations.
        algebra::classify(
            requirement,
            &Domain {
                corpus_complete: stored.corpus_complete,
                analyzer_complete: resolved,
                closure: evidence,
                observations,
            },
            b,
        )
    }
}
/// All result groups remain available; strict eligibility is a separate finite view.
pub struct CandidateSelection {
    pub member: Id<catalog::CatalogMember>,
    pub analysis: Id<attribution::AnalysisContext>,
    pub path: Vec<String>,
    pub requirements: Vec<Classified>,
    pub joint: JointApplicability,
    pub outcome: Outcome,
    _charge: charged::StateCharge,
}
pub struct Selected {
    pub mode: Mode,
    pub candidates: Vec<CandidateSelection>,
    _reservation: Box<dyn resources::Reservation>,
}
impl Selected {
    pub fn group(&self, outcome: Outcome) -> impl Iterator<Item = &CandidateSelection> {
        self.candidates.iter().filter(move |r| r.outcome == outcome)
    }
    pub fn eligible(&self) -> impl Iterator<Item = &CandidateSelection> {
        self.candidates.iter().filter(|r| match self.mode {
            Mode::Strict => r.outcome == Outcome::Supported,
            Mode::Discovery => matches!(
                r.outcome,
                Outcome::Supported | Outcome::Unresolved | Outcome::Conflicting
            ),
        })
    }
}
impl Prepared {
    pub fn select(
        &self,
        selection: &Selection,
        b: &ResourceBudget,
    ) -> Result<Selected, ModelError> {
        algebra::selection_digest(selection)?;
        let count = self
            .output
            .domains
            .iter()
            .filter(|r| r.kind == DomainKind::PublicExposures)
            .count();
        let reservation = b.reserve(
            "selection-candidates",
            count.saturating_mul(size_of::<CandidateSelection>()),
        )?;
        let mut candidates = Vec::new();
        for domain in self
            .output
            .domains
            .iter()
            .filter(|r| r.kind == DomainKind::PublicExposures)
        {
            let member = need(&self.data.source.catalog.members, domain.member)?;
            let mut charge = charged::StateCharge::new(b, "selection-candidate");
            charge.admit(&member.path)?;
            let module = need(&self.data.source.core.modules, member.access)?;
            charge.grow(
                module.qualified_name.len()
                    + module.qualified_name.split('.').count() * size_of::<String>(),
            )?;
            charge.grow(
                selection
                    .requirements
                    .len()
                    .saturating_mul(size_of::<Classified>()),
            )?;
            let mut requirements = Vec::new();
            for requirement in &selection.requirements {
                requirements.push(self.classify(domain.member, domain.analysis, requirement, b)?);
            }
            let refs = requirements.iter().collect::<Vec<_>>();
            let joint = algebra::joint(&refs, selection.joint, b)?;
            let outcome = algebra::aggregate(&refs, joint, selection.joint);
            candidates.push(CandidateSelection {
                member: domain.member,
                analysis: domain.analysis,
                path: public_path(&self.data, member)?,
                requirements,
                joint,
                outcome,
                _charge: charge,
            });
        }
        Ok(Selected {
            mode: selection.mode,
            candidates,
            _reservation: reservation,
        })
    }
}
fn public_path(
    d: &classification::ClassificationData,
    m: &catalog::CatalogMember,
) -> Result<Vec<String>, ModelError> {
    let module = need(&d.source.core.modules, m.access)?;
    let mut path = module
        .qualified_name
        .split('.')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    path.extend(m.path.iter().cloned());
    Ok(path)
}
fn consensus(values: impl IntoIterator<Item = Option<bool>>) -> Option<bool> {
    let mut found = None;
    for value in values {
        let value = value?;
        if found.is_some_and(|old| old != value) {
            return None;
        }
        found = Some(value);
    }
    found
}
fn member_of<'a>(
    d: &'a classification::ClassificationData,
    c: &Context,
) -> Result<Option<&'a catalog::CatalogMember>, ModelError> {
    c.member()
        .map(|id| need(&d.source.catalog.members, id))
        .transpose()
}
fn option_name<'a>(
    d: &'a classification::ClassificationData,
    o: &catalog::CatalogOption,
) -> Result<Option<&'a str>, ModelError> {
    Ok(match need(&d.source.catalog.subjects, o.subject)? {
        catalog::CatalogOptionSubject::Parameter { slot } => {
            let slot = need(&d.source.core.slots, *slot)?;
            let p = need(&d.facts.signature_parameters, slot.parameter)?;
            need(&d.facts.shapes, p.shape)?
                .name
                .as_ref()
                .map(|s| s.as_str())
        }
        catalog::CatalogOptionSubject::Field { field } => {
            Some(need(&d.source.core.fields, *field)?.name.as_str())
        }
        catalog::CatalogOptionSubject::SourceParameter { parameter } => {
            let mut names = d
                .source
                .core
                .parameter_links
                .iter()
                .filter(|link| link.entity == *parameter)
                .filter_map(|link| d.facts.signature_parameters.get(link.parameter))
                .filter_map(|p| d.facts.shapes.get(p.shape))
                .filter_map(|shape| shape.name.as_ref().map(|n| n.as_str()));
            let first = names.next();
            if names.any(|n| Some(n) != first) {
                None
            } else {
                first
            }
        }
    })
}
fn default_state(
    d: &classification::ClassificationData,
    o: &catalog::CatalogOption,
) -> Result<DefaultState, ModelError> {
    Ok(match need(&d.source.catalog.defaults, o.default)? {
        catalog::CatalogDefault::Absent {} => DefaultState::Absent,
        catalog::CatalogDefault::Unavailable {} => DefaultState::OptionalExpressionUnavailable,
        catalog::CatalogDefault::Unknown {} => DefaultState::Unknown,
        catalog::CatalogDefault::Expression { .. } => DefaultState::SourceExpression,
        catalog::CatalogDefault::Factory { .. } => DefaultState::FactoryExpression,
        catalog::CatalogDefault::Literal { literal } => {
            if matches!(need(&d.facts.literals, *literal)?, value::Literal::None) {
                DefaultState::LiteralNone
            } else {
                DefaultState::Literal
            }
        }
    })
}
pub(super) fn type_match(
    d: &classification::ClassificationData,
    term: Id<types::TypeTerm>,
    query: &StructuralType,
    analysis: Id<attribution::AnalysisContext>,
) -> Result<Option<bool>, ModelError> {
    type_match_overlay(d, None, term, query, analysis)
}
pub(super) fn type_match_overlay(
    d: &classification::ClassificationData,
    overlay: Option<&crate::domain::normalized::generic_specialization::Graph>,
    term: Id<types::TypeTerm>,
    query: &StructuralType,
    analysis: Id<attribution::AnalysisContext>,
) -> Result<Option<bool>, ModelError> {
    let t = overlay
        .and_then(|g| g.terms.get(term))
        .or_else(|| d.facts.type_terms.get(term))
        .ok_or_else(|| {
            ModelError::Invalid("structural term absent from base and specialization".into())
        })?;
    if matches!(
        t,
        types::TypeTerm::Other { .. }
            | types::TypeTerm::Truncated { .. }
            | types::TypeTerm::Any { .. }
    ) {
        return Ok(None);
    }
    Ok(match query {
        StructuralType::CanonicalTerm { term: expected } => overlay
            .and_then(|g| g.terms.get(*expected))
            .or_else(|| d.facts.type_terms.get(*expected))
            .map(|_| term == *expected),
        StructuralType::Category { kind } => Some(t.tag() == *kind),
        StructuralType::DeclaredUnionMember { term: expected }
            if overlay
                .and_then(|g| g.terms.get(*expected))
                .or_else(|| d.facts.type_terms.get(*expected))
                .is_none() =>
        {
            None
        }
        StructuralType::DeclaredUnionMember { term: expected } => match t {
            types::TypeTerm::Union { members } => Some(
                d.facts
                    .type_sequences
                    .iter()
                    .any(|r| r.sequence == *members && r.child == *expected)
                    || overlay.is_some_and(|g| {
                        g.members
                            .iter()
                            .any(|r| r.sequence == *members && r.child == *expected)
                    }),
            ),
            _ => Some(false),
        },
        StructuralType::NominalIdentity { module, name } => match t {
            types::TypeTerm::ClassInstance { class, .. }
            | types::TypeTerm::ClassObject { class }
            | types::TypeTerm::TypedDict { class, .. } => {
                let symbol = need(&d.source.core.symbols, *class)?;
                if symbol.context != analysis {
                    None
                } else {
                    let module_name = match need(&d.source.core.provider_modules, symbol.module)? {
                        calls::ProviderModule::Acquired { module } => Some(
                            need(&d.source.core.modules, *module)?
                                .qualified_name
                                .as_str(),
                        ),
                        calls::ProviderModule::Bundled { name, .. }
                        | calls::ProviderModule::Namespace { name, .. } => Some(name.as_str()),
                        calls::ProviderModule::Unresolved { .. } => None,
                    };
                    module_name.map(|m| m == module && symbol.name == *name)
                }
            }
            _ => Some(false),
        },
    })
}
fn type_applies(
    d: &classification::ClassificationData,
    c: &Context,
    name: &str,
    subject: Id<source::Occurrence>,
) -> Result<bool, ModelError> {
    let Context::Signature { invocation, .. } = c else {
        return Ok(false);
    };
    let invocation = need(&d.source.catalog.invocations, *invocation)?;
    for slot in d
        .source
        .core
        .slots
        .iter()
        .filter(|s| s.variant == invocation.variant)
    {
        let raw = need(&d.facts.signature_parameters, slot.parameter)?;
        let shape = need(&d.facts.shapes, raw.shape)?;
        if shape.name.as_ref().is_none_or(|n| n.as_str() != name) {
            continue;
        }
        for entity in d
            .source
            .core
            .slot_entities
            .iter()
            .filter(|e| e.slot == slot.id())
        {
            let link = need(&d.source.core.parameter_links, entity.link)?;
            if matches!(need(&d.source.core.parameters,link.entity)?,ParameterEntity::Source {declaration} if *declaration==subject)
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
fn literal_domain(
    d: &classification::ClassificationData,
    term: Id<types::TypeTerm>,
    literal: Id<value::Literal>,
) -> Result<Option<bool>, ModelError> {
    if d.facts.literals.get(literal).is_none() {
        return Ok(None);
    }
    Ok(match need(&d.facts.type_terms, term)? {
        types::TypeTerm::Literal { value } => Some(*value == literal),
        types::TypeTerm::Union { members } => {
            let mut count = 0;
            let mut known = true;
            let mut found = false;
            for r in d
                .facts
                .type_sequences
                .iter()
                .filter(|r| r.sequence == *members)
            {
                count += 1;
                match need(&d.facts.type_terms, r.child)? {
                    types::TypeTerm::Literal { value } => found |= *value == literal,
                    _ => known = false,
                }
            }
            if found {
                Some(true)
            } else if count > 0 && known {
                Some(false)
            } else {
                None
            }
        }
        _ => None,
    })
}
fn field_literal(
    d: &classification::ClassificationData,
    o: &catalog::CatalogOption,
    literal: Id<value::Literal>,
    context: Id<attribution::AnalysisContext>,
) -> Result<Option<bool>, ModelError> {
    let catalog::CatalogOptionSubject::Field { field } =
        need(&d.source.catalog.subjects, o.subject)?
    else {
        return Ok(None);
    };
    let mut values = Vec::new();
    for link in d
        .source
        .core
        .field_links
        .iter()
        .filter(|r| r.field == *field)
    {
        let raw = need(&d.source.core.field_observations, link.observation)?;
        let q = need(&d.source.core.qualifications, raw.qualification)?;
        if raw.declared
            && q.context == context
            && q.approximation == assertion::Approximation::Exact
            && q.modality == attribution::Modality::Definite
            && q.condition == conditions::Diagram::always().id()
        {
            values.push(literal_domain(d, raw.term, literal)?);
        }
    }
    Ok(consensus(values))
}
fn parameter(
    d: &classification::ClassificationData,
    p: &Predicate,
    c: &Context,
) -> Result<Option<bool>, ModelError> {
    let Context::Signature { invocation, .. } = c else {
        return Ok(None);
    };
    let invocation = need(&d.source.catalog.invocations, *invocation)?;
    let variant = need(&d.source.core.variants, invocation.variant)?;
    let signature = need(&d.facts.signatures, variant.signature)?;
    let name = match p {
        Predicate::DeclaresParameter { name }
        | Predicate::ParameterKind { name, .. }
        | Predicate::ParameterRequired { name, .. }
        | Predicate::ParameterDefaultState { name, .. }
        | Predicate::ParameterDefault { name, .. }
        | Predicate::ParameterType { name, .. }
        | Predicate::VariantParameterType { name, .. } => name,
        _ => return Ok(None),
    };
    let mut found = None;
    for slot in d
        .source
        .core
        .slots
        .iter()
        .filter(|r| r.variant == variant.id())
    {
        let raw = need(&d.facts.signature_parameters, slot.parameter)?;
        let shape = need(&d.facts.shapes, raw.shape)?;
        if shape.name.as_ref().is_none_or(|s| s.as_str() != name) {
            continue;
        }
        if found.is_some() {
            return Ok(None);
        }
        found = Some((slot, shape));
    }
    let Some((slot, shape)) = found else {
        let callable = need(&d.source.catalog.callables, invocation.callable)?;
        let assessment = need(&d.source.core.assessments, callable.assessment)?;
        let complete = if variant.role.runtime_source() {
            assessment.signatures == Knowledge::Known
        } else {
            variant
                .native
                .and_then(|id| d.source.core.native_signatures.get(id))
                .is_some_and(|n| n.complete)
        };
        return Ok((signature.form == calls::SignatureForm::List
            && complete
            && variant.adjustment != SignatureAdjustment::Unknown)
            .then_some(false));
    };
    match p {
        Predicate::DeclaresParameter { .. } => Ok(Some(true)),
        Predicate::ParameterKind { kind, .. } => Ok(Some(shape.kind == *kind)),
        Predicate::ParameterRequired { required, .. } => Ok(Some(shape.required == *required)),
        Predicate::ParameterDefault { value, .. } => {
            if d.facts.literals.get(*value).is_none() {
                return Ok(None);
            }
            let mut values = Vec::new();
            for o in d.source.catalog.options.iter().filter(|r|Some(r.member)==c.member() && d.source.catalog.subjects.get(r.subject).is_some_and(|s|matches!(s,catalog::CatalogOptionSubject::Parameter {slot:s} if *s==slot.id()))) {values.push(match need(&d.source.catalog.defaults,o.default)? {catalog::CatalogDefault::Literal {literal}=>Some(*literal==*value),catalog::CatalogDefault::Absent {}=>Some(false),_=>None});}
            Ok(consensus(values))
        }
        Predicate::ParameterDefaultState { state, .. } => {
            let mut values = Vec::new();
            for o in d.source.catalog.options.iter().filter(|r|Some(r.member)==c.member() && d.source.catalog.subjects.get(r.subject).is_some_and(|s|matches!(s,catalog::CatalogOptionSubject::Parameter {slot:s} if *s==slot.id()))) {let observed=default_state(d,o)?;values.push(if observed==DefaultState::Unknown {None}else{Some(observed==*state)});}
            Ok(consensus(values))
        }
        Predicate::VariantParameterType { role, r#type, .. } => {
            if variant.role != *role {
                return Ok(None);
            }
            let mut values = Vec::new();
            for link in d
                .source
                .core
                .slot_types
                .iter()
                .filter(|r| r.slot == slot.id())
            {
                let raw = need(&d.source.core.signature_types, link.observation)?;
                let q = need(&d.source.core.qualifications, raw.qualification)?;
                if q.context == c.analysis()
                    && q.approximation == assertion::Approximation::Exact
                    && q.modality == attribution::Modality::Definite
                    && q.condition == conditions::Diagram::always().id()
                {
                    values.push(type_match(d, raw.term, r#type, c.analysis())?);
                }
            }
            Ok(consensus(values))
        }
        Predicate::ParameterType { r#type, .. } => {
            let mut values = Vec::new();
            for e in d
                .source
                .core
                .slot_entities
                .iter()
                .filter(|r| r.slot == slot.id())
            {
                let link = need(&d.source.core.parameter_links, e.link)?;
                let ParameterEntity::Source { declaration } =
                    need(&d.source.core.parameters, link.entity)?
                else {
                    continue;
                };
                for obs in d.facts.type_observations.iter().filter(|r| {
                    r.subject == *declaration && r.role == types::TypeRole::Parameter && r.declared
                }) {
                    let q = need(&d.source.core.qualifications, obs.qualification)?;
                    if q.context == c.analysis()
                        && q.approximation == assertion::Approximation::Exact
                        && q.modality == attribution::Modality::Definite
                        && q.condition == conditions::Diagram::always().id()
                    {
                        values.push(type_match(d, obs.term, r#type, c.analysis())?);
                    }
                }
            }
            Ok(consensus(values))
        }
        _ => Ok(None),
    }
}
pub(super) fn candidate_entity(
    d: &classification::ClassificationData,
    candidate: Id<catalog::CatalogCandidate>,
) -> Result<Option<Id<EntityRef>>, ModelError> {
    let r = need(&d.source.catalog.candidates, candidate)?;
    if let Some(path) = r.path {
        return Ok(Some(need(&d.source.catalog.paths, path)?.entity));
    }
    if let Some(alias) = r.alias {
        return Ok(Some(need(&d.source.catalog.aliases, alias)?.entity));
    }
    r.entity
        .map(|e| need(&d.source.core.entity_candidates, e).map(|e| e.entity))
        .transpose()
}
fn evaluate(
    d: &classification::ClassificationData,
    p: &Predicate,
    c: &Context,
    w: &[Witness],
    complete: bool,
) -> Result<Option<bool>, ModelError> {
    let member = member_of(d, c)?;
    match p {
        Predicate::BehavioralRaises { exception } => {
            let Context::Binding { candidate, .. } = c else {
                return Ok(None);
            };
            let owner = candidate_entity(d, *candidate)?;
            let mut results = w.iter().filter_map(|witness| match witness {
                Witness::SummaryException { outcome } => d.facts.exception_outcomes.get(*outcome),
                _ => None,
            });
            let Some(result) = results.next() else {
                return Ok(None);
            };
            if results.next().is_some()
                || result.context != c.analysis()
                || Some(result.owner) != owner
                || member.is_none_or(|member| member.input != result.input)
            {
                return Ok(None);
            }
            let q = need(&d.source.core.qualifications, result.qualification)?;
            if q.context != c.analysis()
                || q.condition != conditions::Diagram::always().id()
                || q.assumptions != assumptions::AssumptionSet::empty().id()
                || q.modality != attribution::Modality::Definite
                || q.approximation != assertion::Approximation::Exact
                || analysis::policy::behavioral_support(result.status, false).is_err()
            {
                return Ok(None);
            }
            Ok(Some(result.exception == Some(*exception)))
        }
        Predicate::PublicPath { path } => Ok(member
            .map(|m| public_path(d, m).map(|p| p == *path))
            .transpose()?),
        Predicate::PublicModule { module } => Ok(member
            .map(|m| need(&d.source.core.modules, m.access).map(|m| m.qualified_name == *module))
            .transpose()?),
        Predicate::MemberKind { kind } => {
            let candidate = match c {
                Context::Binding { candidate, .. } | Context::Signature { candidate, .. } => {
                    *candidate
                }
                _ => return Ok(None),
            };
            let Some(entity) = candidate_entity(d, candidate)? else {
                return Ok(None);
            };
            if *kind == MemberKind::Class
                && matches!(
                    need(&d.source.core.refs, entity)?,
                    EntityRef::Callable { .. }
                )
            {
                return Ok(Some(false));
            }
            let observed = match need(&d.source.core.refs, entity)? {
                EntityRef::Class { .. } => MemberKind::Class,
                EntityRef::Callable { callable } => {
                    let mut values = d
                        .source
                        .core
                        .assessments
                        .iter()
                        .filter(|r| r.callable == *callable && r.context == c.analysis());
                    let a = values.next();
                    if values.next().is_some() {
                        return Ok(None);
                    }
                    match a.and_then(|a| a.descriptor_kind) {
                        Some(DescriptorKind::Function) => MemberKind::Function,
                        Some(DescriptorKind::Property) => MemberKind::Property,
                        Some(_) => MemberKind::Method,
                        None => return Ok(None),
                    }
                }
                EntityRef::Field { .. } => MemberKind::Variable,
                _ => return Ok(None),
            };
            Ok(Some(observed == *kind))
        }
        Predicate::InvocationForm { form } => {
            let candidate = match c {
                Context::Binding { candidate, .. } | Context::Signature { candidate, .. } => {
                    *candidate
                }
                _ => return Ok(None),
            };
            let mut values = Vec::new();
            for callable in d
                .source
                .catalog
                .callables
                .iter()
                .filter(|r| r.candidate == candidate)
            {
                let a = need(&d.source.core.assessments, callable.assessment)?;
                if a.context != c.analysis() {
                    continue;
                }
                let observed = match a.descriptor_kind {
                    Some(DescriptorKind::Function) => InvocationForm::Function,
                    Some(DescriptorKind::InstanceMethod) => InvocationForm::Method,
                    Some(DescriptorKind::StaticMethod) => InvocationForm::Static,
                    Some(DescriptorKind::ClassMethod) => InvocationForm::Class,
                    Some(DescriptorKind::Property) => InvocationForm::Property,
                    None => {
                        values.push(None);
                        continue;
                    }
                };
                values.push(Some(observed == *form));
            }
            Ok(consensus(values))
        }
        Predicate::ClassOwner { path } => {
            let Context::Binding { candidate, .. } = c else {
                return Ok(None);
            };
            let Some(entity) = candidate_entity(d, *candidate)? else {
                return Ok(None);
            };
            let EntityRef::Callable { callable } = need(&d.source.core.refs, entity)? else {
                return Ok(None);
            };
            let Some(CallableEntity::Source { declaration, .. }) =
                d.source.core.source_callables.get(*callable)
            else {
                return Ok(None);
            };
            let parent = d
                .source
                .core
                .declarations
                .iter()
                .find(|r| r.declaration == *declaration)
                .and_then(|r| r.parent);
            let Some(parent) = parent else {
                return Ok(Some(false));
            };
            let class = ClassEntity::Source {
                declaration: parent,
            }
            .id();
            let catalog_candidate = need(&d.source.catalog.candidates, *candidate)?;
            if let Some(p) = catalog_candidate.path {
                let p = need(&d.source.catalog.paths, p)?;
                let parent = need(&d.source.catalog.candidates, p.parent)?;
                let link = need(&d.source.catalog.exposures, parent.exposure)?;
                let owner = need(&d.source.catalog.members, link.member)?;
                if let Some(EntityRef::Class {
                    class: parent_class,
                }) = candidate_entity(d, p.parent)?.and_then(|id| d.source.core.refs.get(id))
                    && *parent_class == class
                {
                    return Ok(Some(public_path(d, owner)? == *path));
                }
            }
            let mut values = Vec::new();
            for owner in d.source.catalog.classes.iter().filter(|r| r.class == class) {
                let m = need(&d.source.catalog.members, owner.member)?;
                if member.is_some_and(|own| own.input == m.input) {
                    values.push(Some(public_path(d, m)? == *path));
                }
            }
            Ok(consensus(values))
        }
        Predicate::DeclaresParameter { .. }
        | Predicate::ParameterKind { .. }
        | Predicate::ParameterRequired { .. }
        | Predicate::ParameterDefaultState { .. }
        | Predicate::ParameterDefault { .. }
        | Predicate::ParameterType { .. }
        | Predicate::VariantParameterType { .. } => parameter(d, p, c),
        Predicate::VariantReturnType { role, r#type } => {
            let Context::Signature { invocation, .. } = c else {
                return Ok(None);
            };
            let invocation = need(&d.source.catalog.invocations, *invocation)?;
            let variant = need(&d.source.core.variants, invocation.variant)?;
            if variant.role != *role {
                return Ok(None);
            }
            let mut values = Vec::new();
            if role.runtime_source() {
                if let Some(CallableEntity::Source { declaration, .. }) = variant
                    .callable
                    .and_then(|id| d.source.core.source_callables.get(id))
                {
                    for raw in d.facts.type_observations.iter().filter(|r| {
                        r.subject == *declaration && r.role == types::TypeRole::Return && r.declared
                    }) {
                        let q = need(&d.source.core.qualifications, raw.qualification)?;
                        if q.context == c.analysis()
                            && q.approximation == assertion::Approximation::Exact
                            && q.modality == attribution::Modality::Definite
                            && q.condition == conditions::Diagram::always().id()
                        {
                            values.push(type_match(d, raw.term, r#type, c.analysis())?);
                        }
                    }
                }
                return Ok(consensus(values));
            }
            for link in d
                .source
                .core
                .return_types
                .iter()
                .filter(|r| r.variant == variant.id())
            {
                let raw = need(&d.source.core.signature_types, link.observation)?;
                let q = need(&d.source.core.qualifications, raw.qualification)?;
                if q.context == c.analysis()
                    && q.approximation == assertion::Approximation::Exact
                    && q.modality == attribution::Modality::Definite
                    && q.condition == conditions::Diagram::always().id()
                {
                    values.push(type_match(d, raw.term, r#type, c.analysis())?);
                }
            }
            Ok(consensus(values))
        }
        Predicate::DeclaresConfigurationField { name }
        | Predicate::ConfigurationDefault { name, .. }
        | Predicate::ConfigurationLiteral { name, .. }
        | Predicate::ConfigurationRelationship { name, .. } => {
            let mut values = Vec::new();
            let mut found = false;
            for witness in w {
                let Witness::Option { option } = witness else {
                    continue;
                };
                let o = need(&d.source.catalog.options, *option)?;
                if option_name(d, o)? != Some(name) {
                    continue;
                }
                found = true;
                match p {
                    Predicate::DeclaresConfigurationField { .. } => values.push(Some(true)),
                    Predicate::ConfigurationLiteral { value, .. } => {
                        values.push(field_literal(d, o, *value, c.analysis())?)
                    }
                    Predicate::ConfigurationDefault { value, .. } => {
                        values.push(if d.facts.literals.get(*value).is_none() {
                            None
                        } else {
                            match need(&d.source.catalog.defaults, o.default)? {
                                catalog::CatalogDefault::Literal { literal } => {
                                    Some(*literal == *value)
                                }
                                catalog::CatalogDefault::Absent {} => Some(false),
                                _ => None,
                            }
                        })
                    }
                    Predicate::ConfigurationRelationship { kind, target, .. } => {
                        if *kind == FieldRelationship::DeclaredParameter {
                            values.push(super::source_fields::declared_parameter(
                                d,
                                o.id(),
                                target,
                                c.analysis(),
                            )?);
                            continue;
                        }
                        for access in d.evidence.accesses.iter().filter(|a| a.option == o.id()) {
                            if access.applicability != Knowledge::Known {
                                values.push(None);
                                continue;
                            }
                            let owner = need(&d.source.core.ownership, access.owner)?;
                            let target_match = match target {
                                FieldTarget::Declaration { entity } => owner.entity == *entity,
                                FieldTarget::Parameter { .. } => {
                                    values.push(None);
                                    continue;
                                }
                            };
                            let kind_match = match kind {
                                FieldRelationship::ExactReader => {
                                    access.basis == c1::AssociationBasis::ExactRead
                                }
                                FieldRelationship::ExactStorage => {
                                    access.basis == c1::AssociationBasis::ExactInitialization
                                }
                                FieldRelationship::DeclaredParameter => {
                                    values.push(None);
                                    continue;
                                }
                            };
                            values.push(Some(target_match && kind_match));
                        }
                    }
                    _ => {}
                }
            }
            if !found {
                Ok(complete.then_some(false))
            } else {
                Ok(consensus(values))
            }
        }
        Predicate::ConfigurationScope { scope } => Ok(match c {
            Context::Configuration {
                scope: observed, ..
            } => Some(observed == scope),
            _ => None,
        }),
        Predicate::ConfigurationOwner { path } => {
            let Context::Configuration { owner, .. } = c else {
                return Ok(None);
            };
            let mut options = Vec::new();
            for witness in w {
                if let Witness::Option { option } = witness {
                    let option = need(&d.source.catalog.options, *option)?;
                    if d.source
                        .catalog
                        .classes
                        .iter()
                        .any(|r| r.member == option.member && r.class == *owner)
                    {
                        options.push(Some(
                            public_path(d, need(&d.source.catalog.members, option.member)?)?
                                == *path,
                        ));
                    }
                }
            }
            if !options.is_empty() {
                return Ok(consensus(options));
            }
            let mut values = Vec::new();
            for class in d
                .source
                .catalog
                .classes
                .iter()
                .filter(|r| r.class == *owner)
            {
                let m = need(&d.source.catalog.members, class.member)?;
                if member.is_some_and(|own| own.input == m.input) {
                    values.push(Some(public_path(d, m)? == *path));
                }
            }
            Ok(consensus(values))
        }
        Predicate::ConfigurationRecordKind { kind } => {
            let mut values = Vec::new();
            for witness in w {
                let Witness::Option { option } = witness else {
                    continue;
                };
                let o = need(&d.source.catalog.options, *option)?;
                let catalog::CatalogOptionSubject::Field { field } =
                    need(&d.source.catalog.subjects, o.subject)?
                else {
                    continue;
                };
                for link in d
                    .source
                    .core
                    .field_links
                    .iter()
                    .filter(|r| r.field == *field)
                {
                    let obs = need(&d.source.core.field_observations, link.observation)?;
                    if need(&d.source.core.qualifications, obs.qualification)?.context
                        == c.analysis()
                    {
                        values.push(Some(obs.record == *kind));
                    }
                }
            }
            Ok(consensus(values))
        }
        Predicate::ScenarioIntent { intent } => {
            let mut values = Vec::new();
            for witness in w {
                if let Witness::Association { association } = witness {
                    let a = need(&d.evidence.associations, *association)?;
                    values.push(
                        if a.intent == c1::Intent::Unknown
                            || a.basis != c1::AssociationBasis::ResolvedTarget
                        {
                            None
                        } else {
                            Some(a.intent == *intent)
                        },
                    );
                }
            }
            Ok(consensus(values))
        }
        Predicate::ScenarioCheck { check, status } => {
            let Context::Scenario { scenario, .. } = c else {
                return Ok(None);
            };
            if w.iter().any(|w|matches!(w,Witness::Association {association} if d.evidence.associations.get(*association).is_some_and(|a|a.basis!=c1::AssociationBasis::ResolvedTarget))){return Ok(None);}
            let s = need(&d.evidence.scenarios, *scenario)?;
            let observed = match check {
                CheckAxis::Parse => s.parse,
                CheckAxis::Binding => s.binding,
                CheckAxis::Environment => s.environment,
                CheckAxis::Execution => s.execution,
                CheckAxis::Extraction => s.extraction,
            };
            Ok(
                if observed == deployment::CheckStatus::NotRun
                    || observed == deployment::CheckStatus::Blocked
                {
                    None
                } else {
                    Some(observed == *status)
                },
            )
        }
        Predicate::SourceAlignment { exact } => {
            Ok(if *exact && matches!(c, Context::Source { .. }) {
                Some(true)
            } else {
                None
            })
        }
        Predicate::ReleaseVersion {
            distribution,
            version,
        } => {
            let Context::Release { release, .. } = c else {
                return Ok(None);
            };
            let release = need(&d.facts.releases, *release)?;
            let package = need(&d.facts.packages, release.package)?;
            Ok((package.name == *distribution).then_some(release.version == *version))
        }
        Predicate::DeploymentDeclaration { field, name } => {
            let expected = match field {
                DeploymentField::RequiresDist => "requires-dist",
                DeploymentField::ProvidesExtra => "provides-extra",
                DeploymentField::RequiresPython => "requires-python",
                DeploymentField::EntryPoint => "entry-point",
                DeploymentField::Launch => "launch",
                DeploymentField::Configuration => "configuration",
            };
            let mut values = Vec::new();
            for witness in w {
                if let Witness::Deployment { deployment } = witness {
                    let r = need(&d.evidence.deployments, *deployment)?;
                    let obs = need(&d.source.facts.deployment, r.observation)?;
                    values.push(
                        if obs.interpretation == deployment::CheckStatus::Passed
                            && obs.field == expected
                            && obs.name.as_deref() == Some(name)
                        {
                            Some(true)
                        } else {
                            None
                        },
                    );
                }
            }
            Ok(consensus(values))
        }
        Predicate::Relationship {
            role,
            target,
            fidelity,
        } => {
            let mut values = Vec::new();
            for witness in w {
                match witness {
                    Witness::Association { association } => {
                        let a = need(&d.evidence.associations, *association)?;
                        if a.basis != c1::AssociationBasis::ResolvedTarget {
                            values.push(None);
                            continue;
                        }
                        let match_target = match target {
                            RelationTarget::Member { member } => a.member == *member,
                            RelationTarget::Declaration { entity } => {
                                if d.source.core.refs.get(*entity).is_none() {
                                    values.push(None);
                                    continue;
                                }
                                need(&d.source.facts.alternatives, a.alternative)?.entity
                                    == Some(*entity)
                            }
                            RelationTarget::Original { .. } => {
                                values.push(None);
                                continue;
                            }
                        };
                        let match_role = match role {
                            RelationRole::Invokes => true,
                            RelationRole::Demonstrates => a.intent == c1::Intent::Demonstration,
                            RelationRole::TestsFailure => a.intent == c1::Intent::ExpectedFailure,
                            _ => {
                                values.push(None);
                                continue;
                            }
                        };
                        values.push(Some(
                            match_target && match_role && *fidelity == Fidelity::ResolvedTarget,
                        ));
                    }
                    Witness::Document { .. }
                    | Witness::FieldAccess { .. }
                    | Witness::ReceiverLocation { .. }
                    | Witness::ConstructorCandidate { .. } => values.push(None),
                    _ => {}
                }
            }
            Ok(consensus(values))
        }
        Predicate::FacetMembership { .. } | Predicate::SpecializedType { .. } => Ok(None),
    }
}
