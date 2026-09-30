//! Native call/signature contract fixtures through real generated PostgreSQL lowerings.
use std::sync::Arc;
use lctx_model::domain::{*, artifact::*, assertion::*, attribution::*, calls::*, conditions::*, input::*, source::*};
use lctx_postgres::generations::{GenerationStore,Error};
use lctx_postgres::testing::Harness;
use lctx_postgres::testing::DisposableDatabase;

#[tokio::test]
async fn call_signature_membership_support_ownership_and_readback() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(model().unwrap()); let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    let bytes = b"f(1)";
    let input = InputRevision::from_entries(vec![ManifestEntry { path: "example.py".into(),content: ContentHash::of(bytes),byte_len: bytes.len() as i64 }]).unwrap();
    let origin = InputOrigin::Tree { label: "contract-fixture".into() };
    let acquisition = InputAcquisition { input: input.id(),origin: origin.id() };
    let source = SourceArtifact::from_bytes(input.id(),"example.py".into(),bytes).unwrap();
    let site = Occurrence { source: source.id(),start: 0,end: 4,syntax_kind: SyntaxKind::ExprCall,role: OccurrenceRole::Call,structural_path: vec![0] };
    let scope = CoverageScope::Input { input: input.id() };
    let context = AnalysisContext { python_version: "3.14.7".into(),python_platform: "linux".into(),search_path: vec!["$input".into()],
        site_package_path: vec![],config_digest: ContentHash::of(b"cfg"),environment_digest: input.manifest,lock_digest: None };
    let provider = Provider { tool: "contract-fixture".into(),revision: "one".into(),build_digest: ContentHash::of(b"build") };
    let (run,families) = ProviderRun::new(provider.id(),context.id(),input.id(),context.config_digest,[FactFamily::Calls,FactFamily::Signatures]).unwrap();
    let (condition,nodes) = Diagram::always().records();
    let qualification = AssertionQualification { context: context.id(),scope: scope.id(),condition: condition.id(),modality: Modality::Definite,approximation: Approximation::Exact };
    let module = ProviderModule::Bundled { provider: provider.id(), bundle: ModuleBundle::Typeshed, name: "example".into() };
    let symbol = ProviderSymbol { provider: provider.id(),context: context.id(),module: module.id(),native_key: "fixture:f".into(),name: "f".into(),kind: SymbolKind::Function };
    let shapes = vec![ParameterShape { name: Some("value".into()),kind: ParameterKind::PositionalOnly,required: true },
        ParameterShape { name: Some("kwargs".into()),kind: ParameterKind::VarKeyword,required: false }];
    let (signature,parameters) = Signature::new(&qualification,symbol.id(),0,SignatureForm::List,&shapes).unwrap();
    let destination = CallDestination::Resolved { symbol: symbol.id() }; let channel = CallChannel::Direct; let receiver = Receiver::None;
    let target = CallTarget { qualification: qualification.id(),site: site.id(),destination: destination.id(),channel: channel.id(),phase: CallPhase::Call,receiver: receiver.id(),implicit: false };
    let (resolution,members) = CallResolution::new(&qualification,site.id(),channel.id(),CallPhase::Call,true,&[target.clone()]).unwrap();
    let call_surface = ProviderSurface { provider: provider.id(),family: FactFamily::Calls,name: "native-target".into() };
    let signature_surface = ProviderSurface { provider: provider.id(),family: FactFamily::Signatures,name: "native-signature".into() };
    let evidence = Evidence::Occurrence { occurrence: site.id() };
    let signature_support = SignatureSupport { assertion: signature.id(),run: run.id(),surface: signature_surface.id(),evidence: evidence.id(),origin: Origin::AnalyzerAssertion,mode: ExtractionMode::NativeTraversal,fidelity: Fidelity::NativeStructural };
    let target_support = CallTargetSupport { assertion: target.id(),run: run.id(),surface: call_surface.id(),evidence: evidence.id(),origin: Origin::AnalyzerAssertion,mode: ExtractionMode::NativeTraversal,fidelity: Fidelity::NativeStructural };
    let resolution_support = CallResolutionSupport { assertion: resolution.id(),run: run.id(),surface: call_surface.id(),evidence: evidence.id(),origin: Origin::AnalyzerAssertion,mode: ExtractionMode::NativeTraversal,fidelity: Fidelity::NativeStructural };
    for wrong_provider in [false,true] {
        let mut g_h = Harness::begin(&store, writer.clone(), lctx_model::domain::stages::Profile::Catalog, budget()).await.unwrap(); let g = g_h.generation();
        macro_rules! copy { ($($row:expr),+ $(,)?) => { $(g_h.copy(&Batch::new(&model,vec![$row.clone()], &budget()).unwrap(), &budget()).await.unwrap();)+ }; }
        copy!(input,origin,acquisition,source,site,scope,context,provider,run,condition,qualification,module,symbol,signature,
            destination,channel,receiver,target,resolution,call_surface,signature_surface,evidence,signature_support,resolution_support);
        g_h.copy(&Batch::new(&model,ArtifactChunk::split(&source,bytes).unwrap().collect(), &budget()).unwrap(), &budget()).await.unwrap();
        g_h.copy(&Batch::new(&model,nodes.clone(), &budget()).unwrap(), &budget()).await.unwrap();
        g_h.copy(&Batch::new(&model,families.clone(), &budget()).unwrap(), &budget()).await.unwrap();
        g_h.copy(&Batch::new(&model,shapes.clone(), &budget()).unwrap(), &budget()).await.unwrap();
        g_h.copy(&Batch::new(&model,parameters.clone(), &budget()).unwrap(), &budget()).await.unwrap();
        g_h.copy(&Batch::new(&model,members.clone(), &budget()).unwrap(), &budget()).await.unwrap();
        if wrong_provider {
            let other = Provider { tool: "other-namespace".into(),..provider.clone() };
            let (other_run,other_families) = ProviderRun::new(other.id(),context.id(),input.id(),context.config_digest,[FactFamily::Calls]).unwrap();
            let other_surface = ProviderSurface { provider: other.id(),..call_surface.clone() };
            let support = CallTargetSupport { run: other_run.id(),surface: other_surface.id(),..target_support.clone() };
            copy!(other,other_run,other_surface,support);
            g_h.copy(&Batch::new(&model,other_families, &budget()).unwrap(), &budget()).await.unwrap();
        } else { copy!(target_support); }
        g_h.seal().await.unwrap();
        if wrong_provider {
            let error = g_h.validate(&budget()).await.unwrap_err();
            assert!(matches!(error,Error::Model(_)) && error.to_string().contains("different provider"),"{error}");
            assert!(g_h.publish().await.is_err()); g_h.abort().await.unwrap();
        } else {
            g_h.validate(&budget()).await.unwrap(); g_h.publish().await.unwrap();
            let mut lease = store.pin(&reader,g, budget()).await.unwrap();
            assert_eq!(lease.read::<Signature>().await.unwrap().rows(),&[signature.clone()]);
            assert_eq!(lease.read::<SignatureParameter>().await.unwrap().rows(),Batch::new(&model,parameters.clone(), &budget()).unwrap().rows());
            assert_eq!(lease.read::<CallTarget>().await.unwrap().rows(),&[target.clone()]);
            assert_eq!(lease.read::<CallResolution>().await.unwrap().rows(),&[resolution.clone()]);
            assert_eq!(lease.read::<CallResolutionMember>().await.unwrap().rows(),members);
        }
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
