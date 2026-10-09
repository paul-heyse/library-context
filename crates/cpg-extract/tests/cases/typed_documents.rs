//! Native document answers through the real stage graph and shared model validators.
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{assertion::*, attribution::*, documents::*, source::*, *};
use typed_driver::{files, rows, run};
inspector!(
    Documents,
    DocumentNode,
    DocumentObservation,
    PassageObservation,
    CodeBlockObservation,
    DocumentLinkObservation,
    DocumentMentionObservation,
    DocumentComponentObservation,
    DocumentAttributeValue,
    DocumentAttributeObservation,
    ProviderCoverage,
    SourceArtifact
);
#[tokio::test]
async fn native_mdx_keeps_nested_components_all_attribute_arms_and_captured_code() {
    let tables = typed_driver::Tables::default();
    run(&files("semantic_documents"), Documents(tables.clone()))
        .await
        .unwrap();
    assert!(
        rows::<DocumentObservation>(&tables)
            .iter()
            .all(|d| d.parsed)
    );
    let passages = rows::<PassageObservation>(&tables);
    assert_eq!(passages.len(), 1);
    assert_eq!(passages[0].heading.as_deref(), Some("Guide"));
    assert!(passages[0].text.contains("## Nested"));
    let components = rows::<DocumentComponentObservation>(&tables);
    assert_eq!(components.len(), 2);
    assert!(
        components
            .iter()
            .any(|c| c.parent.is_some() && c.depth == 1)
    );
    let attributes = rows::<DocumentAttributeValue>(&tables);
    assert!(
        attributes
            .iter()
            .any(|a| matches!(a,DocumentAttributeValue::Bare{name} if name=="enabled"))
    );
    assert!(attributes.iter().any(|a|matches!(a,DocumentAttributeValue::Literal{name,value} if name=="label" && value=="hello")));
    assert!(attributes.iter().any(|a|matches!(a,DocumentAttributeValue::Expression{name,source} if name=="count" && source=="size")));
    assert!(
        attributes
            .iter()
            .any(|a| matches!(a,DocumentAttributeValue::Spread{source} if source=="...props"))
    );
    let blocks = rows::<CodeBlockObservation>(&tables);
    assert_eq!(blocks.len(), 1);
    let block = &blocks[0];
    assert_eq!(block.code, "print(\"hello\")");
    let artifacts = rows::<SourceArtifact>(&tables);
    let captured = artifacts
        .iter()
        .find(|a| Some(a.id()) == block.materialized)
        .unwrap();
    assert_eq!(captured.content, block.content);
    assert_eq!(Some(&captured.path), block.module_path.as_ref());
    assert_eq!(
        rows::<DocumentLinkObservation>(&tables)[0].url,
        "https://example.test/api"
    );
}
#[tokio::test]
async fn native_documents_disclose_malformed_inputs_and_recognize_only_declared_vocabulary() {
    let mut input = files("docs_shapes");
    let root = input.clone();
    input.clear();
    for (path, bytes) in root {
        let path = path
            .strip_prefix("release/")
            .or_else(|| path.strip_prefix("corpus/"))
            .unwrap_or(&path)
            .to_owned();
        input.insert(path, bytes);
    }
    let tables = typed_driver::Tables::default();
    run(&input, Documents(tables.clone())).await.unwrap();
    let sources = rows::<SourceArtifact>(&tables);
    let broken = sources
        .iter()
        .find(|a| a.path == "docs/broken.mdx")
        .unwrap();
    assert!(
        !rows::<DocumentObservation>(&tables)
            .iter()
            .find(|d| d.source == broken.id())
            .unwrap()
            .parsed
    );
    assert!(
        rows::<ProviderCoverage>(&tables)
            .iter()
            .any(|c| c.family == FactFamily::Docs
                && c.status == CoverageStatus::Partial
                && c.reason == Some(ObligationKind::OutsideProviderModel))
    );
    let mentions = rows::<DocumentMentionObservation>(&tables);
    assert!(
        mentions
            .iter()
            .any(|m| m.access_path.as_deref() == Some("pkg.Server")
                && m.class == MentionClass::Exact),
        "mentions: {mentions:#?}"
    );
    assert!(
        !mentions
            .iter()
            .any(|m| m.qualified_name.as_deref() == Some("other.Server.tool"))
    );
    assert!(mentions.iter().any(|m| m.class == MentionClass::Lexical));
    let qualifications = rows::<AssertionQualification>(&tables);
    for mention in mentions.iter().filter(|m| m.class == MentionClass::Lexical) {
        assert_eq!(
            qualifications
                .iter()
                .find(|q| q.id() == mention.qualification)
                .unwrap()
                .modality,
            Modality::Candidate
        );
    }
}
#[tokio::test]
async fn corrupt_materialized_block_is_refused_before_publication() {
    let input = files("semantic_documents");
    let captured = typed_driver::capture(
        &input,
        "typed-driver",
        lctx_model::domain::stages::Profile::Catalog,
    );
    let derived = captured.inputs()[0]
        .captured()
        .artifacts()
        .iter()
        .find(|a| a.path.starts_with("_lctx/"))
        .unwrap();
    let path = captured.inputs()[0].captured().root().join(&derived.path);
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(path, b"print('different')").unwrap();
    let tables = typed_driver::Tables::default();
    let error =
        typed_driver::run_with(
            captured,
            cpg_extract::pyrefly_stage::Pyrefly::new(
                cpg_extract::typed_syntax::SyntaxLimits::default(),
            ),
            Documents(tables),
        )
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("capture")
            || error.to_string().contains("changed")
            || error.to_string().contains("artifact")
    );
}
