//! Requested sections use their existing semantic kernels and exact captured sources.
use crate::records::{need, rows, wire, Prepared,PacketRows};
use lctx_model::domain::{
    resources::ResourceBudget, serving::mappings::PacketOutput, serving::*, *,
};
use lctx_native::{NativeReader, RecordSelection, reader::target_id};
fn key<R: Record>(id: Id<R>) -> Result<ContentHash, ModelError> {
    match graph::target_for_row(derivation::RowRef::of(id))? {
        graph::Target::Entity(id) => Ok(id.0),
        graph::Target::Assertion(id) => Ok(id.0),
        _ => Err(ModelError::Schema("external section key")),
    }
}
pub async fn get(
    reader: &NativeReader,
    preparation: &crate::preparation::Preparation<'_>,
    member: &catalog::CatalogMember,
    domains: &[LibraryDomainPacket],
    r: &GetOperationRequest,
    request: &Request,
    limits: &ResourceLimits,
    b: &ResourceBudget,
) -> Result<OperationPacket, ModelError> {
    let mut requested = r.clone();
    if let Some(demand) = &r.page.evidence_demand.0 {
        for facet in &demand.facets {
            let section = match facet {
                EvidenceFacet::Scenarios => Some(OperationSection::Scenarios),
                EvidenceFacet::Deployment => Some(OperationSection::Deployment),
                EvidenceFacet::Relationships => Some(OperationSection::Relationships),
                EvidenceFacet::Behavior => Some(OperationSection::Behavior),
                _ => None,
            };
            if let Some(section) = section
                && !requested.sections.contains(&section)
            {
                requested.sections.push(section);
            }
        }
    }
    let r = &requested;
    let mut input = OperationCore::binding().lowered().sources;
    for section in &r.sections {
        let kind = match section {
            OperationSection::AccessRoutes => mappings::PacketKind::AccessRoutePacket,
            OperationSection::CallableComparison => mappings::PacketKind::CallableComparisonPacket,
            OperationSection::ContextualTyping => mappings::PacketKind::ContextualTypePacket,
            OperationSection::IncomingReferences => mappings::PacketKind::IncomingReferencePacket,
            OperationSection::Scenarios => mappings::PacketKind::ScenarioPacket,
            OperationSection::Deployment => mappings::PacketKind::DeploymentPacket,
            OperationSection::Relationships => mappings::PacketKind::RelationshipPacket,
            OperationSection::Conflicts => mappings::PacketKind::OperationCandidate,
            OperationSection::Briefs => mappings::PacketKind::CapabilityPacket,
            OperationSection::Behavior => mappings::PacketKind::BehaviorPacket,
        };
        input.extend(kind.binding().lowered().sources);
    }
    input.sort_by_key(Relation::name);
    input.dedup_by_key(|r| r.name());
    let inputs = input
        .iter()
        .map(|r| ValidationInput::of_relation(r, &["id"]))
        .collect::<Vec<_>>();
    let mut roots = vec![target_id(graph::Target::Entity(graph::EntityId::of(
        member.id(),
    )))];
    let mut fields = crate::scope::OWNED_FIELDS.to_vec();
    fields.extend([
        "parent",
        "scenario",
        "association",
        "brief",
        "characterization",
        "reader",
        "class",
        "parameter_option",
        "field_option",
        "entity",
        "link",
    ]);
    if r.sections.contains(&OperationSection::ContextualTyping) {
        fields.push("owner");
    }
    if r.sections.contains(&OperationSection::Behavior) {
        fields.extend(["transfer", "alternative"]);
    }
    if r.sections.contains(&OperationSection::AccessRoutes) {
        fields.extend(["statement", "alias"]);
    }
    if r.sections.contains(&OperationSection::IncomingReferences) {
        let artifacts = reader
            .records::<source::SourceArtifact>(RecordSelection::Scope {
                field: "input".into(),
                values: vec![serde_json::to_value(member.input).map_err(ModelError::codec)?],
            })
            .await?;
        roots.extend(
            artifacts
                .iter()
                .map(|a| target_id(graph::Target::Entity(graph::EntityId::of(a.id())))),
        );
        fields.extend(["source", "read", "artifact"]);
    }
    if r.sections.contains(&OperationSection::Deployment) {
        for capture in domains
            .iter()
            .flat_map(|d| &d.captures)
            .filter(|c| c.release.input == member.input)
        {
            roots.push(target_id(graph::Target::Entity(graph::EntityId::of(
                capture.release.release,
            ))));
        }
        fields.push("release");
    }
    let source = preparation.hydrate(reader, roots, &inputs, &inputs, &fields).await?;
    let source=std::sync::Arc::new(Prepared::new(&source,b));
    let mut core = crate::core::packet(&source, member, domains, limits, b)?;
    crate::defaults::read_originals(reader, &mut core.interpretation, request, b).await?;
    core.limits.maximum_response_bytes = limits.response_bytes(request.page().expanded);
    let mut data = selection::classification::ClassificationData::new(b);
    let mut output = selection::build::Output::new(b);
    for (name, batch) in &source.batches {
        data.visit(name, batch)?;
        output.visit(name, batch)?;
    }
    let quiet = ChannelState {
        lexical: false,
        vector: VectorChannel::Disabled {},
    };
    let _section_metadata = b.reserve("operation-section-leases", 16 * size_of::<charged::StateCharge>())?;
    let mut section_charges = Vec::with_capacity(16);
    macro_rules! paged {
        ($ty:ty,$name:literal,$section:ident,$values:expr) => {{
            let values: Vec<(ContentHash, $ty)> = $values;
            if r.sections.contains(&OperationSection::$section) {
                let mut collector = crate::pagination::PageCollector::new(request, reader.handle(), &quiet,
                    request.tool().name(), $name, Some(member.id()), b);
                for (key, value) in values { collector.push(key, value)?; }
                let (page, charge) = collector.finish(Availability::Available {})?;
                section_charges.push(charge);
                page
            } else {
                SectionPage {
                    availability: Availability::NotRequested {},
                    items: Vec::new(),
                    continuation: Optional::default(),
                    omitted: 0,
                    truncated: false,
                }
            }
        }};
    }
    let access_routes = if r.sections.contains(&OperationSection::AccessRoutes) {
        let mut extra = catalog::access_routes::RouteData::new(b);
        for (name, batch) in &source.batches {
            extra.visit(name, batch)?;
        }
        let result = catalog::access_routes::explain(
            &data,
            &extra,
            member.id(),
            limits.explanation_depth as usize,
            limits.maximum_page_rows as usize,
            b,
        )?;
        let availability = if result.partial {
            Availability::Partial {
                reason: Name::new("access_route_frontier").map_err(wire)?,
            }
        } else {
            Availability::Available {}
        };
        let mut collector = crate::pagination::PageCollector::new(request, reader.handle(), &quiet,
            request.tool().name(), "access_routes", Some(member.id()), b);
        for packet in result.routes { collector.push(crate::pagination::packet_key(&packet)?, packet)?; }
        let (page, charge) = collector.finish(availability)?;
        section_charges.push(charge); page
    } else {
        paged!(AccessRoutePacket, "access_routes", AccessRoutes, Vec::new())
    };
    let callable_comparison = if let Some(c) = &r.comparison.0 {
        let result = normalized::contract_comparison::compare(
            &data,
            member.id(),
            c.analysis,
            c.left,
            c.right,
            b,
        )?;
        paged!(
            CallableComparisonPacket,
            "callable_comparison",
            CallableComparison,
            vec![(
                crate::pagination::packet_key(&result.value)?,
                result.value
            )]
        )
    } else {
        paged!(
            CallableComparisonPacket,
            "callable_comparison",
            CallableComparison,
            Vec::new()
        )
    };
    let contextual_typing = if r.sections.contains(&OperationSection::ContextualTyping) {
        let result = types::contextual::explain(&data, member.id(), b)?;
        paged!(
            ContextualTypePacket,
            "contextual_typing",
            ContextualTyping,
            result
                .value
                .into_iter()
                .map(|p| Ok((
                    crate::pagination::packet_key(&p)?,
                    p
                )))
                .collect::<Result<Vec<_>, ModelError>>()?
        )
    } else {
        paged!(
            ContextualTypePacket,
            "contextual_typing",
            ContextualTyping,
            Vec::new()
        )
    };
    let (incoming_references, reference_scope) =
        if r.sections.contains(&OperationSection::IncomingReferences) {
            let result = normalized::incoming_references::incoming(
                &data,
                member.id(),
                r.reference_parameter.0,
                b,
            )?;
            let scope = result.value.scope;
            let values = result
                .value
                .references
                .into_iter()
                .map(|p| Ok((key(p.reference)?, p)))
                .collect::<Result<Vec<_>, ModelError>>()?;
            (
                paged!(
                    IncomingReferencePacket,
                    "incoming_references",
                    IncomingReferences,
                    values
                ),
                Optional(Some(scope)),
            )
        } else {
            (
                paged!(
                    IncomingReferencePacket,
                    "incoming_references",
                    IncomingReferences,
                    Vec::new()
                ),
                Optional::default(),
            )
        };
    let diagnostic_targets = rows::<catalog::evidence::DiagnosticUseTarget>(&source)?;
    let diagnostic_links = rows::<catalog::evidence::DiagnosticUseLink>(&source)?;
    let diagnostic_assessments = rows::<catalog::evidence::DiagnosticUseAssessment>(&source)?;
    let associations = rows::<catalog::evidence::ScenarioAssociation>(&source)?;
    let scenarios = rows::<catalog::evidence::CatalogScenario>(&source)?;
    let spans = rows::<catalog::evidence::ScenarioSpan>(&source)?;
    let q = rows::<assertion::AssertionQualification>(&source)?;
    let member_associations = if r.sections.contains(&OperationSection::Scenarios) {
        Some(associations.select_for("member", &[member.id()])?)
    } else { None };
    let scenario_count = member_associations.as_ref().map_or(0, |rows| rows.len());
    let _scenario_order = b.reserve("compact-scenario-page", scenario_count.checked_mul(256)
        .ok_or(ModelError::Schema("compact scenario page overflow"))?)?;
    let mut scenario_values = Vec::with_capacity(scenario_count);
    if let Some(associations) = &member_associations {
        let mut originals = None;
        for association in associations {
            let scenario = need(&scenarios, association.scenario)?;
            let context = need(&q, association.qualification)?.context;
            // Every off-page original still earns the same address/attribution checks.
            // Only the selected parent will retain a rich span vector and packet body.
            for span in &spans.select_for("scenario", &[scenario.id()])? {
                if originals.is_none() { originals = Some(crate::originals::Prepared::new(&source)?); }
                originals.as_ref().expect("prepared original ranges").range(&OriginalReference::Catalog { source: span.source },
                    Some(context), Some(core.release.release))?;
            }
            scenario_values.push((key(association.id())?,
                (association.id(), (association, scenario, context))));
        }
    }
    let parents = if r.sections.contains(&OperationSection::Scenarios) {
        crate::pagination::scenario_parents(
            scenario_values,
            request,
            reader.handle(),
            &quiet,
            member.id(),
        )
        .map_err(wire)?
    } else {
        SectionPage {
            availability: Availability::NotRequested {},
            items: Vec::new(),
            continuation: Optional::default(),
            omitted: 0,
            truncated: false,
        }
    };
    let mut scenario_items = Vec::new();
    for (association, (association_row, scenario, context)) in parents.items {
        let originals = spans.select_for("scenario", &[scenario.id()])?.iter().map(|span| {
            crate::originals::range(&source, &OriginalReference::Catalog { source: span.source },
                Some(context), Some(core.release.release))
        }).collect::<Result<Vec<_>, _>>()?;
        let diagnostic_correlations = crate::pagination::scenario_diagnostics(
            scenario_diagnostics(
                association,
                &diagnostic_targets,
                &diagnostic_links,
                &diagnostic_assessments,
            )?,
            association,
            request,
            reader.handle(),
            &quiet,
            member.id(),
        )
        .map_err(wire)?;
        scenario_items.push(ScenarioPacket {
            scenario: scenario.id(), intent: association_row.intent, basis: association_row.basis,
            spans: originals, diagnostic_correlations,
            parse: scenario.parse, binding: scenario.binding,
            environment: scenario.environment, execution: scenario.execution,
        });
    }
    let scenarios = SectionPage {
        availability: parents.availability,
        items: scenario_items,
        continuation: parents.continuation,
        omitted: parents.omitted,
        truncated: parents.truncated,
    };
    let deployment = if r.sections.contains(&OperationSection::Deployment) {
        let deployments = rows::<catalog::evidence::CatalogDeployment>(&source)?;
        let observations = rows::<deployment::DeploymentObservation>(&source)?;
        let links = rows::<catalog::evidence::ReleaseDeployment>(&source)?;
        let links = links.select_for("release", &[core.release.release])?;
        let _compact = b.reserve("compact-deployment-page", links.len().saturating_mul(256))?;
        let mut values = Vec::new();
        for link in &links {
            let deployment = need(&deployments, link.deployment)?;
            let observation = need(&observations, deployment.observation)?;
            let context = need(&q, observation.qualification)?.context;
            Name::new(observation.field.clone()).map_err(wire)?;
            Name::new(observation.name.clone().unwrap_or_else(|| observation.field.clone())).map_err(wire)?;
            Text::<0,8192>::new(observation.original.clone()).map_err(wire)?;
            crate::originals::range(&source, &OriginalReference::Span { span: observation.span }, Some(context), Some(link.release))?;
            values.push((key(deployment.id())?, (deployment.id(), observation, context, link.release)));
        }
        let compact = crate::pagination::page(values, request, reader.handle(), &quiet,
            request.tool().name(), "deployment", Some(member.id()), Availability::Available {}).map_err(wire)?;
        crate::operations::hydrate_page(compact, |(deployment, observation, context, release)| Ok(DeploymentPacket {
            deployment, field: Name::new(observation.field.clone()).map_err(wire)?,
            name: Name::new(observation.name.clone().unwrap_or_else(|| observation.field.clone())).map_err(wire)?,
            value: Text::new(observation.original.clone()).map_err(wire)?,
            originals: vec![crate::originals::range(&source, &OriginalReference::Span { span: observation.span }, Some(context), Some(release))?],
        }))?
    } else { paged!(DeploymentPacket, "deployment", Deployment, Vec::new()) };
    let mut conflict_collector = crate::pagination::PageCollector::new(request, reader.handle(), &quiet,
        request.tool().name(), "conflicts", Some(member.id()), b);
    if r.sections.contains(&OperationSection::Conflicts) {
        let prepared = selection::evaluate::Prepared::from_local_rows(data, output, b)?;
        let requirements: Vec<_> = crate::vocabulary::questions(prepared.data())
            .into_iter()
            .map(|(_, _, predicate)| selection::Requirement {
                predicate,
                quantifier: selection::Quantifier::AnyApplicable,
            })
            .collect();
        for requirements in requirements.chunks(selection::algebra::MAX_REQUIREMENTS) {
            let selected = prepared.select(
                &selection::Selection {
                    requirements: requirements.to_vec(),
                    mode: selection::Mode::Discovery,
                    joint: selection::JointPolicy::IndependentRecords,
                },
                b,
            )?;
            for c in selected
                .candidates
                .iter()
                .filter(|c| c.member == member.id())
            {
                let packet = crate::candidates::packet(c, prepared.data(), domains)?;
                for r in packet
                    .requirements
                    .into_iter()
                    .filter(|r| r.outcome == selection::Outcome::Conflicting)
                {
                    let p = ConflictPacket {
                        requirement: r.requirement,
                        reason: r.reason,
                        contexts: r.contexts,
                        positive: r.positive,
                        negative: r.negative,
                    };
                    conflict_collector.push(crate::pagination::packet_key(&p)?, p)?;
                }
            }
        }
    }
    let conflicts = if r.sections.contains(&OperationSection::Conflicts) {
        let (page, charge) = conflict_collector.finish(Availability::Available {})?;
        section_charges.push(charge); page
    } else { paged!(ConflictPacket, "conflicts", Conflicts, Vec::new()) };
    let briefs = if r.sections.contains(&OperationSection::Briefs) {
        let links = rows::<synthesis::briefs::BriefAssertion>(&source)?;
        let assertions = rows::<synthesis::assertions::ProgrammaticAssertion>(&source)?;
        let invocations = rows::<catalog::CatalogMemberInvocation>(&source)?;
        let mut ids = std::collections::BTreeSet::new();
        for link in &links {
            if let Some(assertion) = assertions.get(link.assertion)?
                && invocations.get(assertion.member)?.is_some_and(|invocation| invocation.member == member.id()) {
                ids.insert(link.brief);
            }
        }
        let prepared = crate::capability::Prepared::new(&source, b)?;
        let mut values = Vec::new();
        let _compact = b.reserve("compact-operation-briefs", ids.len().saturating_mul(256))?;
        for id in ids {
            prepared.validate(id, b)?;
            values.push((key(id)?, id));
        }
        let compact = crate::pagination::page(values, request, reader.handle(), &quiet,
            request.tool().name(), "briefs", Some(member.id()), Availability::Available {}).map_err(wire)?;
        crate::operations::hydrate_page(compact, |id| prepared.packet(id, b))?
    } else { paged!(CapabilityPacket, "briefs", Briefs, Vec::new()) };
    let (relationship_values, _relationship_charge) = relationships(&source, member.id())?;
    let relationships = if r.sections.contains(&OperationSection::Relationships) {
        let compact = crate::pagination::page(relationship_values, request, reader.handle(), &quiet,
            request.tool().name(), "relationships", Some(member.id()), Availability::Available {}).map_err(wire)?;
        crate::operations::hydrate_page(compact, |selected| relationship_packet(&source, selected))?
    } else { paged!(RelationshipPacket, "relationships", Relationships, Vec::new()) };
    let behavior = if r.sections.contains(&OperationSection::Behavior) {
        let mut collector = crate::pagination::PageCollector::new(request, reader.handle(), &quiet,
            request.tool().name(), "behavior", Some(member.id()), b);
        let availability = crate::behavior::visit_packets(&source, member.id(), &mut |packet| {
            collector.push(crate::pagination::packet_key(&packet)?, packet)
        }).await?;
        let (page, charge) = collector.finish(availability)?;
        section_charges.push(charge); page
    } else { paged!(BehaviorPacket, "behavior", Behavior, Vec::new()) };
    Ok(OperationPacket {
        core,
        access_routes,
        callable_comparison,
        contextual_typing,
        incoming_references,
        reference_scope,
        scenarios,
        deployment,
        relationships,
        conflicts,
        briefs,
        behavior,
    })
}
pub(crate) fn compact_push<T>(values: &mut Vec<T>, charge: &mut charged::StateCharge, value: T) -> Result<(), ModelError> {
    if values.len() == values.capacity() {
        let additional = values.capacity().max(4);
        charge.grow(additional.checked_mul(3 * size_of::<T>()).ok_or(ModelError::Schema("compact section capacity overflow"))?)?;
        values.reserve_exact(additional);
    }
    values.push(value); Ok(())
}
enum RelationshipSelection {
    Invocation { target: Id<catalog::CatalogMember>, analysis: Id<attribution::AnalysisContext>,
        fidelity: selection::Fidelity, alternative: Id<normalized::events::NormalizedCallAlternative> },
    SourceField { link: Id<catalog::evidence::SourceFieldLink> },
}
fn relationships(
    source: &Prepared<'_>,
    member: Id<catalog::CatalogMember>,
) -> Result<(Vec<(ContentHash, RelationshipSelection)>, charged::StateCharge), ModelError> {
    let callables = rows::<catalog::CatalogCallable>(source)?;
    let assessments = rows::<normalized::callables::EffectiveCallableAssessment>(source)?;
    let classes = rows::<catalog::CatalogClass>(source)?;
    let mut owned=std::collections::BTreeSet::new();
    for callable in callables.select_for("member",&[member])?.iter(){
        if let Some(assessment)=assessments.get(callable.assessment)?{owned.insert(normalized::entities::EntityRef::Callable{callable:assessment.callable}.id());}
    }
    owned.extend(classes.select_for("member",&[member])?.iter().map(|class|normalized::entities::EntityRef::Class{class:class.class}.id()));
    let witnesses=rows::<selection::Witness>(source)?;
    let events = rows::<normalized::events::NormalizedCallEvent>(source)?;
    let ownership = rows::<normalized::entities::OccurrenceOwnership>(source)?;
    let alternatives = rows::<normalized::events::NormalizedCallAlternative>(source)?;
    let mut values = Vec::new();
    let mut charge = charged::StateCharge::new(source.budget(), "compact-relationship-page");
    let mut assessments_checked=false;
    for event in &events {
        if !owned.contains(&need(&ownership, event.owner)?.entity) {
            continue;
        }
        let event_alternatives=alternatives.select_for("event",&[event.id()])?;
        for alternative in &event_alternatives {
            let Some(entity) = alternative.entity else {
                continue;
            };
            // The previous demanded target loop required every callable assessment,
            // including unrelated callables. Preserve that missing-row refusal once.
            if !assessments_checked{for callable in &callables{need(&assessments,callable.assessment)?;}assessments_checked=true;}
            let target_assessments=assessments.select_derived("@relationship_callable_entity",&[entity],|assessment|normalized::entities::EntityRef::Callable{callable:assessment.callable}.id())?;
            let target_callables=callables.select_for("assessment",&target_assessments.iter().map(Record::id).collect::<Vec<_>>())?;
            for callable in &target_callables {
                witnesses.select_for("member", &[callable.member])?;
                ProofReference::from_canonical(derivation::RowRef::of(alternative.id()))?;
                compact_push(&mut values, &mut charge, (key(alternative.id())?, RelationshipSelection::Invocation {
                    target: callable.member, analysis: event.context,
                    fidelity: if alternative.status == normalized::entities::ResolutionStatus::Resolved {
                        selection::Fidelity::ResolvedTarget
                    } else { selection::Fidelity::CandidateTargets },
                    alternative: alternative.id(),
                }))?;
            }
        }
    }
    let links = rows::<catalog::evidence::SourceFieldLink>(source)?;
    let options = rows::<catalog::CatalogOption>(source)?;
    let associations = rows::<normalized::symbolic_fields::SourceFieldAssociation>(source)?;
    let readers = rows::<normalized::symbolic_fields::SourceFieldReader>(source)?;
    let reader_links = rows::<normalized::symbolic_fields::SourceFieldReaderLink>(source)?;
    let q = rows::<assertion::AssertionQualification>(source)?;
    let member_options=options.select_for("member",&[member])?;
    for link in &links {
        if member_options.get(link.parameter_option)?.is_none()&&member_options.get(link.field_option)?.is_none() {
            continue;
        }
        let a = need(&associations, link.association)?;
        let l = need(&reader_links, link.reader)?;
        let r = need(&readers, l.reader)?;
        need(&q, a.qualification)?;
        for premise in [derivation::RowRef::of(link.id()), derivation::RowRef::of(a.id()), derivation::RowRef::of(l.id())] {
            ProofReference::from_canonical(premise)?;
        }
        let _ = r; // The reader companion is required even for off-page links.
        compact_push(&mut values, &mut charge, (key(link.id())?, RelationshipSelection::SourceField { link: link.id() }))?;
    }
    Ok((values, charge))
}

