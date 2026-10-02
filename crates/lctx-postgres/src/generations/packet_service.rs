//! Indivisible operation contracts; optional bodies are hydrated by their own section services.
use super::catalog_service::{name, retain, serialized_len, wire_error};
use super::{CatalogService, Error, RequestExecution};
use lctx_model::domain::{
    calls::*,
    catalog::*,
    normalized::{callables::*, entities::*},
    serving::*,
    *,
};
use std::collections::BTreeSet;
struct CoreAllowance(usize);
impl CoreAllowance {
    fn items<T>(&mut self, count: usize) {
        self.0 = self
            .0
            .saturating_add(count.saturating_mul(2 * size_of::<T>() + 64));
    }
    fn bytes(&mut self, bytes: usize, copies: usize) {
        self.0 = self.0.saturating_add(bytes.saturating_mul(copies));
    }
    fn literal(&mut self, row: &value::Literal) {
        self.items::<LiteralPacket>(1);
        self.items::<Id<value::Literal>>(1);
        self.bytes(row.heap_bytes(), 4);
    }
}
impl CatalogService {
    pub async fn operation(
        &self,
        e: &RequestExecution,
        r: &GetOperationRequest,
    ) -> Result<GetOperationResponse, Error> {
        self.check_execution(e)?;
        let this = self.clone();
        let request = r.clone();
        let retained = e.clone();
        let resolved = e
            .cpu(move |b| {
                let rows =
                    this.candidates(&selection::Selection::default(), &request.library, b)?;
                let _matching = b.reserve(
                    "operation-matching-candidates",
                    rows.len()
                        .saturating_mul(2 * size_of::<OperationCandidate>()),
                )?;
                let mut matches = Vec::new();
                for (c, _, _) in rows {
                    let m = this
                        .prepared()
                        .data()
                        .source
                        .catalog
                        .members
                        .get(c.member)
                        .ok_or(Error::Contract)?;
                    if this.resolves(m, &request.operation)? {
                        matches.push(c);
                    }
                }
                retain(&retained, &matches)?;
                Ok(matches)
            })
            .await?;
        let _resolved_ids = e.budget().reserve(
            "operation-resolved-member-ids",
            resolved.len().saturating_mul(128),
        )?;
        let members = resolved.iter().map(|c| c.member).collect::<BTreeSet<_>>();
        if r.page.cursor.0.is_some() && members.len() != 1 {
            return Err(Error::Codec(
                "continuation member is no longer uniquely resolved".into(),
            ));
        }
        let operation = if members.is_empty() {
            OperationResolution::Missing {
                coverage: Availability::Available {},
            }
        } else if members.len() > 1 {
            OperationResolution::Ambiguous {
                candidates: resolved,
            }
        } else {
            let member = *members.first().ok_or(Error::Contract)?;
            let mut cores = self.operation_cores(e, &[member], r.page.expanded).await?;
            let core = cores.pop().ok_or(Error::Contract)?;
            OperationResolution::Unique {
                packet: self.optional_sections(e, r, core).await?,
            }
        };
        let response = self
            .fit_operation_response(
                e,
                r,
                GetOperationResponse {
                    generation: self.generation(),
                    operation,
                },
            )
            .await?;
        retain(e, &response)?;
        e.confirm().await?;
        Ok(response)
    }
    /// One set-based read per presentation relation for the exact selected member set.
    pub async fn operation_cores(
        &self,
        e: &RequestExecution,
        members: &[Id<CatalogMember>],
        expanded: bool,
    ) -> Result<Vec<OperationCore>, Error> {
        self.check_execution(e)?;
        let _member_ids = e.budget().reserve(
            "mandatory-member-ids",
            members
                .len()
                .saturating_mul(2 * size_of::<Id<CatalogMember>>()),
        )?;
        let this = self.clone();
        let members = members.to_vec();
        let retained = e.clone();
        let cores = e
            .cpu(move |b| {
                let allowance =
                    members
                        .iter()
                        .try_fold(2 * size_of::<Vec<OperationCore>>(), |total, id| {
                            this.core_allowance(*id)
                                .map(|bytes| total.saturating_add(bytes))
                        })?;
                let mut charge = b.reserve("mandatory-operation-packets", allowance)?;
                let mut cores = Vec::new();
                let mut bytes = 0usize;
                for member in members {
                    let core = this.core(member, expanded)?;
                    check_indivisible(&core, expanded)?;
                    bytes = bytes
                        .saturating_add(serialized_len(&core)?.saturating_mul(2))
                        .saturating_add(2 * size_of::<OperationCore>());
                    charge.try_resize(allowance.max(bytes))?;
                    cores.push(core);
                }
                retain(&retained, &cores)?;
                Ok(cores)
            })
            .await?;
        let lookup_items = cores
            .iter()
            .map(|c| {
                c.options.len().saturating_add(
                    c.signatures
                        .iter()
                        .map(|s| {
                            s.parameters
                                .iter()
                                .chain(s.effective_parameters.iter())
                                .map(|p| p.types.len())
                                .sum::<usize>()
                                .saturating_add(s.return_types.len())
                        })
                        .sum::<usize>(),
                )
            })
            .sum::<usize>();
        let _lookup_ids = e.budget().reserve(
            "mandatory-hydration-id-sets",
            lookup_items.saturating_mul(128),
        )?;
        let terms = cores
            .iter()
            .flat_map(|c| c.signatures.iter())
            .flat_map(|s| {
                s.parameters
                    .iter()
                    .chain(s.effective_parameters.iter())
                    .flat_map(|p| p.types.iter().copied())
                    .chain(s.return_types.iter().copied())
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let evidence_ids = cores
            .iter()
            .flat_map(|c| c.options.iter().map(|o| o.evidence))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let (presentations, evidence) = e
            .query(move |lease| {
                Box::pin(async move {
                    Ok((
                        lease
                            .read_for::<types::TypePresentation, types::TypeTerm>("term", &terms)
                            .await?,
                        lease
                            .read_ids::<CatalogOptionEvidence>(&evidence_ids)
                            .await?,
                    ))
                })
            })
            .await?;
        for core in &cores {
            for option in &core.options {
                if !evidence
                    .rows()
                    .iter()
                    .any(|row| row.id() == option.evidence)
                {
                    return Err(Error::Contract);
                }
            }
        }
        let this = self.clone();
        let retained = e.clone();
        let result = e
            .cpu(move |budget| {
                let mut allowance = cores
                    .len()
                    .saturating_mul(2 * size_of::<Vec<TypePresentationPacket>>());
                for core in &cores {
                    for row in presentations.rows() {
                        if this.core_has_presentation(core, row) {
                            allowance = allowance
                                .saturating_add(2 * size_of::<TypePresentationPacket>())
                                .saturating_add(row.heap_bytes().saturating_mul(2));
                        }
                    }
                }
                let _construction =
                    budget.reserve("mandatory-type-presentation-dtos", allowance)?;
                let mut cores = cores;
                for core in &mut cores {
                    for row in presentations.rows() {
                        if this.core_has_presentation(core, row) {
                            core.type_presentations.push(
                                TypePresentationPacket::from_canonical(row).map_err(wire_error)?,
                            );
                        }
                    }
                    check_indivisible(core, expanded)?;
                }
                retain(&retained, &cores)?;
                Ok(cores)
            })
            .await?;
        Ok(result)
    }
    fn core_has_presentation(&self, core: &OperationCore, row: &types::TypePresentation) -> bool {
        core.signatures.iter().any(|s| {
            s.return_types.contains(&row.term)
                || s.parameters
                    .iter()
                    .chain(s.effective_parameters.iter())
                    .any(|p| p.types.contains(&row.term))
        }) && self
            .prepared()
            .data()
            .source
            .core
            .qualifications
            .get(row.qualification)
            .is_some_and(|q| core.signatures.iter().any(|s| s.analysis == q.context))
    }

    /// A construction allowance over only the selected member's borrowed canonical metadata.
    /// This counts Vec growth and temporary ID sets, including payload-bearing defaults/types;
    /// it is an allocation reservation against the existing request pool, not a new row limit.
    fn core_allowance(&self, id: Id<CatalogMember>) -> Result<usize, Error> {
        let d = self.prepared().data();
        let c = &d.source.catalog;
        let n = &d.source.core;
        let f = &d.facts;
        let member = c.members.get(id).ok_or(Error::Absent)?;
        let module = n.modules.get(member.access).ok_or(Error::Contract)?;
        let mut allowance = CoreAllowance(2 * size_of::<OperationCore>());
        allowance.bytes(
            module
                .qualified_name
                .len()
                .saturating_add(member.path.iter().map(String::len).sum::<usize>()),
            4,
        );
        allowance.items::<Name>(
            module
                .qualified_name
                .split('.')
                .count()
                .saturating_add(member.path.len()),
        );
        allowance.0 = allowance.0.saturating_add(self.release_allowance(member)?);
        let exposures = c.exposures.iter().filter(|r| r.member == id).count();
        allowance.items::<Id<CatalogExposure>>(exposures);
        allowance.items::<Id<CatalogCandidate>>(
            c.candidates
                .iter()
                .filter(|r| c.exposures.get(r.exposure).is_some_and(|e| e.member == id))
                .count(),
        );
        for option in c.options.iter().filter(|r| r.member == id) {
            allowance.items::<OptionPacket>(1);
            let default = c.defaults.get(option.default).ok_or(Error::Contract)?;
            allowance.items::<DefaultValue>(1);
            if let CatalogDefault::Literal { literal } = default {
                allowance.literal(f.literals.get(*literal).ok_or(Error::Contract)?);
            }
        }
        for callable in c.callables.iter().filter(|r| r.member == id) {
            let assessment = n
                .assessments
                .get(callable.assessment)
                .ok_or(Error::Contract)?;
            allowance.items::<Knowledge>(1);
            for invocation in c.invocations.iter().filter(|r| r.callable == callable.id()) {
                allowance.items::<InvocationPacket>(1);
                allowance.items::<SignaturePacket>(1);
                let variant = n.variants.get(invocation.variant).ok_or(Error::Contract)?;
                let signature = f.signatures.get(variant.signature).ok_or(Error::Contract)?;
                for parameter in f
                    .signature_parameters
                    .iter()
                    .filter(|r| r.signature == signature.id())
                {
                    let shape = f.shapes.get(parameter.shape).ok_or(Error::Contract)?;
                    let slots = n
                        .slots
                        .iter()
                        .filter(|s| s.variant == variant.id() && s.parameter == parameter.id())
                        .count();
                    let links = n
                        .parameter_links
                        .iter()
                        .filter(|l| l.parameter == parameter.id())
                        .count();
                    allowance.items::<ParameterPacket>(slots.saturating_add(1));
                    allowance.items::<DefaultValue>(slots.saturating_add(2));
                    allowance.bytes(
                        shape.name.as_ref().map_or(0, |name| name.as_str().len()),
                        slots.saturating_add(2),
                    );
                    allowance.items::<Id<ParameterEntity>>(
                        links.saturating_mul(slots.saturating_add(2)),
                    );
                    for slot in n
                        .slots
                        .iter()
                        .filter(|s| s.variant == variant.id() && s.parameter == parameter.id())
                    {
                        allowance.items::<Id<ParameterEntity>>(
                            n.slot_entities
                                .iter()
                                .filter(|e| e.slot == slot.id())
                                .count(),
                        );
                    }
                    for observation in f.type_observations.iter().filter(|r|r.role==types::TypeRole::Parameter&&n.parameter_links.iter().any(|l|l.parameter==parameter.id()&&matches!(n.parameters.get(l.entity),Some(ParameterEntity::Source{declaration})if *declaration==r.subject))){
                        allowance.items::<Id<types::TypeTerm>>(slots.saturating_add(2));
                        if let Some(types::TypeTerm::Literal{value})=f.type_terms.get(observation.term){allowance.literal(f.literals.get(*value).ok_or(Error::Contract)?);}
                    }
                    // Source/effective defaults retain original nominal references; option vectors
                    // and their temporary agreement checks can span every selected member option.
                    allowance.items::<Id<CatalogDefault>>(
                        c.options
                            .iter()
                            .filter(|o| o.member == id)
                            .count()
                            .saturating_mul(slots.saturating_add(1)),
                    );
                }
                if let Some(CallableEntity::Source { declaration, .. }) =
                    n.source_callables.get(assessment.callable)
                {
                    for observation in f
                        .type_observations
                        .iter()
                        .filter(|r| r.subject == *declaration && r.role == types::TypeRole::Return)
                    {
                        allowance.items::<Id<types::TypeTerm>>(2);
                        if let Some(types::TypeTerm::Literal { value }) =
                            f.type_terms.get(observation.term)
                        {
                            allowance.literal(f.literals.get(*value).ok_or(Error::Contract)?);
                        }
                    }
                }
            }
        }
        Ok(allowance.0)
    }
    fn core(&self, id: Id<CatalogMember>, expanded: bool) -> Result<OperationCore, Error> {
        let d = self.prepared().data();
        let c = &d.source.catalog;
        let n = &d.source.core;
        let f = &d.facts;
        let m = c.members.get(id).ok_or(Error::Absent)?;
        let releases = self.member_releases(m)?;
        if releases.len() != 1 {
            return Err(Error::Contract);
        }
        let release_id = *releases.first().ok_or(Error::Contract)?;
        let release = f.releases.get(release_id).ok_or(Error::Contract)?;
        let package = f.packages.get(release.package).ok_or(Error::Contract)?;
        let exposures = c
            .exposures
            .iter()
            .filter(|r| r.member == id)
            .map(Record::id)
            .collect::<Vec<_>>();
        let exposure_set = exposures.iter().copied().collect::<BTreeSet<_>>();
        let candidates = c
            .candidates
            .iter()
            .filter(|r| exposure_set.contains(&r.exposure))
            .map(Record::id)
            .collect::<Vec<_>>();
        let mut invocations = Vec::new();
        let mut signatures = Vec::new();
        let mut knowledge = Vec::new();
        let mut basis = BTreeSet::new();
        let mut literal_ids = BTreeSet::new();
        let mut options = Vec::new();
        for option in c.options.iter().filter(|r| r.member == id) {
            let default = c.defaults.get(option.default).ok_or(Error::Contract)?;
            if let CatalogDefault::Literal { literal } = default {
                literal_ids.insert(*literal);
            }
            options.push(OptionPacket {
                option: option.id(),
                subject: option.subject,
                evidence: option.evidence,
                default: DefaultValue::from_canonical(default),
            });
        }
        for callable in c.callables.iter().filter(|r| r.member == id) {
            let assessment = n
                .assessments
                .get(callable.assessment)
                .ok_or(Error::Contract)?;
            knowledge.push(assessment.signatures);
            basis.insert(callable.basis as i16);
            for invocation in c.invocations.iter().filter(|r| r.callable == callable.id()) {
                invocations.push(InvocationPacket {
                    callable: callable.id(),
                    invocation: invocation.id(),
                    assessment: assessment.id(),
                    analysis: assessment.context,
                    knowledge: assessment.signatures,
                    form: Nullable(assessment.descriptor_kind.map(|kind| match kind {
                        DescriptorKind::Function => selection::InvocationForm::Function,
                        DescriptorKind::InstanceMethod => selection::InvocationForm::Method,
                        DescriptorKind::StaticMethod => selection::InvocationForm::Static,
                        DescriptorKind::ClassMethod => selection::InvocationForm::Class,
                        DescriptorKind::Property => selection::InvocationForm::Property,
                    })),
                });
                let variant = n.variants.get(invocation.variant).ok_or(Error::Contract)?;
                let source = f.signatures.get(variant.signature).ok_or(Error::Contract)?;
                let mut parameters = Vec::new();
                let mut effective_parameters = Vec::new();
                for parameter in f
                    .signature_parameters
                    .iter()
                    .filter(|r| r.signature == source.id())
                {
                    let shape = f.shapes.get(parameter.shape).ok_or(Error::Contract)?;
                    let formals = n
                        .parameter_links
                        .iter()
                        .filter(|l| l.parameter == parameter.id())
                        .map(|l| l.entity)
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>();
                    let mut types = BTreeSet::new();
                    for formal in &formals {
                        if let Some(ParameterEntity::Source { declaration }) =
                            n.parameters.get(*formal)
                        {
                            for observation in f.type_observations.iter().filter(|r| {
                                r.subject == *declaration && r.role == types::TypeRole::Parameter
                            }) {
                                if n.qualifications
                                    .get(observation.qualification)
                                    .is_some_and(|q| q.context == variant.context)
                                {
                                    types.insert(observation.term);
                                }
                            }
                        }
                    }
                    let mut source_default = if shape.required {
                        DefaultValue::Absent {}
                    } else {
                        DefaultValue::Unknown {}
                    };
                    for option in c.options.iter().filter(|o| o.member == id) {
                        if let Some(CatalogOptionSubject::SourceParameter { parameter }) =
                            c.subjects.get(option.subject)
                            && formals.contains(parameter)
                        {
                            source_default = DefaultValue::from_canonical(
                                c.defaults.get(option.default).ok_or(Error::Contract)?,
                            );
                        }
                    }
                    let slots = n
                        .slots
                        .iter()
                        .filter(|s| s.variant == variant.id() && s.parameter == parameter.id())
                        .collect::<Vec<_>>();
                    let source_slot = if slots.len() == 1 {
                        Some(slots[0].id())
                    } else {
                        None
                    };
                    let mut raw = ParameterPacket {
                        parameter: parameter.id(),
                        slot: Nullable(source_slot),
                        formals: formals.clone(),
                        ordinal: parameter.ordinal,
                        name: Nullable(shape.name.as_ref().map(|n| name(n.as_str())).transpose()?),
                        kind: shape.kind,
                        required: shape.required,
                        types: types.into_iter().collect(),
                        default: source_default,
                    };
                    for slot in slots {
                        let mut effective = raw.clone();
                        effective.slot = Nullable(Some(slot.id()));
                        effective.ordinal = slot.ordinal;
                        effective.formals = n
                            .slot_entities
                            .iter()
                            .filter(|e| e.slot == slot.id())
                            .map(|e| {
                                n.parameter_links
                                    .get(e.link)
                                    .map(|l| l.entity)
                                    .ok_or(Error::Contract)
                            })
                            .collect::<Result<BTreeSet<_>, _>>()?
                            .into_iter()
                            .collect();
                        let default_rows=c.options.iter().filter(|o|o.member==id&&matches!(c.subjects.get(o.subject),Some(CatalogOptionSubject::Parameter{slot:id})if *id==slot.id())).map(|o|c.defaults.get(o.default).ok_or(Error::Contract)).collect::<Result<Vec<_>,_>>()?;
                        effective.default = if default_rows.len() == 1 {
                            DefaultValue::from_canonical(default_rows[0])
                        } else if default_rows.is_empty() {
                            match slot.default {
                                DefaultSlot::Required | DefaultSlot::Collector => {
                                    DefaultValue::Absent {}
                                }
                                DefaultSlot::DefinitionTime => DefaultValue::Unknown {},
                            }
                        } else if default_rows.windows(2).all(|w| w[0] == w[1]) {
                            DefaultValue::from_canonical(default_rows[0])
                        } else {
                            DefaultValue::Unknown {}
                        };
                        raw.default = effective.default.clone();
                        effective_parameters.push(effective);
                    }
                    parameters.push(raw);
                }
                parameters.sort_by_key(|p| (p.ordinal, p.parameter));
                effective_parameters.sort_by_key(|p| (p.ordinal, p.parameter));
                let mut returns = BTreeSet::new();
                if let Some(CallableEntity::Source { declaration, .. }) =
                    n.source_callables.get(assessment.callable)
                {
                    for observation in f
                        .type_observations
                        .iter()
                        .filter(|r| r.subject == *declaration && r.role == types::TypeRole::Return)
                    {
                        if n.qualifications
                            .get(observation.qualification)
                            .is_some_and(|q| q.context == variant.context)
                        {
                            returns.insert(observation.term);
                        }
                    }
                }
                signatures.push(SignaturePacket {
                    source: source.id(),
                    variant: variant.id(),
                    analysis: variant.context,
                    form: source.form,
                    adjustment: variant.adjustment,
                    parameters,
                    effective_parameters,
                    return_types: returns.into_iter().collect(),
                    complete: assessment.signatures == Knowledge::Known
                        && source.form == SignatureForm::List
                        && variant.adjustment != SignatureAdjustment::Unknown,
                });
            }
        }
        let terms = signatures
            .iter()
            .flat_map(|s| {
                s.parameters
                    .iter()
                    .flat_map(|p| p.types.iter().copied())
                    .chain(s.return_types.iter().copied())
            })
            .collect::<BTreeSet<_>>();
        for term in terms {
            if let Some(types::TypeTerm::Literal { value }) = f.type_terms.get(term) {
                literal_ids.insert(*value);
            }
        }
        let literal_values = literal_ids
            .into_iter()
            .map(|id| {
                f.literals
                    .get(id)
                    .ok_or(Error::Contract)
                    .and_then(|r| LiteralPacket::from_canonical(r).map_err(Error::from))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let signature_knowledge = if knowledge.contains(&Knowledge::Conflicting) {
            Knowledge::Conflicting
        } else if !knowledge.is_empty() && knowledge.iter().all(|k| *k == Knowledge::Known) {
            Knowledge::Known
        } else {
            Knowledge::Unknown
        };
        Ok(OperationCore {
            member: id,
            name: name(self.path(m)?.join("."))?,
            release: ReleaseIdentity {
                input: m.input,
                release: release_id,
                distribution: name(package.name.clone())?,
                version: name(release.version.clone())?,
            },
            access: AccessProvenance {
                module: m.access,
                path: self
                    .path(m)?
                    .into_iter()
                    .map(name)
                    .collect::<Result<Vec<_>, _>>()?,
                exposures,
                candidates,
                basis: Nullable(if basis.len() == 1 {
                    c.callables.iter().find(|r| r.member == id).map(|r| r.basis)
                } else {
                    None
                }),
            },
            invocations,
            signatures,
            signature_knowledge,
            options,
            literal_values,
            type_presentations: vec![],
            limits: PacketLimits {
                maximum_page_rows: ResourceLimits::default().maximum_page_rows,
                maximum_response_bytes: ResourceLimits::default().response_bytes(expanded),
                signature_indivisible: true,
            },
        })
    }
}
fn check_indivisible<T: serde::Serialize>(value: &T, expanded: bool) -> Result<(), Error> {
    let bytes = serialized_len(value)?;
    if bytes as u64 > ResourceLimits::default().response_bytes(expanded) {
        return Err(Error::ResourceRefused(
            "indivisible mandatory signature packet",
        ));
    }
    Ok(())
}
