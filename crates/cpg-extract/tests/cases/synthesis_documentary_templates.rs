//! Exact native MDX premises for S0's retained authored templates.
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{assertion::*, documents::*, *};
use typed_driver::{files, rows, run};
inspector!(
    Templates,
    DocumentNode,
    DocumentComponentObservation,
    DocumentAttributeObservation,
    DocumentAttributeValue,
    Evidence,
    PassageObservation
);
#[tokio::test]
async fn native_components_keep_original_inner_lead_parent_and_ignore_inline_code_and_fences() {
    let files = files("synthesis_sources");
    let text = std::str::from_utf8(files.get("guide.mdx").unwrap()).unwrap();
    let tables = typed_driver::Tables::default();
    run(&files, Templates(tables.clone())).await.unwrap();
    let components = rows::<DocumentComponentObservation>(&tables);
    let nodes = rows::<DocumentNode>(&tables);
    let evidence = rows::<Evidence>(&tables);
    let original = |span: EvidenceSourceSpanId| {
        let Evidence::SourceSpan { start, end, .. } =
            evidence.iter().find(|e| e.id() == span.id()).unwrap()
        else {
            panic!("span arm")
        };
        &text[*start as usize..*end as usize]
    };
    let warnings = components
        .iter()
        .filter(|c| c.name.as_deref() == Some("Warning"))
        .collect::<Vec<_>>();
    assert_eq!(
        warnings.len(),
        3,
        "two Flow warnings plus one actual inline Text warning; code/fenced warnings are absent"
    );
    let nested = warnings.iter().find(|c| c.parent.is_some()).unwrap();
    assert_eq!(nested.form, ComponentForm::Flow);
    assert_eq!(
        original(nested.inner.unwrap()),
        "Keep the credentials private when configuring the timeout."
    );
    let parent = components
        .iter()
        .find(|c| Some(c.component) == nested.parent)
        .unwrap();
    assert_eq!(parent.name.as_deref(), Some("ParamField"));
    assert_eq!(
        original(parent.lead.unwrap()),
        "Timeout controls the documented waiting interval."
    );
    let node = nodes
        .iter()
        .find(|n| n.id() == parent.component.id())
        .unwrap();
    assert!(original(node.span()).contains("<Warning title=\"Credentials\">"));
    assert!(warnings.iter().any(|c| c.form == ComponentForm::Text));
    for warning in warnings {
        let node = nodes
            .iter()
            .find(|n| n.id() == warning.component.id())
            .unwrap();
        assert!(!original(node.span()).contains("Fenced caution"));
    }
}
