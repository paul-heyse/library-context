//! Exact captured byte ranges. No parsing, lossy decoding or alternative source-body store.
use super::{
    build::{Data, invalid, need},
    *,
};
use crate::domain::{
    artifact::{ARTIFACT_CHUNK_BYTES, ArtifactChunkKey},
    assertion::Evidence,
    resources::{Reservation, ResourceBudget},
    source::*,
};
pub struct Text {
    pub value: String,
    _reservation: Box<dyn Reservation>,
}
pub fn coordinates(
    d: &Data,
    original: &AnchorSource,
) -> Result<(Id<SourceArtifact>, i64, i64), ModelError> {
    match original {
        AnchorSource::OccurrenceSlice{occurrence,start,end}=>{let r=need(&d.source.core.occurrences,*occurrence)?;if *start<r.start||*end>r.end||end<start{return Err(invalid("interpretation slice exceeds canonical occurrence"));}Ok((r.source,*start,*end))},
        AnchorSource::Occurrence { occurrence } => { let r=need(&d.source.core.occurrences,*occurrence)?; Ok((r.source,r.start,r.end)) },
        AnchorSource::Prose { slice } => {
            let slice = need(&d.synthesis.prose_slices, *slice)?;
            let (artifact, start, end) = match need(&d.synthesis.prose_sources, slice.source)? {
                crate::domain::synthesis::documentary::ProseSource::Literal {
                    occurrence,
                    literal,
                } => {
                    let r = need(&d.source.core.occurrences, *occurrence)?;
                    let value = match need(&d.facts.literals, *literal)? {
                        value::Literal::String { value } => value,
                        _ => {
                            return Err(invalid("brief interpreted anchor is not a native string"));
                        }
                    };
                    let start = usize::try_from(slice.start).map_err(ModelError::codec)?;
                    let end = usize::try_from(slice.end).map_err(ModelError::codec)?;
                    if value.get(start..end).is_none() {
                        return Err(invalid("brief interpreted slice exceeds native literal"));
                    }
                    return Ok((r.source, r.start, r.end));
                }
                crate::domain::synthesis::documentary::ProseSource::Occurrence { occurrence } => {
                    let r = need(&d.source.core.occurrences, *occurrence)?;
                    (r.source, r.start, r.end)
                }
                crate::domain::synthesis::documentary::ProseSource::Span { span } => {
                    match need(&d.source.facts.canonical_evidence, span.id())? {
                        Evidence::SourceSpan { source, start, end } => (*source, *start, *end),
                        _ => return Err(invalid("brief prose anchor is not a source span")),
                    }
                }
            };
            if slice.start < 0 || slice.end < slice.start || slice.end > end - start {
                return Err(invalid("brief prose slice exceeds original source"));
            }
            Ok((artifact, start + slice.start, start + slice.end))
        }
        AnchorSource::Artifact { artifact } => Ok((*artifact, 0, d.artifact_bounds(*artifact)?.1)),
        AnchorSource::Span { span } => match need(&d.source.facts.canonical_evidence, span.id())? {
            Evidence::SourceSpan { source, start, end } => Ok((*source, *start, *end)),
            _ => Err(invalid("retrieval anchor is not canonical source span")),
        },
        AnchorSource::Original { source } => match need(&d.evidence.original_sources, *source)? {
            c1::OriginalSource::Artifact { artifact } => {
                Ok((*artifact, 0, d.artifact_bounds(*artifact)?.1))
            }
            c1::OriginalSource::Occurrence { occurrence } => {
                let r = need(&d.source.core.occurrences, *occurrence)?;
                Ok((r.source, r.start, r.end))
            }
            c1::OriginalSource::Span { span } => {
                match need(&d.source.facts.canonical_evidence, span.id())? {
                    Evidence::SourceSpan { source, start, end } => Ok((*source, *start, *end)),
                    _ => Err(invalid("retrieval anchor is not canonical source span")),
                }
            }
        },
    }
}
pub fn read(d: &Data, original: &AnchorSource, b: &ResourceBudget) -> Result<Text, ModelError> {
    let (id, start, end) = coordinates(d, original)?;
    read_range(d,id,start,end,b)
}
pub fn read_range(d:&Data,id:Id<SourceArtifact>,start:i64,end:i64,b:&ResourceBudget)->Result<Text,ModelError>{
    if start < 0 || end < start || end > d.artifact_bounds(id)?.1 {
        return Err(invalid("retrieval original coordinates outside artifact"));
    }
    let len = usize::try_from(end - start).map_err(ModelError::codec)?;
    let reservation = b.reserve("retrieval-original-range", len + size_of::<Text>())?;
    let mut bytes = Vec::with_capacity(len);
    let mut offset = start as usize;
    while offset < end as usize {
        let id = Id::of(&ArtifactChunkKey {
            artifact: id,
            ordinal: (offset / ARTIFACT_CHUNK_BYTES) as i64,
        });
        let chunk = need(&d.facts.chunks, id)?;
        let first = offset % ARTIFACT_CHUNK_BYTES;
        let count = (end as usize - offset).min(ARTIFACT_CHUNK_BYTES - first);
        let part = chunk
            .body
            .0
            .get(first..first + count)
            .ok_or_else(|| invalid("retrieval original chunk range missing"))?;
        bytes.extend_from_slice(part);
        offset += count;
    }
    Ok(Text {
        value: String::from_utf8(bytes).map_err(|_| invalid("retrieval original is not UTF-8"))?,
        _reservation: reservation,
    })
}

