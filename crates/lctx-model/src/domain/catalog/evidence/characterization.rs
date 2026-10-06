//! Proof-linked source characterization, independent of API association and execution.
use super::{
    build::{EvidenceData, EvidenceOutput, invalid, need},
    *,
};
use crate::domain::{
    analysis::native::NativeAssertionPremise, assertion::*, attribution::*, source::*,
};
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(
    name = "catalog_source_characterizations",
    rule = "source_characterization"
)]
pub struct SourceCharacterization {
    #[model(key, premise)]
    pub native: Id<NativeAssertionPremise>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub artifact: Id<SourceArtifact>,
    #[model(key)]
    pub source: Id<OriginalSource>,
}
/// Containment only, never a resolved API target or an executed-test claim.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_source_characterization_scenarios")]
pub struct SourceCharacterizationScenario {
    #[model(key)]
    pub characterization: Id<SourceCharacterization>,
    #[model(key)]
    pub scenario: Id<CatalogScenario>,
}
fn coordinates(
    d: &EvidenceData,
    source: &OriginalSource,
) -> Result<(Id<SourceArtifact>, i64, i64), ModelError> {
    match source {
        OriginalSource::Artifact { artifact } => {
            Ok((*artifact, 0, need(&d.core.artifacts, *artifact)?.byte_len))
        }
        OriginalSource::Occurrence { occurrence } => {
            let o = need(&d.core.occurrences, *occurrence)?;
            Ok((o.source, o.start, o.end))
        }
        OriginalSource::Span { span } => match need(&d.facts.canonical_evidence, span.id())? {
            Evidence::SourceSpan { source, start, end } => Ok((*source, *start, *end)),
            _ => Err(invalid(
                "source characterization has a non-span evidence reference",
            )),
        },
    }
}
fn supported(
    d: &EvidenceData,
    q: &AssertionQualification,
    artifact: Id<SourceArtifact>,
    family: FactFamily,
    tool: &str,
    attribution: Option<SupportAttribution>,
) -> bool {
    let Some(SupportAttribution {
        run,
        surface,
        evidence,
        origin,
        mode,
        fidelity,
    }) = attribution
    else {
        return false;
    };
    let (Some(run), Some(surface), Some(source)) = (
        d.facts.runs.get(run),
        d.facts.characterization_surfaces.get(surface),
        d.core.artifacts.get(artifact),
    ) else {
        return false;
    };
    let Some(provider) = d.facts.characterization_providers.get(run.provider) else {
        return false;
    };
    let scoped = match d.facts.characterization_scopes.get(q.scope) {
        Some(CoverageScope::Artifact { artifact: a }) => *a == artifact,
        Some(CoverageScope::Module { module }) => d
            .core
            .modules
            .get(*module)
            .is_some_and(|m| m.source == artifact),
        Some(CoverageScope::Input { input }) => *input == source.input,
        _ => false,
    };
    scoped
        && provider.tool == tool
        && run.provider == surface.provider
        && run.input == source.input
        && run.context == q.context
        && surface.family == family
        && q.assumptions == assumptions::AssumptionSet::empty_id()
        && q.modality == Modality::Definite
        && q.approximation == Approximation::Exact
        && q.condition == conditions::Diagram::always().id()
        && origin == Origin::AnalyzerAssertion
        && mode == ExtractionMode::NativeTraversal
        && fidelity == Fidelity::NativeStructural
        && d.facts.canonical_evidence.get(evidence).is_some()
}
fn add(
    d: &EvidenceData,
    out: &mut EvidenceOutput,
    premise: NativeAssertionPremise,
    q: Id<AssertionQualification>,
    artifact: Id<SourceArtifact>,
    source: OriginalSource,
) -> Result<Id<SourceCharacterization>, ModelError> {
    // Existing native inventory must contain this exact pair. No support is synthesized here.
    if d.facts.characterization_native.get(premise.id()) != Some(&premise) {
        return Err(invalid(
            "source characterization exact native premise absent",
        ));
    }
    let qrow = need(&d.core.qualifications, q)?;
    let (actual, start, end) = coordinates(d, &source)?;
    let a = need(&d.core.artifacts, artifact)?;
    if actual != artifact || start < 0 || end < start || end > a.byte_len {
        return Err(invalid("source characterization crosses captured artifact"));
    }
    let original = out.original_sources.insert(source)?;
    let id = out
        .source_characterizations
        .insert(SourceCharacterization {
            native: premise.id(),
            qualification: q,
            artifact,
            source: original,
        })?;
    let mut scenarios = Vec::new();
    for span in out.spans.iter().filter(|s| s.role == SpanRole::Primary) {
        let s = need(&out.original_sources, span.source)?;
        let (other, lo, hi) = coordinates(d, s)?;
        let scenario = need(&out.scenarios, span.scenario)?;
        let source = need(&out.scenario_sources, scenario.source)?;
        let context = match source {
            ScenarioSource::Python { context, .. } => Some(*context),
            ScenarioSource::Fence { observation } => Some(
                need(
                    &d.core.qualifications,
                    need(&d.facts.blocks, *observation)?.qualification,
                )?
                .context,
            ),
        };
        if context == Some(qrow.context) && other == artifact && lo <= start && end <= hi {
            scenarios.push(span.scenario);
        }
    }
    scenarios.sort();
    scenarios.dedup();
    for scenario in scenarios {
        out.source_characterization_scenarios
            .insert(SourceCharacterizationScenario {
                characterization: id,
                scenario,
            })?;
    }
    Ok(id)
}
/// Exact event-source correspondence; it survives unresolved targets and does not imply execution.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_source_usages", rule = "source_usage")]
pub struct SourceUsage {
    #[model(key, premise)]
    pub characterization: Id<SourceCharacterization>,
    #[model(key)]
    pub event: Id<normalized::events::NormalizedCallEvent>,
}
pub(super) fn derive(d: &EvidenceData, out: &mut EvidenceOutput) -> Result<(), ModelError> {
    for annotation in d.facts.diagnostic_annotations.iter() {
        let artifact = match need(&d.facts.diagnostic_subjects, annotation.diagnostic)? {
            diagnostics::DiagnosticSubject::Ruff { observation } => {
                need(&d.facts.ruff_diagnostics, *observation)?.artifact
            }
            diagnostics::DiagnosticSubject::Pyrefly { observation } => {
                need(&d.facts.pyrefly_diagnostics, *observation)?.artifact
            }
        };
        if let Some(span) = annotation.span {
            let (actual, start, end) = coordinates(d, &OriginalSource::Span { span })?;
            if actual != artifact
                || start < 0
                || end < start
                || end > need(&d.core.artifacts, artifact)?.byte_len
            {
                return Err(invalid(
                    "native diagnostic annotation changes captured parent artifact",
                ));
            }
        }
    }
    macro_rules! diagnostics {
        ($rows:expr,$supports:expr,$variant:ident,$family:expr,$tool:expr) => {
            for row in $rows.iter() {
                let q = need(&d.core.qualifications, row.qualification)?;
                for support in $supports.iter().filter(|s| s.assertion == row.id()) {
                    let source = row.primary.map_or(
                        OriginalSource::Artifact {
                            artifact: row.artifact,
                        },
                        |span| OriginalSource::Span { span },
                    );
                    let expected = row
                        .primary
                        .map(|span| span.id())
                        .unwrap_or_else(|| Evidence::Invocation { run: support.run }.id());
                    if support.evidence == expected
                        && supported(d, q, row.artifact, $family, $tool, support.attribution())
                    {
                        add(
                            d,
                            out,
                            NativeAssertionPremise::$variant {
                                assertion: row.id(),
                                support: support.id(),
                            },
                            row.qualification,
                            row.artifact,
                            source,
                        )?;
                    }
                }
            }
        };
    }
    diagnostics!(
        d.facts.ruff_diagnostics,
        d.facts.ruff_diagnostic_supports,
        RuffDiagnosticObservation,
        FactFamily::Lexical,
        "ruff"
    );
    diagnostics!(
        d.facts.pyrefly_diagnostics,
        d.facts.pyrefly_diagnostic_supports,
        PyreflyDiagnosticObservation,
        FactFamily::Types,
        "pyrefly"
    );
    for row in d.facts.parameter_definitions.iter() {
        let parameter = need(&d.core.parameter_syntax, row.parameter)?;
        let occurrence = need(&d.core.occurrences, row.subject)?;
        let q = need(&d.core.qualifications, row.qualification)?;
        if parameter.parameter != row.subject
            || parameter.qualification != row.qualification
            || row.query_offset < occurrence.start
            || row.query_offset > occurrence.end
        {
            return Err(invalid(
                "native parameter answer changes exact canonical parameter/context",
            ));
        }
        for support in d
            .facts
            .parameter_definition_supports
            .iter()
            .filter(|s| s.assertion == row.id())
        {
            if d.facts.canonical_evidence.get(support.evidence)
                == Some(&Evidence::Occurrence {
                    occurrence: row.subject,
                })
                && supported(
                    d,
                    q,
                    occurrence.source,
                    FactFamily::Types,
                    "pyrefly",
                    support.attribution(),
                )
            {
                add(
                    d,
                    out,
                    NativeAssertionPremise::NativeParameterDefinitionObservation {
                        assertion: row.id(),
                        support: support.id(),
                    },
                    row.qualification,
                    occurrence.source,
                    OriginalSource::Occurrence {
                        occurrence: row.subject,
                    },
                )?;
            }
        }
    }
    for source in d.facts.usage_event_sources.iter() {
        let event = need(&d.facts.events, source.event)?;
        let row = need(&d.facts.usage_sites, source.observation)?;
        let occurrence = need(&d.core.occurrences, row.site)?;
        let q = need(&d.core.qualifications, row.qualification)?;
        if row.site != event.site || row.origin != event.origin || q.context != event.context {
            return Err(invalid(
                "source usage changes exact native event source identity",
            ));
        }
        for support in d
            .facts
            .usage_site_supports
            .iter()
            .filter(|s| s.assertion == row.id())
        {
            let located = match d.facts.canonical_evidence.get(support.evidence) {
                Some(Evidence::SourceSpan { source, start, end }) => {
                    *source == occurrence.source
                        && *start <= occurrence.start
                        && occurrence.end <= *end
                }
                Some(Evidence::Occurrence {
                    occurrence: subject,
                }) => *subject == row.site,
                Some(Evidence::Invocation { run }) => *run == support.run,
                _ => false,
            };
            if located
                && supported(
                    d,
                    q,
                    occurrence.source,
                    FactFamily::Calls,
                    "pyrefly",
                    support.attribution(),
                )
            {
                let characterization = add(
                    d,
                    out,
                    NativeAssertionPremise::ProviderCallSite {
                        assertion: row.id(),
                        support: support.id(),
                    },
                    row.qualification,
                    occurrence.source,
                    OriginalSource::Occurrence {
                        occurrence: row.site,
                    },
                )?;
                out.source_usages.insert(SourceUsage {
                    characterization,
                    event: event.id(),
                })?;
            }
        }
    }
    Ok(())
}
