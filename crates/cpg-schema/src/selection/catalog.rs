//! Typed catalog adapter for the pure classifier. Loading and publication are caller effects.
use super::*;
use crate::{catalog::*, evidence::*, table::table};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicableContext {
    pub context: SelectionContext,
    pub complete: bool,
    pub evidence: Vec<WitnessEvidence>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScopedDomain {
    pub domain: SelectionDomain,
    pub corpus_complete: bool,
    pub analyzer_complete: bool,
    pub contexts: Vec<ApplicableContext>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReleaseScope {
    pub release_id: Id,
    pub distributions: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DomainDetail {
    pub module: Option<String>,
    pub class_owner: Option<String>,
    pub releases: Vec<ReleaseScope>,
    pub domains: Vec<ScopedDomain>,
}
table!(
    /// Scoped declaration-domain closure, derived with source catalog inputs.
    CatalogSelectionDomains,CatalogSelectionDomainsRow="catalog_selection_domains",
    row_derives=[serde::Serialize,serde::Deserialize], family=Findings,
    key=[snapshot_id,member_id],checks=[],
    { #[serde(skip)] snapshot_id:Id, member_id:Id, domain_id:Id, detail:String }
);
/// A request-scoped projection of a validated published domain. `domain_id` still cites the
/// complete canonical record; `detail` contains only the requested domain kinds. It must never
/// be written back as a canonical row. Publication validates the full record before projection.
#[derive(Default, Clone, Deserialize)]
pub struct DomainInput {
    pub member_id: Id,
    pub domain_id: Id,
    pub detail: String,
}
impl TryFrom<CatalogSelectionDomainsRow> for DomainInput {
    type Error = WireError;
    fn try_from(row: CatalogSelectionDomainsRow) -> Result<Self, Self::Error> {
        canonical_domain(&row)?;
        Ok(Self {
            member_id: row.member_id,
            domain_id: row.domain_id,
            detail: row.detail,
        })
    }
}
fn canonical_domain(row: &CatalogSelectionDomainsRow) -> Result<DomainDetail, WireError> {
    let detail: DomainDetail = serde_json::from_str(&row.detail)?;
    validate_detail(row.member_id, &detail)?;
    if row.domain_id
        != crate::IdHasher::new("selection-domain-v1")
            .id(row.member_id)
            .str(&row.detail)
            .finish_id()
    {
        return Err(WireError("corrupt selection domain digest".into()));
    }
    if detail.domains.len() != 7
        || detail
            .domains
            .iter()
            .map(|d| d.domain)
            .collect::<BTreeSet<_>>()
            .len()
            != 7
    {
        return Err(WireError("corrupt selection domain inventory".into()));
    }
    Ok(detail)
}
/// Explicit typed inputs, shared by PG hydration and independently testable classifiers.
#[derive(Default, Clone)]
pub struct Inputs {
    pub facets: Vec<FacetInput>,
    pub facet_status: Vec<FacetStatusInput>,
    pub members: Vec<CatalogMembersRow>,
    pub domains: Vec<DomainInput>,
    pub bindings: Vec<CatalogBindingsRow>,
    pub signatures: Vec<CatalogSignaturesRow>,
    pub parameters: Vec<CatalogParametersRow>,
    pub types: Vec<CatalogTypesRow>,
    pub type_args: Vec<CatalogTypeArgsRow>,
    pub type_observations: Vec<CatalogTypeObservationsRow>,
    pub surfaces: Vec<CatalogSurfacesRow>,
    pub configurations: Vec<CatalogConfigurationsRow>,
    pub field_links: Vec<CatalogFieldLinksRow>,
    pub scenarios: Vec<ScenarioInput>,
    pub associations: Vec<AssociationInput>,
    pub deployments: Vec<DeploymentInput>,
    pub artifacts: Vec<ArtifactInput>,
    pub spans: Vec<SpanInput>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct FacetInput {
    pub node_id: Id,
    pub facet: String,
    pub value: String,
    pub verdict: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct FacetStatusInput {
    pub node_id: Id,
    pub facet: String,
    pub verdict: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioInput {
    pub scenario_id: Id,
    pub detail: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct DeploymentInput {
    pub deployment_id: Id,
    pub span_id: Id,
    pub detail: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct SpanInput {
    pub span_id: Id,
    pub artifact_id: Id,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ArtifactInput {
    pub artifact_id: Id,
    pub release_id: Id,
    pub alignment: Alignment,
}
#[derive(Debug, Clone, Deserialize)]
pub struct AssociationInput {
    pub association_id: Id,
    pub member_id: Option<Id>,
    pub release_id: Option<Id>,
    pub evidence_id: Id,
    pub evidence_kind: EvidenceKind,
    pub role: String,
    pub basis: String,
    pub support: String,
}

pub struct PreparedCatalog<'a> {
    inputs: &'a Inputs,
    member_bindings: BTreeMap<Id, Vec<&'a CatalogBindingsRow>>,
    surfaces: BTreeMap<Id, Vec<&'a CatalogSurfacesRow>>,
    fields: BTreeMap<Id, Vec<&'a CatalogConfigurationsRow>>,
    field_links: BTreeMap<Id, Vec<&'a CatalogFieldLinksRow>>,
    member_paths: BTreeMap<&'a str, Vec<&'a CatalogMembersRow>>,
    release_associations: BTreeMap<Id, Vec<&'a AssociationInput>>,
    spans: BTreeMap<Id, &'a SpanInput>,
    artifacts: BTreeMap<Id, &'a ArtifactInput>,
    deployment_sources: BTreeMap<Id, &'a DeploymentInput>,
    type_args: BTreeMap<Id, Vec<&'a CatalogTypeArgsRow>>,
    facets: BTreeMap<Id, Vec<&'a FacetInput>>,
    facet_status: BTreeMap<Id, Vec<&'a FacetStatusInput>>,
    domains: BTreeMap<Id, (Id, DomainDetail)>,
    signatures: BTreeMap<Id, &'a CatalogSignaturesRow>,
    parameters: BTreeMap<Id, Vec<&'a CatalogParametersRow>>,
    bindings: BTreeMap<Id, &'a CatalogBindingsRow>,
    types: BTreeMap<Id, &'a CatalogTypesRow>,
    observations: BTreeMap<Id, Vec<&'a CatalogTypeObservationsRow>>,
    associations: BTreeMap<Id, Vec<&'a AssociationInput>>,
    scenarios: BTreeMap<Id, ScenarioDetail>,
    deployments: BTreeMap<Id, DeploymentDetail>,
}
impl<'a> PreparedCatalog<'a> {
    pub fn new(inputs: &'a Inputs) -> Result<Self, WireError> {
        let counts = [
            inputs.members.len(),
            inputs.domains.len(),
            inputs.bindings.len(),
            inputs.signatures.len(),
            inputs.parameters.len(),
            inputs.types.len(),
            inputs.type_args.len(),
            inputs.type_observations.len(),
            inputs.surfaces.len(),
            inputs.configurations.len(),
            inputs.field_links.len(),
            inputs.scenarios.len(),
            inputs.associations.len(),
            inputs.deployments.len(),
            inputs.artifacts.len(),
            inputs.spans.len(),
            inputs.facets.len(),
            inputs.facet_status.len(),
        ];
        if counts.iter().any(|n| *n > 200_000)
            || counts.iter().sum::<usize>() > 2_000_000
            || inputs
                .domains
                .iter()
                .map(|r| r.detail.len())
                .chain(inputs.scenarios.iter().map(|r| r.detail.len()))
                .chain(inputs.deployments.iter().map(|r| r.detail.len()))
                .chain(inputs.associations.iter().map(|r| r.support.len()))
                .sum::<usize>()
                > 64 * 1024 * 1024
        {
            return Err(WireError("resource_refused: selection input budget".into()));
        }
        fn index<'a, T, K: Ord>(
            rows: &'a [T],
            key: impl Fn(&'a T) -> K,
        ) -> BTreeMap<K, Vec<&'a T>> {
            let mut out = BTreeMap::<K, Vec<_>>::new();
            for r in rows {
                out.entry(key(r)).or_default().push(r);
            }
            out
        }
        let mut domains = BTreeMap::new();
        for row in &inputs.domains {
            let detail: DomainDetail = serde_json::from_str(&row.detail)?;
            validate_detail(row.member_id, &detail)?;
            let names: BTreeSet<_> = detail.domains.iter().map(|d| d.domain).collect();
            if names.len() != detail.domains.len()
                || domains
                    .insert(row.member_id, (row.domain_id, detail))
                    .is_some()
            {
                return Err(WireError("corrupt selection domains".into()));
            }
        }
        if inputs.members.len() != domains.len()
            || inputs
                .members
                .iter()
                .map(|m| m.member_id)
                .collect::<BTreeSet<_>>()
                != domains.keys().copied().collect()
        {
            return Err(WireError("selection domain membership closure".into()));
        }
        if inputs
            .bindings
            .iter()
            .map(|b| b.binding_id)
            .collect::<BTreeSet<_>>()
            .len()
            != inputs.bindings.len()
            || inputs
                .signatures
                .iter()
                .map(|s| s.signature_id)
                .collect::<BTreeSet<_>>()
                .len()
                != inputs.signatures.len()
        {
            return Err(WireError("duplicate selection identity".into()));
        }
        let mut parameters: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for p in &inputs.parameters {
            parameters.entry(p.signature_id).or_default().push(p);
        }
        let mut observations: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for p in &inputs.type_observations {
            observations.entry(p.subject_node_id).or_default().push(p);
        }
        let mut associations: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for a in &inputs.associations {
            if let Some(m) = a.member_id {
                associations.entry(m).or_default().push(a);
            }
        }
        Ok(Self {
            inputs,
            domains,
            parameters,
            observations,
            associations,
            member_bindings: index(&inputs.bindings, |r| r.member_id),
            surfaces: index(&inputs.surfaces, |r| r.declaration_node_id),
            fields: index(&inputs.configurations, |r| r.class_node_id),
            field_links: index(&inputs.field_links, |r| r.field_id),
            member_paths: index(&inputs.members, |r| r.access_path.as_str()),
            release_associations: {
                let mut m = BTreeMap::<_, Vec<_>>::new();
                for a in &inputs.associations {
                    if let Some(id) = a.release_id {
                        m.entry(id).or_default().push(a);
                    }
                }
                m
            },
            spans: inputs.spans.iter().map(|s| (s.span_id, s)).collect(),
            artifacts: inputs
                .artifacts
                .iter()
                .map(|s| (s.artifact_id, s))
                .collect(),
            deployment_sources: inputs
                .deployments
                .iter()
                .map(|s| (s.deployment_id, s))
                .collect(),
            type_args: index(&inputs.type_args, |r| r.parent_term_id),
            facets: index(&inputs.facets, |r| r.node_id),
            facet_status: index(&inputs.facet_status, |r| r.node_id),
            signatures: inputs
                .signatures
                .iter()
                .map(|s| (s.signature_id, s))
                .collect(),
            bindings: inputs.bindings.iter().map(|b| (b.binding_id, b)).collect(),
            types: inputs.types.iter().map(|t| (t.term_id, t)).collect(),
            scenarios: inputs
                .scenarios
                .iter()
                .map(|s| Ok((s.scenario_id, serde_json::from_str(&s.detail)?)))
                .collect::<Result<_, serde_json::Error>>()?,
            deployments: inputs
                .deployments
                .iter()
                .map(|d| Ok((d.deployment_id, serde_json::from_str(&d.detail)?)))
                .collect::<Result<_, serde_json::Error>>()?,
        })
    }
    pub fn classify(&self, selection: &Selection) -> Result<Vec<CandidateSelection>, WireError> {
        if selection.requirements.len() > 16 {
            return Err(WireError("at most sixteen requirements".into()));
        }
        let required: BTreeSet<_> = selection
            .requirements
            .iter()
            .filter_map(Requirement::input_domain)
            .collect();
        if self.domains.values().any(|(_, detail)| {
            let loaded: BTreeSet<_> = detail.domains.iter().map(|d| d.domain).collect();
            !required.is_subset(&loaded)
        }) {
            return Err(WireError("selection domain was not loaded".into()));
        }
        self.validate_operands(selection)?;
        let mut results = Vec::new();
        let mut work = 0usize;
        let mut bytes = 0usize;
        for member in &self.inputs.members {
            let mut terms = Vec::new();
            for requirement in &selection.requirements {
                // Charge the indexed visits before traversing a domain or constructing witnesses.
                work = work.saturating_add(self.visit_cost(member, requirement));
                if work > 2_000_000 {
                    return Err(WireError(
                        "resource_refused: aggregate selection work".into(),
                    ));
                }
                let domain = self.evaluate(member, requirement)?;
                work = work.saturating_add(
                    domain
                        .observations
                        .iter()
                        .map(|o| 1 + o.evidence.len() + o.admissible.len())
                        .sum::<usize>(),
                );
                if work > 2_000_000 {
                    return Err(WireError(
                        "resource_refused: aggregate selection work".into(),
                    ));
                }
                terms.push(super::classify(requirement, &domain)?);
            }
            let joint = super::joint(&terms, selection.joint);
            let outcome = aggregate(&terms, joint, selection.joint);
            let candidate = CandidateSelection {
                member_id: PublicMemberId::from_storage(member.member_id),
                operation_id: member.operation_node_id.map(OperationId::from_storage),
                access_path: member.access_path.clone(),
                kind: member.kind.clone(),
                outcome,
                requirements: terms.into_iter().map(|t| t.result).collect(),
                joint,
                ranking: None,
            };
            bytes = bytes.saturating_add(serde_json::to_vec(&candidate)?.len());
            if bytes > 64 * 1024 * 1024 {
                return Err(WireError(
                    "resource_refused: aggregate selection evidence".into(),
                ));
            }
            results.push(candidate);
        }
        results.sort_by_key(|r| r.member_id);
        Ok(results)
    }
    fn validate_operands(&self, selection: &Selection) -> Result<(), WireError> {
        let needs_nodes = selection.requirements.iter().any(|r| {
            matches!(
                r,
                Requirement::ConfigurationRelationship {
                    target: FieldTarget::Node { .. },
                    ..
                } | Requirement::Relationship {
                    target: RelationTarget::Declaration { .. },
                    ..
                }
            )
        });
        let mut nodes = BTreeSet::new();
        if needs_nodes {
            nodes.extend(
                self.inputs
                    .bindings
                    .iter()
                    .filter_map(|b| b.declaration_node_id),
            );
            nodes.extend(self.inputs.signatures.iter().map(|s| s.callable_node_id));
            nodes.extend(
                self.inputs
                    .parameters
                    .iter()
                    .filter_map(|p| p.formal_node_id),
            );
            for link in &self.inputs.field_links {
                nodes.extend(link.reader_node_id);
                nodes.extend(link.formal_node_id);
                nodes.insert(link.class_node_id);
            }
            for a in &self.inputs.associations {
                let supports: Vec<AssociationSupport> = serde_json::from_str(&a.support)?;
                nodes.extend(supports.into_iter().map(|s| s.target_id));
            }
        }
        for requirement in &selection.requirements {
            let present = match requirement {
                Requirement::ParameterType {
                    r#type:
                        StructuralType::CanonicalTerm { term }
                        | StructuralType::DeclaredUnionMember { term },
                    ..
                } => self.types.contains_key(&term.storage()),
                Requirement::ConfigurationRelationship {
                    target: FieldTarget::Node { node },
                    ..
                }
                | Requirement::Relationship {
                    target: RelationTarget::Declaration { node },
                    ..
                } => nodes.contains(node),
                Requirement::ConfigurationRelationship {
                    target: FieldTarget::Parameter { signature, ordinal },
                    ..
                } => self
                    .parameters
                    .get(&signature.storage())
                    .is_some_and(|ps| ps.iter().any(|p| p.ordinal == i64::from(*ordinal))),
                Requirement::Relationship {
                    target: RelationTarget::Member { member },
                    ..
                } => self.domains.contains_key(&member.storage()),
                Requirement::Relationship {
                    target: RelationTarget::Evidence { evidence },
                    ..
                } => match evidence {
                    EvidenceRef::Span(id) => self.spans.contains_key(&id.storage()),
                    EvidenceRef::Scenario(id) => self.scenarios.contains_key(&id.storage()),
                    EvidenceRef::Deployment(id) => self.deployments.contains_key(&id.storage()),
                },
                _ => true,
            };
            if !present {
                return Err(WireError(
                    "selection operand is absent from the pinned catalog generation".into(),
                ));
            }
        }
        Ok(())
    }
    fn member_kind(&self, member: &CatalogMembersRow, detail: &DomainDetail) -> Option<String> {
        if member.kind == "unknown" {
            return None;
        }
        if matches!(
            member.kind.as_str(),
            "function" | "async_function" | "method"
        ) {
            let property = self
                .member_bindings
                .get(&member.member_id)
                .into_iter()
                .flatten()
                .filter(|b| {
                    !matches!(
                        b.role.as_str(),
                        "shadowed_source" | "provider_public_observation"
                    )
                })
                .filter_map(|b| b.declaration_node_id)
                .flat_map(|id| self.surfaces.get(&id).into_iter().flatten())
                .any(|s| s.binding_mode.as_deref() == Some("property"));
            return Some(
                if property {
                    "property"
                } else if detail.class_owner.is_some() || member.kind == "method" {
                    "method"
                } else {
                    "function"
                }
                .into(),
            );
        }
        Some(member.kind.clone())
    }
    fn visit_cost(&self, member: &CatalogMembersRow, requirement: &Requirement) -> usize {
        let mut cost = 1usize;
        let mut charge = |n: usize| {
            cost = cost.saturating_add(n);
        };
        let bindings = self
            .member_bindings
            .get(&member.member_id)
            .map_or(&[][..], Vec::as_slice);
        for b in bindings {
            charge(
                1 + b
                    .declaration_node_id
                    .and_then(|n| self.surfaces.get(&n))
                    .map_or(0, Vec::len),
            );
        }
        if let Some(id) = member.operation_node_id {
            charge(
                self.facets.get(&id).map_or(0, Vec::len)
                    + self.facet_status.get(&id).map_or(0, Vec::len),
            );
        }
        for a in self
            .associations
            .get(&member.member_id)
            .into_iter()
            .flatten()
        {
            charge(1 + a.support.len());
        }
        if let Requirement::ConfigurationOwner { path, .. } = requirement {
            for m in self.member_paths.get(path.as_str()).into_iter().flatten() {
                charge(1 + self.member_bindings.get(&m.member_id).map_or(0, Vec::len));
            }
        }
        for c in self.domains[&member.member_id]
            .1
            .domains
            .iter()
            .filter(|d| Some(d.domain) == requirement.input_domain())
            .flat_map(|d| &d.contexts)
        {
            charge(1 + c.evidence.len());
            match &c.context {
                SelectionContext::Signature { signature, .. } => {
                    for p in self
                        .parameters
                        .get(&signature.storage())
                        .into_iter()
                        .flatten()
                    {
                        charge(1);
                        for o in p
                            .formal_node_id
                            .and_then(|id| self.observations.get(&id))
                            .into_iter()
                            .flatten()
                        {
                            charge(1 + self.type_args.get(&o.term_id).map_or(0, Vec::len));
                        }
                    }
                }
                SelectionContext::Configuration { owner, .. } => {
                    charge(bindings.len());
                    for f in self.fields.get(owner).into_iter().flatten() {
                        charge(
                            1 + self.field_links.get(&f.field_id).map_or(0, Vec::len)
                                + self.type_args.get(&f.term_id).map_or(0, Vec::len),
                        );
                    }
                }
                SelectionContext::Release { release } => {
                    charge(self.release_associations.get(release).map_or(0, Vec::len))
                }
                _ => {}
            }
        }
        cost
    }
    fn evaluate(
        &self,
        member: &CatalogMembersRow,
        requirement: &Requirement,
    ) -> Result<Domain, WireError> {
        if let Requirement::FacetMembership { facet, value, .. } = requirement {
            let status = member
                .operation_node_id
                .and_then(|id| self.facet_status.get(&id))
                .into_iter()
                .flatten()
                .find(|s| s.facet == facet.as_str());
            let matching: Vec<_> = member
                .operation_node_id
                .and_then(|id| self.facets.get(&id))
                .into_iter()
                .flatten()
                .filter(|f| f.facet == facet.as_str() && f.value == value.as_str())
                .collect();
            let positive = matching
                .iter()
                .any(|f| matches!(f.verdict.as_str(), "established" | "conditional"));
            let closed = status.is_some_and(|s| s.verdict == "established");
            let closure = status
                .map(|s| WitnessEvidence::FacetCoverage {
                    facet: s.facet.clone(),
                    verdict: s.verdict.clone(),
                })
                .into_iter()
                .collect::<Vec<_>>();
            let mut evidence = matching
                .iter()
                .map(|f| WitnessEvidence::Facet {
                    facet: f.facet.clone(),
                    value: f.value.clone(),
                    verdict: f.verdict.clone(),
                })
                .collect::<Vec<_>>();
            evidence.extend(closure.clone());
            return Ok(Domain {
                corpus_complete: closed,
                analyzer_complete: closed,
                closure,
                observations: vec![Observation {
                    context: SelectionContext::Member {
                        member: PublicMemberId::from_storage(member.member_id),
                    },
                    basis: EvidenceBasis::BoundedModel,
                    value: if positive {
                        Some(true)
                    } else if matching.is_empty() && closed {
                        Some(false)
                    } else {
                        None
                    },
                    evidence,
                    admissible: vec![],
                }],
            });
        }
        if let Requirement::DeploymentDeclaration {
            field: field @ (DeploymentField::Launch | DeploymentField::Configuration),
            name,
            ..
        } = requirement
        {
            let mut observations = Vec::new();
            for a in self
                .associations
                .get(&member.member_id)
                .into_iter()
                .flatten()
                .filter(|a| a.evidence_kind == EvidenceKind::Deployment)
            {
                let d = self
                    .deployments
                    .get(&a.evidence_id)
                    .ok_or_else(|| WireError("deployment missing".into()))?;
                let source = self
                    .deployment_sources
                    .get(&a.evidence_id)
                    .ok_or_else(|| WireError("deployment span missing".into()))?;
                observations.push(Observation {
                    context: SelectionContext::Source {
                        member: PublicMemberId::from_storage(member.member_id),
                        span: SpanId::from_storage(source.span_id),
                    },
                    basis: EvidenceBasis::SourceDeclaration,
                    value: deployment_matches(d, *field, name.as_str()).then_some(true),
                    evidence: vec![WitnessEvidence::Original {
                        evidence: EvidenceRef::Deployment(DeploymentId::from_storage(
                            a.evidence_id,
                        )),
                    }],
                    admissible: vec![],
                });
            }
            return Ok(Domain {
                corpus_complete: false,
                analyzer_complete: false,
                closure: vec![],
                observations,
            });
        }
        let (domain_id, detail) = self
            .domains
            .get(&member.member_id)
            .ok_or_else(|| WireError("missing canonical selection domain".into()))?;
        let scope = detail
            .domains
            .iter()
            .find(|d| d.domain == requirement.domain())
            .ok_or_else(|| WireError("missing domain kind".into()))?;
        let mut domain = Domain {
            corpus_complete: scope.corpus_complete,
            analyzer_complete: scope.analyzer_complete,
            closure: vec![WitnessEvidence::Domain { id: *domain_id }],
            observations: vec![],
        };
        let mut add = |context: SelectionContext,
                       basis: EvidenceBasis,
                       value: Option<bool>,
                       evidence: Vec<WitnessEvidence>,
                       admissible: Vec<SelectionContext>| {
            domain.observations.push(Observation {
                context,
                basis,
                value,
                evidence,
                admissible,
            });
        };
        for scoped in &scope.contexts {
            let context = &scoped.context;
            let mut evidence = scoped.evidence.clone();
            evidence.push(WitnessEvidence::Domain { id: *domain_id });
            let mut value = None;
            let mut basis = EvidenceBasis::SourceDeclaration;
            match context {
                SelectionContext::Member { .. } => {
                    value = match requirement {
                        Requirement::PublicPath { path, .. } => {
                            Some(member.access_path == path.as_str())
                        }
                        Requirement::PublicModule { module, .. } => {
                            detail.module.as_ref().map(|m| m == module.as_str())
                        }
                        Requirement::ClassOwner { path, .. } => {
                            detail.class_owner.as_ref().map(|m| m == path.as_str())
                        }
                        Requirement::MemberKind { kind, .. } => self
                            .member_kind(member, detail)
                            .map(|actual| actual == word(kind)),
                        Requirement::InvocationForm { form, .. } => {
                            let bindings = self
                                .member_bindings
                                .get(&member.member_id)
                                .into_iter()
                                .flatten()
                                .filter(|b| {
                                    !matches!(
                                        b.role.as_str(),
                                        "shadowed_source" | "provider_public_observation"
                                    )
                                })
                                .collect::<Vec<_>>();
                            if bindings.is_empty() {
                                add(context.clone(), basis, None, evidence.clone(), vec![]);
                            }
                            for b in bindings {
                                let surfaces: Vec<_> = b
                                    .declaration_node_id
                                    .and_then(|id| self.surfaces.get(&id))
                                    .into_iter()
                                    .flatten()
                                    .collect();
                                let known = b.declaration_node_id.is_some()
                                    && surfaces.iter().all(|s| {
                                        s.admission == "body_preserved"
                                            || s.binding_mode.is_some()
                                            || s.protocol.is_some()
                                    });
                                let mut modes = BTreeSet::new();
                                let mut refs = evidence.clone();
                                refs.push(WitnessEvidence::Fact {
                                    id: b.source_fact_id,
                                });
                                for surface in surfaces {
                                    modes.extend(surface.binding_mode.clone());
                                    modes.extend(surface.protocol.clone());
                                    refs.push(WitnessEvidence::Fact {
                                        id: surface.source_fact_id,
                                    });
                                }
                                if modes.is_empty()
                                    && known
                                    && let Some(kind) = self.member_kind(member, detail)
                                {
                                    modes.insert(kind);
                                }
                                let context = SelectionContext::Binding {
                                    member: PublicMemberId::from_storage(member.member_id),
                                    binding: BindingId::from_storage(b.binding_id),
                                };
                                add(
                                    context.clone(),
                                    basis,
                                    known.then(|| modes.contains(&word(form))),
                                    refs,
                                    vec![context],
                                );
                            }
                            continue;
                        }
                        _ => None,
                    };
                }
                SelectionContext::Signature {
                    signature, binding, ..
                } => {
                    let sig = self
                        .signatures
                        .get(&signature.storage())
                        .ok_or_else(|| WireError("domain signature missing".into()))?;
                    let b = self
                        .bindings
                        .get(&binding.storage())
                        .ok_or_else(|| WireError("domain binding missing".into()))?;
                    if b.member_id != member.member_id {
                        return Err(WireError("foreign domain binding".into()));
                    }
                    if sig.role == "provider_constructor" {
                        basis = EvidenceBasis::ProviderDeclaration;
                    }
                    let name = match requirement {
                        Requirement::DeclaresParameter { name, .. }
                        | Requirement::ParameterKind { name, .. }
                        | Requirement::ParameterRequired { name, .. }
                        | Requirement::ParameterDefaultState { name, .. }
                        | Requirement::ParameterDefault { name, .. }
                        | Requirement::ParameterType { name, .. } => Some(name.as_str()),
                        _ => None,
                    };
                    let parameters = self
                        .parameters
                        .get(&signature.storage())
                        .map_or(&[][..], Vec::as_slice);
                    if let Some(name) = name {
                        let p = parameters.iter().find(|p| p.name.as_deref() == Some(name));
                        if let Some(p) = p {
                            evidence.extend(
                                p.syntax_fact_id
                                    .into_iter()
                                    .chain(p.semantics_fact_id)
                                    .map(|id| WitnessEvidence::Fact { id }),
                            );
                            let matcher = match requirement {
                                Requirement::ParameterType { r#type, .. } => Some(r#type.clone()),
                                _ => None,
                            };
                            if let Some(matcher) = matcher {
                                let observations: Vec<_> = p
                                    .formal_node_id
                                    .and_then(|id| self.observations.get(&id))
                                    .into_iter()
                                    .flatten()
                                    .filter(|o| o.declared)
                                    .collect();
                                if observations.is_empty() {
                                    add(
                                        context.clone(),
                                        EvidenceBasis::ProviderDeclaration,
                                        None,
                                        evidence.clone(),
                                        vec![],
                                    );
                                }
                                for observation in observations {
                                    let mut refs = evidence.clone();
                                    refs.push(WitnessEvidence::Fact {
                                        id: observation.source_fact_id,
                                    });
                                    add(
                                        context.clone(),
                                        EvidenceBasis::ProviderDeclaration,
                                        self.type_match(observation.term_id, &matcher),
                                        refs,
                                        vec![context.clone()],
                                    );
                                }
                                continue;
                            }
                            value = match requirement {
                                Requirement::DeclaresParameter { .. } => Some(true),
                                Requirement::ParameterKind { kind, .. } => {
                                    p.kind.as_ref().map(|k| k == &parameter_kind(*kind))
                                }
                                Requirement::ParameterRequired { required, .. } => {
                                    p.required.map(|r| r == *required)
                                }
                                Requirement::ParameterDefaultState { state, .. } => {
                                    Some(p.default_state == word(state))
                                }
                                Requirement::ParameterDefault { value, .. } => {
                                    primitive_match(p.literal_json.as_deref(), value)
                                }
                                Requirement::ParameterType { .. } => None,
                                _ => None,
                            };
                        } else if scoped.complete {
                            value = Some(false);
                        }
                    }
                }
                SelectionContext::Configuration { owner, scope, .. } => {
                    let declarations: BTreeSet<_> = self
                        .member_bindings
                        .get(&member.member_id)
                        .into_iter()
                        .flatten()
                        .filter(|b| {
                            !matches!(
                                b.role.as_str(),
                                "shadowed_source" | "provider_public_observation"
                            )
                        })
                        .filter_map(|b| b.declaration_node_id)
                        .collect();
                    let fields: Vec<_> = self
                        .fields
                        .get(owner)
                        .into_iter()
                        .flatten()
                        .filter(|f| {
                            declarations.contains(owner)
                                || self.field_links.get(&f.field_id).into_iter().flatten().any(
                                    |l| match scope {
                                        ConfigurationScopeKind::Object => l
                                            .reader_node_id
                                            .is_some_and(|n| declarations.contains(&n)),
                                        ConfigurationScopeKind::PerCall => {
                                            self.signatures.get(&l.signature_id).is_some_and(|s| {
                                                declarations.contains(&s.callable_node_id)
                                            })
                                        }
                                    },
                                )
                        })
                        .collect();
                    let name = match requirement {
                        Requirement::DeclaresConfigurationField { name, .. }
                        | Requirement::ConfigurationDefault { name, .. }
                        | Requirement::ConfigurationLiteral { name, .. }
                        | Requirement::ConfigurationRelationship { name, .. } => {
                            Some(name.as_str())
                        }
                        _ => None,
                    };
                    if let Some(name) = name {
                        let matching: Vec<_> = fields
                            .iter()
                            .filter(|f| f.name == name || f.alias.as_deref() == Some(name))
                            .collect();
                        for field in matching {
                            let mut refs = evidence.clone();
                            refs.push(WitnessEvidence::Fact {
                                id: field.source_fact_id,
                            });
                            let v = match requirement {
                                Requirement::DeclaresConfigurationField { .. } => Some(true),
                                Requirement::ConfigurationDefault { value, .. } => {
                                    primitive_match(field.literal_json.as_deref(), value)
                                }
                                Requirement::ConfigurationLiteral { value, .. } => {
                                    self.literal_domain(field.term_id, value)
                                }
                                Requirement::ConfigurationRelationship { kind, target, .. } => {
                                    let found = self
                                        .field_links
                                        .get(&field.field_id)
                                        .into_iter()
                                        .flatten()
                                        .filter(|l| l.kind == word(kind))
                                        .find(|l| match (kind, target) {
                                            (
                                                FieldRelationship::ExactReader,
                                                FieldTarget::Node { node },
                                            ) => l.reader_node_id == Some(*node),
                                            (
                                                FieldRelationship::DeclaredParameter
                                                | FieldRelationship::ExactStorage,
                                                FieldTarget::Node { node },
                                            ) => l.formal_node_id == Some(*node),
                                            (
                                                FieldRelationship::DeclaredParameter
                                                | FieldRelationship::ExactStorage,
                                                FieldTarget::Parameter { signature, ordinal },
                                            ) => {
                                                l.signature_id == signature.storage()
                                                    && l.ordinal == i64::from(*ordinal)
                                            }
                                            _ => false,
                                        });
                                    if let Some(l) = found {
                                        refs.push(WitnessEvidence::Fact {
                                            id: l.source_fact_id,
                                        });
                                        refs.push(WitnessEvidence::FieldLink {
                                            id: l.link_id,
                                            field: l.field_id,
                                            signature: Some(l.signature_id),
                                            ordinal: l.ordinal.try_into().map_err(|_| {
                                                WireError("field-link ordinal".into())
                                            })?,
                                            formal: l.formal_node_id,
                                            reader: l.reader_node_id,
                                            relationship: *kind,
                                        });
                                        Some(true)
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            };
                            add(context.clone(), basis, v, refs, vec![context.clone()]);
                        }
                        // Empty observed field sets do not establish absence without scope closure.
                        continue;
                    }
                    value = match requirement {
                        Requirement::ConfigurationOwner { path, .. } => Some(
                            self.member_paths
                                .get(path.as_str())
                                .into_iter()
                                .flatten()
                                .any(|m| {
                                    self.member_bindings
                                        .get(&m.member_id)
                                        .into_iter()
                                        .flatten()
                                        .any(|b| {
                                            !matches!(
                                                b.role.as_str(),
                                                "shadowed_source" | "provider_public_observation"
                                            ) && b.declaration_node_id == Some(*owner)
                                        })
                                }),
                        ),
                        Requirement::ConfigurationScope {
                            scope: expected, ..
                        } => Some(scope == expected),
                        Requirement::ConfigurationRecordKind { model, .. } => {
                            Some(fields.iter().any(|f| f.record_kind == model.as_str()))
                        }
                        _ => None,
                    };
                }
                SelectionContext::Scenario { scenario, .. } => {
                    let s = self
                        .scenarios
                        .get(&scenario.storage())
                        .ok_or_else(|| WireError("domain scenario missing".into()))?;
                    evidence.push(WitnessEvidence::Original {
                        evidence: EvidenceRef::Scenario(*scenario),
                    });
                    value = match requirement {
                        Requirement::ScenarioIntent { intent, .. } => Some(s.intent == *intent),
                        Requirement::ScenarioCheck { check, status, .. } => Some(
                            match check {
                                ScenarioCheck::Parse => &s.checks.parse,
                                ScenarioCheck::Binding => &s.checks.binding,
                                ScenarioCheck::Environment => &s.checks.environment,
                                ScenarioCheck::Execution => &s.checks.execution,
                            } == status,
                        ),
                        _ => None,
                    };
                }
                SelectionContext::Release { release } => match requirement {
                    Requirement::ReleaseVersion {
                        distribution,
                        version,
                        ..
                    } => {
                        value = detail
                            .releases
                            .iter()
                            .find(|r| r.release_id == *release)
                            .and_then(|r| {
                                r.distributions.iter().find_map(|d| {
                                    d.rsplit_once("==")
                                        .filter(|(name, _)| *name == distribution.as_str())
                                        .map(|(_, v)| v == version.as_str())
                                })
                            });
                    }
                    Requirement::DeploymentDeclaration { field, name, .. } => {
                        for a in self
                            .release_associations
                            .get(release)
                            .into_iter()
                            .flatten()
                            .filter(|a| a.evidence_kind == EvidenceKind::Deployment)
                        {
                            let d = self.deployments.get(&a.evidence_id).ok_or_else(|| {
                                WireError("deployment association missing".into())
                            })?;
                            if deployment_matches(d, *field, name.as_str()) {
                                evidence.push(WitnessEvidence::Original {
                                    evidence: EvidenceRef::Deployment(DeploymentId::from_storage(
                                        a.evidence_id,
                                    )),
                                });
                                value = Some(true);
                            }
                        }
                    }
                    _ => {}
                },
                SelectionContext::Source { span, .. } => {
                    if let Requirement::SourceAlignment { alignment, .. } = requirement {
                        let source = self
                            .spans
                            .get(&span.storage())
                            .ok_or_else(|| WireError("source scope span missing".into()))?;
                        let artifact = self
                            .artifacts
                            .get(&source.artifact_id)
                            .ok_or_else(|| WireError("source scope artifact missing".into()))?;
                        value = Some(artifact.alignment == *alignment);
                        evidence.push(WitnessEvidence::Original {
                            evidence: EvidenceRef::Span(*span),
                        });
                    }
                }
                SelectionContext::Binding { .. } | SelectionContext::Runtime { .. } => {}
            }
            add(
                context.clone(),
                basis,
                value,
                evidence,
                vec![context.clone()],
            );
        }
        if let Requirement::Relationship {
            role,
            target,
            fidelity,
            ..
        } = requirement
        {
            for a in self
                .associations
                .get(&member.member_id)
                .into_iter()
                .flatten()
            {
                let supports: Vec<AssociationSupport> = serde_json::from_str(&a.support)?;
                if a.role != word(role) || a.basis != word(fidelity) {
                    continue;
                }
                let evidence = EvidenceRef::new(a.evidence_kind.clone(), a.evidence_id);
                let mut refs = vec![WitnessEvidence::Original {
                    evidence: evidence.clone(),
                }];
                let found = match target {
                    RelationTarget::Declaration { node } => {
                        for support in supports.iter().filter(|s| s.target_id == *node) {
                            refs.push(WitnessEvidence::Association {
                                id: a.association_id,
                                role: *role,
                                fidelity: *fidelity,
                                target: target.clone(),
                                edge: Some(support.edge_id),
                                modality: Some(support.modality.clone()),
                                phase: Some(support.phase.clone()),
                            });
                            refs.push(WitnessEvidence::Fact {
                                id: support.fact_id,
                            });
                        }
                        refs.len() > 1
                    }
                    RelationTarget::Member { member: target } => {
                        a.member_id == Some(target.storage())
                    }
                    RelationTarget::Evidence { evidence: target } => target == &evidence,
                };
                if found {
                    if refs.len() == 1 {
                        refs.push(WitnessEvidence::Association {
                            id: a.association_id,
                            role: *role,
                            fidelity: *fidelity,
                            target: target.clone(),
                            edge: None,
                            modality: None,
                            phase: None,
                        });
                    }
                    add(
                        SelectionContext::Member {
                            member: PublicMemberId::from_storage(member.member_id),
                        },
                        if supports.is_empty() {
                            EvidenceBasis::SourceDeclaration
                        } else {
                            EvidenceBasis::ProviderDeclaration
                        },
                        Some(true),
                        refs,
                        vec![],
                    );
                }
            }
        }
        Ok(domain)
    }
    fn type_match(&self, term: Id, matcher: &StructuralType) -> Option<bool> {
        let t = self.types.get(&term)?;
        match matcher {
            StructuralType::CanonicalTerm { term: expected } => Some(term == expected.storage()),
            StructuralType::Category { category } => Some(t.kind == category.as_str()),
            StructuralType::NominalIdentity { module, name } => {
                match (&t.class_module, &t.class_key) {
                    (Some(m), Some(k)) => Some(m == module.as_str() && k == name.as_str()),
                    _ => None,
                }
            }
            StructuralType::DeclaredUnionMember { term: expected } => {
                if t.kind != "union" {
                    return Some(false);
                }
                Some(
                    self.type_args
                        .get(&term)
                        .into_iter()
                        .flatten()
                        .any(|a| a.child_term_id == expected.storage()),
                )
            }
        }
    }
    fn literal_domain(&self, term: Id, value: &ExactPrimitive) -> Option<bool> {
        let t = self.types.get(&term)?;
        if t.kind == "literal" {
            return primitive_match(t.literal_json.as_deref(), value);
        }
        if t.kind != "union" {
            return None;
        }
        let values: Vec<_> = self
            .type_args
            .get(&term)
            .into_iter()
            .flatten()
            .map(|a| {
                self.types
                    .get(&a.child_term_id)
                    .filter(|t| t.kind == "literal")
                    .and_then(|t| primitive_match(t.literal_json.as_deref(), value))
            })
            .collect();
        if values.contains(&Some(true)) {
            Some(true)
        } else if !values.is_empty() && values.iter().all(|v| *v == Some(false)) {
            Some(false)
        } else {
            None
        }
    }
}
fn primitive_match(raw: Option<&str>, expected: &ExactPrimitive) -> Option<bool> {
    let actual: serde_json::Value = serde_json::from_str(raw?).ok()?;
    let expected = match expected {
        ExactPrimitive::None(()) => serde_json::Value::Null,
        ExactPrimitive::Bool(v) => (*v).into(),
        ExactPrimitive::Int(v) => (*v).into(),
        ExactPrimitive::Str(v) => v.as_str().into(),
    };
    Some(actual == expected)
}
fn word<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .expect("finite enum")
        .as_str()
        .expect("finite enum string")
        .into()
}
fn parameter_kind(v: ParameterMatchKind) -> String {
    match v {
        ParameterMatchKind::PositionalOnly => "positional_only",
        ParameterMatchKind::Positional => "positional_or_keyword",
        ParameterMatchKind::VarArgs => "var_positional",
        ParameterMatchKind::KeywordOnly => "keyword_only",
        ParameterMatchKind::VarKwargs => "var_keyword",
    }
    .into()
}
/// Operands are exact normalized package/extra/entry-point names, a Python constraint,
/// a JSON pointer, or the original launch text. Interpretation must have succeeded.
fn deployment_matches(d: &DeploymentDetail, field: DeploymentField, operand: &str) -> bool {
    if field == DeploymentField::Launch {
        return matches!(
            d.field.as_str(),
            "original_bash_snippet"
                | "original_sh_snippet"
                | "original_shell_snippet"
                | "original_console_snippet"
        ) && d.original == operand;
    }
    if d.interpretation != crate::evidence::CheckStatus::Passed {
        return false;
    }
    match field {
        DeploymentField::RequiresDist => {
            d.field == "requires-dist" && d.name.as_deref() == Some(operand)
        }
        DeploymentField::ProvidesExtra => {
            d.field == "provides-extra" && d.name.as_deref() == Some(operand)
        }
        DeploymentField::RequiresPython => {
            d.field == "requires-python" && d.constraint.as_deref() == Some(operand)
        }
        DeploymentField::EntryPoint => {
            d.field.starts_with("entry-point:") && d.name.as_deref() == Some(operand)
        }
        DeploymentField::Launch => d.field == "launch" && d.original == operand,
        DeploymentField::Configuration => d.field.strip_prefix("configuration:") == Some(operand),
    }
}

fn validate_detail(member: Id, detail: &DomainDetail) -> Result<(), WireError> {
    for scope in &detail.domains {
        if scope
            .contexts
            .iter()
            .map(|c| &c.context)
            .collect::<BTreeSet<_>>()
            .len()
            != scope.contexts.len()
        {
            return Err(WireError("duplicate selection context".into()));
        }
        if scope.corpus_complete
            && scope.analyzer_complete
            && scope.contexts.iter().any(|c| !c.complete)
        {
            return Err(WireError(
                "selection closure over incomplete context".into(),
            ));
        }
        for c in &scope.contexts {
            if super::member(&c.context).is_some_and(|m| m.storage() != member) {
                return Err(WireError("foreign selection member".into()));
            }
            let valid = matches!(
                (scope.domain, &c.context),
                (
                    SelectionDomain::PublicExposures,
                    SelectionContext::Member { .. }
                ) | (
                    SelectionDomain::SignatureVariants,
                    SelectionContext::Signature { .. }
                ) | (
                    SelectionDomain::ConfigurationFields,
                    SelectionContext::Configuration { .. }
                ) | (
                    SelectionDomain::Scenarios,
                    SelectionContext::Scenario { .. }
                ) | (
                    SelectionDomain::SourceArtifacts,
                    SelectionContext::Source { .. }
                ) | (
                    SelectionDomain::ReleaseDeclarations,
                    SelectionContext::Release { .. }
                )
            );
            if !valid {
                return Err(WireError(
                    "context does not belong to selection domain".into(),
                ));
            }
            if c.complete && c.evidence.is_empty() {
                return Err(WireError("unattributed closure".into()));
            }
        }
    }
    Ok(())
}

/// Follow only attributed member associations to the original spans they actually contain.
pub fn associated_sources(
    associations: &[CatalogAssociationsRow],
    scenarios: &[CatalogScenariosRow],
    deployments: &[CatalogDeploymentsRow],
) -> Result<BTreeMap<(Id, Id), BTreeSet<EvidenceRef>>, WireError> {
    let scenarios: BTreeMap<_, ScenarioDetail> = scenarios
        .iter()
        .map(|s| Ok((s.scenario_id, serde_json::from_str(&s.detail)?)))
        .collect::<Result<_, serde_json::Error>>()?;
    let deployments: BTreeMap<_, _> = deployments
        .iter()
        .map(|d| (d.deployment_id, d.span_id))
        .collect();
    let mut sources = BTreeMap::<_, BTreeSet<_>>::new();
    for a in associations {
        let Some(member) = a.member_id else { continue };
        let (reference, spans) = match a.evidence_kind.as_str() {
            "span" => (
                EvidenceRef::Span(SpanId::from_storage(a.evidence_id)),
                vec![a.evidence_id],
            ),
            "scenario" => (
                EvidenceRef::Scenario(ScenarioId::from_storage(a.evidence_id)),
                scenarios
                    .get(&a.evidence_id)
                    .ok_or_else(|| WireError("associated scenario missing".into()))?
                    .spans
                    .clone(),
            ),
            "deployment" => (
                EvidenceRef::Deployment(DeploymentId::from_storage(a.evidence_id)),
                vec![
                    *deployments
                        .get(&a.evidence_id)
                        .ok_or_else(|| WireError("associated deployment missing".into()))?,
                ],
            ),
            _ => return Err(WireError("invalid associated evidence kind".into())),
        };
        for span in spans {
            sources
                .entry((member, span))
                .or_default()
                .insert(reference.clone());
        }
    }
    Ok(sources)
}

pub fn validate_projection(
    tables: &BTreeMap<String, Vec<arrow_array::RecordBatch>>,
) -> Result<(), crate::serving_projection::ProjectionError> {
    use crate::serving_projection::{corrupt, projected_rows as read};
    let members = read::<CatalogMembers>(tables)?;
    let rows = read::<CatalogSelectionDomains>(tables)?;
    let bindings: BTreeMap<_, _> = read::<CatalogBindings>(tables)?
        .into_iter()
        .map(|b| (b.binding_id, b))
        .collect();
    let signatures: BTreeMap<_, _> = read::<CatalogSignatures>(tables)?
        .into_iter()
        .map(|s| (s.signature_id, s))
        .collect();
    let constructors = read::<CatalogConstructors>(tables)?;
    let associations = read::<CatalogAssociations>(tables)?;
    let sources = associated_sources(
        &associations,
        &read::<CatalogScenarios>(tables)?,
        &read::<CatalogDeployments>(tables)?,
    )
    .map_err(|e| corrupt(e.to_string()))?;
    let fields = read::<CatalogConfigurations>(tables)?;
    let field_links = read::<CatalogFieldLinks>(tables)?;
    let artifacts = read::<CatalogArtifacts>(tables)?;
    let releases: BTreeSet<_> = artifacts.iter().map(|a| a.release_id).collect();
    let evidence: BTreeSet<_> = read::<crate::catalog::CatalogEvidence>(tables)?
        .iter()
        .map(|e| e.evidence_id)
        .collect();
    let deployments: BTreeSet<_> = read::<CatalogDeployments>(tables)?
        .iter()
        .map(|e| e.deployment_id)
        .collect();
    let scenarios: BTreeSet<_> = read::<CatalogScenarios>(tables)?
        .iter()
        .map(|s| s.scenario_id)
        .collect();
    let spans: BTreeSet<_> = read::<CatalogSpans>(tables)?
        .iter()
        .map(|s| s.span_id)
        .collect();
    if members.iter().map(|m| m.member_id).collect::<BTreeSet<_>>()
        != rows.iter().map(|r| r.member_id).collect::<BTreeSet<_>>()
        || members.len() != rows.len()
    {
        return Err(corrupt("selection domain membership closure"));
    }
    for row in rows {
        let detail = canonical_domain(&row).map_err(|e| corrupt(e.to_string()))?;
        for scope in detail.domains {
            for c in scope.contexts {
                if super::member(&c.context).is_some_and(|m| m.storage() != row.member_id) {
                    return Err(corrupt("foreign selection context member"));
                }
                if c.complete && c.evidence.is_empty() {
                    return Err(corrupt("unattributed domain closure"));
                }
                for witness in &c.evidence {
                    let valid = match witness {
                        WitnessEvidence::Original {
                            evidence: EvidenceRef::Span(id),
                        } => spans.contains(&id.storage()),
                        WitnessEvidence::Original {
                            evidence: EvidenceRef::Scenario(id),
                        } => scenarios.contains(&id.storage()),
                        WitnessEvidence::Original {
                            evidence: EvidenceRef::Deployment(id),
                        } => deployments.contains(&id.storage()),
                        WitnessEvidence::Catalog { id } => evidence.contains(&id.storage()),
                        WitnessEvidence::Domain { id } => *id == row.domain_id,
                        WitnessEvidence::Fact { .. } => true, // Raw fact references are verified by canonical reconstruction before publication.
                        _ => false,
                    };
                    if !valid {
                        return Err(corrupt("selection witness closure"));
                    }
                }
                match c.context {
                    SelectionContext::Signature {
                        binding, signature, ..
                    } => {
                        let b = bindings
                            .get(&binding.storage())
                            .ok_or_else(|| corrupt("missing domain binding"))?;
                        let s = signatures
                            .get(&signature.storage())
                            .ok_or_else(|| corrupt("missing domain signature"))?;
                        if b.member_id != row.member_id
                            || !(b.declaration_node_id == Some(s.callable_node_id)
                                || constructors.iter().any(|c| {
                                    Some(c.class_node_id) == b.declaration_node_id
                                        && c.signature_id == s.signature_id
                                }))
                        {
                            return Err(corrupt("signature unrelated to domain binding"));
                        }
                    }
                    SelectionContext::Scenario { scenario, .. }
                        if !scenarios.contains(&scenario.storage())
                            || !associations.iter().any(|a| {
                                a.member_id == Some(row.member_id)
                                    && a.evidence_kind == "scenario"
                                    && a.evidence_id == scenario.storage()
                            }) =>
                    {
                        return Err(corrupt("unrelated domain scenario"));
                    }
                    SelectionContext::Source { span, .. }
                        if !spans.contains(&span.storage())
                            || !sources.contains_key(&(row.member_id, span.storage())) =>
                    {
                        return Err(corrupt("unrelated domain source span"));
                    }
                    SelectionContext::Configuration { owner, scope, .. } => {
                        let declarations: BTreeSet<_> = bindings
                            .values()
                            .filter(|b| {
                                b.member_id == row.member_id
                                    && !matches!(
                                        b.role.as_str(),
                                        "shadowed_source" | "provider_public_observation"
                                    )
                            })
                            .filter_map(|b| b.declaration_node_id)
                            .collect();
                        let related = fields.iter().any(|f| f.class_node_id == owner)
                            && (declarations.contains(&owner)
                                || field_links.iter().any(|l| {
                                    l.class_node_id == owner
                                        && match scope {
                                            ConfigurationScopeKind::Object => l
                                                .reader_node_id
                                                .is_some_and(|n| declarations.contains(&n)),
                                            ConfigurationScopeKind::PerCall => {
                                                signatures.get(&l.signature_id).is_some_and(|s| {
                                                    declarations.contains(&s.callable_node_id)
                                                })
                                            }
                                        }
                                }));
                        if !related {
                            return Err(corrupt("unrelated configuration owner"));
                        }
                    }
                    SelectionContext::Release { release }
                        if !releases.contains(&release)
                            || !detail.releases.iter().any(|r| r.release_id == release) =>
                    {
                        return Err(corrupt("foreign release scope"));
                    }
                    SelectionContext::Runtime { .. } => {
                        return Err(corrupt(
                            "runtime domain requires validated evaluation producer",
                        ));
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
