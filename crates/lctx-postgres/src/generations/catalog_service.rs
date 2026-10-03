//! Prepared catalog metadata reuses the one admitted finite classifier.
use super::{AdmittedSelection, Error, GenerationService, RequestExecution};
use lctx_model::domain::serving::identity::{PolicyIdentity, WireIdentity, policy_identity};
use lctx_model::domain::{
    self,
    catalog::CatalogMember,
    normalized::{Rows, callables::Knowledge, entities::*},
    selection::{
        self,
        algebra::{ClaimContext, RequirementWitness},
        evaluate::CandidateSelection,
    },
    serving::*,
    *,
};
use std::{collections::BTreeSet, sync::Arc};
struct State {
    service: GenerationService,
    selection: AdmittedSelection,
    distributions: Rows<input::InputDistribution>,
    ownership: Rows<input::ArtifactOwnership>,
    verifications: Rows<input::DistributionVerification>,
    policy: PolicyIdentity,
    wire: WireIdentity,
    _charge: Box<dyn domain::resources::Reservation>,
}
#[derive(Clone)]
pub struct CatalogService {
    state: Arc<State>,
}
pub(super) fn wire_error(e: WireError) -> Error {
    match e {
        WireError::ResourceRefused(_) => Error::ResourceRefused("serving representation"),
        _ => Error::Codec(e.to_string()),
    }
}
struct CountBytes(usize);
impl std::io::Write for CountBytes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("serving byte count overflow"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub(super) fn serialized_len<T: serde::Serialize>(value: &T) -> Result<usize, Error> {
    let mut bytes = CountBytes(0);
    serde_json::to_writer(&mut bytes, value).map_err(|e| Error::Codec(e.to_string()))?;
    Ok(bytes.0)
}
pub(super) fn retain<T: serde::Serialize>(
    execution: &RequestExecution,
    value: &T,
) -> Result<(), Error> {
    execution.retain(
        "serving-result-dtos",
        serialized_len(value)?
            .saturating_mul(2)
            .saturating_add(size_of::<T>()),
    )
}
pub(super) fn name(s: impl Into<String>) -> Result<Name, Error> {
    let s = s.into();
    if s.len() > 512 {
        return Err(Error::ResourceRefused("wire name bytes"));
    }
    Name::new(s).map_err(wire_error)
}
impl CatalogService {
    pub async fn prepare(service: GenerationService) -> Result<Self, Error> {
        let guard = service.guard();
        guard.check().await?;
        let mut held = guard.state.lease.lock().await;
        let lease = held.as_mut().ok_or(Error::State)?;
        let binding = lctx_model::domain::serving::mappings::prepared_binding(
            lctx_model::domain::serving::mappings::PreparedDependency::CatalogIdentity,
        );
        if !binding.permits::<input::InputDistribution>()
            || !binding.permits::<input::ArtifactOwnership>()
            || !binding.permits::<input::DistributionVerification>()
        {
            return Err(Error::Contract);
        }
        let mut distributions = Rows::new(&lease.budget);
        lease
            .visit_verified::<input::InputDistribution>(|batch| {
                for row in batch.rows() {
                    distributions.insert(row.clone())?;
                }
                Ok(())
            })
            .await?;
        let mut ownership = Rows::new(&lease.budget);
        lease
            .visit_verified::<input::ArtifactOwnership>(|batch| {
                for row in batch.rows() {
                    ownership.insert(row.clone())?;
                }
                Ok(())
            })
            .await?;
        let mut verifications = Rows::new(&lease.budget);
        lease
            .visit_verified::<input::DistributionVerification>(|batch| {
                for row in batch.rows() {
                    verifications.insert(row.clone())?;
                }
                Ok(())
            })
            .await?;
        let charge = lease
            .budget
            .reserve("serving-catalog-owner", size_of::<State>() + 64)?;
        let policy = policy_identity(&(
            "catalog-complete/v1",
            "public-path-member-analysis-order",
            "source-declaration-ownership",
        ))?;
        let wire = wire_identity();
        drop(held);
        guard.check().await?;
        let selection = service.selection();
        for row in ownership.iter() {
            if verifications.get(row.distribution).is_none() {
                return Err(Error::Contract);
            }
        }
        for row in distributions.iter() {
            if selection
                .prepared()
                .data()
                .facts
                .releases
                .get(row.release)
                .is_none()
            {
                return Err(Error::Contract);
            }
        }
        Ok(Self {
            state: Arc::new(State {
                service,
                selection,
                distributions,
                ownership,
                verifications,
                policy,
                wire,
                _charge: charge,
            }),
        })
    }
    pub fn service(&self) -> &GenerationService {
        &self.state.service
    }
    pub(super) fn distributions(&self) -> &Rows<input::InputDistribution> {
        &self.state.distributions
    }
    pub(super) fn prepared(&self) -> &selection::evaluate::Prepared {
        self.state.selection.prepared()
    }
    pub(super) fn wire(&self) -> WireIdentity {
        self.state.wire
    }
    pub(super) fn generation(&self) -> GenerationKey {
        GenerationKey(*self.state.service.generation().bytes())
    }
    pub(super) fn check_execution(&self, e: &RequestExecution) -> Result<(), Error> {
        if !e.shares_guard(&self.state.service.guard()) {
            return Err(Error::Contract);
        }
        Ok(())
    }
    pub(super) fn member_releases(
        &self,
        m: &catalog::CatalogMember,
    ) -> Result<BTreeSet<Id<input::Release>>, Error> {
        let module = self
            .prepared()
            .data()
            .source
            .core
            .modules
            .get(m.access)
            .ok_or(Error::Contract)?;
        let ownership = self
            .state
            .ownership
            .iter()
            .filter(|r| r.artifact == module.source)
            .collect::<Vec<_>>();
        if ownership.is_empty() {
            Ok(self
                .state
                .distributions
                .iter()
                .filter(|r| r.input == m.input && r.role == input::DistributionRole::FirstParty)
                .map(|r| r.release)
                .collect())
        } else {
            ownership
                .into_iter()
                .map(|r| {
                    self.state
                        .verifications
                        .get(r.distribution)
                        .map(|v| v.release)
                        .ok_or(Error::Contract)
                })
                .collect()
        }
    }
    /// Borrowed release/package metadata for packet construction admission, without allocating
    /// the member's release-ID set before its caller has reserved that set and rendered names.
    pub(super) fn release_allowance(&self, m: &catalog::CatalogMember) -> Result<usize, Error> {
        let d = self.prepared().data();
        let module = d.source.core.modules.get(m.access).ok_or(Error::Contract)?;
        let mut bytes = 0usize;
        let ownership_present = self
            .state
            .ownership
            .iter()
            .any(|r| r.artifact == module.source);
        let mut add_release = |release| -> Result<(), Error> {
            let release = d.facts.releases.get(release).ok_or(Error::Contract)?;
            let package = d
                .facts
                .packages
                .get(release.package)
                .ok_or(Error::Contract)?;
            bytes = bytes.saturating_add(128).saturating_add(
                package
                    .name
                    .len()
                    .saturating_add(release.version.len())
                    .saturating_mul(3),
            );
            Ok(())
        };
        if ownership_present {
            for row in self
                .state
                .ownership
                .iter()
                .filter(|r| r.artifact == module.source)
            {
                add_release(
                    self.state
                        .verifications
                        .get(row.distribution)
                        .ok_or(Error::Contract)?
                        .release,
                )?;
            }
        } else {
            for row in self
                .state
                .distributions
                .iter()
                .filter(|r| r.input == m.input && r.role == input::DistributionRole::FirstParty)
            {
                add_release(row.release)?;
            }
        }
        Ok(bytes)
    }
    pub(super) fn belongs(&self, m: &catalog::CatalogMember, library: &Name) -> bool {
        let d = self.prepared().data();
        self.member_releases(m).is_ok_and(|releases| {
            releases.iter().any(|release| {
                self.state.distributions.iter().any(|r| {
                    r.input == m.input
                        && r.release == *release
                        && r.role == input::DistributionRole::FirstParty
                }) && d
                    .facts
                    .releases
                    .get(*release)
                    .and_then(|r| d.facts.packages.get(r.package))
                    .is_some_and(|p| p.name == library.as_str())
            })
        })
    }
    pub(super) fn path(&self, m: &CatalogMember) -> Result<Vec<String>, Error> {
        let module = self
            .prepared()
            .data()
            .source
            .core
            .modules
            .get(m.access)
            .ok_or(Error::Contract)?;
        let mut out = module
            .qualified_name
            .split('.')
            .map(str::to_owned)
            .collect::<Vec<_>>();
        out.extend(m.path.clone());
        Ok(out)
    }
    pub(super) fn resolves(
        &self,
        m: &CatalogMember,
        selector: &OperationSelector,
    ) -> Result<bool, Error> {
        Ok(match selector {
            OperationSelector::Member { member } => m.id() == *member,
            OperationSelector::PublicPath { path } => self
                .prepared()
                .data()
                .source
                .core
                .modules
                .get(m.access)
                .ok_or(Error::Contract)?
                .qualified_name
                .split('.')
                .chain(m.path.iter().map(String::as_str))
                .eq(path.iter().map(Name::as_str)),
        })
    }
    pub(super) fn candidates(
        &self,
        selection: &selection::Selection,
        library: &Name,
        b: &domain::resources::ResourceBudget,
    ) -> Result<
        Vec<(
            OperationCandidate,
            selection::Outcome,
            Id<attribution::AnalysisContext>,
        )>,
        Error,
    > {
        let selected = self.prepared().select(selection, b)?;
        let mut rows = Vec::new();
        let mut charge = b.reserve("serving-catalog-results", 0)?;
        let mut bytes = 0usize;
        for c in &selected.candidates {
            let member = self
                .prepared()
                .data()
                .source
                .catalog
                .members
                .get(c.member)
                .ok_or(Error::Contract)?;
            let module = self
                .prepared()
                .data()
                .source
                .core
                .modules
                .get(member.access)
                .ok_or(Error::Contract)?;
            let _membership = b.reserve(
                "catalog-release-membership",
                self.state
                    .ownership
                    .iter()
                    .filter(|r| r.artifact == module.source)
                    .count()
                    .saturating_add(
                        self.state
                            .distributions
                            .iter()
                            .filter(|r| r.input == member.input)
                            .count(),
                    )
                    .saturating_mul(128),
            )?;
            if !self.belongs(member, library) {
                continue;
            }
            // Account Vec growth, duplicated witness IDs and temporary BTree sets before cloning.
            let path_bytes = module
                .qualified_name
                .len()
                .saturating_add(member.path.iter().map(String::len).sum::<usize>());
            let mut estimated = size_of::<(
                OperationCandidate,
                selection::Outcome,
                Id<attribution::AnalysisContext>,
            )>()
            .saturating_mul(2)
            .saturating_add(path_bytes.saturating_mul(4))
            .saturating_add(
                module
                    .qualified_name
                    .split('.')
                    .count()
                    .saturating_add(member.path.len())
                    .saturating_mul(2 * size_of::<String>()),
            );
            for r in &c.requirements {
                estimated = estimated
                    .saturating_add(2 * size_of::<RequirementResult>())
                    .saturating_add(r.requirement.predicate.heap_bytes().saturating_mul(2))
                    .saturating_add(
                        r.closure
                            .len()
                            .saturating_mul(2 * size_of::<Id<selection::Witness>>()),
                    );
                for witness in &r.witnesses {
                    let count = match witness {
                        RequirementWitness::Positive { evidence, .. }
                        | RequirementWitness::Negative { evidence, .. } => evidence.len(),
                        RequirementWitness::Conflict {
                            positive, negative, ..
                        } => positive.len().saturating_add(negative.len()),
                    };
                    let runtime_values = match witness {
                        RequirementWitness::Positive { evidence, .. } | RequirementWitness::Negative { evidence, .. } => evidence.iter().filter(|w| matches!(w, selection::Witness::SummaryException { .. })).count(),
                        RequirementWitness::Conflict { positive, negative, .. } => positive.iter().chain(negative.iter()).filter(|w| matches!(w, selection::Witness::SummaryException { .. })).count(),
                    };
                    estimated = estimated.saturating_add(runtime_values.saturating_mul(4096));
                    estimated = estimated
                        .saturating_add(2 * size_of::<RequirementWitnessPacket>())
                        .saturating_add(128)
                        .saturating_add(count.saturating_mul(192));
                }
            }
            estimated = estimated.saturating_add(
                self.prepared()
                    .data()
                    .source
                    .catalog
                    .callables
                    .iter()
                    .filter(|r| r.member == member.id())
                    .count()
                    .saturating_mul(2 * size_of::<Knowledge>()),
            );
            charge.try_resize(bytes.saturating_add(estimated))?;
            let candidate = self.candidate(c)?;
            let actual = serialized_len(&candidate)?
                .saturating_mul(2)
                .saturating_add(
                    2 * size_of::<(
                        OperationCandidate,
                        selection::Outcome,
                        Id<attribution::AnalysisContext>,
                    )>(),
                );
            bytes = bytes.saturating_add(estimated.max(actual));
            charge.try_resize(bytes)?;
            rows.push((candidate, c.outcome, c.analysis));
        }
        rows.sort_by(|a, b| {
            a.0.name
                .as_str()
                .cmp(b.0.name.as_str())
                .then(a.0.member.cmp(&b.0.member))
                .then(a.2.cmp(&b.2))
        });
        Ok(rows)
    }
    fn candidate(&self, c: &CandidateSelection) -> Result<OperationCandidate, Error> {
        let d = self.prepared().data();
        let member = d
            .source
            .catalog
            .members
            .get(c.member)
            .ok_or(Error::Contract)?;
        let mut requirements = Vec::new();
        for r in &c.requirements {
            let mut contexts = BTreeSet::new();
            let mut positive = BTreeSet::new();
            let mut negative = BTreeSet::new();
            let mut claims = Vec::new();
            for witness in &r.witnesses {
                let (context, basis, p, n) = match witness {
                    RequirementWitness::Positive {
                        context,
                        basis,
                        evidence,
                    } => (context, *basis, evidence.as_slice(), &[][..]),
                    RequirementWitness::Negative {
                        context,
                        basis,
                        evidence,
                    } => (context, *basis, &[][..], evidence.as_slice()),
                    RequirementWitness::Conflict {
                        context,
                        basis,
                        positive,
                        negative,
                    } => (context, *basis, positive.as_slice(), negative.as_slice()),
                };
                let ClaimContext::Declaration(context) = context else {
                    return Err(Error::Contract);
                };
                contexts.insert(context.id());
                let mut behavioral_exceptions = Vec::new();
                for witness in p.iter().chain(n.iter()) {
                    if let selection::Witness::SummaryException { outcome } = witness {
                        let result = self.prepared().data().facts.exception_outcomes.get(*outcome).ok_or(Error::Contract)?;
                        let q = self.prepared().data().source.core.qualifications.get(result.qualification).ok_or(Error::Contract)?;
                        behavioral_exceptions.push(BehavioralExceptionPacket::from_canonical(result, q)?);
                    }
                }
                claims.push(RequirementWitnessPacket {
                    context: context.id(),
                    basis,
                    behavioral_exceptions,
                    positive: p.iter().map(Record::id).collect(),
                    negative: n.iter().map(Record::id).collect(),
                });
                for w in p {
                    positive.insert(w.id());
                }
                for w in n {
                    negative.insert(w.id());
                }
            }
            requirements.push(RequirementResult {
                claims,
                closure: r.closure.iter().map(Record::id).collect(),
                requirement: r.requirement.clone(),
                outcome: r.outcome,
                reason: r.reason,
                contexts: contexts.into_iter().collect(),
                positive: positive.into_iter().collect(),
                negative: negative.into_iter().collect(),
                corpus_complete: r.corpus_complete,
                analyzer_complete: r.analyzer_complete,
                examined: r.examined as u64,
                total: Nullable(r.total.map(|n| n as u64)),
            });
        }
        let knowledge = d
            .source
            .catalog
            .callables
            .iter()
            .filter(|r| r.member == member.id())
            .map(|r| {
                d.source
                    .core
                    .assessments
                    .get(r.assessment)
                    .map(|a| a.signatures)
                    .ok_or(Error::Contract)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let signature_knowledge = if knowledge.contains(&Knowledge::Conflicting) {
            Knowledge::Conflicting
        } else if !knowledge.is_empty() && knowledge.iter().all(|k| *k == Knowledge::Known) {
            Knowledge::Known
        } else {
            Knowledge::Unknown
        };
        Ok(OperationCandidate {
            member: c.member,
            analysis: c.analysis,
            name: name(self.path(member)?.join("."))?,
            requirements,
            joint: c.joint,
            signature_knowledge,
        })
    }
    pub(super) fn page<T>(
        &self,
        request: &Request,
        group: &str,
        section: &str,
        rows: Vec<T>,
    ) -> Result<SectionPage<T>, Error> {
        let size = request.page().size;
        if size == 0 || size > ResourceLimits::default().maximum_page_rows {
            return Err(Error::ResourceRefused("page rows"));
        }
        let binding = CursorBinding {
            generation: self.generation(),
            request: request.canonical_identity().map_err(wire_error)?,
            policy: self.state.policy,
            wire: self.state.wire,
            channels: ChannelState {
                lexical: false,
                vector: VectorChannel::Disabled {},
            }
            .identity(),
            group: name(group)?,
            section: name(section)?,
            member: None,
            ordering: ContentHash::of(b"catalog-public-path/member/analysis/v1"),
        };
        let offset = match &request.page().cursor.0 {
            Some(token) => Cursor::decode(token, &binding).map_err(wire_error)?.offset,
            None => 0,
        };
        let offset = usize::try_from(offset).map_err(|_| Error::Codec("cursor offset".into()))?;
        if offset > rows.len() {
            return Err(Error::Codec("cursor offset outside complete domain".into()));
        }
        let total = rows.len();
        let end = offset.saturating_add(size as usize).min(total);
        let continuation = if end < total {
            Optional(Some(
                Cursor {
                    binding,
                    offset: end as u64,
                }
                .encode()
                .map_err(wire_error)?,
            ))
        } else {
            Optional::default()
        };
        Ok(SectionPage {
            availability: Availability::Available {},
            items: rows.into_iter().skip(offset).take(end - offset).collect(),
            continuation,
            omitted: (total - end) as u64,
            truncated: end < total,
        })
    }
    pub async fn find(
        &self,
        e: &RequestExecution,
        r: &FindOperationsRequest,
    ) -> Result<FindOperationsResponse, Error> {
        self.check_execution(e)?;
        let this = self.clone();
        let r = r.clone();
        let retained = e.clone();
        e.cpu(move |b| {
            let rows = this.candidates(&r.selection.0, &r.library, b)?;
            let total = rows.len() as u64;
            let mut supported = Vec::new();
            let mut unresolved = Vec::new();
            let mut conflicting = Vec::new();
            for (candidate, outcome, _) in rows {
                match outcome {
                    selection::Outcome::Supported => supported.push(candidate),
                    selection::Outcome::Unresolved
                        if r.selection.0.mode == selection::Mode::Discovery =>
                    {
                        unresolved.push(candidate)
                    }
                    selection::Outcome::Conflicting
                        if r.selection.0.mode == selection::Mode::Discovery =>
                    {
                        conflicting.push(candidate)
                    }
                    _ => {}
                }
            }
            if let Some(token) = &r.page.cursor.0 {
                let cursor = peek_cursor(token)?;
                if !["supported", "unresolved", "conflicting"]
                    .contains(&cursor.binding.group.as_str())
                {
                    return Err(Error::Codec("unknown catalog continuation group".into()));
                }
            }
            let request = Request::FindOperations(r); // A continuation belongs to exactly one group; other groups start at zero.
            let mut group_request = request.clone();
            group_request = clear_other_cursor(group_request, "supported")?;
            let supported = this.page(&group_request, "supported", "members", supported)?;
            let unresolved = this.page(
                &clear_other_cursor(request.clone(), "unresolved")?,
                "unresolved",
                "members",
                unresolved,
            )?;
            let conflicting = this.page(
                &clear_other_cursor(request, "conflicting")?,
                "conflicting",
                "members",
                conflicting,
            )?;
            let response = FindOperationsResponse {
                generation: this.generation(),
                supported,
                unresolved,
                conflicting,
                extent: SelectionExtent::CompleteDomain { total },
            };
            retain(&retained, &response)?;
            Ok(response)
        })
        .await
    }
    pub async fn compare(
        &self,
        e: &RequestExecution,
        r: &CompareOperationsRequest,
    ) -> Result<CompareOperationsResponse, Error> {
        self.check_execution(e)?;
        let this = self.clone();
        let r = r.clone();
        let retained = e.clone();
        e.cpu(move |b| {
            if r.operations.is_empty() || r.operations.len() > 5 {
                return Err(Error::ResourceRefused("comparison members"));
            }
            let rows = this.candidates(&r.selection.0, &r.library, b)?;
            let mut operations = Vec::new();
            for requested in r.operations {
                let mut candidates = Vec::new();
                for (c, _, _) in &rows {
                    let m = this
                        .prepared()
                        .data()
                        .source
                        .catalog
                        .members
                        .get(c.member)
                        .ok_or(Error::Contract)?;
                    if this.resolves(m, &requested)? {
                        candidates.push(c.clone());
                    }
                }
                let ambiguous = candidates
                    .iter()
                    .map(|c| c.member)
                    .collect::<BTreeSet<_>>()
                    .len()
                    > 1;
                operations.push(ComparisonEntry {
                    requested,
                    candidates,
                    ambiguous,
                });
            }
            let response = CompareOperationsResponse {
                generation: this.generation(),
                operations,
            };
            retain(&retained, &response)?;
            Ok(response)
        })
        .await
    }
    pub(super) fn ownership(&self, m: &CatalogMember) -> Option<Option<Id<ClassEntity>>> {
        let d = self.prepared().data();
        let mut owners = BTreeSet::new();
        let exposures = d
            .source
            .catalog
            .exposures
            .iter()
            .filter(|r| r.member == m.id())
            .map(Record::id)
            .collect::<BTreeSet<_>>();
        let candidates = d
            .source
            .catalog
            .candidates
            .iter()
            .filter(|r| exposures.contains(&r.exposure))
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }
        for c in candidates {
            let entity = c
                .path
                .and_then(|p| d.source.catalog.paths.get(p).map(|p| p.entity))
                .or_else(|| {
                    c.entity
                        .and_then(|e| d.source.core.entity_candidates.get(e).map(|e| e.entity))
                })
                .or_else(|| {
                    c.alias
                        .and_then(|a| d.source.catalog.aliases.get(a).map(|a| a.entity))
                });
            match entity.and_then(|id| d.source.core.refs.get(id)) {
                Some(EntityRef::Callable { callable }) => {
                    let Some(CallableEntity::Source { declaration, .. }) =
                        d.source.core.source_callables.get(*callable)
                    else {
                        return None;
                    };
                    let declarations = d
                        .source
                        .core
                        .declarations
                        .iter()
                        .filter(|r| r.declaration == *declaration)
                        .collect::<Vec<_>>();
                    if declarations.is_empty() {
                        return None;
                    }
                    for decl in declarations {
                        let owner = match decl.parent {
                            None => None,
                            Some(parent) => {
                                let actual_class = d.source.core.declarations.iter().any(|r| {
                                    r.declaration == parent
                                        && r.kind == syntax::DeclarationKind::Class
                                });
                                if actual_class {
                                    let class = ClassEntity::Source {
                                        declaration: parent,
                                    }
                                    .id();
                                    if !d.source.catalog.classes.iter().any(|r| r.class == class)
                                        && !d.source.core.refs.iter().any(
                                            |r| matches!(r,EntityRef::Class{class:id}if *id==class),
                                        )
                                    {
                                        return None;
                                    }
                                    Some(class)
                                } else {
                                    None
                                }
                            }
                        };
                        owners.insert(owner);
                    }
                }
                Some(EntityRef::Class { class }) => {
                    let declarations = d
                        .source
                        .core
                        .declarations
                        .iter()
                        .filter(|r| {
                            r.kind == syntax::DeclarationKind::Class
                                && ClassEntity::Source {
                                    declaration: r.declaration,
                                }
                                .id()
                                    == *class
                        })
                        .collect::<Vec<_>>();
                    if declarations.is_empty() {
                        return None;
                    }
                    for declaration in declarations {
                        let parent = declaration.parent.and_then(|parent| {
                            d.source
                                .core
                                .declarations
                                .iter()
                                .find(|r| {
                                    r.declaration == parent
                                        && r.kind == syntax::DeclarationKind::Class
                                })
                                .map(|_| {
                                    ClassEntity::Source {
                                        declaration: parent,
                                    }
                                    .id()
                                })
                        });
                        owners.insert(parent);
                    }
                }
                Some(EntityRef::Module { .. }) => {
                    owners.insert(None);
                }
                _ => return None,
            }
        }
        if owners.len() == 1 {
            owners.into_iter().next()
        } else {
            None
        }
    }
    pub(super) fn class_owner(&self, m: &CatalogMember) -> Option<Id<ClassEntity>> {
        self.ownership(m).flatten()
    }
    pub async fn browse(
        &self,
        e: &RequestExecution,
        r: &BrowseLibraryRequest,
    ) -> Result<BrowseLibraryResponse, Error> {
        self.check_execution(e)?;
        let this = self.clone();
        let r = r.clone();
        let retained = e.clone();
        e.cpu(move |b| {
            let d = this.prepared().data();
            match r.scope {
                BrowseScope::Module { module } => {
                    if d.source.core.modules.get(module).is_none() {
                        return Err(Error::Absent);
                    }
                }
                BrowseScope::Class { member } => {
                    let m = d.source.catalog.members.get(member).ok_or(Error::Absent)?;
                    if !this.belongs(m, &r.library) {
                        return Err(Error::Absent);
                    }
                    if !d.source.catalog.classes.iter().any(|c| c.member == member) {
                        return Err(Error::Codec(
                            "requested class scope is not established".into(),
                        ));
                    }
                }
                _ => {}
            }
            let eligible = this
                .candidates(&r.selection.0, &r.library, b)?
                .into_iter()
                .filter(|(_, o, _)| {
                    *o == selection::Outcome::Supported
                        || (r.selection.0.mode == selection::Mode::Discovery
                            && *o != selection::Outcome::Contradicted)
                })
                .map(|(c, _, _)| c)
                .collect::<Vec<_>>();
            let class_ids = match r.scope {
                BrowseScope::Class { member } => d
                    .source
                    .catalog
                    .classes
                    .iter()
                    .filter(|c| c.member == member)
                    .map(|c| c.class)
                    .collect::<BTreeSet<_>>(),
                _ => BTreeSet::new(),
            };
            let scoped = eligible
                .into_iter()
                .filter(|candidate| {
                    let Some(m) = d.source.catalog.members.get(candidate.member) else {
                        return false;
                    };
                    match r.scope {
                        BrowseScope::Library {} => true,
                        BrowseScope::Module { module } => m.access == module,
                        BrowseScope::Class { .. } => match this.ownership(m) {
                            Some(Some(owner)) => class_ids.contains(&owner),
                            Some(None) => false,
                            None => true,
                        },
                    }
                })
                .collect::<Vec<_>>();
            let scoped_members = scoped.iter().map(|c| c.member).collect::<BTreeSet<_>>();
            let mut entries = Vec::new();
            let mut unknown_members = BTreeSet::new();
            let mut seen = BTreeSet::new();
            for candidate in scoped {
                let m = d
                    .source
                    .catalog
                    .members
                    .get(candidate.member)
                    .ok_or(Error::Contract)?;
                let ownership = this.ownership(m);
                if ownership.is_none() {
                    unknown_members.insert(m.id());
                }
                match r.view {
                    BrowseView::Members => entries.push(BrowseEntry::Member {
                        candidate,
                        ownership: if ownership.is_some() {
                            Availability::Available {}
                        } else {
                            Availability::Partial {
                                reason: name("class ownership not established")?,
                            }
                        },
                    }),
                    BrowseView::Modules => {
                        if seen.insert(m.access.bytes().to_vec()) {
                            let module =
                                d.source.core.modules.get(m.access).ok_or(Error::Contract)?;
                            let count = d
                                .source
                                .catalog
                                .members
                                .iter()
                                .filter(|other| {
                                    other.access == m.access && scoped_members.contains(&other.id())
                                })
                                .count();
                            entries.push(BrowseEntry::Module {
                                module: m.access,
                                name: name(module.qualified_name.clone())?,
                                members: count as u64,
                            });
                        }
                    }
                    BrowseView::Classes => {
                        if d.source.catalog.classes.iter().any(|c| c.member == m.id())
                            && seen.insert(m.id().bytes().to_vec())
                        {
                            let classes = d
                                .source
                                .catalog
                                .classes
                                .iter()
                                .filter(|c| c.member == m.id())
                                .map(|c| c.class)
                                .collect::<BTreeSet<_>>();
                            let count = d
                                .source
                                .catalog
                                .members
                                .iter()
                                .filter(|other| {
                                    scoped_members.contains(&other.id())
                                        && this
                                            .class_owner(other)
                                            .is_some_and(|o| classes.contains(&o))
                                })
                                .count();
                            entries.push(BrowseEntry::Class {
                                member: m.id(),
                                name: candidate.name,
                                members: count as u64,
                            });
                        }
                    }
                    BrowseView::Vocabulary => {}
                }
            }
            if r.view == BrowseView::Vocabulary {
                let mut facets = std::collections::BTreeMap::<
                    i16,
                    (
                        selection::Facet,
                        BTreeSet<String>,
                        BTreeSet<Id<CatalogMember>>,
                    ),
                >::new();
                let mut add =
                    |facet: selection::Facet, value: String, member: Id<CatalogMember>| {
                        let entry = facets
                            .entry(facet as i16)
                            .or_insert_with(|| (facet, BTreeSet::new(), BTreeSet::new()));
                        entry.1.insert(value);
                        entry.2.insert(member);
                    };
                for member in &scoped_members {
                    let m = d
                        .source
                        .catalog
                        .members
                        .get(*member)
                        .ok_or(Error::Contract)?;
                    let module = d.source.core.modules.get(m.access).ok_or(Error::Contract)?;
                    add(
                        selection::Facet::Module,
                        module.qualified_name.clone(),
                        *member,
                    );
                    if d.source.catalog.classes.iter().any(|c| c.member == *member) {
                        add(selection::Facet::Kind, "class".into(), *member);
                    }
                    for callable in d
                        .source
                        .catalog
                        .callables
                        .iter()
                        .filter(|c| c.member == *member)
                    {
                        let assessment = d
                            .source
                            .core
                            .assessments
                            .get(callable.assessment)
                            .ok_or(Error::Contract)?;
                        if let Some(kind) = assessment.descriptor_kind {
                            add(
                                selection::Facet::Kind,
                                match kind {
                                    normalized::callables::DescriptorKind::Function => "function",
                                    normalized::callables::DescriptorKind::Property => "property",
                                    _ => "method",
                                }
                                .into(),
                                *member,
                            );
                        }
                        if let Some(value) = assessment.asynchronous {
                            add(selection::Facet::Async, value.to_string(), *member);
                        }
                        for invocation in d
                            .source
                            .catalog
                            .invocations
                            .iter()
                            .filter(|i| i.callable == callable.id())
                        {
                            for slot in d
                                .source
                                .core
                                .slots
                                .iter()
                                .filter(|s| s.variant == invocation.variant)
                            {
                                let p = d
                                    .facts
                                    .signature_parameters
                                    .get(slot.parameter)
                                    .ok_or(Error::Contract)?;
                                let shape = d.facts.shapes.get(p.shape).ok_or(Error::Contract)?;
                                if let Some(value) = &shape.name {
                                    add(
                                        selection::Facet::Parameter,
                                        value.as_str().to_owned(),
                                        *member,
                                    );
                                }
                            }
                        }
                    }
                }
                for (_, (facet, values, members)) in facets {
                    entries.push(BrowseEntry::Vocabulary {
                        facet,
                        values: values
                            .into_iter()
                            .map(name)
                            .collect::<Result<Vec<_>, _>>()?,
                        members: members.len() as u64,
                    });
                }
            }
            let total = entries.len() as u64;
            let page = this.page(
                &Request::BrowseLibrary(r.clone()),
                "browse",
                "entries",
                entries,
            )?;
            let response = BrowseLibraryResponse {
                generation: this.generation(),
                scope: r.scope,
                view: r.view,
                entries: page,
                extent: SelectionExtent::CompleteDomain { total },
                unknown_ownership: unknown_members.len() as u64,
            };
            retain(&retained, &response)?;
            Ok(response)
        })
        .await
    }
}
fn clear_other_cursor(mut request: Request, group: &str) -> Result<Request, Error> {
    if let Request::FindOperations(r) = &mut request
        && let Some(token) = &r.page.cursor.0
    {
        // Decode only enough to choose the group; full binding is checked by page.
        let cursor = peek_cursor(token)?;
        if cursor.binding.group.as_str() != group {
            r.page.cursor = Optional::default();
        }
    }
    Ok(request)
}

fn peek_cursor(token: &CursorToken) -> Result<Cursor, Error> {
    let text = token.as_str();
    if !text.len().is_multiple_of(2) || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::Codec("cursor encoding".into()));
    }
    let bytes = text
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| {
            std::str::from_utf8(c)
                .ok()
                .and_then(|s| u8::from_str_radix(s, 16).ok())
                .ok_or_else(|| Error::Codec("cursor encoding".into()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::from_slice(&bytes).map_err(|e| Error::Codec(e.to_string()))
}
