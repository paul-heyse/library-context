//! Original-code mapping is admitted before any extracted statement offset is translated.
//! This pure operation owns no native span or evidence record and never searches for code.
use super::documentary;
use crate::domain::{
    analysis::native::NativeAssertionPremise,
    assertion::{Evidence, EvidenceSourceSpanId},
    attribution::Fidelity,
    documents::{CodeBlockObservation, DocumentNode},
    resources::ResourceBudget,
    source::{Occurrence, SourceArtifact},
    *,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingBoundary {
    MissingMaterialization,
    NativeEvidenceUnavailable,
    ForeignContext,
    UnsupportedFence,
    TransformedCode,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalRange {
    pub artifact: Id<SourceArtifact>,
    pub start: i64,
    pub end: i64,
}
/// Opaque receipt over exact whole original and materialized bytes, not an independent span.
pub struct FenceMapping {
    block: Id<CodeBlockObservation>,
    premise: Id<NativeAssertionPremise>,
    span: EvidenceSourceSpanId,
    materialized: Id<SourceArtifact>,
    original: OriginalRange,
    code_len: i64,
}
impl FenceMapping {
    pub fn block(&self) -> Id<CodeBlockObservation> {
        self.block
    }
    pub fn premise(&self) -> Id<NativeAssertionPremise> {
        self.premise
    }
    pub fn span(&self) -> EvidenceSourceSpanId {
        self.span
    }
    pub fn materialized(&self) -> Id<SourceArtifact> {
        self.materialized
    }
    /// Offsets must name an actual occurrence of this admitted captured artifact.
    pub fn occurrence(
        &self,
        d: &documentary::Data,
        row: &Occurrence,
        b: &ResourceBudget,
    ) -> Result<OriginalRange, ModelError> {
        if d.occurrences.get(row.id()) != Some(row)
            || row.source != self.materialized
            || row.start < 0
            || row.end < row.start
            || row.end > self.code_len
        {
            return Err(invalid("statement is not in admitted materialized code"));
        }
        let _bytes = documentary::read_range(d, row.source, row.start, row.end, b)?;
        Ok(OriginalRange {
            artifact: self.original.artifact,
            start: self.original.start + row.start,
            end: self.original.start + row.end,
        })
    }
}
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(r: &normalized::Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    r.get(id)
        .ok_or_else(|| invalid("original code mapping input absent"))
}

/// Simple unindented fenced blocks only. The parser convention drops at most the one LF directly
/// before the closing delimiter. Any other normalization (including CRLF or indentation) refuses.
fn fence_body(original: &str, code: &str) -> Result<(usize, usize), MappingBoundary> {
    let first = original
        .find('\n')
        .ok_or(MappingBoundary::UnsupportedFence)?;
    let opening = &original[..first];
    let marker = opening
        .as_bytes()
        .first()
        .copied()
        .ok_or(MappingBoundary::UnsupportedFence)?;
    if marker != b'`' && marker != b'~' {
        return Err(MappingBoundary::UnsupportedFence);
    }
    let count = opening.bytes().take_while(|c| *c == marker).count();
    if count < 3 || opening[count..].contains(['`', '\r']) {
        return Err(MappingBoundary::UnsupportedFence);
    }
    let tail = original.strip_suffix('\n').unwrap_or(original);
    let closing_start = tail
        .rfind('\n')
        .map(|n| n + 1)
        .ok_or(MappingBoundary::UnsupportedFence)?;
    if closing_start < first + 1 {
        return Err(MappingBoundary::UnsupportedFence);
    }
    let closing = &tail[closing_start..];
    let close_count = closing.bytes().take_while(|c| *c == marker).count();
    if close_count < count
        || !closing[close_count..]
            .bytes()
            .all(|c| c == b' ' || c == b'\t')
    {
        return Err(MappingBoundary::UnsupportedFence);
    }
    let body = &original[first + 1..closing_start];
    if body == code {
        return Ok((first + 1, closing_start));
    }
    if body.strip_suffix('\n') == Some(code) {
        return Ok((first + 1, closing_start - 1));
    }
    Err(MappingBoundary::TransformedCode)
}
pub fn admit_fence(
    d: &documentary::Data,
    block: &CodeBlockObservation,
    context: Id<attribution::AnalysisContext>,
    b: &ResourceBudget,
) -> Result<Result<FenceMapping, MappingBoundary>, ModelError> {
    block.validate()?;
    let Some(materialized) = block.materialized else {
        return Ok(Err(MappingBoundary::MissingMaterialization));
    };
    let q = need(&d.qualifications, block.qualification)?;
    if q.context != context {
        return Ok(Err(MappingBoundary::ForeignContext));
    }
    let premise=d.native.iter().filter(|p|matches!(p,NativeAssertionPremise::CodeBlockObservation{assertion,..}if *assertion==block.id())).filter(|p|d.native_qualifications.iter().any(|n|n.premise==p.id()&&n.qualification==block.qualification&&n.family==attribution::FactFamily::Docs&&n.fidelity!=Fidelity::DisplayOnly)).min_by_key(|p|p.id());
    let Some(premise) = premise else {
        return Ok(Err(MappingBoundary::NativeEvidenceUnavailable));
    };
    let node = need(&d.nodes, block.block.id())?;
    let DocumentNode::CodeBlock { span, .. } = node else {
        return Err(invalid("code block names another document node variant"));
    };
    let Evidence::SourceSpan { source, start, end } = need(&d.canonical_evidence, span.id())?
    else {
        return Err(invalid("code block anchor is not original span"));
    };
    let original_artifact = need(&d.artifacts, *source)?;
    let artifact = need(&d.artifacts, materialized)?;
    if original_artifact.input != artifact.input
        || q.scope != (source::CoverageScope::Artifact { artifact: *source }).id()
    {
        return Ok(Err(MappingBoundary::ForeignContext));
    }
    let original = documentary::read_range(d, *source, *start, *end, b)?;
    let captured = documentary::read_range(d, materialized, 0, artifact.byte_len, b)?;
    if artifact.content != block.content || captured.value != block.code {
        return Ok(Err(MappingBoundary::TransformedCode));
    }
    let (body_start, body_end) = match fence_body(&original.value, &block.code) {
        Ok(r) => r,
        Err(reason) => return Ok(Err(reason)),
    };
    Ok(Ok(FenceMapping {
        block: block.id(),
        premise: premise.id(),
        span: *span,
        materialized,
        original: OriginalRange {
            artifact: *source,
            start: *start + body_start as i64,
            end: *start + body_end as i64,
        },
        code_len: artifact.byte_len,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id<T>(n: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    fn fixture() -> (
        ResourceBudget,
        documentary::Data,
        CodeBlockObservation,
        Occurrence,
    ) {
        let (b, mut d, _) = documentary::tests::fixture("\"Run.\"", "Run.");
        let context = d.core_invocations.iter().next().unwrap().context;
        let input = d.artifacts.iter().next().unwrap().input;
        let original = "Before.\n```python\nπ()\n```\nAfter.\n";
        let code = "π()";
        let doc =
            SourceArtifact::from_bytes(input, "guide.mdx".into(), original.as_bytes()).unwrap();
        let materialized = SourceArtifact::from_bytes(
            input,
            "_lctx_blocks/guide/block.py".into(),
            code.as_bytes(),
        )
        .unwrap();
        for (artifact, bytes) in [
            (&doc, original.as_bytes()),
            (&materialized, code.as_bytes()),
        ] {
            d.artifacts.insert(artifact.clone()).unwrap();
            for chunk in artifact::ArtifactChunk::split(artifact, bytes).unwrap() {
                d.chunks.insert(chunk).unwrap();
            }
        }
        let evidence = Evidence::SourceSpan {
            source: doc.id(),
            start: 8,
            end: 26,
        };
        let span = assertion::EvidenceSourceSpanId::of(&evidence).unwrap();
        d.canonical_evidence.insert(evidence).unwrap();
        let node = DocumentNode::CodeBlock { span, ordinal: 0 };
        d.nodes.insert(node.clone()).unwrap();
        let q = assertion::AssertionQualification {
            context,
            scope: (source::CoverageScope::Artifact { artifact: doc.id() }).id(),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: assertion::Approximation::Exact,
        };
        d.qualifications.insert(q.clone()).unwrap();
        let block = CodeBlockObservation {
            qualification: q.id(),
            block: documents::DocumentNodeCodeBlockId::of(&node).unwrap(),
            passage: documents::DocumentNodePassageId::of(&DocumentNode::Passage {
                span,
                ordinal: 0,
            })
            .unwrap(),
            language: Some("python".into()),
            meta: None,
            code: code.into(),
            content: materialized.content,
            module_path: Some(materialized.path.clone()),
            materialized: Some(materialized.id()),
        };
        let premise = NativeAssertionPremise::CodeBlockObservation {
            assertion: block.id(),
            support: id(32),
        };
        d.native_qualifications
            .insert(analysis::native::NativeQualification {
                premise: premise.id(),
                qualification: q.id(),
                family: attribution::FactFamily::Docs,
                fidelity: Fidelity::NativeStructural,
                status: analysis::policy::EvidenceStatus::Documented,
            })
            .unwrap();
        d.native.insert(premise).unwrap();
        let mut occurrence = d.occurrences.iter().next().unwrap().clone();
        occurrence.source = materialized.id();
        occurrence.start = 0;
        occurrence.end = code.len() as i64;
        d.occurrences.insert(occurrence.clone()).unwrap();
        (b, d, block, occurrence)
    }
    #[test]
    fn native_whole_fence_admission_preserves_original_anchor_and_exact_occurrence() {
        let (b, d, block, occurrence) = fixture();
        let context = d.qualifications.get(block.qualification).unwrap().context;
        let admitted = admit_fence(&d, &block, context, &b).unwrap().unwrap();
        let range = admitted.occurrence(&d, &occurrence, &b).unwrap();
        assert_eq!((range.start, range.end), (18, 22));
        assert_eq!(d.artifacts.get(range.artifact).unwrap().path, "guide.mdx");
        assert_eq!(
            admitted.span(),
            d.nodes.get(block.block.id()).unwrap().span()
        );
        let mut forged = occurrence.clone();
        forged.start += 1;
        assert!(admitted.occurrence(&d, &forged, &b).is_err());
        assert!(matches!(
            admit_fence(&d, &block, id(80), &b).unwrap(),
            Err(MappingBoundary::ForeignContext)
        ));
    }
    #[test]
    fn missing_native_pair_changed_materialization_and_relocated_span_refuse() {
        for case in 0..3 {
            let (b, mut d, mut block, _) = fixture();
            let context = d.qualifications.get(block.qualification).unwrap().context;
            match case {
                0 => d.native_qualifications = normalized::Rows::new(&b),
                1 => {
                    block.materialized = Some(
                        d.artifacts
                            .iter()
                            .find(|a| a.path == "api.py")
                            .unwrap()
                            .id(),
                    );
                }
                _ => {
                    let evidence = Evidence::SourceSpan {
                        source: d
                            .artifacts
                            .iter()
                            .find(|a| a.path == "guide.mdx")
                            .unwrap()
                            .id(),
                        start: 7,
                        end: 25,
                    };
                    let span = assertion::EvidenceSourceSpanId::of(&evidence).unwrap();
                    d.canonical_evidence.insert(evidence).unwrap();
                    let node = DocumentNode::CodeBlock { span, ordinal: 0 };
                    d.nodes.insert(node.clone()).unwrap();
                    block.block = documents::DocumentNodeCodeBlockId::of(&node).unwrap();
                }
            }
            assert!(admit_fence(&d, &block, context, &b).unwrap().is_err());
        }
    }
    #[test]
    fn simple_fence_translation_is_whole_body_equality_not_search() {
        for (original, code, start) in [
            ("```python\nrun()\n```", "run()", 10),
            ("~~~~python\nrun()\n~~~~\n", "run()\n", 11),
            ("```python\nπ()\n```\n", "π()", 10),
            ("```python\n```", "", 10),
        ] {
            let mapped = fence_body(original, code).unwrap();
            assert_eq!(mapped.0, start);
            assert_eq!(&original[mapped.0..mapped.1], code);
        }
    }
    #[test]
    fn transformed_indented_partial_and_relocated_code_cannot_map() {
        for (original, code) in [
            ("  ```python\nrun()\n  ```", "run()"),
            ("```python\n  run()\n```", "run()"),
            ("```python\r\nrun()\r\n```", "run()"),
            ("```python\nsetup()\nrun()\n```", "run()"),
            ("text run()\n```python\nother()\n```", "run()"),
            ("```python\nrun()\n~~~", "run()"),
            ("````python\nrun()\n```", "run()"),
        ] {
            assert!(fence_body(original, code).is_err(), "{original:?}");
        }
    }
}
