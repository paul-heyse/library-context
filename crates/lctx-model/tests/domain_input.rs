//! The acquisition model (cutover plan A1): every captured artifact has at most one ownership class
//! (owned, unowned or derived); only derived artifacts live under `_lctx/`, each derived from an
//! original document of its own input and within its bytes; an environment fingerprint names a
//! release its input distributes. Facts admission requires a class for every artifact
//! (`domain_admission`). Every control states its answer first.
#[path = "fixtures/input.rs"]
mod fixture;
use fixture::{BLOCK, BLOCK_PATH, Fixture};
use lctx_model::domain::{
    artifact::ArtifactChunk, input::*, resources::ResourceBudget, source::SourceArtifact, *,
};

fn budget() -> ResourceBudget {
    ResourceBudget::fixed(1 << 30).unwrap()
}
fn input_model() -> Result<ValidatedModel, ModelError> {
    ValidatedModel::declared(vec![
        Relation::of::<Package>(),
        Relation::of::<Release>(),
        Relation::of::<InputRevision>(),
        Relation::of::<InputOrigin>(),
        Relation::of::<InputAcquisition>(),
        Relation::of::<CorpusLibrary>(),
        Relation::of::<ArtifactUse>(),
        Relation::of::<InputDistribution>(),
        Relation::of::<DistributionVerification>(),
        Relation::of::<EnvironmentFingerprint>(),
        Relation::of::<SourceArtifact>(),
        Relation::of::<ArtifactChunk>(),
        Relation::of::<ArtifactOwnership>(),
        Relation::of::<UnownedArtifact>(),
        Relation::of::<DerivedArtifact>(),
    ])
}
fn validate(fixture: &Fixture) -> Result<ContentHash, ModelError> {
    let model = input_model()?;
    let mut generation = Vec::new();
    macro_rules! put { ($($ty:ty),+) => { $( generation.push((<$ty>::NAME,Batch::new(&model,fixture.rows::<$ty>(),&budget())?.arrow().clone())); )+ }; }
    input_relations!(put);
    validation::replay::replay(&model, &generation, &budget())
}
fn refused(fixture: &Fixture, why: &str, expected: &str) {
    let result = validate(fixture);
    assert!(
        matches!(&result, Err(ModelError::Invalid(message)) if message.contains(expected)),
        "{why}: expected `{expected}`, got {result:?}"
    );
}
const CLASSES: &str = "more than one of the owned, unowned and derived classes";
const RESERVED: &str =
    "only derived artifacts, and all of them, live under the reserved _lctx/ namespace";
const DOCUMENT: &str =
    "a derivation needs an original document of the same input, within its bytes";

#[test]
fn a_classified_installed_input_validates() {
    validate(&Fixture::new()).unwrap();
}

#[test]
fn an_artifact_has_at_most_one_class() {
    let mut owned_and_unowned = Fixture::new();
    let owned = owned_and_unowned.artifact("demo/__init__.py").id();
    owned_and_unowned.unowned.push(UnownedArtifact {
        artifact: owned,
        acquisition: owned_and_unowned.acquisition.id(),
    });
    refused(
        &owned_and_unowned,
        "an artifact both owned and unowned",
        CLASSES,
    );
    let mut unowned_and_derived = Fixture::new();
    let block = unowned_and_derived.artifact(BLOCK_PATH).id();
    unowned_and_derived.unowned.push(UnownedArtifact {
        artifact: block,
        acquisition: unowned_and_derived.acquisition.id(),
    });
    refused(
        &unowned_and_derived,
        "a derived artifact also unowned",
        CLASSES,
    );
    let mut foreign = Fixture::new();
    let loose = foreign.artifact("extra.pyi").id();
    foreign.unowned = vec![UnownedArtifact {
        artifact: loose,
        acquisition: foreign.other_acquisition.id(),
    }];
    refused(
        &foreign,
        "an unowned artifact claimed by another input's acquisition",
        "belongs to another acquisition's input",
    );
}