fn relationship_packet(source: &Prepared<'_>, selected: RelationshipSelection) -> Result<RelationshipPacket, ModelError> {
    match selected {
        RelationshipSelection::Invocation { target, analysis, fidelity, alternative } => {
            let witnesses = rows::<selection::Witness>(source)?.select_for("member", &[target])?;
            Ok(RelationshipPacket::Invocation { target, analysis, role: selection::RelationRole::Invokes, fidelity,
                witnesses: witnesses.iter().map(Record::id).collect(),
                proof: vec![ProofReference::from_canonical(derivation::RowRef::of(alternative))?] })
        }
        RelationshipSelection::SourceField { link } => {
            let links = rows::<catalog::evidence::SourceFieldLink>(source)?;
            let link = need(&links, link)?;
            let associations = rows::<normalized::symbolic_fields::SourceFieldAssociation>(source)?;
            let association = need(&associations, link.association)?;
            let reader_links = rows::<normalized::symbolic_fields::SourceFieldReaderLink>(source)?;
            let reader_link = need(&reader_links, link.reader)?;
            let readers = rows::<normalized::symbolic_fields::SourceFieldReader>(source)?;
            let reader = need(&readers, reader_link.reader)?;
            let qualifications = rows::<assertion::AssertionQualification>(source)?;
            Ok(RelationshipPacket::SourceField { link: link.id(), parameter_option: link.parameter_option,
                field_option: link.field_option, parameter: association.parameter, association: association.id(),
                reader_link: reader_link.id(), reader: reader.id(), access: reader.access, owner: link.reader_owner,
                analysis: need(&qualifications, association.qualification)?.context,
                source_association: link.source_association, runtime_value: link.runtime_value,
                proof: [derivation::RowRef::of(link.id()), derivation::RowRef::of(association.id()), derivation::RowRef::of(reader_link.id())]
                    .into_iter().map(ProofReference::from_canonical).collect::<Result<Vec<_>, _>>()?,
            })
        }
    }
}

