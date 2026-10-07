//! Exact original source addresses and captured attribution; bytes remain in the native store.
use crate::records::{need, rows, wire, Prepared as CanonicalPrepared,PacketRows};
use lctx_model::domain::{serving::*, *};
pub fn target(source: &OriginalReference) -> Result<graph::Target, ModelError> {
    Ok(match source {
        OriginalReference::Catalog { source } => {
            graph::Target::Entity(graph::EntityId::of(*source))
        }
        OriginalReference::Anchor { anchor } => {
            graph::target_for_row(derivation::RowRef::of(*anchor))?
        }
        OriginalReference::Prose { slice } => {
            graph::target_for_row(derivation::RowRef::of(*slice))?
        }
        OriginalReference::Artifact { artifact } => {
            graph::Target::Entity(graph::EntityId::of(*artifact))
        }
        OriginalReference::Occurrence { occurrence } => {
            graph::Target::Entity(graph::EntityId::of(*occurrence))
        }
        OriginalReference::Span { span } => graph::Target::Entity(graph::EntityId::of(span.id())),
    })
}
/// One typed original-address view for a bounded packet cohort.
pub struct Prepared {
    artifacts: PacketRows<source::SourceArtifact>,
    occurrences: PacketRows<source::Occurrence>,
    evidence: PacketRows<assertion::Evidence>,
    anchors: PacketRows<retrieval::OriginalAnchor>,
    units: PacketRows<retrieval::Unit>,
    anchor_sources: PacketRows<retrieval::AnchorSource>,
    catalog_sources: PacketRows<catalog::evidence::OriginalSource>,
    slices: PacketRows<synthesis::documentary::ProseSlice>,
    prose_sources: PacketRows<synthesis::documentary::ProseSource>,
    runs: PacketRows<attribution::ProviderRun>,
    distributions: PacketRows<input::InputDistribution>,
    corpora: PacketRows<input::CorpusLibrary>,
}
impl Prepared {
    pub fn new(data: &CanonicalPrepared<'_>) -> Result<Self, ModelError> {
        Ok(Self {
            artifacts: rows(data)?,
            occurrences: rows(data)?,
            evidence: rows(data)?,
            anchors: rows(data)?,
            units: rows(data)?,
            anchor_sources: rows(data)?,
            catalog_sources: rows(data)?,
            slices: rows(data)?,
            prose_sources: rows(data)?,
            runs: rows(data)?,
            distributions: rows(data)?,
            corpora: rows(data)?,
        })
    }
    pub fn range(
        &self,
        source: &OriginalReference,
        context: Option<Id<attribution::AnalysisContext>>,
        release: Option<Id<input::Release>>,
    ) -> Result<OriginalRange, ModelError> {
        let artifacts = &self.artifacts;
        let occurrences = &self.occurrences;
        let evidence = &self.evidence;
        let mut original = source.clone();
        let mut chosen_context = context;
        let mut captured_slice = None;
        if let OriginalReference::Anchor { anchor } = original {
            let anchors = &self.anchors;
            let a = need(anchors, anchor)?;
            let units = &self.units;
            let unit = need(units, a.unit)?;
            if context.is_some_and(|requested| requested != unit.context) {
                return Err(ModelError::Conflict(
                    "original anchor context differs from demand",
                ));
            }
            chosen_context = Some(unit.context);
            let sources = &self.anchor_sources;
            original = match need(sources, a.original)? {
                retrieval::AnchorSource::Original { source } => {
                    OriginalReference::Catalog { source: *source }
                }
                retrieval::AnchorSource::Span { span } => OriginalReference::Span { span: *span },
                retrieval::AnchorSource::SpanSlice { span, start, end } => {
                    captured_slice = Some((*start, *end));
                    OriginalReference::Span { span: *span }
                }
                retrieval::AnchorSource::Artifact { artifact } => OriginalReference::Artifact {
                    artifact: *artifact,
                },
                retrieval::AnchorSource::Prose { slice } => {
                    OriginalReference::Prose { slice: *slice }
                }
                retrieval::AnchorSource::Occurrence { occurrence } => {
                    OriginalReference::Occurrence {
                        occurrence: *occurrence,
                    }
                }
                retrieval::AnchorSource::OccurrenceSlice {
                    occurrence,
                    start,
                    end,
                } => {
                    captured_slice = Some((*start, *end));
                    OriginalReference::Occurrence {
                        occurrence: *occurrence,
                    }
                }
            };
        }
        if let OriginalReference::Catalog { source } = original {
            let sources = &self.catalog_sources;
            original = match need(sources, source)? {
                catalog::evidence::OriginalSource::Artifact { artifact } => {
                    OriginalReference::Artifact {
                        artifact: *artifact,
                    }
                }
                catalog::evidence::OriginalSource::Occurrence { occurrence } => {
                    OriginalReference::Occurrence {
                        occurrence: *occurrence,
                    }
                }
                catalog::evidence::OriginalSource::Span { span } => {
                    OriginalReference::Span { span: *span }
                }
            };
        }
        let prose = if let OriginalReference::Prose { slice } = original {
            Some(slice)
        } else {
            None
        };
        let mut encoding = "raw_bytes";
        let (artifact, start, end) = if let Some(id) = prose {
            let slices = &self.slices;
            let slice = need(slices, id)?;
            let sources = &self.prose_sources;
            match need(sources, slice.source)? {
                synthesis::documentary::ProseSource::Occurrence { occurrence } => {
                    let o = need(occurrences, *occurrence)?;
                    if slice.end > o.end - o.start {
                        return Err(ModelError::Schema("prose raw slice bounds"));
                    }
                    (o.source, o.start + slice.start, o.start + slice.end)
                }
                synthesis::documentary::ProseSource::Literal { occurrence, .. } => {
                    let o = need(occurrences, *occurrence)?;
                    encoding = "native_literal_utf8_slice";
                    (o.source, o.start, o.end)
                }
                synthesis::documentary::ProseSource::Span { span } => {
                    match need(evidence, span.id())? {
                        assertion::Evidence::SourceSpan { source, start, end } => {
                            if slice.end > end - start {
                                return Err(ModelError::Schema("prose raw slice bounds"));
                            }
                            (*source, *start + slice.start, *start + slice.end)
                        }
                        _ => return Err(ModelError::Schema("prose span evidence")),
                    }
                }
            }
        } else {
            match original {
                OriginalReference::Artifact { artifact } => {
                    let a = need(artifacts, artifact)?;
                    (artifact, 0, a.byte_len)
                }
                OriginalReference::Occurrence { occurrence } => {
                    let o = need(occurrences, occurrence)?;
                    (o.source, o.start, o.end)
                }
                OriginalReference::Span { span } => match need(evidence, span.id())? {
                    assertion::Evidence::SourceSpan { source, start, end } => {
                        (*source, *start, *end)
                    }
                    _ => return Err(ModelError::Schema("source span evidence")),
                },
                _ => return Err(ModelError::Schema("original resolution")),
            }
        };
        let (start, end) = if let Some((a, z)) = captured_slice {
            if a < start || z > end || z < a {
                return Err(ModelError::Schema("original interpretation slice bounds"));
            }
            (a, z)
        } else {
            (start, end)
        };
        let a = need(artifacts, artifact)?;
        if start < 0 || end < start || end > a.byte_len {
            return Err(ModelError::Schema("original byte bounds"));
        }
        let runs = &self.runs;
        let contexts = runs.select_for("input",&[a.input])?
            .iter()
            .map(|r| r.context)
            .collect::<std::collections::BTreeSet<_>>();
        if context.is_some_and(|c| !contexts.contains(&c)) {
            return Err(ModelError::Conflict(
                "original context outside captured input",
            ));
        }
        let context = match chosen_context {
            Some(c) if contexts.contains(&c) => c,
            Some(_) => {
                return Err(ModelError::Conflict(
                    "original context outside captured input",
                ));
            }
            None => {
                if contexts.len() != 1 {
                    return Err(ModelError::Conflict(
                        "original context attribution ambiguity",
                    ));
                }
                *contexts.first().expect("single context")
            }
        };
        let distributions = &self.distributions;
        let corpora = &self.corpora;
        let mut captured_inputs=corpora.select_for("corpus",&[a.input])?.iter().map(|corpus|corpus.library).collect::<Vec<_>>();
        captured_inputs.push(a.input);
        let releases = distributions.select_for("input",&captured_inputs)?
            .iter()
            .filter(|d| d.role == input::DistributionRole::FirstParty)
            .map(|d| d.release)
            .collect::<std::collections::BTreeSet<_>>();
        let release = match release {
            Some(r) if releases.contains(&r) => r,
            Some(_) => {
                return Err(ModelError::Conflict(
                    "original release outside captured input",
                ));
            }
            None => {
                if releases.len() != 1 {
                    return Err(ModelError::Conflict(
                        "original release attribution ambiguity",
                    ));
                }
                *releases.first().expect("single release")
            }
        };
        Ok(OriginalRange {
            source: source.clone(),
            artifact,
            start: start as u64,
            end: end as u64,
            digest: a.content,
            encoding: Name::new(encoding).map_err(wire)?,
            release,
            context,
        })
    }
}
pub fn range(
    data: &CanonicalPrepared<'_>,
    source: &OriginalReference,
    context: Option<Id<attribution::AnalysisContext>>,
    release: Option<Id<input::Release>>,
) -> Result<OriginalRange, ModelError> {
    Prepared::new(data)?.range(source, context, release)
}
#[cfg(test)]
mod controls {
    use super::*;
    fn packet<R:Record>(rows:Vec<R>)->PacketRows<R>{PacketRows::new(rows,&lctx_model::domain::resources::ResourceBudget::fixed(1<<20).unwrap()).unwrap()}
    fn append<R:Record>(rows:&mut PacketRows<R>,row:R){let mut values=rows.rows().to_vec();values.push(row);*rows=packet(values);}
    fn change<R:Record>(rows:&mut PacketRows<R>,edit:impl FnOnce(&mut Vec<R>)){let mut values=rows.rows().to_vec();edit(&mut values);*rows=packet(values);}
    fn id<R: Record>(byte: u8) -> Id<R> {
        serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
    }
    #[test]
    fn prepared_originals_preserve_captured_slices_and_prose_encodings() {
        let artifact = source::SourceArtifact::from_bytes(
            id(1),
            "fixture.py".into(),
            b"012345678901234567890123456789",
        )
        .unwrap();
        let occurrence = source::Occurrence {
            source: artifact.id(),
            start: 4,
            end: 20,
            syntax_kind: source::SyntaxKind::ModExpression,
            role: source::OccurrenceRole::Syntax,
            structural_path: vec![],
        };
        let unit = retrieval::Unit {
            input: id(1),
            context: id(2),
            family: retrieval::Family::Source,
            origin: id(3),
            corpus: id(4),
            title: "source".into(),
        };
        let original = retrieval::AnchorSource::OccurrenceSlice {
            occurrence: occurrence.id(),
            start: 7,
            end: 12,
        };
        let anchor = retrieval::OriginalAnchor {
            unit: unit.id(),
            ordinal: 0,
            original: original.id(),
        };
        let raw = synthesis::documentary::ProseSource::Occurrence {
            occurrence: occurrence.id(),
        };
        let literal = synthesis::documentary::ProseSource::Literal {
            occurrence: occurrence.id(),
            literal: id(5),
        };
        let raw_slice = synthesis::documentary::ProseSlice {
            source: raw.id(),
            start: 2,
            end: 5,
        };
        let literal_slice = synthesis::documentary::ProseSlice {
            source: literal.id(),
            start: 2,
            end: 5,
        };
        let mut prepared = Prepared {
            artifacts: packet(vec![artifact]),
            occurrences: packet(vec![occurrence]),
            evidence: packet(vec![]),
            anchors: packet(vec![anchor.clone()]),
            units: packet(vec![unit]),
            anchor_sources: packet(vec![original]),
            catalog_sources: packet(vec![]),
            slices: packet(vec![raw_slice.clone(), literal_slice.clone()]),
            prose_sources: packet(vec![raw, literal]),
            runs: packet(vec![attribution::ProviderRun {
                provider: id(7),
                context: id(2),
                input: id(1),
                configuration: ContentHash::of(b"run"),
                requested_families: ContentHash::of(b"families"),
            }]),
            distributions: packet(vec![input::InputDistribution {
                input: id(1),
                release: id(6),
                role: input::DistributionRole::FirstParty,
            }]),
            corpora: packet(vec![]),
        };
        let range = prepared
            .range(
                &OriginalReference::Anchor {
                    anchor: anchor.id(),
                },
                Some(id(2)),
                Some(id(6)),
            )
            .unwrap();
        assert_eq!((range.start, range.end, range.context), (7, 12, id(2)));
        let range = prepared
            .range(
                &OriginalReference::Prose {
                    slice: raw_slice.id(),
                },
                Some(id(2)),
                Some(id(6)),
            )
            .unwrap();
        assert_eq!(
            (range.start, range.end, range.encoding.as_str()),
            (6, 9, "raw_bytes")
        );
        let range = prepared
            .range(
                &OriginalReference::Prose {
                    slice: literal_slice.id(),
                },
                Some(id(2)),
                Some(id(6)),
            )
            .unwrap();
        assert_eq!(
            (range.start, range.end, range.encoding.as_str()),
            (4, 20, "native_literal_utf8_slice")
        );
        let mut other_run = prepared.runs[0].clone();
        other_run.context = id(9);
        append(&mut prepared.runs,other_run);
        assert!(
            prepared
                .range(
                    &OriginalReference::Anchor {
                        anchor: anchor.id()
                    },
                    Some(id(9)),
                    Some(id(6))
                )
                .is_err()
        );
        assert!(
            prepared
                .range(
                    &OriginalReference::Prose {
                        slice: raw_slice.id()
                    },
                    Some(id(2)),
                    Some(id(9))
                )
                .is_err()
        );
        let span = assertion::Evidence::SourceSpan {
            source: prepared.artifacts[0].id(),
            start: 4,
            end: 20,
        };
        let span_id = assertion::EvidenceSourceSpanId::of(&span).unwrap();
        let slice = retrieval::AnchorSource::SpanSlice {
            span: span_id,
            start: 8,
            end: 11,
        };
        let slice_anchor = retrieval::OriginalAnchor {
            unit: prepared.units[0].id(),
            ordinal: 1,
            original: slice.id(),
        };
        append(&mut prepared.evidence,span);
        append(&mut prepared.anchor_sources,slice);
        append(&mut prepared.anchors,slice_anchor.clone());
        let range = prepared
            .range(
                &OriginalReference::Anchor {
                    anchor: slice_anchor.id(),
                },
                Some(id(2)),
                Some(id(6)),
            )
            .unwrap();
        assert_eq!((range.start, range.end), (8, 11));
        let mut invalid = prepared;
        invalid.anchor_sources = packet(vec![retrieval::AnchorSource::OccurrenceSlice {
            occurrence: invalid.occurrences[0].id(),
            start: 3,
            end: 12,
        }]);
        let original=invalid.anchor_sources[0].id();
        change(&mut invalid.anchors,|rows|rows[0].original=original);
        assert!(
            invalid
                .range(
                    &OriginalReference::Anchor {
                        anchor: invalid.anchors[0].id()
                    },
                    Some(id(2)),
                    Some(id(6))
                )
                .is_err()
        );
        let outside = retrieval::AnchorSource::SpanSlice {
            span: span_id,
            start: 3,
            end: 11,
        };
        change(&mut invalid.anchors,|rows|rows[1].original=outside.id());
        append(&mut invalid.anchor_sources,outside);
        assert!(
            invalid
                .range(
                    &OriginalReference::Anchor {
                        anchor: invalid.anchors[1].id()
                    },
                    Some(id(2)),
                    Some(id(6))
                )
                .is_err(),
            "exact heading slice stays within its captured parent span"
        );
        invalid.distributions = packet(vec![input::InputDistribution {
            input: id(10),
            release: id(6),
            role: input::DistributionRole::FirstParty,
        }]);
        invalid.corpora = packet(vec![input::CorpusLibrary {
            corpus: id(1),
            library: id(10),
        }]);
        assert!(
            invalid
                .range(
                    &OriginalReference::Prose {
                        slice: raw_slice.id()
                    },
                    Some(id(2)),
                    Some(id(6))
                )
                .is_ok(),
            "captured corpus mapping retains release attribution"
        );
        invalid.distributions=packet(vec![]);
        assert!(
            invalid
                .range(
                    &OriginalReference::Prose {
                        slice: raw_slice.id()
                    },
                    Some(id(2)),
                    Some(id(6))
                )
                .is_err(),
            "missing captured release refuses explicit attribution"
        );
        invalid.runs=packet(vec![]);
        assert!(
            invalid
                .range(
                    &OriginalReference::Prose {
                        slice: raw_slice.id()
                    },
                    Some(id(2)),
                    Some(id(6))
                )
                .is_err(),
            "missing provider context refuses explicit attribution"
        );
    }
}
