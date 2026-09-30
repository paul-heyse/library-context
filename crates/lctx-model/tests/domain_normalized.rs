//! Pure normalization controls require no acquisition, frontend, store or async runtime.
use lctx_model::domain::{*, assertion::*, attribution::*, calls::*, conditions::Diagram, input::*, normalized::{entities::*, entity_normalization::*}, resources::ResourceBudget, source::*, symbols::*};
fn fixture() -> (EntityData, ResourceBudget, ProviderSymbol) {
    let budget = ResourceBudget::fixed(4 << 20).unwrap();
    let mut data = EntityData::new(&budget);
    let bytes = include_bytes!("../../../fixtures/python/normalized_entities/dual.py");
    let input = InputRevision::from_entries(vec![ManifestEntry { path: "dual.py".into(), content: ContentHash::of(bytes), byte_len: bytes.len() as i64 }]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "dual.py".into(), bytes).unwrap();
    let module = Module { source: source.id(), qualified_name: "dual".into() };
    data.modules.insert(module.clone()).unwrap();
    let root = Occurrence { source: source.id(), start: 0, end: bytes.len() as i64, syntax_kind: SyntaxKind::ModModule, role: OccurrenceRole::Syntax, structural_path: vec![0] };
    let function = Occurrence { syntax_kind: SyntaxKind::StmtFunctionDef, role: OccurrenceRole::Declaration, structural_path: vec![0, 0], ..root.clone() };
    data.occurrences.insert(root).unwrap(); data.occurrences.insert(function).unwrap();
    let context = AnalysisContext { python_version: "3.14.7".into(), python_platform: "linux".into(), search_path: vec![], site_package_path: vec![], config_digest: ContentHash::of(b"pure"), environment_digest: ContentHash::of(b"pure"), lock_digest: None };
    let provider = Provider { tool: "pure-provider".into(), revision: "1".into(), build_digest: ContentHash::of(b"provider") };
    let module = ProviderModule::Acquired { module: module.id() }; data.provider_modules.insert(module.clone()).unwrap();
    let symbol = ProviderSymbol { provider: provider.id(), context: context.id(), module: module.id(), native_key: "no-source-link".into(), name: "same".into(), kind: SymbolKind::Function };
    data.symbols.insert(symbol.clone()).unwrap();
    let (condition, _) = Diagram::always().records();
    let q = AssertionQualification { context: context.id(), scope: CoverageScope::Artifact { artifact: source.id() }.id(), condition: condition.id(), modality: Modality::Definite, approximation: Approximation::Exact };
    data.qualifications.insert(q).unwrap();
    (data, budget, symbol)
}
#[test]
fn syntax_identity_and_missing_correspondence_are_separate_pure_answers() {
    let (data, budget, symbol) = fixture();
    let output = normalize(data.inputs(), &budget).unwrap();
    assert_eq!(output.callables.len(), 1, "one source def in the fixture");
    assert_eq!(output.owners.len(), 2);
    let resolution = output.resolutions.iter().find(|r| r.symbol == symbol.id()).unwrap();
    assert_eq!(resolution.status, ResolutionStatus::Unresolved);
    assert_eq!(resolution.reason, EntityReason::MissingDeclaration);
    assert!(resolution.entity.is_none(), "same spelling supplies no declaration proof");
    let source: Vec<_> = data.occurrences.iter().cloned().collect();
    for row in output.owners.iter() { assert_eq!(row.owner, occurrence_owner::owner_of(data.occurrences.get(row.occurrence).unwrap(), &source).unwrap()); }
}
#[test]
fn explicit_synthesis_and_namespace_coordinates_do_not_guess_source_attachment() {
    let (mut data, budget, symbol) = fixture();
    let qualification = data.qualifications.iter().next().unwrap().id();
    data.function_traits.insert(FunctionTraitObservation { qualification, symbol: symbol.id(), overload: false, staticmethod: false, classmethod: false, property_getter: false, property_setter: false, stub: false, origin: FunctionOrigin::Synthesized, defining_class: None, overrides: None }).unwrap();
    let namespace = ProviderModule::Namespace { provider: symbol.provider, context: symbol.context, name: "dual".into() };
    data.provider_modules.insert(namespace.clone()).unwrap();
    let namespace_symbol = ProviderSymbol { module: namespace.id(), kind: SymbolKind::Module, ..symbol.clone() };
    data.symbols.insert(namespace_symbol.clone()).unwrap();
    let output = normalize(data.inputs(), &budget).unwrap();
    let resolution = output.resolutions.iter().find(|r| r.symbol == symbol.id()).unwrap();
    assert_eq!(resolution.reason, EntityReason::ProviderSynthetic);
    let EntityRef::Callable { callable } = output.refs.get(resolution.entity.unwrap()).unwrap() else { panic!("callable") };
    assert_eq!(output.callables.get(*callable), Some(&CallableEntity::Synthetic { symbol: symbol.id() }));
    assert_eq!(output.resolutions.iter().find(|r| r.symbol == namespace_symbol.id()).unwrap().status, ResolutionStatus::Unresolved);
}
#[test]
fn normalized_codec_preserves_unicode_nul_and_rejects_false_resolved_shape() {
    let (data, budget, symbol) = fixture();
    let occurrence = data.occurrences.iter().next().unwrap().id();
    let field = FieldEntity { class: ClassEntity::Source { declaration: occurrence }.id(), name: "é\0field".into() };
    assert_eq!(FieldEntity::decode(&FieldEntity::encode(std::slice::from_ref(&field)).unwrap()).unwrap(), [field]);
    let mut resolution = SymbolEntityResolution { symbol: symbol.id(), context: symbol.context, policy: normalized::policy_revision(), status: ResolutionStatus::Resolved, entity: None, reason: EntityReason::DeclarationAgreement };
    assert!(resolution.validate().is_err());
    resolution.status = ResolutionStatus::Unresolved; resolution.validate().unwrap();
    drop(data); assert_eq!(budget.reserved(), 0);
}