/// Actual ranges the deterministic root renderer reads. Coordinates remain the same model
/// operation used by rendering and necessary admission; the compiler selects only their chunks.
pub struct Ranges {
    values: charged::ChargedSet<(Id<SourceArtifact>, i64, i64)>,
    charge: charged::StateCharge,
}
impl Ranges {
    fn new(b: &ResourceBudget) -> Self {
        Self {
            values: Default::default(),
            charge: charged::StateCharge::new(b, "retrieval-original-coordinates"),
        }
    }
    fn add(&mut self, d: &Data, anchor: AnchorSource) -> Result<(), ModelError> {
        let (artifact, start, end) = coordinates(d, &anchor)?;
        let source = need(&d.source.core.artifacts, artifact)?;
        if start < 0 || end < start || end > source.byte_len {
            return Err(invalid("retrieval original coordinates outside artifact"));
        }
        self.values
            .insert(&mut self.charge, (artifact, start, end))?;
        Ok(())
    }
    pub fn iter(&self) -> impl Iterator<Item = &(Id<SourceArtifact>, i64, i64)> {
        self.values.iter()
    }
}
pub fn root_ranges(
    d: &Data,
    id: Id<c1::EvidenceRoot>,
    b: &ResourceBudget,
) -> Result<Ranges, ModelError> {
    let root = need(&d.evidence.roots, id)?;
    let mut ranges = Ranges::new(b);
    match need(&d.evidence.subjects, root.subject)? {
        c1::RootSubject::Member { member } => {
            for (_,anchor,_) in super::construction::definitions(d,*member,root.context)? {
                if let Some(anchor)=anchor { for (context,_) in super::construction::enclosing(d,&anchor,root.context)?{ranges.add(d,context)?;} ranges.add(d,anchor)?; }
            }
        }
        c1::RootSubject::Scenario { scenario } => {
            for span in d
                .evidence
                .spans
                .iter()
                .filter(|span| span.scenario == *scenario)
            {
                ranges.add(
                    d,
                    AnchorSource::Original {
                        source: span.source,
                    },
                )?;
            }
        }
        c1::RootSubject::Document { observation } => {
            let document = need(&d.source.facts.documents, *observation)?;
            let mut any = false;
            for passage in d.source.facts.passages.iter().filter(|passage| {
                d.source
                    .core
                    .qualifications
                    .get(passage.qualification)
                    .is_some_and(|q| q.context == root.context)
            }) {
                let node = need(&d.source.facts.nodes, passage.passage.id())?;
                let anchor = AnchorSource::Span { span: node.span() };
                if coordinates(d, &anchor)?.0 != document.source {
                    continue;
                }
                any = true;
                ranges.add(d, anchor)?;
            }
            if !any {
                ranges.add(
                    d,
                    AnchorSource::Artifact {
                        artifact: document.source,
                    },
                )?;
            }
        }
        c1::RootSubject::Deployment { deployment } => {
            let row = need(&d.evidence.deployments, *deployment)?;
            let observation = need(&d.source.facts.deployment, row.observation)?;
            ranges.add(
                d,
                AnchorSource::Span {
                    span: observation.span,
                },
            )?;
        }
        c1::RootSubject::Option { option } => {for anchor in super::construction::option_anchors(d,need(&d.source.catalog.options,*option)?)?{ranges.add(d,anchor)?;}},
        c1::RootSubject::Release { .. } => {},
        c1::RootSubject::Source{artifact}=>ranges.add(d,AnchorSource::Artifact{artifact:*artifact})?,
    }
    Ok(ranges)
}
