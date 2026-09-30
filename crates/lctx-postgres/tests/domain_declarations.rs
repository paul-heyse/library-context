#[path = "../../lctx-model/tests/fixtures/declarations.rs"]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, calls::*, conditions::*, declarations::*, input::*,
    source::*, *,
};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::DisposableDatabase;
use lctx_postgres::testing::Harness;
use std::sync::Arc;

#[tokio::test]
async fn declaration_links_and_call_syntax_round_trip_and_refuse_stray_parameters() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(ValidatedModel::validate(facts_relations()).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    for stray in [false, true] {
        let mut fixture = Fixture::new();
        if stray {
            fixture.set_parameters(vec![
                (fixture.members[0].id(), fixture.stray.id()),
                (fixture.members[1].id(), fixture.b.id()),
            ]);
        }
        let mut generation_h = Harness::begin(
            &store,
            writer.clone(),
            lctx_model::domain::stages::Profile::Catalog,
            budget(),
        )
        .await
        .unwrap();
        let generation = generation_h.generation();
        macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(
            generation_h.copy(&Batch::new(&model, fixture.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap();
        )+ }; }
        copy!(
            InputRevision,
            InputOrigin,
            InputAcquisition,
            AnalysisContext,
            Provider,
            ProviderRun,
            RunFamily,
            ProviderSurface,
            CoverageScope,
            Condition,
            ConditionNode,
            AssertionQualification,
            SourceArtifact,
            ArtifactChunk,
            Occurrence,
            Module,
            ProviderModule,
            ProviderSymbol,
            ParameterShape,
            Signature,
            SignatureParameter,
            SignatureSupport,
            SymbolDeclaration,
            SymbolDeclarationSupport,
            ParameterDeclaration,
            ParameterDeclarationSupport,
            CallSyntax,
            CallSyntaxSupport,
            CallArgument,
            Evidence
        );
        generation_h.seal().await.unwrap();
        if stray {
            assert!(matches!(
                generation_h.validate(&budget()).await,
                Err(Error::Model(_))
            ));
            assert!(generation_h.publish().await.is_err());
            generation_h.abort().await.unwrap();
        } else {
            let digest = generation_h.validate(&budget()).await.unwrap();
            assert_eq!(
                fixture.validate().unwrap(),
                digest,
                "in-memory and stored validation agree"
            );
            generation_h.publish().await.unwrap();
            let mut lease = store.pin(&reader, generation, budget()).await.unwrap();
            assert_eq!(
                lease.read::<ParameterDeclaration>().await.unwrap().rows(),
                fixture.rows::<ParameterDeclaration>()
            );
            assert_eq!(
                lease.read::<CallArgument>().await.unwrap().rows(),
                fixture.rows::<CallArgument>()
            );
            lease.release().await.unwrap();
            store.retire(generation).await.unwrap();
        }
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(
        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
    )
    .unwrap()
}