/// Preserve stored diagnostic correspondence; containment alone never supplies a target.
fn scenario_diagnostics(
    association: Id<catalog::evidence::ScenarioAssociation>,
    targets: &PacketRows<catalog::evidence::DiagnosticUseTarget>,
    links: &PacketRows<catalog::evidence::DiagnosticUseLink>,
    assessments: &PacketRows<catalog::evidence::DiagnosticUseAssessment>,
) -> Result<Vec<(ContentHash, Id<catalog::evidence::DiagnosticUseAssessment>)>, ModelError> {
    let mut selected = std::collections::BTreeSet::new();
    for target in &targets.select_for("association",&[association])? {
        let link = need(links, target.link)?;
        need(assessments, link.assessment)?;
        selected.insert(link.assessment);
    }
    selected.into_iter().map(|id| Ok((key(id)?, id))).collect()
}
#[cfg(test)]
mod scenario_tests {
    use super::*;
    use catalog::evidence::{
        DiagnosticUseAssessment, DiagnosticUseLink, DiagnosticUseStatus, DiagnosticUseTarget,
    };
    fn id<R: Record>(byte: u8) -> Id<R> {
        serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
    }
    #[test]
    fn diagnostic_correlations_preserve_only_exact_targets_and_refuse_missing_owned_links() {
        let association = id(1);
        let other = id(2);
        let assessment = DiagnosticUseAssessment {
            characterization: id(3),
            status: DiagnosticUseStatus::AmbiguousUse,
            uses: 2,
            remainder: false,
        };
        let unrelated = DiagnosticUseAssessment {
            characterization: id(4),
            status: DiagnosticUseStatus::UniqueUse,
            uses: 1,
            remainder: false,
        };
        let first = DiagnosticUseLink {
            assessment: assessment.id(),
            usage: id(5),
            subject: id(6),
            event_source: id(7),
        };
        let second = DiagnosticUseLink {
            usage: id(8),
            ..first.clone()
        };
        let excluded = DiagnosticUseLink {
            assessment: unrelated.id(),
            ..first.clone()
        };
        let targets = vec![
            DiagnosticUseTarget {
                association,
                link: first.id(),
            },
            DiagnosticUseTarget {
                association,
                link: second.id(),
            },
            DiagnosticUseTarget {
                association: other,
                link: excluded.id(),
            },
        ];
        let budget=ResourceBudget::fixed(1<<20).unwrap();
        let targets=PacketRows::new(targets,&budget).unwrap();
        let links = PacketRows::new(vec![first.clone(), second.clone(), excluded],&budget).unwrap();
        let assessments = PacketRows::new(vec![assessment.clone(), unrelated],&budget).unwrap();
        let actual = scenario_diagnostics(association, &targets, &links, &assessments).unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].1, assessment.id());
        assert!(scenario_diagnostics(association, &targets, &PacketRows::new(vec![second],&budget).unwrap(), &assessments).is_err());
    }
}
