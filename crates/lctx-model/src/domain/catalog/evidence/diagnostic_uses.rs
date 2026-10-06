//! Canonical primary-range correspondence. Containment and secondary locations are not use identity.
use super::{
    build::{EvidenceData, EvidenceOutput, invalid, need},
    *,
};
use crate::domain::{
    analysis::native::NativeAssertionPremise, charged::StateCharge, resources::ResourceBudget,
    syntax::SyntaxPlacement,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum DiagnosticUseStatus {
    Unassociated = 0,
    UniqueUse = 1,
    AmbiguousUse = 2,
    UnavailablePrimary = 3,
    IncompleteCorrespondence = 4,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(
    name = "catalog_diagnostic_use_assessments",
    rule = "diagnostic_use_assessment"
)]
pub struct DiagnosticUseAssessment {
    #[model(key, premise)]
    pub characterization: Id<SourceCharacterization>,
    pub status: DiagnosticUseStatus,
    pub uses: i64,
    /// Exact primary syntax correspondence was unavailable or encountered a finite ancestor bound.
    pub remainder: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_diagnostic_use_links", rule = "diagnostic_use_link")]
pub struct DiagnosticUseLink {
    #[model(key)]
    pub assessment: Id<DiagnosticUseAssessment>,
    #[model(key, premise)]
    pub usage: Id<SourceUsage>,
    #[model(key)]
    pub subject: Id<source::Occurrence>,
    #[model(key, premise)]
    pub event_source: Id<normalized::events::CallEventSource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_diagnostic_use_paths", rule = "diagnostic_use_path")]
pub struct DiagnosticUsePath {
    #[model(key)]
    pub link: Id<DiagnosticUseLink>,
    #[model(key)]
    pub ordinal: i64,
    #[model(key, premise)]
    pub placement: Id<SyntaxPlacement>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(
    name = "catalog_diagnostic_use_targets",
    rule = "diagnostic_use_target"
)]
pub struct DiagnosticUseTarget {
    #[model(key)]
    pub link: Id<DiagnosticUseLink>,
    #[model(key, premise)]
    pub association: Id<ScenarioAssociation>,
}
fn exact(
    d: &EvidenceData,
    placement: &SyntaxPlacement,
    context: Id<attribution::AnalysisContext>,
) -> bool {
    d.core
        .qualifications
        .get(placement.qualification)
        .is_some_and(|q| {
            super::build::exact(q, context)
                && q.assumptions == assumptions::AssumptionSet::empty_id()
                && q.condition == conditions::Diagram::always().id()
        })
}
pub(super) fn derive(
    d: &EvidenceData,
    out: &mut EvidenceOutput,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(b, "diagnostic-use-correspondence");
    charge.grow(
        d.core
            .occurrences
            .len()
            .saturating_mul(256)
            .saturating_add(d.core.placements.len().saturating_mul(256))
            .saturating_add(out.source_usages.len().saturating_mul(256))
            .saturating_add(d.facts.usage_event_sources.len().saturating_mul(256))
            .saturating_add(out.associations.len().saturating_mul(256)),
    )?;
    let mut coordinates = BTreeMap::<_, Vec<_>>::new();
    for row in d.core.occurrences.iter() {
        coordinates
            .entry((row.source, row.start, row.end))
            .or_default()
            .push(row.id());
    }
    let mut placements = BTreeMap::<_, Vec<_>>::new();
    for row in d.core.placements.iter() {
        placements.entry(row.occurrence).or_default().push(row);
    }
    let mut events = BTreeMap::<_, Vec<_>>::new();
    for usage in out.source_usages.iter() {
        let event = need(&d.facts.events, usage.event)?;
        events
            .entry((event.context, event.site))
            .or_default()
            .push(usage);
    }
    let mut event_sources = BTreeMap::<_, Vec<_>>::new();
    for row in d.facts.usage_event_sources.iter() {
        event_sources.entry(row.event).or_default().push(row);
    }
    let mut event_associations = BTreeMap::<_, Vec<_>>::new();
    for row in out.associations.iter() {
        let alternative = need(&d.facts.alternatives, row.alternative)?;
        event_associations
            .entry(alternative.event)
            .or_default()
            .push(row);
    }
    // Freeze diagnostic roots: new correlations never become native identity premises.
    let mut diagnostics = Vec::new();
    for row in out.source_characterizations.iter() {
        if matches!(
            need(&d.facts.characterization_native, row.native)?,
            NativeAssertionPremise::RuffDiagnosticObservation { .. }
                | NativeAssertionPremise::PyreflyDiagnosticObservation { .. }
        ) {
            charge.grow(size_of::<SourceCharacterization>().saturating_mul(4))?;
            diagnostics.push(row.clone());
        }
    }
    for diagnostic in diagnostics {
        let context = need(&d.core.qualifications, diagnostic.qualification)?.context;
        let source = need(&out.original_sources, diagnostic.source)?;
        let primary = match source {
            OriginalSource::Span { span } => match need(&d.facts.canonical_evidence, span.id())? {
                assertion::Evidence::SourceSpan { source, start, end } => {
                    if *source != diagnostic.artifact {
                        return Err(invalid(
                            "diagnostic use primary changes original artifact/view",
                        ));
                    }
                    Some((*source, *start, *end))
                }
                _ => return Err(invalid("diagnostic use primary has foreign evidence kind")),
            },
            _ => None,
        };
        let mut candidates = BTreeMap::<_, Vec<_>>::new();
        let mut remainder = false;
        if let Some(primary) = primary {
            for subject in coordinates.get(&primary).into_iter().flatten() {
                let mut current = *subject;
                let mut path = Vec::new();
                let mut visited = BTreeSet::new();
                for depth in 0..=64 {
                    charge.grow(256)?;
                    if !visited.insert(current) || depth == 64 {
                        remainder = true;
                        break;
                    }
                    if let Some(usages) = events.get(&(context, current)) {
                        for usage in usages {
                            for event_source in
                                event_sources.get(&usage.event).into_iter().flatten()
                            {
                                let native = need(&d.facts.usage_sites, event_source.observation)?;
                                if native.site != current
                                    || need(&d.core.qualifications, native.qualification)?.context
                                        != context
                                {
                                    return Err(invalid(
                                        "diagnostic use changes event source context",
                                    ));
                                }
                                let key = (usage.id(), *subject, event_source.id());
                                // Reserve each fanout copy before cloning the retained path.
                                charge.grow(
                                    256usize.saturating_add(
                                        path.len()
                                            .saturating_mul(size_of::<Id<SyntaxPlacement>>())
                                            .saturating_mul(4),
                                    ),
                                )?;
                                candidates.insert(key, path.clone());
                            }
                        }
                        // Nearest containing event only: a call nested in an argument is not the outer call.
                        break;
                    }
                    let mut admitted = placements
                        .get(&current)
                        .into_iter()
                        .flatten()
                        .filter(|p| exact(d, p, context));
                    let Some(placement) = admitted.next() else {
                        break;
                    };
                    if admitted.next().is_some() {
                        remainder = true;
                        break;
                    }
                    let Some(parent) = placement.parent else {
                        break;
                    };
                    let child = need(&d.core.occurrences, current)?;
                    let ancestor = need(&d.core.occurrences, parent)?;
                    if child.source != diagnostic.artifact
                        || ancestor.source != child.source
                        || ancestor.start > child.start
                        || child.end > ancestor.end
                    {
                        return Err(invalid(
                            "diagnostic use syntax ancestry changes source/view",
                        ));
                    }
                    path.push(placement.id());
                    current = parent;
                }
            }
        }
        let unique_uses = candidates
            .keys()
            .map(|(usage, _, _)| *usage)
            .collect::<BTreeSet<_>>();
        let unique_subjects = candidates
            .keys()
            .map(|(_, subject, _)| *subject)
            .collect::<BTreeSet<_>>();
        let unique_events = unique_uses
            .iter()
            .map(|u| need(&out.source_usages, *u).map(|r| r.event))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let status = if primary.is_none() {
            DiagnosticUseStatus::UnavailablePrimary
        } else if remainder {
            DiagnosticUseStatus::IncompleteCorrespondence
        } else if unique_events.is_empty() {
            DiagnosticUseStatus::Unassociated
        }
        // One admitted primary subject is required even when alternative paths converge.
        // Repeated native supports for that same subject/event do not create ambiguity.
        else if unique_events.len() == 1 && unique_subjects.len() == 1 {
            DiagnosticUseStatus::UniqueUse
        } else {
            DiagnosticUseStatus::AmbiguousUse
        };
        let assessment = out
            .diagnostic_use_assessments
            .insert(DiagnosticUseAssessment {
                characterization: diagnostic.id(),
                status,
                uses: unique_uses.len() as i64,
                remainder,
            })?;
        for ((usage, subject, event_source), path) in candidates {
            let link = out.diagnostic_use_links.insert(DiagnosticUseLink {
                assessment,
                usage,
                subject,
                event_source,
            })?;
            for (ordinal, placement) in path.into_iter().enumerate() {
                out.diagnostic_use_paths.insert(DiagnosticUsePath {
                    link,
                    ordinal: ordinal as i64,
                    placement,
                })?;
            }
            let event = need(&d.facts.events, need(&out.source_usages, usage)?.event)?;
            for association in event_associations.get(&event.id()).into_iter().flatten() {
                if need(&d.core.qualifications, association.qualification)?.context == context {
                    out.diagnostic_use_targets.insert(DiagnosticUseTarget {
                        link,
                        association: association.id(),
                    })?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{assertion::*, attribution::*, calls::*, lexical::SyntaxField, source::*};
    fn id<T>(n: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    fn fixture() -> (
        ResourceBudget,
        EvidenceData,
        EvidenceOutput,
        SourceArtifact,
        AssertionQualification,
    ) {
        let b = ResourceBudget::fixed(8 << 20).unwrap();
        let mut d = EvidenceData::new(&b);
        let out = EvidenceOutput::new(&b);
        let artifact =
            SourceArtifact::from_bytes(id(1), "calls.py".into(), b"consume(value)\nother(value)\n")
                .unwrap();
        d.core.artifacts.insert(artifact.clone()).unwrap();
        let q = AssertionQualification {
            context: id(2),
            scope: CoverageScope::Artifact {
                artifact: artifact.id(),
            }
            .id(),
            condition: conditions::Diagram::always().id(),
            assumptions: assumptions::AssumptionSet::empty_id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        d.core.qualifications.insert(q.clone()).unwrap();
        (b, d, out, artifact, q)
    }
    fn occurrence(
        d: &mut EvidenceData,
        a: &SourceArtifact,
        start: i64,
        end: i64,
        kind: SyntaxKind,
        role: OccurrenceRole,
        path: i32,
    ) -> Id<Occurrence> {
        d.core
            .occurrences
            .insert(Occurrence {
                source: a.id(),
                start,
                end,
                syntax_kind: kind,
                role,
                structural_path: vec![path],
            })
            .unwrap()
    }
    fn diagnostic(
        d: &mut EvidenceData,
        out: &mut EvidenceOutput,
        a: &SourceArtifact,
        q: &AssertionQualification,
        span: Option<(i64, i64)>,
        ordinal: i64,
    ) -> Id<SourceCharacterization> {
        let primary = span.map(|(start, end)| {
            let evidence = Evidence::SourceSpan {
                source: a.id(),
                start,
                end,
            };
            d.facts.canonical_evidence.insert(evidence.clone()).unwrap();
            EvidenceSourceSpanId::of(&evidence).unwrap()
        });
        let raw = diagnostics::RuffDiagnosticObservation {
            qualification: q.id(),
            artifact: a.id(),
            ordinal,
            primary,
            location: if primary.is_some() {
                diagnostics::DiagnosticLocation::Available
            } else {
                diagnostics::DiagnosticLocation::Unavailable
            },
            rule: diagnostics::SelectedRuffRule::UndefinedName,
            native_id: "F821".into(),
            native_code: "F821".into(),
            severity: diagnostics::DiagnosticSeverity::Error,
            channel: diagnostics::DiagnosticChannel::Emitted,
            message: "unknown name".into(),
            settings: ContentHash::of(b"settings"),
        };
        d.facts.ruff_diagnostics.insert(raw.clone()).unwrap();
        let native = d
            .facts
            .characterization_native
            .insert(NativeAssertionPremise::RuffDiagnosticObservation {
                assertion: raw.id(),
                support: id(3),
            })
            .unwrap();
        let source = out
            .original_sources
            .insert(
                primary.map_or(OriginalSource::Artifact { artifact: a.id() }, |span| {
                    OriginalSource::Span { span }
                }),
            )
            .unwrap();
        out.source_characterizations
            .insert(SourceCharacterization {
                native,
                qualification: q.id(),
                artifact: a.id(),
                source,
            })
            .unwrap()
    }
    fn usage(
        d: &mut EvidenceData,
        out: &mut EvidenceOutput,
        a: &SourceArtifact,
        q: &AssertionQualification,
        site: Id<Occurrence>,
        _tag: u8,
    ) -> Id<SourceUsage> {
        let raw = ProviderCallSite {
            qualification: q.id(),
            site,
            origin: CallOrigin::explicit(),
            kind: PysaSiteKind::Regular,
            caller: id(7),
            callee: PysaCalleeKind::Call,
            is_attribute: None,
        };
        d.facts.usage_sites.insert(raw.clone()).unwrap();
        let event = normalized::events::NormalizedCallEvent {
            site,
            origin: raw.origin,
            context: q.context,
            owner: id(8),
        };
        d.facts.events.insert(event.clone()).unwrap();
        d.facts
            .usage_event_sources
            .insert(normalized::events::CallEventSource {
                event: event.id(),
                observation: raw.id(),
            })
            .unwrap();
        let source = out
            .original_sources
            .insert(OriginalSource::Occurrence { occurrence: site })
            .unwrap();
        let native = d
            .facts
            .characterization_native
            .insert(NativeAssertionPremise::ProviderCallSite {
                assertion: raw.id(),
                support: id(9),
            })
            .unwrap();
        let characterization = out
            .source_characterizations
            .insert(SourceCharacterization {
                native,
                qualification: q.id(),
                artifact: a.id(),
                source,
            })
            .unwrap();
        out.source_usages
            .insert(SourceUsage {
                characterization,
                event: event.id(),
            })
            .unwrap()
    }
    #[test]
    fn primary_child_correlates_nearest_event_without_enclosing_or_secondary_inference() {
        let (b, mut d, mut out, a, q) = fixture();
        let call = occurrence(
            &mut d,
            &a,
            0,
            14,
            SyntaxKind::ExprCall,
            OccurrenceRole::Call,
            0,
        );
        let arg = occurrence(
            &mut d,
            &a,
            8,
            13,
            SyntaxKind::ExprName,
            OccurrenceRole::Read,
            1,
        );
        d.core
            .placements
            .insert(SyntaxPlacement {
                qualification: q.id(),
                occurrence: arg,
                parent: Some(call),
                field: SyntaxField::Argument,
                ordinal: 0,
            })
            .unwrap();
        let use_ = usage(&mut d, &mut out, &a, &q, call, 11);
        let diagnosis = diagnostic(&mut d, &mut out, &a, &q, Some((8, 13)), 0);
        let enclosing = diagnostic(&mut d, &mut out, &a, &q, Some((0, 26)), 1);
        let absent = diagnostic(&mut d, &mut out, &a, &q, None, 2);
        derive(&d, &mut out, &b).unwrap();
        let row = out
            .diagnostic_use_assessments
            .iter()
            .find(|r| r.characterization == diagnosis)
            .unwrap();
        assert_eq!(row.status, DiagnosticUseStatus::UniqueUse);
        assert_eq!(row.uses, 1);
        assert!(!row.remainder);
        let link = out
            .diagnostic_use_links
            .iter()
            .find(|l| l.assessment == row.id())
            .unwrap();
        assert_eq!(link.usage, use_);
        assert_eq!(link.subject, arg);
        assert_eq!(out.diagnostic_use_paths.len(), 1);
        assert_eq!(
            out.diagnostic_use_assessments
                .iter()
                .find(|r| r.characterization == enclosing)
                .unwrap()
                .status,
            DiagnosticUseStatus::Unassociated
        );
        assert_eq!(
            out.diagnostic_use_assessments
                .iter()
                .find(|r| r.characterization == absent)
                .unwrap()
                .status,
            DiagnosticUseStatus::UnavailablePrimary
        );
        let mut removed = EvidenceOutput::new(&b);
        macro_rules! clone_rows {($($f:ident:$ty:ty,)*)=>{$(removed.visit(<$ty>::NAME,&<$ty as Record>::encode(&out.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        crate::catalog_evidence_outputs!(clone_rows);
        removed.diagnostic_use_links = normalized::Rows::new(&b);
        removed.diagnostic_use_assessments = normalized::Rows::new(&b);
        assert!(
            removed.matches(&out).is_err(),
            "coupled assessment/link removal cannot retain exact C1 membership"
        );
        let denied = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(
            derive(&d, &mut removed, &denied),
            Err(ModelError::Resource { .. })
        ));
    }
    #[test]
    fn duplicate_diagnoses_keep_their_exact_use_and_ignore_same_spelled_foreign_site() {
        let (b, mut d, mut out, a, q) = fixture();
        let first = occurrence(
            &mut d,
            &a,
            0,
            14,
            SyntaxKind::ExprCall,
            OccurrenceRole::Call,
            0,
        );
        let subject = occurrence(
            &mut d,
            &a,
            8,
            13,
            SyntaxKind::ExprName,
            OccurrenceRole::Read,
            1,
        );
        let other = occurrence(
            &mut d,
            &a,
            15,
            27,
            SyntaxKind::ExprCall,
            OccurrenceRole::Call,
            2,
        );
        let other_subject = occurrence(
            &mut d,
            &a,
            21,
            26,
            SyntaxKind::ExprName,
            OccurrenceRole::Read,
            3,
        );
        d.core
            .placements
            .insert(SyntaxPlacement {
                qualification: q.id(),
                occurrence: subject,
                parent: Some(first),
                field: SyntaxField::Argument,
                ordinal: 0,
            })
            .unwrap();
        d.core
            .placements
            .insert(SyntaxPlacement {
                qualification: q.id(),
                occurrence: other_subject,
                parent: Some(other),
                field: SyntaxField::Argument,
                ordinal: 0,
            })
            .unwrap();
        let expected = usage(&mut d, &mut out, &a, &q, first, 11);
        let unrelated = usage(&mut d, &mut out, &a, &q, other, 12);
        diagnostic(&mut d, &mut out, &a, &q, Some((8, 13)), 0);
        diagnostic(&mut d, &mut out, &a, &q, Some((8, 13)), 1);
        derive(&d, &mut out, &b).unwrap();
        assert_eq!(out.diagnostic_use_assessments.len(), 2);
        assert_eq!(out.diagnostic_use_links.len(), 2);
        assert!(
            out.diagnostic_use_assessments
                .iter()
                .all(|row| row.status == DiagnosticUseStatus::UniqueUse
                    && row.uses == 1
                    && !row.remainder)
        );
        assert!(
            out.diagnostic_use_links
                .iter()
                .all(|link| link.usage == expected
                    && link.usage != unrelated
                    && link.subject == subject)
        );
    }
    #[test]
    fn converging_distinct_primary_subjects_remain_ambiguous() {
        let (b, mut d, mut out, a, q) = fixture();
        let call = occurrence(
            &mut d,
            &a,
            0,
            14,
            SyntaxKind::ExprCall,
            OccurrenceRole::Call,
            0,
        );
        let first = occurrence(
            &mut d,
            &a,
            8,
            13,
            SyntaxKind::ExprName,
            OccurrenceRole::Read,
            1,
        );
        let second = occurrence(
            &mut d,
            &a,
            8,
            13,
            SyntaxKind::ExprName,
            OccurrenceRole::Read,
            2,
        );
        for (ordinal, subject) in [first, second].into_iter().enumerate() {
            d.core
                .placements
                .insert(SyntaxPlacement {
                    qualification: q.id(),
                    occurrence: subject,
                    parent: Some(call),
                    field: SyntaxField::Argument,
                    ordinal: ordinal as i64,
                })
                .unwrap();
        }
        usage(&mut d, &mut out, &a, &q, call, 11);
        diagnostic(&mut d, &mut out, &a, &q, Some((8, 13)), 0);
        derive(&d, &mut out, &b).unwrap();
        let assessment = out.diagnostic_use_assessments.iter().next().unwrap();
        assert_eq!(assessment.status, DiagnosticUseStatus::AmbiguousUse);
        assert_eq!(assessment.uses, 1);
        assert!(!assessment.remainder);
        assert_eq!(out.diagnostic_use_links.len(), 2);
        assert_eq!(out.diagnostic_use_paths.len(), 2);
        assert_eq!(
            out.diagnostic_use_links
                .iter()
                .map(|link| link.subject)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([first, second])
        );
    }
    #[test]
    fn duplicate_supports_for_one_subject_and_event_remain_unique() {
        let (b, mut d, mut out, a, q) = fixture();
        let call = occurrence(
            &mut d,
            &a,
            0,
            14,
            SyntaxKind::ExprCall,
            OccurrenceRole::Call,
            0,
        );
        let first = usage(&mut d, &mut out, &a, &q, call, 11);
        let existing = out.source_usages.get(first).unwrap().clone();
        let raw = d.facts.usage_sites.iter().next().unwrap();
        let native = d
            .facts
            .characterization_native
            .insert(NativeAssertionPremise::ProviderCallSite {
                assertion: raw.id(),
                support: id(10),
            })
            .unwrap();
        let mut characterization = out
            .source_characterizations
            .get(existing.characterization)
            .unwrap()
            .clone();
        characterization.native = native;
        let characterization = out
            .source_characterizations
            .insert(characterization)
            .unwrap();
        let second = out
            .source_usages
            .insert(SourceUsage {
                characterization,
                event: existing.event,
            })
            .unwrap();
        assert_ne!(first, second);
        diagnostic(&mut d, &mut out, &a, &q, Some((0, 14)), 0);
        derive(&d, &mut out, &b).unwrap();
        let assessment = out.diagnostic_use_assessments.iter().next().unwrap();
        assert_eq!(assessment.status, DiagnosticUseStatus::UniqueUse);
        assert_eq!(assessment.uses, 2);
        assert!(!assessment.remainder);
        assert_eq!(out.diagnostic_use_links.len(), 2);
        assert!(
            out.diagnostic_use_links
                .iter()
                .all(|link| link.subject == call)
        );
    }
    #[test]
    fn foreign_primary_artifact_cannot_acquire_an_exact_use() {
        let (b, mut d, mut out, a, q) = fixture();
        let diagnosis = diagnostic(&mut d, &mut out, &a, &q, Some((0, 14)), 0);
        let foreign =
            SourceArtifact::from_bytes(a.input, "foreign.py".into(), b"consume(value)\n").unwrap();
        d.core.artifacts.insert(foreign.clone()).unwrap();
        let call = occurrence(
            &mut d,
            &foreign,
            0,
            14,
            SyntaxKind::ExprCall,
            OccurrenceRole::Call,
            0,
        );
        usage(&mut d, &mut out, &foreign, &q, call, 11);
        let span = assertion::Evidence::SourceSpan {
            source: foreign.id(),
            start: 0,
            end: 14,
        };
        d.facts.canonical_evidence.insert(span.clone()).unwrap();
        let source = out
            .original_sources
            .insert(OriginalSource::Span {
                span: EvidenceSourceSpanId::of(&span).unwrap(),
            })
            .unwrap();
        let mut rows = out
            .source_characterizations
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        for row in &mut rows {
            if row.id() == diagnosis {
                row.source = source
            }
        }
        out.source_characterizations = normalized::Rows::new(&b);
        for row in rows {
            out.source_characterizations.insert(row).unwrap();
        }
        assert!(
            matches!(derive(&d,&mut out,&b),Err(ModelError::Invalid(message)) if message.contains("primary changes original artifact/view"))
        );
        assert!(out.diagnostic_use_links.is_empty());
    }
    #[test]
    fn ambiguous_exact_subjects_and_foreign_ancestry_preserve_unknown_or_fail() {
        let (b, mut d, mut out, a, q) = fixture();
        let call1 = occurrence(
            &mut d,
            &a,
            0,
            14,
            SyntaxKind::ExprCall,
            OccurrenceRole::Call,
            0,
        );
        let call2 = occurrence(
            &mut d,
            &a,
            0,
            14,
            SyntaxKind::ExprCall,
            OccurrenceRole::Call,
            2,
        );
        usage(&mut d, &mut out, &a, &q, call1, 11);
        usage(&mut d, &mut out, &a, &q, call2, 12);
        diagnostic(&mut d, &mut out, &a, &q, Some((0, 14)), 0);
        derive(&d, &mut out, &b).unwrap();
        assert_eq!(
            out.diagnostic_use_assessments.iter().next().unwrap().status,
            DiagnosticUseStatus::AmbiguousUse
        );
        assert_eq!(out.diagnostic_use_links.len(), 2);
        let (b, mut d, mut out, a, q) = fixture();
        let arg = occurrence(
            &mut d,
            &a,
            8,
            13,
            SyntaxKind::ExprName,
            OccurrenceRole::Read,
            1,
        );
        let foreign = SourceArtifact::from_bytes(id(1), "foreign.py".into(), b"value\n").unwrap();
        let parent = occurrence(
            &mut d,
            &foreign,
            0,
            6,
            SyntaxKind::ModModule,
            OccurrenceRole::Syntax,
            2,
        );
        d.core
            .placements
            .insert(SyntaxPlacement {
                qualification: q.id(),
                occurrence: arg,
                parent: Some(parent),
                field: SyntaxField::Argument,
                ordinal: 0,
            })
            .unwrap();
        diagnostic(&mut d, &mut out, &a, &q, Some((8, 13)), 0);
        assert!(derive(&d, &mut out, &b).is_err());
    }
}
