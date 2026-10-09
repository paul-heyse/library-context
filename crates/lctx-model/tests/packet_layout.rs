//! Boxing retained packet payloads changes allocation layout, never their wire shape.
use lctx_model::domain::{
    ContentHash,
    assertion::Approximation,
    attribution::{ExtractionMode, Fidelity, Modality, Origin},
    serving::*,
};
use serde_json::{Value, json};

fn id() -> Vec<u8> {
    vec![7; 16]
}
fn hash() -> ContentHash {
    ContentHash::of(b"packet-layout-control")
}
fn basis() -> Value {
    json!({"set":id(),"members_digest":hash(),"definitions":[]})
}
fn native_support() -> Value {
    json!({"support":{"kind":"assertion","assertion":vec![7;32]},"run":id(),"input":id(),"context":id(),"environment":hash(),"provider":"pyrefly","provider_revision":"pinned","provider_build":hash(),"surface":"types","evidence":{"kind":"invocation","run":id()},"fidelity":Fidelity::NativeStructural})
}

#[test]
fn boxed_override_universe_keeps_exact_wire_payload_and_schema() {
    let universe = json!({"universe":id(),"context":id(),"input":id(),"environment":hash(),"model_definition":hash(),"support":id(),"catalog":id(),"model":id(),"source_name":"pinned-model","format":1,"source":"model source"});
    let expected = json!({"kind":"no_extra_overrides","assumption":id(),"class":id(),"symbol":id(),"synthesized":false,"dataclass":true,"named_tuple":false,"typed_dict":false,"support":native_support(),"universe":universe});
    let packet: ClaimAssumptionPacket = serde_json::from_value(expected.clone()).unwrap();
    assert_eq!(serde_json::to_value(&packet).unwrap(), expected);
    assert_eq!(
        schemars::schema_for!(Box<AssumptionUniversePacket>),
        schemars::schema_for!(AssumptionUniversePacket)
    );
    let mut unknown = expected.clone();
    unknown["universe"]["unowned"] = json!(true);
    assert!(serde_json::from_value::<ClaimAssumptionPacket>(unknown).is_err());
    let mut missing = expected;
    missing["universe"]
        .as_object_mut()
        .unwrap()
        .remove("support");
    assert!(serde_json::from_value::<ClaimAssumptionPacket>(missing).is_err());
    assert!(
        std::mem::size_of::<ClaimAssumptionPacket>()
            < std::mem::size_of::<AssumptionNativeSupportPacket>()
                + std::mem::size_of::<AssumptionUniversePacket>()
    );
}

#[test]
fn boxed_capture_correspondence_keeps_exact_wire_payload_and_schema() {
    let support = json!({"support":{"kind":"assertion","assertion":vec![7;32]},"run":id(),"input":id(),"context":id(),"environment":hash(),"provider_id":id(),"provider":"pyrefly","provider_revision":"pinned","provider_build":hash(),"surface":"signatures","evidence":{"kind":"invocation","run":id()},"origin":Origin::AnalyzerAssertion,"mode":ExtractionMode::NativeTraversal,"fidelity":Fidelity::ReportProjection});
    let correspondence = json!({"assertion":id(),"qualification":id(),"scope":id(),"condition":id(),"modality":Modality::Definite,"approximation":Approximation::Exact,"claim_basis":basis(),"source_correspondence_only":true,"support":support});
    let expected = json!({"kind":"entry","formal":id(),"parameter":id(),"declaration":id(),"signature":id(),"ordinal":0,"name":"value","source_correspondence":correspondence});
    let packet: CapturedValueSourcePacket = serde_json::from_value(expected.clone()).unwrap();
    assert_eq!(serde_json::to_value(&packet).unwrap(), expected);
    assert_eq!(
        schemars::schema_for!(Box<CaptureSourceDeclarationPacket>),
        schemars::schema_for!(CaptureSourceDeclarationPacket)
    );
    let mut unknown = expected.clone();
    unknown["source_correspondence"]["runtime_authority"] = json!(true);
    assert!(serde_json::from_value::<CapturedValueSourcePacket>(unknown).is_err());
    let mut missing = expected;
    missing["source_correspondence"]
        .as_object_mut()
        .unwrap()
        .remove("qualification");
    assert!(serde_json::from_value::<CapturedValueSourcePacket>(missing).is_err());
    assert!(
        std::mem::size_of::<CapturedValueSourcePacket>()
            < std::mem::size_of::<CaptureSourceDeclarationPacket>()
    );
}
