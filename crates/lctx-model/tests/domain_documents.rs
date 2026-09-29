#[path = "fixtures/documents.rs"] mod fixture;
use fixture::Fixture;
use lctx_model::domain::{*, assertion::*, documents::*};

#[test]
fn document_structure_and_attribution_preserve_bytes_and_typed_attribute_variants() {
    let fixture = Fixture::new();
    for name in ["document_structure",DocumentSupport::NAME,PassageSupport::NAME,CodeBlockSupport::NAME,DocumentLinkSupport::NAME,
        DocumentMentionSupport::NAME,DocumentComponentSupport::NAME,DocumentAttributeSupport::NAME] {
        fixture.check(fixture.model.invariants().iter().find(|i| i.name == name).unwrap()).unwrap();
    }
    // Parent spans across headings; the child's owning passage can differ from its parent's.
    assert_eq!(fixture.rows::<DocumentComponentObservation>().len(),2);
    let values = fixture.rows::<DocumentAttributeValue>();
    assert_eq!(values.len(),4);
    assert!(values.iter().any(|v| matches!(v,DocumentAttributeValue::Spread { source } if source == "props")));
    assert!(values.iter().any(|v| matches!(v,DocumentAttributeValue::Expression { name,source } if name == "count" && source == "size")));
    assert_eq!(DocumentAttributeValue::decode(Batch::new(&fixture.model,values.clone()).unwrap().arrow()).unwrap(),values);
}

#[test]
fn document_subjects_cannot_smuggle_foreign_optional_spans() {
    let mut fixture = Fixture::new(); fixture.foreign_inner();
    assert!(fixture.check(&DocumentNode::invariants()[0]).is_err());
    assert!(fixture.check(&DocumentComponentSupport::invariants()[0]).is_err());
}

#[test]
fn document_shapes_and_parent_depth_are_validated() {
    let mut fixture = Fixture::new();
    let mut components = fixture.rows::<DocumentComponentObservation>();
    components.iter_mut().find(|r| r.parent.is_some()).unwrap().depth = 2;
    fixture.put(components);
    assert!(fixture.check(&DocumentNode::invariants()[0]).is_err());
    let mut bad = fixture.component.clone(); bad.parent = Some(bad.component);
    assert!(bad.validate().is_err());
    let mut block = fixture.rows::<CodeBlockObservation>().pop().unwrap(); block.code.push('x');
    assert!(block.validate().is_err());
    let mut passage = fixture.rows::<PassageObservation>().pop().unwrap(); passage.level = 7;
    assert!(passage.validate().is_err());
    let invocation = Evidence::Invocation { run: fixture.rows::<lctx_model::domain::attribution::ProviderRun>()[0].id() };
    assert!(EvidenceSourceSpanId::of(&invocation).is_err());
}
