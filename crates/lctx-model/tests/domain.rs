use lctx_model::{Domain, domain::{Batch, ContentHash, Id, ModelError, Record, Relation, ValidatedModel, model, attribution::*, input::*, source::*, stages::*}};

fn source() -> SourceArtifact {
    let input = InputRevision::from_entries(vec![ManifestEntry { path: "demo.py".into(), content: ContentHash::of(b"x = 1"), byte_len: 5 }]).unwrap();
    SourceArtifact::from_bytes(input.id(), "demo.py".into(), b"x = 1").unwrap()
}
#[test]
fn production_records_round_trip_explicit_schema() {
    let model = model().unwrap();
    let source = source();
    let batch = Batch::new(&model, vec![source.clone()]).unwrap();
    let roundtrip = Batch::<SourceArtifact>::read(&model, batch.arrow()).unwrap();
    assert_eq!(roundtrip.rows(), &[source]);
    assert!(std::sync::Arc::ptr_eq(&roundtrip.arrow().columns()[0], &batch.arrow().columns()[0]));
    let context = AnalysisContext { python_version: "3.14".into(), python_platform: "linux".into(), search_path: vec!["src".into(), "stubs".into()], site_package_path: vec![], config_digest: ContentHash::of(b"config"), environment_digest: ContentHash::of(b"env"), lock_digest: None };
    let batch = Batch::new(&model, vec![context.clone()]).unwrap();
    assert_eq!(Batch::<AnalysisContext>::read(&model, batch.arrow()).unwrap().rows(), &[context]);
    assert!(Batch::<SourceArtifact>::new(&model, vec![]).unwrap().rows().is_empty());
}
#[test]
fn source_kind_and_structural_roles_separate_identity() {
    let py = source();
    let mut stub = py.clone(); stub.path = "demo.pyi".into();
    assert_ne!(py.id(), stub.id());
    for path in ["/demo.py", "a/../demo.py", "a/./demo.py", "a//demo.py", "a\0b"] {
        let mut invalid = py.clone(); invalid.path = path.into();
        assert!(Batch::new(&model().unwrap(), vec![invalid]).is_err());
    }
    let a = Occurrence { source: py.id(), start: 0, end: 5, syntax_kind: SyntaxKind::WithItem, role: OccurrenceRole::WithItem, structural_path: vec![0] };
    let mut b = a.clone(); b.structural_path = vec![1];
    assert_ne!(a.id(), b.id());
    let mut invalid = a.clone(); invalid.start = 6;
    assert!(Batch::new(&model().unwrap(), vec![invalid]).is_err());
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "named_items")]
struct NamedItem {
    #[model(key)] name: String,
    payload: String,
}
#[test]
fn key_and_payload_conflicts_are_distinct() {
    let model = ValidatedModel::validate(vec![Relation::of::<NamedItem>()]).unwrap();
    let a = NamedItem { name: "a".into(), payload: "one".into() };
    let b = NamedItem { name: "a".into(), payload: "two".into() };
    assert_eq!(a.key(), b.key()); assert_eq!(a.id(), b.id());
    assert!(matches!(Batch::new(&model, vec![a.clone(), b]), Err(ModelError::Conflict("named_items"))));
    assert_eq!(Batch::new(&model, vec![a.clone(), a]).unwrap().rows().len(), 1);
}
#[test]
fn reference_membership_is_checked_and_cycles_are_allowed() {
    assert!(ValidatedModel::validate(vec![Relation::of::<Module>()]).is_err());
    assert!(ValidatedModel::validate(vec![Relation::of::<Package>(), Relation::of::<Package>()]).is_err());
    #[derive(Debug, Clone, PartialEq, Eq, Domain)]
    #[model(name = "recursive_nodes")]
    struct RecursiveNode { #[model(key)] name: String, parent: Option<Id<RecursiveNode>> }
    assert!(ValidatedModel::validate(vec![Relation::of::<RecursiveNode>()]).is_ok());
}
#[test]
fn reference_and_provenance_are_orthogonal() {
    let model = model().unwrap();
    let run = model.require::<SyntaxSupport>().unwrap().fields().iter().find(|f| f.name() == "run").unwrap();
    assert!(run.is_provenance()); assert!(run.is_key());
    assert_eq!(run.target().unwrap().1, ProviderRun::NAME);
}
#[test]
fn stages_refuse_self_cycles_missing_writers_and_unknown_relations() {
    let model = model().unwrap();
    let r = RelationUse::of::<Package>();
    assert!(Schedule::build(&model, vec![Stage { name: "cycle", inputs: vec![r], outputs: vec![r], profiles: vec![Profile::Catalog, Profile::Behavioral], effect: Effect::Pure, code: ContentHash::of(b"test-producer"), configuration: ContentHash::of(b"test-config") }], &[r], Profile::Catalog).is_err());
    assert!(Schedule::build(&model, vec![], &[r], Profile::Catalog).is_err());
    let good = Stage { name: "packages", inputs: vec![], outputs: vec![r], profiles: vec![Profile::Catalog, Profile::Behavioral], effect: Effect::Pure, code: ContentHash::of(b"test-producer"), configuration: ContentHash::of(b"test-config") };
    assert_eq!(Schedule::build(&model, vec![good.clone()], &[r], Profile::Catalog).unwrap().stages()[0].name, "packages");
    let mut second = good.clone(); second.name = "also_packages";
    assert!(Schedule::build(&model, vec![good, second], &[r], Profile::Catalog).is_err());
    assert!(Schedule::build(&model, vec![], &[RelationUse::of::<NamedItem>()], Profile::Catalog).is_err());
}
#[test]
fn decoded_ids_and_physical_schemas_are_verified() {
    use std::sync::Arc;
    use arrow_array::{ArrayRef, FixedSizeBinaryArray, RecordBatch};
    let model = model().unwrap();
    let batch = Batch::new(&model, vec![source()]).unwrap();
    let mut columns = batch.arrow().columns().to_vec();
    columns[0] = Arc::new(FixedSizeBinaryArray::try_from_iter([[0_u8;16]].into_iter()).unwrap()) as ArrayRef;
    let corrupt = RecordBatch::try_new(batch.arrow().schema(), columns).unwrap();
    assert!(matches!(Batch::<SourceArtifact>::read(&model, &corrupt), Err(ModelError::Identity(_))));
    assert!(Batch::<Module>::read(&model, batch.arrow()).is_err());
}

#[test]
fn semantic_validation_revision_changes_model_but_not_arrow_schema() {
    fn first(_: &Before) -> Result<(), ModelError> { Ok(()) }
    fn second(row: &After) -> Result<(), ModelError> {
        if row.name.is_empty() { return Err(ModelError::Invalid("empty".into())); } Ok(())
    }
    #[derive(Debug, Clone, PartialEq, Eq, Domain)]
    #[model(name = "contracts", validate = first)]
    struct Before { #[model(key)] name: String }
    #[derive(Debug, Clone, PartialEq, Eq, Domain)]
    #[model(name = "contracts", validate = second)]
    struct After { #[model(key)] name: String }
    assert_eq!(Before::schema(), After::schema());
    let before = ValidatedModel::validate(vec![Relation::of::<Before>()]).unwrap();
    let after = ValidatedModel::validate(vec![Relation::of::<After>()]).unwrap();
    assert_ne!(before.digest(), after.digest());
}

#[test]
fn tagged_sums_preserve_active_optional_null_and_reject_inactive_payloads() {
    use lctx_model::DomainSum;
    use std::sync::Arc;
    use arrow_array::{ArrayRef, Int16Array, RecordBatch, StringArray};
    #[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
    #[model(name = "observed_defaults")]
    enum ObservedDefault {
        #[model(code = 0)] Absent,
        #[model(code = 1)] Present { rendered: Option<String> },
    }
    let model = ValidatedModel::validate(vec![Relation::of::<ObservedDefault>()]).unwrap();
    let rows = vec![ObservedDefault::Absent, ObservedDefault::Present { rendered: None }, ObservedDefault::Present { rendered: Some("None".into()) }];
    let batch = Batch::new(&model, rows).unwrap();
    assert_eq!(Batch::<ObservedDefault>::read(&model, batch.arrow()).unwrap().rows(), batch.rows());
    assert_ne!(ObservedDefault::Absent.id(), ObservedDefault::Present { rendered: None }.id());
    let absent = Batch::new(&model, vec![ObservedDefault::Absent]).unwrap();
    let mut columns = absent.arrow().columns().to_vec();
    columns[2] = Arc::new(StringArray::from(vec!["illegal"])) as ArrayRef;
    let malformed = RecordBatch::try_new(absent.arrow().schema(), columns).unwrap();
    assert!(ObservedDefault::decode(&malformed).is_err());
    let mut columns = absent.arrow().columns().to_vec();
    columns[1] = Arc::new(Int16Array::from(vec![9])) as ArrayRef;
    let unknown = RecordBatch::try_new(absent.arrow().schema(), columns).unwrap();
    assert!(ObservedDefault::decode(&unknown).is_err());
}

#[test]
fn subtype_references_require_the_right_arm() {
    let source = source();
    let module = Module { source: source.id(), qualified_name: "x".into() };
    let module_scope = CoverageScope::Module { module: module.id() };
    let release_scope = CoverageScope::Input { input: source.input };
    assert!(CoverageScopeModuleId::of(&module_scope).is_ok());
    assert!(CoverageScopeModuleId::of(&release_scope).is_err());
    #[derive(Debug, Clone, PartialEq, Eq, Domain)]
    #[model(name = "module_scope_links")]
    struct ModuleScopeLink { #[model(key)] scope: CoverageScopeModuleId }
    let mut relations = model().unwrap().relations().to_vec(); relations.push(Relation::of::<ModuleScopeLink>());
    let model = ValidatedModel::validate(relations).unwrap();
    let link = ModuleScopeLink { scope: CoverageScopeModuleId::of(&module_scope).unwrap() };
    let batch = Batch::new(&model, vec![link.clone()]).unwrap();
    assert_eq!(Batch::<ModuleScopeLink>::read(&model, batch.arrow()).unwrap().rows(), &[link]);
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "binary_evidence")]
struct BinaryEvidence {
    #[model(key)] name: String,
    bytes: lctx_model::domain::EvidenceBytes,
    optional: Option<lctx_model::domain::EvidenceBytes>,
}
#[test]
fn binary_evidence_and_streamed_content_preserve_payload_and_batch_independence() {
    use lctx_model::domain::EvidenceBytes;
    let model = ValidatedModel::validate(vec![Relation::of::<BinaryEvidence>()]).unwrap();
    let batch = Batch::new(&model, vec![
        BinaryEvidence { name: "invalid_utf8".into(), bytes: EvidenceBytes(vec![0, 255, 128, 0]), optional: None },
        BinaryEvidence { name: "empty".into(), bytes: EvidenceBytes(vec![]), optional: Some(EvidenceBytes(vec![])) },
    ]).unwrap();
    assert_eq!(Batch::<BinaryEvidence>::read(&model, batch.arrow()).unwrap().rows(), batch.rows());
    let relation = model.require::<BinaryEvidence>().unwrap();
    let mut all = relation.content(); relation.hash_rows(batch.arrow(), &mut all).unwrap();
    let mut chunks = relation.content();
    for i in 0..batch.rows().len() { relation.hash_rows(&batch.arrow().slice(i, 1), &mut chunks).unwrap(); }
    assert_eq!(all.finish(), chunks.finish());
    let mut duplicates = relation.content();
    relation.hash_rows(batch.arrow(), &mut duplicates).unwrap();
    assert!(relation.hash_rows(batch.arrow(), &mut duplicates).is_err());
    let mut changed = batch.rows()[0].clone();
    let before = changed.content_digest(); let key = changed.id();
    changed.bytes.0.push(1);
    assert_eq!(key, changed.id()); assert_ne!(before, changed.content_digest());
}

#[test]
fn input_identity_is_content_based_and_acquisition_is_independent() {
    let entry = |path: &str, bytes: &[u8]| ManifestEntry { path: path.into(), content: ContentHash::of(bytes), byte_len: bytes.len() as i64 };
    let first = InputRevision::from_entries(vec![entry("a.py", b"x=1"), entry("b.pyi", b"x:int")]).unwrap();
    let reordered = InputRevision::from_entries(vec![entry("b.pyi", b"x:int"), entry("a.py", b"x=1")]).unwrap();
    assert_eq!(first.id(), reordered.id());
    let changed = InputRevision::from_entries(vec![entry("a.py", b"x=2"), entry("b.pyi", b"x:int")]).unwrap();
    assert_ne!(first.id(), changed.id());
    assert!(InputRevision::from_entries(vec![entry("a.py", b"a"), entry("a.py", b"b")]).is_err());
    assert!(InputRevision::from_entries(vec![entry("/checkout/a.py", b"a")]).is_err());
    let origin = InputOrigin::Tree { label: "same label".into() };
    assert_ne!(InputAcquisition { input: first.id(), origin: origin.id() }.id(), InputAcquisition { input: changed.id(), origin: origin.id() }.id());
    let model = model().unwrap();
    let origins = Batch::new(&model, vec![origin, InputOrigin::Installed { library: "demo".into(), requirement: "demo==1".into(), lock_digest: ContentHash::of(b"lock"), installer: None }, InputOrigin::Corpus { repository: "upstream".into(), revision: "commit".into() }]).unwrap();
    assert_eq!(Batch::<InputOrigin>::read(&model, origins.arrow()).unwrap().rows(), origins.rows());
    let artifact = SourceArtifact::from_bytes(first.id(), "a.py".into(), &[255, 0]).unwrap();
    assert!(lctx_model::domain::artifact::ArtifactChunk::split(&artifact, &[0, 0]).is_err());
}

#[test]
fn provider_invocation_and_coverage_require_the_exact_requested_contract() {
    let artifact = source();
    let provider = Provider { tool: "parser".into(), revision: "1".into(), build_digest: ContentHash::of(b"provider") };
    let context = AnalysisContext { python_version: "3.14".into(), python_platform: "linux".into(), search_path: vec![], site_package_path: vec![], config_digest: ContentHash::of(b"config"), environment_digest: ContentHash::of(b"env"), lock_digest: None };
    let (run, families) = ProviderRun::new(provider.id(), context.id(), artifact.input, context.config_digest, [FactFamily::Syntax]).unwrap();
    let (extra, _) = ProviderRun::new(provider.id(), context.id(), artifact.input, context.config_digest, [FactFamily::Syntax, FactFamily::Flow]).unwrap();
    assert_ne!(run.id(), extra.id());
    let (shuffled, _) = ProviderRun::new(provider.id(), context.id(), artifact.input, context.config_digest, [FactFamily::Flow, FactFamily::Syntax, FactFamily::Syntax]).unwrap();
    assert_eq!(shuffled.id(), extra.id());
    let scope = CoverageScope::Artifact { artifact: artifact.id() };
    let expected = CoverageExpectation { input: artifact.input, scope: scope.id(), provider: provider.id(), context: context.id(), family: FactFamily::Syntax, run: Some(run.id()) };
    let mut outcome = ProviderCoverage { scope: scope.id(), provider: provider.id(), context: context.id(), family: FactFamily::Syntax, run: Some(run.id()), status: CoverageStatus::CompleteUnderStatedModel, reason: None, diagnostic: None };
    let check = |outcome: &ProviderCoverage| validate_coverage_contract(std::slice::from_ref(&expected), std::slice::from_ref(outcome), std::slice::from_ref(&run), &families);
    assert!(check(&outcome).is_ok()); // complete-empty does not require invented observations
    assert!(validate_coverage_contract(std::slice::from_ref(&expected), &[], std::slice::from_ref(&run), &families).is_err());
    assert!(validate_coverage_contract(std::slice::from_ref(&expected), std::slice::from_ref(&outcome), std::slice::from_ref(&run), &[]).is_err());
    outcome.status = CoverageStatus::Partial;
    assert!(check(&outcome).is_err());
    outcome.reason = Some(ObligationKind::SyntaxError);
    assert!(check(&outcome).is_ok());
    outcome.status = CoverageStatus::Failed;
    assert!(check(&outcome).is_err());
    outcome.status = CoverageStatus::Unavailable;
    outcome.reason = Some(ObligationKind::UndecodableSource);
    assert!(check(&outcome).is_ok());
    let mut unrequested = expected.clone(); unrequested.run = None;
    outcome.status = CoverageStatus::NotRequested; outcome.run = None; outcome.reason = None;
    assert!(validate_coverage_contract(&[unrequested], &[outcome.clone()], &[], &[]).is_ok());
    assert!(check(&outcome).is_err());
}

#[test]
fn indexed_attachment_matches_independent_scalar_oracle_and_retains_ambiguity() {
    use lctx_model::domain::attachment::*;
    let artifact = SourceArtifact::from_bytes(source().input, "demo.py".into(), &[b' '; 100]).unwrap();
    let other = SourceArtifact::from_bytes(artifact.input, "demo.pyi".into(), &[b' '; 100]).unwrap();
    let mut occurrences = Vec::new();
    for (kind, role) in [(SyntaxKind::ExprName, OccurrenceRole::Read), (SyntaxKind::ExprAttribute, OccurrenceRole::Read), (SyntaxKind::ExprSubscript, OccurrenceRole::Read)] {
        for i in 0..40 {
            occurrences.push(Occurrence { source: artifact.id(), start: i, end: 100 - i, syntax_kind: kind, role, structural_path: vec![i as i32] });
        }
    }
    let duplicate_span = Occurrence { source: artifact.id(), start: 20, end: 80, syntax_kind: SyntaxKind::ExprName, role: OccurrenceRole::Read, structural_path: vec![100] };
    occurrences.push(duplicate_span);
    let index = OccurrenceIndex::new(&occurrences,lctx_model::domain::resources::ResourceBudget::fixed(1024*1024).unwrap()).unwrap();
    for kind in [SyntaxKind::ExprName, SyntaxKind::ExprAttribute, SyntaxKind::ExprSubscript] {
        for start in 0..105 {
            for end in [start, start + 1, start + 15] {
                let query = AttachmentQuery { source: artifact.id(), start, end, syntax_kind: kind, role: OccurrenceRole::Read, structural_path: None };
                let candidates: Vec<_> = occurrences.iter().filter(|o| o.source == query.source && o.syntax_kind == kind && o.role == query.role && o.start <= start && o.end >= end).collect();
                let expected = if let Some(width) = candidates.iter().map(|o| o.end-o.start).min() {
                    let mut ids: Vec<_> = candidates.iter().filter(|o| o.end-o.start == width).map(|o| o.id()).collect(); ids.sort();
                    if ids.len() > 1 { Attachment::Ambiguous(ids) }
                    else if width == end-start { Attachment::Exact(ids[0]) } else { Attachment::Innermost(ids[0]) }
                } else { Attachment::Unmatched };
                assert_eq!(index.attach(&query, AttachmentBudget::default()).unwrap().value(), &expected);
            }
        }
    }
    let mut query = AttachmentQuery { source: artifact.id(), start: 20, end: 80, syntax_kind: SyntaxKind::ExprName, role: OccurrenceRole::Read, structural_path: None };
    assert!(matches!(index.attach(&query, AttachmentBudget::default()).unwrap().value(), Attachment::Ambiguous(_)));
    assert_eq!(index.attach(&query, AttachmentBudget { visited_nodes: 0, alternatives: 256 }).unwrap().value(), &Attachment::BudgetExceeded);
    query.structural_path = Some(vec![20]);
    assert!(matches!(index.attach(&query, AttachmentBudget::default()).unwrap().value(), Attachment::Exact(_)));
    query.source = other.id();
    assert_eq!(index.attach(&query, AttachmentBudget::default()).unwrap().value(), &Attachment::Unmatched);
    occurrences.reverse();
    let reverse = OccurrenceIndex::new(&occurrences,lctx_model::domain::resources::ResourceBudget::fixed(1024*1024).unwrap()).unwrap();
    query.source = artifact.id(); query.structural_path = None;
    assert_eq!(reverse.attach(&query, AttachmentBudget::default()).unwrap().value(), index.attach(&query, AttachmentBudget::default()).unwrap().value());
}

#[test]
fn canonical_artifact_chunks_preserve_original_bytes_and_refuse_incomplete_proofs() {
    use lctx_model::domain::artifact::*;
    let input = InputRevision::from_entries(vec![]).unwrap();
    let mut bytes = vec![255; ARTIFACT_CHUNK_BYTES * 2 + 17];
    bytes[ARTIFACT_CHUNK_BYTES] = 0;
    let artifact = SourceArtifact::from_bytes(input.id(), "capture.bin".into(), &bytes).unwrap();
    let chunks: Vec<_> = ArtifactChunk::split(&artifact, &bytes).unwrap().collect();
    assert_eq!(chunks.iter().map(|c| c.body.0.len()).collect::<Vec<_>>(), [ARTIFACT_CHUNK_BYTES, ARTIFACT_CHUNK_BYTES, 17]);
    let verify = |chunks: &[ArtifactChunk]| {
        let mut proof = ArtifactVerifier::new(&artifact)?;
        for chunk in chunks { proof.push(chunk)?; }
        proof.finish()
    };
    verify(&chunks).unwrap();
    assert!(verify(&chunks[..2]).is_err());
    assert!(verify(&[chunks[1].clone(), chunks[0].clone(), chunks[2].clone()]).is_err());
    assert!(verify(&[chunks[0].clone(), chunks[0].clone()]).is_err());
    let mut corrupt = chunks.clone(); corrupt[0].body.0[0] = 1;
    assert!(verify(&corrupt).is_err());
    let mut swapped = chunks.clone(); swapped.swap(0, 1); swapped[0].ordinal = 0; swapped[1].ordinal = 1;
    assert!(verify(&swapped).is_err());
    let other = SourceArtifact::from_bytes(input.id(), "other.bin".into(), &bytes).unwrap();
    let mut misplaced = chunks.clone(); misplaced[0].artifact = other.id();
    assert!(verify(&misplaced).is_err());
    let empty = SourceArtifact::from_bytes(input.id(), "empty.bin".into(), b"").unwrap();
    assert_eq!(ArtifactChunk::split(&empty, b"").unwrap().count(), 0);
    ArtifactVerifier::new(&empty).unwrap().finish().unwrap();
    let model = model().unwrap();
    let a = Batch::new(&model, chunks.clone()).unwrap();
    let b = Batch::new(&model, chunks.into_iter().rev().collect()).unwrap();
    assert_eq!(a.rows(), b.rows());
    assert_eq!(ArtifactChunk::decode(a.arrow()).unwrap(), a.rows());
    // This is the actual model-owned cross-relation validator, not a test-only reconstruction.
    let invariant = SourceArtifact::invariants().remove(0);
    for missing in [false, true] {
        let mut check = (invariant.create)();
        check.visit(SourceArtifact::NAME, Batch::new(&model, vec![artifact.clone(), empty.clone()]).unwrap().arrow()).unwrap();
        if !missing {
            for chunk in ArtifactChunk::split(&artifact, &bytes).unwrap() {
                check.visit(ArtifactChunk::NAME, Batch::new(&model, vec![chunk]).unwrap().arrow()).unwrap();
            }
        }
        assert_eq!(check.finish().is_err(), missing);
    }
}

#[test]
fn fragmented_capture_has_one_canonical_representation_and_detects_changed_source() {
    use lctx_model::domain::artifact::*;
    let bytes: Vec<u8> = (0..ARTIFACT_CHUNK_BYTES * 2 + 73).map(|i| (i % 251) as u8).collect();
    let artifact = SourceArtifact::from_bytes(source().input, "source.bin".into(), &bytes).unwrap();
    let expected: Vec<_> = ArtifactChunk::split(&artifact, &bytes).unwrap().collect();
    for fragment in [1, 8191, ARTIFACT_CHUNK_BYTES, ARTIFACT_CHUNK_BYTES + 101] {
        let mut emitted = Vec::new();
        let mut capture = ArtifactCapture::new(&artifact).unwrap();
        for part in bytes.chunks(fragment) { capture.feed(part, |row| { emitted.push(row); Ok(()) }).unwrap(); }
        capture.finish(|row| { emitted.push(row); Ok(()) }).unwrap();
        assert_eq!(emitted, expected);
    }
    let mut rejected = ArtifactCapture::new(&artifact).unwrap();
    assert!(rejected.feed(&bytes, |_| Err(ModelError::Invalid("sink refused".into()))).is_err());
    assert!(rejected.finish(|_| Ok(())).is_err());
    let mut changed = bytes.clone(); changed[0] ^= 1;
    let mut capture = ArtifactCapture::new(&artifact).unwrap();
    capture.feed(&changed, |_| Ok(())).unwrap();
    assert!(capture.finish(|_| Ok(())).is_err());
    let mut capture = ArtifactCapture::new(&artifact).unwrap();
    capture.feed(&bytes[..bytes.len()-1], |_| Ok(())).unwrap();
    assert!(capture.finish(|_| Ok(())).is_err());
}

#[test]
fn corpus_uses_and_distribution_verification_cannot_cross_undeclared_inputs() {
    use lctx_model::domain::InvariantCheck;
    let model = model().unwrap();
    let library = source().input;
    let corpus = InputRevision::from_entries(vec![]).unwrap();
    let origin = InputOrigin::Tree { label: "library".into() };
    let corpus_origin = InputOrigin::Corpus { repository: "repo".into(), revision: "rev".into() };
    let acquired = InputAcquisition { input: library, origin: origin.id() };
    let acquired_corpus = InputAcquisition { input: corpus.id(), origin: corpus_origin.id() };
    let link = CorpusLibrary { corpus: corpus.id(), library };
    let usage = ArtifactUse { artifact: source().id(), input: corpus.id(), role: SourceRole::Example };
    fn feed<R: Record>(model: &ValidatedModel, check: &mut dyn InvariantCheck, rows: Vec<R>) -> Result<(), ModelError> {
        check.visit(R::NAME, Batch::new(model, rows)?.arrow())
    }
    for linked in [false, true] {
        let mut check = (InputAcquisition::invariants()[0].create)();
        feed(&model, &mut *check, vec![origin.clone(), corpus_origin.clone()]).unwrap();
        feed(&model, &mut *check, vec![acquired.clone(), acquired_corpus.clone()]).unwrap();
        if linked { feed(&model, &mut *check, vec![link.clone()]).unwrap(); }
        feed(&model, &mut *check, vec![source()]).unwrap();
        assert_eq!(feed(&model, &mut *check, vec![usage.clone()]).is_ok(), linked);
    }
    let release = Release { package: Package { name: "demo".into() }.id(), version: "1".into() };
    let verified = DistributionVerification { acquisition: acquired.id(), release: release.id(), record_digest: ContentHash::of(b"record"), artifact_sha256: vec![] };
    for member in [false, true] {
        let mut check = (InputAcquisition::invariants()[0].create)();
        feed(&model, &mut *check, vec![origin.clone()]).unwrap();
        feed(&model, &mut *check, vec![acquired.clone()]).unwrap();
        if member { feed(&model, &mut *check, vec![InputDistribution { input: library, release: release.id(), role: DistributionRole::FirstParty }]).unwrap(); }
        assert_eq!(feed(&model, &mut *check, vec![verified.clone()]).is_ok(), member);
    }
}

#[test]
fn borrowed_identity_and_streamed_codecs_preserve_owned_key_contracts() {
    use lctx_model::DomainSum;
    #[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
    #[model(name = "empty_payload_choices")]
    enum Choice { #[model(code = 0)] No, #[model(code = 1)] Yes }
    #[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
    #[model(name = "optional_payload_choices")]
    enum Payload {
        #[model(code = 0)] Empty,
        #[model(code = 1)] Text { value: String, note: Option<String> },
    }
    let m = ValidatedModel::validate(vec![Relation::of::<Choice>(),Relation::of::<Payload>(),Relation::of::<NamedItem>()]).unwrap();
    let choices = vec![Choice::No,Choice::Yes];
    for row in &choices { assert_eq!(row.id(),Id::of(&row.key())); }
    assert_eq!(Choice::decode(&Choice::encode(&choices).unwrap()).unwrap(),choices);
    assert_eq!(Choice::encode(&[]).unwrap().schema(),Choice::schema());
    let rows = vec![Payload::Empty,Payload::Text { value: "x".repeat(1 << 20),note: None },
        Payload::Text { value: "body".into(),note: Some("note".into()) }];
    for row in &rows { assert_eq!(row.id(),Id::of(&row.key())); }
    let encoded = Batch::new(&m,rows).unwrap();
    assert_eq!(Payload::decode(encoded.arrow()).unwrap(),encoded.rows());
    let item = NamedItem { name: "key".repeat(1024),payload: "payload".into() };
    assert_eq!(item.id(),Id::of(&item.key()));
}
