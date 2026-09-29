use arrow_array::{FixedSizeBinaryArray, RecordBatch, StringArray};
use cpg_schema::{
    bundle,
    serving_projection::{ArtifactReceipt, Manifest, receipt, validate_batch},
};
use std::sync::Arc;
fn lexical(values: Vec<Option<&str>>) -> RecordBatch {
    let schema = bundle::files(1024)
        .into_iter()
        .find(|f| f.name == "lexical_text")
        .unwrap()
        .schema;
    let ids = FixedSizeBinaryArray::try_from_iter((0..values.len()).map(|_| [1u8; 16])).unwrap();
    RecordBatch::try_new(
        schema,
        vec![Arc::new(ids), Arc::new(StringArray::from(values))],
    )
    .unwrap()
}
#[test]
fn content_preserves_multiplicity_and_ignores_batch_boundaries() {
    let b = lexical(vec![Some(""), Some("abc"), Some("abc")]);
    let whole = receipt("lexical_text", 1024, std::slice::from_ref(&b)).unwrap();
    assert_eq!(
        whole,
        receipt("lexical_text", 1024, &[b.slice(0, 1), b.slice(1, 2)]).unwrap()
    );
    assert_eq!(
        whole,
        receipt("lexical_text", 1024, &[b.slice(1, 2), b.slice(0, 1)]).unwrap()
    );
    assert_ne!(
        whole.content_digest,
        receipt("lexical_text", 1024, &[b.slice(0, 2)])
            .unwrap()
            .content_digest
    );
    assert_eq!(receipt("lexical_text", 1024, &[]).unwrap().rows, 0);
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("driver".into(), "postgres".into());
    let drift = b
        .clone()
        .with_schema(Arc::new(
            b.schema().as_ref().clone().with_metadata(metadata),
        ))
        .unwrap();
    assert!(validate_batch("lexical_text", 1024, &drift).is_err());
}
#[test]
fn full_manifest_identity_rejects_missing_relations_and_mixed_spec() {
    let relations = bundle::files(1024)
        .into_iter()
        .map(|f| (f.name.to_owned(), receipt(f.name, 1024, &[]).unwrap()))
        .collect();
    let mut m = Manifest {
        capabilities: cpg_schema::catalog::Capabilities::for_profile(
            cpg_schema::catalog::CompileProfile::Behavioral,
        ),
        format: cpg_schema::serving_projection::FORMAT,
        bundle_format: cpg_schema::serving_projection::BUNDLE_FORMAT,
        context: cpg_schema::serving_projection::ServingContext {
            library: "fixture".into(),
            requirement: "fixture==1".into(),
            summary: Default::default(),
        },
        snapshot_id: "01".repeat(16),
        snapshot_digest: "02".repeat(32),
        compiler_digest: "03".repeat(32),
        projection_digest: cpg_schema::serving_projection::definition_digest(),
        catalog_digest: Some(cpg_schema::models::Catalog::committed_digest().hex()),
        kernel_format: Some(1),
        entry_value_effect_digest: Some("07".repeat(32)),
        spec_hash: Some("05".repeat(32)),
        dimensions: 1024,
        relations,
        artifacts: cpg_schema::serving_projection::artifact_names()
            .into_iter()
            .map(|n| {
                (
                    n,
                    ArtifactReceipt {
                        sha256: "06".repeat(32),
                        bytes: 2,
                        format: 1,
                    },
                )
            })
            .collect(),
    };
    let first = m.generation().unwrap();
    let mut catalog = m.clone();
    catalog.capabilities = cpg_schema::catalog::Capabilities::for_profile(
        cpg_schema::catalog::CompileProfile::Catalog,
    );
    catalog.catalog_digest = None;
    catalog.kernel_format = None;
    catalog.entry_value_effect_digest = None;
    let names = cpg_schema::serving_projection::artifacts_for(&catalog.capabilities);
    catalog.artifacts.retain(|name, _| names.contains(name));
    assert!(
        catalog.generation().is_ok(),
        "catalog does not require native closure"
    );
    catalog.capabilities.native_value_paths = true;
    assert!(
        catalog.generation().is_err(),
        "advertised native closure must be complete"
    );
    catalog.capabilities.native_value_paths = false;
    catalog.relations.get_mut("briefs").unwrap().rows = 1;
    assert!(
        catalog.generation().is_err(),
        "unadvertised enrichment cannot leak rows"
    );
    m.artifacts.get_mut("operations.arrow").unwrap().bytes = 3;
    assert_ne!(first, m.generation().unwrap());
    m.spec_hash = None;
    assert!(m.generation().is_err());
    m.spec_hash = Some("05".repeat(32));
    m.relations.remove("supports");
    assert!(m.generation().is_err());
}

#[test]
fn independently_framed_single_row_known_answer() {
    let b = lexical(vec![Some("abc")]);
    assert_eq!(
        receipt("lexical_text", 1024, &[b]).unwrap().content_digest,
        "5abf3db0c7f250a6f0909c9c994298b4be1fb4e8ba0858b2cdb436b5be44f87b"
    );
}

#[test]
fn ipc_shared_buffers_and_slices_have_the_same_logical_budget() {
    let b = lexical(vec![Some("abc")]);
    let mut bytes = Vec::new();
    let mut writer = arrow_ipc::writer::FileWriter::try_new(&mut bytes, &b.schema()).unwrap();
    writer.write(&b).unwrap();
    writer.finish().unwrap();
    drop(writer);
    let mut reader =
        arrow_ipc::reader::FileReader::try_new(std::io::Cursor::new(bytes), None).unwrap();
    let decoded = reader.next().unwrap().unwrap();
    assert_eq!(cpg_schema::serving_projection::batch_bytes(&b).unwrap(), 27);
    assert_eq!(
        cpg_schema::serving_projection::batch_bytes(&decoded).unwrap(),
        27
    );
    assert_eq!(
        receipt("lexical_text", 1024, &[b]).unwrap(),
        receipt("lexical_text", 1024, &[decoded]).unwrap()
    );
}

#[test]
fn definition_encoding_is_independent_of_serde_json_map_features() {
    assert_eq!(
        cpg_schema::serving_projection::definition_digest(),
        "1130c1cc95996d2efa2a08eb9ac12c1639b12ad42f8fa26809910db911ac7260"
    );
}
