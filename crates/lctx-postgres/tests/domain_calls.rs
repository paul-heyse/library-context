//! Native call/signature contract fixtures through real generated PostgreSQL lowerings.
use std::sync::Arc;
use lctx_model::domain::{*, artifact::*, assertion::*, attribution::*, calls::*, conditions::*, input::*, source::*};
use lctx_postgres::generations::{GenerationStore,Error};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres,testcontainers::{ImageExt,runners::AsyncRunner}};

#[tokio::test]
async fn call_signature_membership_support_ownership_and_readback() {
    let (image,tag) = lctx_postgres::serving::TEST_IMAGE.trim().split_once(':').unwrap();
    let container = Postgres::default().with_name(image).with_tag(tag).start().await.expect("Docker and pinned PostgreSQL18 required");
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = |role: &str| format!("postgres://{role}:postgres@127.0.0.1:{port}/postgres");
    let owner = PgPool::connect(&url("postgres")).await.unwrap();
    sqlx::raw_sql("CREATE ROLE lctx_importer LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_serving LOGIN PASSWORD 'postgres'").execute(&owner).await.unwrap();
    let writer = PgPool::connect(&url("lctx_importer")).await.unwrap();
    let reader = PgPool::connect(&url("lctx_serving")).await.unwrap();
    let model = Arc::new(model().unwrap()); let store = GenerationStore::install(owner,model.clone()).await.unwrap();
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
    let symbol = ProviderSymbol { provider: provider.id(),context: context.id(),module: "example".into(),native_key: "fixture:f".into(),name: "f".into(),kind: SymbolKind::Function };
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
        let g = store.create_conformance(ContentHash::of(b"call-fixture"),"catalog").await.unwrap();
        macro_rules! copy { ($($row:expr),+ $(,)?) => { $(store.copy(&writer,g,&Batch::new(&model,vec![$row.clone()]).unwrap()).await.unwrap();)+ }; }
        copy!(input,origin,acquisition,source,site,scope,context,provider,run,condition,qualification,symbol,signature,
            destination,channel,receiver,target,resolution,call_surface,signature_surface,evidence,signature_support,resolution_support);
        store.copy(&writer,g,&Batch::new(&model,ArtifactChunk::split(&source,bytes).unwrap().collect()).unwrap()).await.unwrap();
        store.copy(&writer,g,&Batch::new(&model,nodes.clone()).unwrap()).await.unwrap();
        store.copy(&writer,g,&Batch::new(&model,families.clone()).unwrap()).await.unwrap();
        store.copy(&writer,g,&Batch::new(&model,shapes.clone()).unwrap()).await.unwrap();
        store.copy(&writer,g,&Batch::new(&model,parameters.clone()).unwrap()).await.unwrap();
        store.copy(&writer,g,&Batch::new(&model,members.clone()).unwrap()).await.unwrap();
        if wrong_provider {
            let other = Provider { tool: "other-namespace".into(),..provider.clone() };
            let (other_run,other_families) = ProviderRun::new(other.id(),context.id(),input.id(),context.config_digest,[FactFamily::Calls]).unwrap();
            let other_surface = ProviderSurface { provider: other.id(),..call_surface.clone() };
            let support = CallTargetSupport { run: other_run.id(),surface: other_surface.id(),..target_support.clone() };
            copy!(other,other_run,other_surface,support);
            store.copy(&writer,g,&Batch::new(&model,other_families).unwrap()).await.unwrap();
        } else { copy!(target_support); }
        store.seal(g).await.unwrap();
        if wrong_provider {
            let error = store.validate(g).await.unwrap_err();
            assert!(matches!(error,Error::Model(_)) && error.to_string().contains("different provider"),"{error}");
            assert!(store.publish(g).await.is_err()); store.abort(g).await.unwrap();
        } else {
            store.validate(g).await.unwrap(); store.publish(g).await.unwrap();
            let mut lease = store.pin(&reader,g).await.unwrap();
            assert_eq!(lease.read::<Signature>().await.unwrap().rows(),&[signature.clone()]);
            assert_eq!(lease.read::<SignatureParameter>().await.unwrap().rows(),Batch::new(&model,parameters.clone()).unwrap().rows());
            assert_eq!(lease.read::<CallTarget>().await.unwrap().rows(),&[target.clone()]);
            assert_eq!(lease.read::<CallResolution>().await.unwrap().rows(),&[resolution.clone()]);
            assert_eq!(lease.read::<CallResolutionMember>().await.unwrap().rows(),members);
        }
    }
}