#[test]
fn only_derived_artifacts_live_under_the_reserved_namespace() {
    let mut files = Fixture::new().files;
    files.insert("_lctx/stray.py".into(), b"x = 1\n".to_vec());
    let mut stray = Fixture::with_files(files.clone());
    refused(&stray, "an unclassified artifact under _lctx/", RESERVED);
    let artifact = stray.artifact("_lctx/stray.py").id();
    stray.unowned.push(UnownedArtifact {
        artifact,
        acquisition: stray.acquisition.id(),
    });
    refused(&stray, "an original artifact under _lctx/", RESERVED);
    let mut outside = Fixture::new();
    let id = outside.artifact("extra.pyi").id();
    if let DerivedArtifact::PythonCodeBlock { artifact, .. } = &mut outside.derived[0] {
        *artifact = id;
    }
    outside
        .unowned
        .retain(|u| u.artifact != outside.derived[0].artifact());
    refused(&outside, "a derived artifact outside _lctx/", RESERVED);
}

#[test]
fn a_derivation_names_an_original_document_of_its_own_input() {
    let mut orphan = Fixture::new();
    let id = SourceArtifact::from_bytes(orphan.input.id(), "missing.md".into(), b"gone")
        .unwrap()
        .id();
    if let DerivedArtifact::PythonCodeBlock { document, .. } = &mut orphan.derived[0] {
        *document = id;
    }
    refused(
        &orphan,
        "a derived artifact without its document",
        "references an absent",
    );
    let mut foreign = Fixture::new();
    if let DerivedArtifact::PythonCodeBlock { document, .. } = &mut foreign.derived[0] {
        *document = foreign.other.id();
    }
    refused(&foreign, "a document of another input", DOCUMENT);
    let mut beyond = Fixture::new();
    if let DerivedArtifact::PythonCodeBlock { fence_end, .. } = &mut beyond.derived[0] {
        *fence_end = fixture::README.len() as i64 + 1;
    }
    refused(&beyond, "a fence beyond the document's bytes", DOCUMENT);
    let mut files = Fixture::new().files;
    files.insert("_lctx/d_block/block_0.py".into(), BLOCK.to_vec());
    let mut chained = Fixture::with_files(files);
    let (first, second) = (
        chained.artifact(BLOCK_PATH).id(),
        chained.artifact("_lctx/d_block/block_0.py").id(),
    );
    chained.derived.push(DerivedArtifact::PythonCodeBlock {
        artifact: second,
        document: first,
        ordinal: 0,
        fence_start: 0,
        fence_end: 1,
    });
    refused(&chained, "a document that is itself derived", DOCUMENT);
    let mut twin = Fixture::new();
    if let DerivedArtifact::PythonCodeBlock { fence_end, .. } = &mut twin.derived[0] {
        *fence_end = fixture::README.len() as i64;
    }
    validate(&twin).unwrap();
    let mut bad = Fixture::new().derived[0].clone();
    if let DerivedArtifact::PythonCodeBlock {
        fence_start,
        fence_end,
        ..
    } = &mut bad
    {
        *fence_start = 3;
        *fence_end = 2;
    }
    assert!(bad.validate().is_err(), "an unordered fence");
}

#[test]
fn an_environment_fingerprint_names_a_distributed_release() {
    let mut undistributed = Fixture::new();
    undistributed.fingerprint.release = undistributed.undistributed.id();
    refused(
        &undistributed,
        "a fingerprint for a release its input does not distribute",
        "names a release its input does not distribute",
    );
    assert!(
        EnvironmentFingerprint {
            python_version: " ".into(),
            ..Fixture::new().fingerprint
        }
        .validate()
        .is_err()
    );
}

#[test]
fn source_role_codes_are_append_only() {
    let codes: Vec<i16> = [
        SourceRole::Release,
        SourceRole::Example,
        SourceRole::Test,
        SourceRole::DocBlock,
        SourceRole::Dependency,
        SourceRole::Document,
        SourceRole::DistributionMetadata,
        SourceRole::Configuration,
        SourceRole::TaskReceipt,
    ]
    .iter()
    .map(|r| r.code())
    .collect();
    assert_eq!(codes, (0..9).collect::<Vec<i16>>());
    let row = Fixture::new().derived[0].clone();
    assert_eq!(row.tag(), 0);
    assert_eq!(
        DerivedArtifact::TaskReceipt {
            artifact: row.artifact(),
            input: Fixture::new().input.id()
        }
        .tag(),
        1
    );
}
