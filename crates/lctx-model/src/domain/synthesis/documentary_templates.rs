//! Native authored MDX templates. Public labels describe documentary applicability only.
use super::documentary::{
    self, Data, DocumentaryConclusion, DocumentarySource, Output, ProseSlice, ProseSource,
};
use crate::domain::{
    analysis::native::NativeAssertionPremise, catalog::*, documents::*, normalized::Rows,
    resources::ResourceBudget, *,
};
use crate::{Domain, DomainCode};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ComponentRole {
    Warning = 0,
    Parameter = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum BoundaryReason {
    NativeEvidenceUnavailable = 0,
    UnknownField = 1,
    AmbiguousField = 2,
    NestedField = 3,
    MissingOriginalSpan = 4,
    ForeignContext = 5,
    InlineComponent = 6,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "synthesis_documentary_component_boundaries")]
pub struct ComponentBoundary {
    #[model(key)]
    pub member: Id<CatalogMemberInvocation>,
    #[model(key)]
    pub association: Id<evidence::DocumentAssociation>,
    #[model(key)]
    pub component: Id<DocumentComponentObservation>,
    #[model(key)]
    pub reason: BoundaryReason,
}
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(r: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    r.get(id)
        .ok_or_else(|| invalid("documentary template premise absent"))
}
fn native(
    d: &Data,
    q: Id<assertion::AssertionQualification>,
    predicate: impl Fn(&NativeAssertionPremise) -> bool,
) -> Option<Id<NativeAssertionPremise>> {
    d.native
        .iter()
        .filter(|r| {
            predicate(r)
                && d.native_qualifications.iter().any(|n| {
                    n.premise == r.id()
                        && n.qualification == q
                        && n.family == attribution::FactFamily::Docs
                        && n.status == analysis::policy::EvidenceStatus::Documented
                        && n.fidelity != attribution::Fidelity::DisplayOnly
                })
        })
        .map(Record::id)
        .min()
}
fn literal_attribute<'a>(
    d: &'a Data,
    component: DocumentNodeComponentId,
    name: &str,
    q: Id<assertion::AssertionQualification>,
) -> Result<Option<(&'a str, Id<NativeAssertionPremise>)>, ModelError> {
    let mut found = None;
    for a in d
        .component_attributes
        .iter()
        .filter(|a| a.component == component && a.qualification == q)
    {
        let value = need(&d.component_values, a.value)?;
        let DocumentAttributeValue::Literal { name: n, value } = value else {
            if matches!(value,DocumentAttributeValue::Expression{name:n,..}|DocumentAttributeValue::Bare{name:n}if n==name)
                || matches!(value, DocumentAttributeValue::Spread { .. })
            {
                return Ok(None);
            }
            continue;
        };
        if n != name {
            continue;
        }
        let Some(premise) = native(
            d,
            q,
            |p| matches!(p,NativeAssertionPremise::DocumentAttributeObservation{assertion,..}if *assertion==a.id()),
        ) else {
            return Ok(None);
        };
        if found.is_some() {
            return Ok(None);
        }
        found = Some((value.as_str(), premise));
    }
    Ok(found)
}
fn parent<'a>(
    d: &'a Data,
    row: &DocumentComponentObservation,
) -> Result<Option<&'a DocumentComponentObservation>, ModelError> {
    let Some(parent) = row.parent else {
        return Ok(None);
    };
    let mut rows = d.components.iter().filter(|r| {
        r.component == parent && r.qualification == row.qualification && r.passage == row.passage
    });
    let found = rows
        .next()
        .ok_or_else(|| invalid("native component parent unavailable"))?;
    if rows.next().is_some() || found.depth + 1 != row.depth {
        return Err(invalid(
            "native component parent ambiguous or depth differs",
        ));
    }
    Ok(Some(found))
}
fn nearest_field<'a>(
    d: &'a Data,
    row: &DocumentComponentObservation,
) -> Result<Option<&'a DocumentComponentObservation>, ModelError> {
    let mut p = parent(d, row)?;
    for _ in 0..=d.components.len() {
        let Some(row) = p else { return Ok(None) };
        if native(d,row.qualification,|p|matches!(p,NativeAssertionPremise::DocumentComponentObservation{assertion,..}if *assertion==row.id())).is_none(){return Err(invalid("nearest component ancestry lacks native evidence"));}
        if row.name.as_deref() == Some("ParamField") {
            return Ok(Some(row));
        }
        p = parent(d, row)?;
    }
    Err(invalid("document component ancestry cycle"))
}
/// This is lookup of an authored public parameter label, never source/effective correspondence.
type MatchedOptions = (Option<Id<CatalogOption>>, Option<Id<CatalogOption>>);

