//! Model contract fixtures, not a qualified behavioral producer.
use std::sync::Arc;
use lctx_model::domain::{*,artifact::*,assertion::*,attribution::*,calls::*,conditions::*,input::*,source::*,transfer::*,value::*};
use lctx_postgres::generations::{GenerationStore,Error};
use lctx_postgres::testing::DisposableDatabase;

#[tokio::test]
async fn transfer_control_selection_survive_postgres_and_cross_scope_call_site_refuses() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(model().unwrap()); let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    let bytes = b"x y";
    let input = InputRevision::from_entries(vec![ManifestEntry { path: "x.py".into(),content: ContentHash::of(bytes),byte_len: 3 },
        ManifestEntry { path: "other.py".into(),content: ContentHash::of(b"z"),byte_len: 1 }]).unwrap();
    let origin = InputOrigin::Tree { label: "transfer-contract".into() };
    let acquisition = InputAcquisition { input: input.id(),origin: origin.id() };
    let source = SourceArtifact::from_bytes(input.id(),"x.py".into(),bytes).unwrap();
    let other = SourceArtifact::from_bytes(input.id(),"other.py".into(),b"z").unwrap();
    let scope = CoverageScope::Artifact { artifact: source.id() };
    let context = AnalysisContext { python_version: "3.14.7".into(),python_platform: "linux".into(),search_path: vec!["$input".into()],
        site_package_path: vec![],config_digest: ContentHash::of(b"cfg"),environment_digest: input.manifest,lock_digest: None };
    let provider = Provider { tool: "contract-fixture".into(),revision: "one".into(),build_digest: ContentHash::of(b"build") };
    let (run,families) = ProviderRun::new(provider.id(),context.id(),input.id(),context.config_digest,[FactFamily::Flow]).unwrap();
    let surface = ProviderSurface { provider: provider.id(),family: FactFamily::Flow,name: "flow".into() };
    let module = ProviderModule::Bundled { provider: provider.id(),name: "x".into() };
    let symbol = ProviderSymbol { provider: provider.id(),context: context.id(),module: module.id(),native_key: "f".into(),name: "f".into(),kind: SymbolKind::Function };
    let occurrences: Vec<_> = (0..3).map(|i| Occurrence { source: source.id(),start: i,end: i+1,syntax_kind: SyntaxKind::ExprName,
        role: OccurrenceRole::Read,structural_path: vec![i as i32] }).collect();
    let other_site = Occurrence { source: other.id(),start: 0,end: 1,..occurrences[0].clone() };
    let roots: Vec<_> = occurrences.iter().map(|o| PlaceRoot::Occurrence { occurrence: o.id() }).collect();
    let path = AccessPath::empty(); let places: Vec<_> = roots.iter().map(|r| Place { root: r.id(),path: path.id() }).collect();
    let predicate = Predicate::Truthy;
    let other_root = PlaceRoot::Occurrence { occurrence: other_site.id() };
    let other_place = Place { root: other_root.id(),path: path.id() };
    for boundary in 0..3 {
        let atom = EvaluationAtom { evaluation: occurrences[1].id(),context: context.id(),predicate: predicate.id(),operand: Some(if boundary == 2 { other_place.id() } else { places[1].id() }) };
        let diagram = Diagram::from_atom(atom.id()); let (condition,nodes) = diagram.records();
        let qualification = AssertionQualification { context: context.id(),scope: scope.id(),condition: condition.id(),modality: Modality::Definite,approximation: Approximation::Exact };
        let influence = ControlInfluence { qualification: qualification.id(),input: places[1].id(),atom: atom.id(),evaluation: atom.evaluation };
        let evidence = Evidence::Occurrence { occurrence: occurrences[0].id() };
        let influence_evidence = Evidence::Occurrence { occurrence: atom.evaluation };
        let control_support = ControlSupport { assertion: influence.id(),run: run.id(),surface: surface.id(),evidence: influence_evidence.id(),
            origin: Origin::DerivedAnalysis,mode: ExtractionMode::GraphAnalysis,fidelity: Fidelity::NormalizedStructural };
        let key = TransferKey { owner: symbol.id(),input: places[0].id(),output: places[2].id(),context: context.id(),scope: scope.id(),
            modality: Modality::Definite,approximation: Approximation::Exact,kind: TransferKind::Identity,
            call_site: Some(if boundary == 1 { other_site.id() } else { occurrences[0].id() }),provenance: ProvenanceClass::FlowLocal };
        let branch = TransferBranch::new(key.clone(),qualification.clone(),diagram.clone()).unwrap();
        let alternative = branch.alternative(); let selection = branch.selection(&influence,&qualification).unwrap().unwrap();
        let support = TransferSupport { assertion: alternative.id(),run: run.id(),surface: surface.id(),evidence: evidence.id(),
            origin: Origin::DerivedAnalysis,mode: ExtractionMode::GraphAnalysis,fidelity: Fidelity::NormalizedStructural };
        let g = store.create_conformance(ContentHash::of(b"transfer-fixture"),"behavioral").await.unwrap();
        macro_rules! copy { ($($row:expr),+ $(,)?) => { $(store.copy(&writer,g,&Batch::new(&model,vec![$row.clone()], &budget()).unwrap(), &budget()).await.unwrap();)+ }; }
        copy!(input,origin,acquisition,source,other,scope,context,provider,run,surface,module,symbol,path,predicate,atom,condition,qualification,
            influence,evidence,influence_evidence,control_support,key,alternative,selection,support,other_site,other_root,other_place);
        macro_rules! copies { ($($rows:expr),+ $(,)?) => { $(store.copy(&writer,g,&Batch::new(&model,$rows.clone(), &budget()).unwrap(), &budget()).await.unwrap();)+ }; }
        copies!(families,occurrences,roots,places,nodes);
        store.copy(&writer,g,&Batch::new(&model,ArtifactChunk::split(&source,bytes).unwrap().collect(), &budget()).unwrap(), &budget()).await.unwrap();
        store.copy(&writer,g,&Batch::new(&model,ArtifactChunk::split(&other,b"z").unwrap().collect(), &budget()).unwrap(), &budget()).await.unwrap();
        store.seal(g).await.unwrap();
        if boundary != 0 {
            let error = store.validate(g, &budget()).await.unwrap_err();
            assert!(matches!(error,Error::Model(_)) && error.to_string().contains("scope"),"{error}");
            assert!(store.publish(g).await.is_err()); store.abort(g).await.unwrap();
        } else {
            store.validate(g, &budget()).await.unwrap(); store.publish(g).await.unwrap();
            let mut lease = store.pin(&reader,g, budget()).await.unwrap();
            assert_eq!(lease.read::<TransferKey>().await.unwrap().rows(),&[key]);
            assert_eq!(lease.read::<TransferAlternative>().await.unwrap().rows(),&[alternative]);
            assert_eq!(lease.read::<TransferSupport>().await.unwrap().rows(),&[support]);
            assert_eq!(lease.read::<ControlInfluence>().await.unwrap().rows(),&[influence.clone()]);
            assert_eq!(lease.read::<ControlSupport>().await.unwrap().rows(),&[control_support.clone()]);
            assert_eq!(lease.read::<Selection>().await.unwrap().rows(),&[selection.clone()]);
            let derivation: (String,String,Vec<u8>) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT rule,conclusion_relation,conclusion_id FROM {}.derivations",g.schema()))).fetch_one(&reader).await.unwrap();
            assert_eq!(derivation,("control_selects_transfer".into(),Selection::NAME.into(),selection.id().bytes().to_vec()));
            let premises: Vec<(String,String,Vec<u8>)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT role,premise_relation,premise_id FROM {}.derivation_premises ORDER BY role",g.schema()))).fetch_all(&reader).await.unwrap();
            assert_eq!(premises,vec![("alternative".into(),TransferAlternative::NAME.into(),branch.alternative().id().bytes().to_vec()),
                ("influence".into(),ControlInfluence::NAME.into(),influence.id().bytes().to_vec())]);
        }
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