fn option(
    d: &Data,
    member: Id<CatalogMember>,
    name: &str,
    context: Id<attribution::AnalysisContext>,
) -> Result<Result<MatchedOptions, BoundaryReason>, ModelError> {
    let (mut effective, mut original) = (None, None);
    for option in d.options.iter().filter(|r| r.member == member) {
        let (destination, matches) = match need(&d.option_subjects, option.subject)? {
            CatalogOptionSubject::Parameter { slot } => {
                let slot = need(&d.option_slots, *slot)?;
                let variant = need(&d.option_variants, slot.variant)?;
                let p = need(&d.signature_parameters, slot.parameter)?;
                let signature = need(&d.parameter_signatures, p.signature)?;
                if variant.context != context
                    || need(&d.qualifications, signature.qualification)?.context != context
                {
                    continue;
                }
                if p.signature != variant.signature {
                    return Err(invalid("documentary effective option changes signature"));
                }
                (
                    &mut effective,
                    need(&d.parameter_shapes, p.shape)?
                        .name
                        .as_ref()
                        .is_some_and(|n| n.as_str() == name),
                )
            }
            CatalogOptionSubject::SourceParameter { parameter } => {
                let mut matches = false;
                for link in d.parameter_links.iter().filter(|l| l.entity == *parameter) {
                    let p = need(&d.signature_parameters, link.parameter)?;
                    let signature = need(&d.parameter_signatures, p.signature)?;
                    if need(&d.qualifications, signature.qualification)?.context != context {
                        continue;
                    }
                    matches |= need(&d.parameter_shapes, p.shape)?
                        .name
                        .as_ref()
                        .is_some_and(|n| n.as_str() == name);
                }
                (&mut original, matches)
            }
            _ => continue,
        };
        if matches {
            if destination.is_some() {
                return Ok(Err(BoundaryReason::AmbiguousField));
            }
            *destination = Some(option.id());
        }
    }
    Ok(if effective.is_none() && original.is_none() {
        Err(BoundaryReason::UnknownField)
    } else {
        Ok((effective, original))
    })
}
fn boundary(
    out: &mut Output,
    member: Id<CatalogMemberInvocation>,
    association: Id<evidence::DocumentAssociation>,
    component: Id<DocumentComponentObservation>,
    reason: BoundaryReason,
) -> Result<(), ModelError> {
    out.component_boundaries.insert(ComponentBoundary {
        member,
        association,
        component,
        reason,
    })?;
    Ok(())
}
pub fn build(d: &Data, out: &mut Output, b: &ResourceBudget) -> Result<(), ModelError> {
    for frame in d.member_frames.iter() {
        let inv = need(&d.core_invocations, frame.invocation)?;
        for association in d
            .document_associations
            .iter()
            .filter(|a| a.member == frame.member)
        {
            let candidate = need(&d.mention_candidates, association.candidate)?;
            let exposure = need(&d.public_exposures, candidate.exposure)?;
            if exposure.context != inv.context
                || !d
                    .exposures
                    .iter()
                    .any(|e| e.member == frame.member && e.exposure == exposure.id())
            {
                continue;
            }
            let assessment = need(&d.mention_assessments, candidate.assessment)?;
            let mention = need(&d.mentions, assessment.observation)?;
            let q = need(&d.qualifications, mention.qualification)?;
            if q.context != inv.context {
                continue;
            }
            let Some(scope) = native(
                d,
                mention.qualification,
                |p| matches!(p,NativeAssertionPremise::DocumentMentionObservation{assertion,..}if *assertion==mention.id()),
            ) else {
                continue;
            };
            for component in d.components.iter().filter(|c| c.passage == mention.passage) {
                let Some(role) = (match component.name.as_deref() {
                    Some("Warning") => Some(ComponentRole::Warning),
                    Some("ParamField") => Some(ComponentRole::Parameter),
                    _ => None,
                }) else {
                    continue;
                };
                let cq = need(&d.qualifications, component.qualification)?;
                if (cq.context, cq.scope, cq.condition) != (q.context, q.scope, q.condition) {
                    boundary(
                        out,
                        frame.id(),
                        association.id(),
                        component.id(),
                        BoundaryReason::ForeignContext,
                    )?;
                    continue;
                }
                let Some(premise) = native(
                    d,
                    component.qualification,
                    |p| matches!(p,NativeAssertionPremise::DocumentComponentObservation{assertion,..}if *assertion==component.id()),
                ) else {
                    boundary(
                        out,
                        frame.id(),
                        association.id(),
                        component.id(),
                        BoundaryReason::NativeEvidenceUnavailable,
                    )?;
                    continue;
                };
                if component.form != ComponentForm::Flow {
                    boundary(
                        out,
                        frame.id(),
                        association.id(),
                        component.id(),
                        BoundaryReason::InlineComponent,
                    )?;
                    continue;
                }
                let nearest = nearest_field(d, component)?;
                if role == ComponentRole::Parameter && nearest.is_some() {
                    boundary(
                        out,
                        frame.id(),
                        association.id(),
                        component.id(),
                        BoundaryReason::NestedField,
                    )?;
                    continue;
                }
                let field = if role == ComponentRole::Parameter {
                    Some(component)
                } else {
                    nearest
                };
                let (mut field_premise, mut option_id, mut source_option_id) = (None, None, None);
                if let Some(field) = field {
                    let Some((label, attribute)) =
                        literal_attribute(d, field.component, "body", field.qualification)?
                    else {
                        boundary(
                            out,
                            frame.id(),
                            association.id(),
                            component.id(),
                            BoundaryReason::UnknownField,
                        )?;
                        continue;
                    };
                    match option(d, frame.member, label, inv.context)? {
                        Ok((effective, original)) => {
                            field_premise = Some(attribute);
                            option_id = effective;
                            source_option_id = original;
                        }
                        Err(reason) => {
                            boundary(out, frame.id(), association.id(), component.id(), reason)?;
                            continue;
                        }
                    }
                }
                let Some(inner) = component.inner else {
                    boundary(
                        out,
                        frame.id(),
                        association.id(),
                        component.id(),
                        BoundaryReason::MissingOriginalSpan,
                    )?;
                    continue;
                };
                let excerpt = if role == ComponentRole::Parameter {
                    let Some(lead) = component.lead else {
                        boundary(
                            out,
                            frame.id(),
                            association.id(),
                            component.id(),
                            BoundaryReason::MissingOriginalSpan,
                        )?;
                        continue;
                    };
                    lead
                } else {
                    inner
                };
                let body = ProseSource::Span { span: inner };
                let clip = ProseSource::Span { span: excerpt };
                let (a, s, e) = documentary::original(d, &body)?;
                let (c, cs, ce) = documentary::original(d, &clip)?;
                let passage = ProseSource::Span {
                    span: need(&d.nodes, mention.passage.id())?.span(),
                };
                let (pa, ps, pe) = documentary::original(d, &passage)?;
                if a != c
                    || a != pa
                    || cs < s
                    || ce > e
                    || s < ps
                    || e > pe
                    || cq.scope != (source::CoverageScope::Artifact { artifact: a }).id()
                {
                    return Err(invalid(
                        "documentary component inner/lead is not its exact original passage",
                    ));
                }
                let outer = ProseSource::Span {
                    span: need(&d.nodes, component.component.id())?.span(),
                };
                let (oa, os, oe) = documentary::original(d, &outer)?;
                if oa != a || s <= os || e >= oe || cs == ce {
                    return Err(invalid(
                        "documentary component inner/lead is outside original component",
                    ));
                }
                if role == ComponentRole::Parameter {
                    for child in d.components.iter().filter(|c| {
                        c.parent == Some(component.component)
                            && c.qualification == component.qualification
                    }) {
                        let child = ProseSource::Span {
                            span: need(&d.nodes, child.component.id())?.span(),
                        };
                        let (ca, cstart, cend) = documentary::original(d, &child)?;
                        if ca != a || cs < cend && ce > cstart {
                            return Err(invalid(
                                "documentary parameter lead overlaps a nested component",
                            ));
                        }
                    }
                }
                let passages = d.passages.iter().filter(|p| {
                    p.passage == component.passage && p.qualification == component.qualification
                });
                if !passages.into_iter().any(|p|native(d,p.qualification,|n|matches!(n,NativeAssertionPremise::PassageObservation{assertion,..}if *assertion==p.id())).is_some()){boundary(out,frame.id(),association.id(),component.id(),BoundaryReason::NativeEvidenceUnavailable)?;continue;}
                let title =
                    literal_attribute(d, component.component, "title", component.qualification)?
                        .map(|(_, p)| p);
                let artifact = need(&d.artifacts, a)?;
                let name = artifact
                    .path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&artifact.path)
                    .split('.')
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if name.contains("changelog") || name == "whats-new" || name == "changes" {
                    continue;
                }
                let source_input = artifact.input;
                let source = out.sources.insert(DocumentarySource::Component {
                    member: frame.id(),
                    association: association.id(),
                    component: component.id(),
                    premise,
                    scope,
                    field: field_premise,
                    title,
                    option: option_id,
                    source_option: source_option_id,
                    role,
                    source_input,
                    source_qualification: component.qualification,
                })?;
                let prose_source = out.prose_sources.insert(body)?;
                let excerpt_source = out.prose_sources.insert(clip)?;
                let prose = out.slices.insert(ProseSlice {
                    source: prose_source,
                    start: 0,
                    end: e - s,
                })?;
                let excerpt = out.slices.insert(ProseSlice {
                    source: excerpt_source,
                    start: 0,
                    end: ce - cs,
                })?;
                let text = documentary::read_range(d, c, cs, ce, b)?;
                let qualification = assertion::AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
                    context: inv.context,
                    scope: (source::CoverageScope::Input { input: inv.input }).id(),
                    condition: conditions::Diagram::always().id(),
                    modality: attribution::Modality::Candidate,
                    approximation: assertion::Approximation::Over,
                };
                out.qualifications.insert(qualification.clone())?;
                out.conclusions.insert(DocumentaryConclusion::authored(
                    source,
                    prose,
                    excerpt,
                    ContentHash::of(text.value.as_bytes()),
                    qualification.id(),
                    analysis::policy::EvidenceStatus::Documented,
                ))?;
            }
        }
    }
    Ok(())
}
pub fn text(
    d: &Data,
    out: &Output,
    c: &DocumentaryConclusion,
    b: &ResourceBudget,
) -> Result<String, ModelError> {
    let DocumentarySource::Component {
        component,
        title,
        option,
        source_option,
        role,
        ..
    } = need(&out.sources, c.source)?
    else {
        return Err(invalid("documentary component template has another source"));
    };
    let component = need(&d.components, *component)?;
    let part = documentary::read_slice(d, out, need(&out.slices, c.excerpt)?, b)?;
    let _allowance = b.reserve("documentary-component-template", part.value.len() + 512)?;
    let base = ProseSource::Span {
        span: need(&d.nodes, component.passage.id())?.span(),
    };
    let (a, _, _) = documentary::original(d, &base)?;
    let path = &need(&d.artifacts, a)?.path;
    let heading = d
        .passages
        .iter()
        .find(|p| p.passage == component.passage && p.qualification == component.qualification)
        .and_then(|p| p.heading.as_deref())
        .unwrap_or("preamble");
    let mut label = String::new();
    if let DocumentarySource::Component {
        field: Some(field), ..
    } = need(&out.sources, c.source)?
    {
        let NativeAssertionPremise::DocumentAttributeObservation { assertion, .. } =
            need(&d.native, *field)?
        else {
            return Err(invalid(
                "documentary field label lacks exact native attribute",
            ));
        };
        let attribute = need(&d.component_attributes, *assertion)?;
        let DocumentAttributeValue::Literal { value, .. } =
            need(&d.component_values, attribute.value)?
        else {
            return Err(invalid("documentary field label is not literal"));
        };
        label.push_str(&format!(", about documented parameter label `{value}`"));
    }
    if let Some(option) = option {
        let option = need(&d.options, *option)?;
        label.push_str(&match need(&d.option_subjects, option.subject)? {
            CatalogOptionSubject::Parameter { slot } => {
                format!(", about declared effective signature slot {}", slot.hex())
            }
            CatalogOptionSubject::SourceParameter { parameter } => format!(
                ", about original source parameter {}; effective applicability remains unresolved",
                parameter.hex()
            ),
            _ => {
                return Err(invalid(
                    "documented component parameter is not a declared parameter",
                ));
            }
        });
    }
    if let Some(source_option) = source_option {
        let source = need(&d.options, *source_option)?;
        let CatalogOptionSubject::SourceParameter { parameter } =
            need(&d.option_subjects, source.subject)?
        else {
            return Err(invalid(
                "documentary source option is not a source parameter",
            ));
        };
        label.push_str(&format!(", about original source parameter {}; source/effective correspondence is not established by this label",parameter.hex()));
    }
    let titled = if let Some(title) = title {
        let premise = need(&d.native, *title)?;
        let NativeAssertionPremise::DocumentAttributeObservation { assertion, .. } = premise else {
            return Err(invalid("documentary title lacks native attribute"));
        };
        let attr = need(&d.component_attributes, *assertion)?;
        let DocumentAttributeValue::Literal { value, .. } = need(&d.component_values, attr.value)?
        else {
            return Err(invalid("documentary title is not literal"));
        };
        format!(" (“{value}”)")
    } else {
        String::new()
    };
    Ok(format!(
        "The documentation {}{titled}{label}, in `{path}` § {heading}: {}",
        if *role == ComponentRole::Warning {
            "warns"
        } else {
            "describes this parameter"
        },
        part.value
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        calls::*,
        normalized::{callables::*, entities::*},
    };
    fn id<T>(n: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    fn span(
        d: &mut Data,
        a: Id<source::SourceArtifact>,
        start: usize,
        end: usize,
    ) -> assertion::EvidenceSourceSpanId {
        let row = assertion::Evidence::SourceSpan {
            source: a,
            start: start as i64,
            end: end as i64,
        };
        let span = assertion::EvidenceSourceSpanId::of(&row).unwrap();
        d.canonical_evidence.insert(row).unwrap();
        span
    }
    fn add_component(
        d: &mut Data,
        a: Id<source::SourceArtifact>,
        text: &str,
        tag: &str,
        parent: Option<&DocumentComponentObservation>,
        label: Option<&str>,
        ordinal: i64,
    ) -> DocumentComponentObservation {
        let start = text.find(&format!("<{tag}")).unwrap();
        let open = start + text[start..].find('>').unwrap() + 1;
        let end_body = open + text[open..].find(&format!("</{tag}>")).unwrap();
        let end = end_body + tag.len() + 3;
        let inner_start =
            open + text[open..end_body].len() - text[open..end_body].trim_start().len();
        let inner_end = open + text[open..end_body].trim_end().len();
        let lead_end = inner_start
            + text[inner_start..inner_end]
                .find('\n')
                .unwrap_or(inner_end - inner_start);
        let node = DocumentNode::Component {
            span: span(d, a, start, end),
            ordinal,
        };
        let component = DocumentNodeComponentId::of(&node).unwrap();
        d.nodes.insert(node).unwrap();
        let passage = d.passages.iter().next().unwrap().clone();
        let row = DocumentComponentObservation {
            qualification: passage.qualification,
            component,
            passage: passage.passage,
            parent: parent.map(|r| r.component),
            depth: parent.map_or(0, |r| r.depth + 1),
            name: Some(tag.into()),
            form: ComponentForm::Flow,
            inner: Some(span(d, a, inner_start, inner_end)),
            lead: Some(span(d, a, inner_start, lead_end)),
        };
        let q = d.qualifications.get(row.qualification).unwrap().clone();
        d.components.insert(row.clone()).unwrap();
        documentary::tests::pair(
            d,
            NativeAssertionPremise::DocumentComponentObservation {
                assertion: row.id(),
                support: id(131 + ordinal as u8),
            },
            &q,
        );
        if let Some(label) = label {
            let value = d
                .component_values
                .insert(DocumentAttributeValue::Literal {
                    name: "body".into(),
                    value: label.into(),
                })
                .unwrap();
            let attribute = DocumentAttributeObservation {
                qualification: q.id(),
                component,
                ordinal: 0,
                value,
            };
            d.component_attributes.insert(attribute.clone()).unwrap();
            documentary::tests::pair(
                d,
                NativeAssertionPremise::DocumentAttributeObservation {
                    assertion: attribute.id(),
                    support: id(141 + ordinal as u8),
                },
                &q,
            );
        }
        row
    }
    fn add_option(d: &mut Data, name: &str, effective: bool, ordinal: i64) -> Id<CatalogOption> {
        let core = d.core_invocations.iter().next().unwrap().clone();
        let q = d
            .qualifications
            .iter()
            .find(|q| q.context == core.context)
            .unwrap()
            .clone();
        let signature = Signature {
            qualification: q.id(),
            scope: q.scope,
            symbol: id(151 + ordinal as u8),
            variant: ordinal,
            form: SignatureForm::List,
            parameters: ContentHash::of(name.as_bytes()),
        };
        d.parameter_signatures.insert(signature.clone()).unwrap();
        let shape = d
            .parameter_shapes
            .insert(ParameterShape {
                name: Some(name.into()),
                kind: ParameterKind::KeywordOnly,
                required: true,
            })
            .unwrap();
        let parameter = d
            .signature_parameters
            .insert(SignatureParameter {
                signature: signature.id(),
                ordinal: 0,
                shape,
            })
            .unwrap();
        let subject = if effective {
            let variant = d
                .option_variants
                .insert(SignatureVariant {
                    signature: signature.id(),
                    context: core.context,
                    resolution: id(161),
                    callable: None,
                    assessment: None,
                    adjustment: SignatureAdjustment::None,
                })
                .unwrap();
            let slot = d
                .option_slots
                .insert(SignatureSlot {
                    parameter,
                    variant,
                    ordinal: 0,
                    default: DefaultSlot::Required,
                })
                .unwrap();
            CatalogOptionSubject::Parameter { slot }
        } else {
            let entity = ParameterEntity::Source {
                declaration: id(171 + ordinal as u8),
            }
            .id();
            d.parameter_links
                .insert(ParameterEntityLink {
                    parameter,
                    entity,
                    declaration: None,
                })
                .unwrap();
            CatalogOptionSubject::SourceParameter { parameter: entity }
        };
        let subject = d.option_subjects.insert(subject).unwrap();
        d.options
            .insert(CatalogOption {
                member: d.member_frames.iter().next().unwrap().member,
                subject,
                evidence: id(181),
                default: id(182),
            })
            .unwrap()
    }
    fn components(out: &Output) -> usize {
        out.sources
            .iter()
            .filter(|s| matches!(s, DocumentarySource::Component { .. }))
            .count()
    }
    #[test]
    fn warning_and_parameter_replay_keep_nearest_field_and_distinct_signature_options() {
        let text = "`pkg.api.run` runs.\n\n<ParamField body=\"timeout\">\nTimeout prose.\n\n<Warning>\nNever share the token.\n</Warning>\n</ParamField>\n";
        let (b, mut d, _, a) = documentary::tests::passage_fixture(true, text, (1, 12));
        let field = add_component(&mut d, a, text, "ParamField", None, Some("timeout"), 0);
        add_component(&mut d, a, text, "Warning", Some(&field), None, 1);
        let original = add_option(&mut d, "timeout", false, 0);
        let effective = add_option(&mut d, "timeout", true, 1);
        let out = documentary::build(&d, &b).unwrap();
        assert_eq!(components(&out), 2);
        documentary::tests::replay(&d, &out, &b).unwrap();
        for source in out.sources.iter() {
            if let DocumentarySource::Component {
                option,
                source_option,
                source_input,
                ..
            } = source
            {
                assert_eq!(*option, Some(effective));
                assert_eq!(*source_option, Some(original));
                assert_eq!(*source_input, d.artifacts.get(a).unwrap().input);
            }
        }
        for c in out.conclusions.iter().filter(|c| {
            matches!(
                out.sources.get(c.source),
                Some(DocumentarySource::Component { .. })
            )
        }) {
            let rendered = text_for_test(&d, &out, c, &b);
            assert!(rendered.contains("source/effective correspondence is not established"));
            assert!(rendered.contains("guide.mdx"));
            assert!(!rendered.contains("Timeout prose. Never share"));
        }
        let core = d.core_invocations.iter().next().unwrap();
        let invocation = analysis::synthesis::Invocation::new(
            core.input,
            core.context,
            super::super::build::definition().1.id(),
            None,
            [],
        )
        .0;
        let mut invocations = Rows::new(&b);
        invocations.insert(invocation).unwrap();
        let assertions =
            super::super::assertions::build_documentary(&d, &out, &invocations, &b).unwrap();
        assert!(assertions.assertions.iter().any(|r| r.kind()
            == analysis::policy::AssertionKind::DocumentedWarning
            && r.text().contains("Never share the token.")));
        assert!(
            assertions
                .assertions
                .iter()
                .any(|r| r.kind() == analysis::policy::AssertionKind::Parameter
                    && r.text().contains("Timeout prose."))
        );
    }
    fn text_for_test(
        d: &Data,
        out: &Output,
        c: &DocumentaryConclusion,
        b: &ResourceBudget,
    ) -> String {
        super::text(d, out, c, b).unwrap()
    }
    #[test]
    fn unknown_ambiguous_and_nested_fields_are_explicit_refusals() {
        let text =
            "`pkg.api.run` runs.\n\n<ParamField body=\"timeout\">\nTimeout.\n</ParamField>\n";
        for ambiguous in [false, true] {
            let (b, mut d, _, a) = documentary::tests::passage_fixture(false, text, (1, 12));
            add_component(&mut d, a, text, "ParamField", None, Some("timeout"), 0);
            if ambiguous {
                add_option(&mut d, "timeout", true, 0);
                add_option(&mut d, "timeout", true, 1);
            }
            let out = documentary::build(&d, &b).unwrap();
            assert_eq!(components(&out), 0);
            assert!(out.component_boundaries.iter().any(|r| r.reason
                == if ambiguous {
                    BoundaryReason::AmbiguousField
                } else {
                    BoundaryReason::UnknownField
                }));
            documentary::tests::replay(&d, &out, &b).unwrap();
        }
        // A nested field cannot supply its prose as a top-level parameter description.
        let text = "`pkg.api.run` runs.\n\n<ParamField body=\"outer\">\nOuter.\n\n<Other>\nInner.\n</Other>\n</ParamField>\n";
        let (b, mut d, _, a) = documentary::tests::passage_fixture(false, text, (1, 12));
        let parent = add_component(&mut d, a, text, "ParamField", None, Some("outer"), 0);
        let mut child = add_component(&mut d, a, text, "Other", Some(&parent), Some("inner"), 1);
        d.components = replace_component(
            &d.components,
            &child,
            |r| r.name = Some("ParamField".into()),
            &b,
        );
        child.name = Some("ParamField".into());
        let q = d.qualifications.get(child.qualification).unwrap().clone();
        documentary::tests::pair(
            &mut d,
            NativeAssertionPremise::DocumentComponentObservation {
                assertion: child.id(),
                support: id(194),
            },
            &q,
        );
        let out = documentary::build(&d, &b).unwrap();
        assert!(
            out.component_boundaries
                .iter()
                .any(|r| r.reason == BoundaryReason::NestedField)
        );
    }
    fn replace_component(
        rows: &Rows<DocumentComponentObservation>,
        old: &DocumentComponentObservation,
        change: impl Fn(&mut DocumentComponentObservation),
        b: &ResourceBudget,
    ) -> Rows<DocumentComponentObservation> {
        let mut result = Rows::new(b);
        for row in rows.iter() {
            let mut row = row.clone();
            if row.id() == old.id() {
                change(&mut row);
            }
            result.insert(row).unwrap();
        }
        result
    }
    #[test]
    fn inline_unknown_and_fenced_content_do_not_acquire_template_authority() {
        let text = "`pkg.api.run` runs.\n\n<Warning>\nCaution.\n</Warning>\n\n`<Warning>`\n\n```mdx\n<Warning>not a component</Warning>\n```\n";
        let (b, mut d, _, a) = documentary::tests::passage_fixture(false, text, (1, 12));
        let warning = add_component(&mut d, a, text, "Warning", None, None, 0);
        d.components = replace_component(
            &d.components,
            &warning,
            |r| r.form = ComponentForm::Text,
            &b,
        );
        let mut inline = warning.clone();
        inline.form = ComponentForm::Text;
        let q = d.qualifications.get(inline.qualification).unwrap().clone();
        documentary::tests::pair(
            &mut d,
            NativeAssertionPremise::DocumentComponentObservation {
                assertion: inline.id(),
                support: id(193),
            },
            &q,
        );
        let out = documentary::build(&d, &b).unwrap();
        assert_eq!(components(&out), 0);
        assert!(
            out.component_boundaries
                .iter()
                .any(|r| r.reason == BoundaryReason::InlineComponent)
        );
        d.components = Rows::new(&b);
        let mut unknown = warning;
        unknown.name = Some("UnknownTemplate".into());
        d.components.insert(unknown).unwrap();
        assert_eq!(components(&documentary::build(&d, &b).unwrap()), 0);
    }
    #[test]
    fn forged_spans_and_coupled_output_erasure_refuse_replay() {
        let text = "`pkg.api.run` runs.\n\n<Warning>\nCaution.\n</Warning>\n";
        let (b, mut d, _, a) = documentary::tests::passage_fixture(false, text, (1, 12));
        let warning = add_component(&mut d, a, text, "Warning", None, None, 0);
        let mut out = documentary::build(&d, &b).unwrap();
        assert_eq!(components(&out), 1);
        let sources = out
            .sources
            .iter()
            .filter(|s| !matches!(s, DocumentarySource::Component { .. }))
            .cloned()
            .try_fold(Rows::new(&b), |mut r, s| {
                r.insert(s)?;
                Ok::<_, ModelError>(r)
            })
            .unwrap();
        out.sources = sources;
        let conclusions = out
            .conclusions
            .iter()
            .filter(|c| out.sources.get(c.source).is_some())
            .cloned()
            .try_fold(Rows::new(&b), |mut r, c| {
                r.insert(c)?;
                Ok::<_, ModelError>(r)
            })
            .unwrap();
        out.conclusions = conclusions;
        assert!(documentary::tests::replay(&d, &out, &b).is_err());
        let forged = span(&mut d, a, 0, text.len());
        d.components = replace_component(&d.components, &warning, |r| r.inner = Some(forged), &b);
        let mut altered = warning;
        altered.inner = Some(forged);
        let q = d.qualifications.get(altered.qualification).unwrap().clone();
        documentary::tests::pair(
            &mut d,
            NativeAssertionPremise::DocumentComponentObservation {
                assertion: altered.id(),
                support: id(192),
            },
            &q,
        );
        assert!(documentary::build(&d, &b).is_err());
    }
}
