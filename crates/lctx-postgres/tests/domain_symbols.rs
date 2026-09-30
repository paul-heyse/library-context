//! Symbol contracts (cutover plan A6) through the permanent PostgreSQL lowering: the fixture
//! publishes and reads back its definitions, ancestry, public names and module resolutions; a
//! class listed in its own MRO is refused at validation. Not a producer test.
#[path = "../../lctx-model/tests/fixtures/symbols.rs"] #[macro_use] mod fixture;
use std::sync::Arc;
use fixture::Fixture;
use lctx_model::domain::{*, artifact::ArtifactChunk, assertion::*, attribution::*, calls::*, conditions::*, input::*, source::*, stages::Profile, symbols::*};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::{DisposableDatabase, Harness, fixtures::budget};

#[tokio::test]
async fn symbols_publish_and_a_class_in_its_own_mro_is_refused() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    let valid = Fixture::new();
    let mut own = Fixture::new();
    let sequence = SymbolSequence::new(&[own.sym["Base"].id(), own.sym["Service"].id()]).unwrap();
    own.sequences.push(sequence.clone()); own.ancestry[3].ancestors = sequence.0.id(); own.sync();
    for (fixture, admitted) in [(valid, true), (own, false)] {
        let mut attempt = Harness::begin(&store, db.writer.clone(), Profile::Catalog, budget()).await.unwrap();
        let generation = attempt.generation();
        macro_rules! copy { ($($ty:ty),+) => { $( attempt.copy(&Batch::new(&model, fixture.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap(); )+ }; }
        symbol_relations!(copy);
        attempt.seal().await.unwrap();
        if !admitted {
            let refused = attempt.validate(&budget()).await;
            assert!(matches!(&refused, Err(Error::Model(ModelError::Invalid(message))) if message.contains("a class is not its own ancestor")), "{refused:?}");
            attempt.abort().await.unwrap();
            continue;
        }
        attempt.validate(&budget()).await.unwrap();
        attempt.publish().await.unwrap();
        let mut lease = store.pin(&db.reader, generation, budget()).await.unwrap();
        fn sorted<R: Record>(mut rows: Vec<R>) -> Vec<R> { rows.sort_by_key(Record::id); rows }
        assert_eq!(lease.read::<SymbolObservation>().await.unwrap().rows(), sorted(fixture.symbols.clone()).as_slice());
        assert_eq!(lease.read::<ClassAncestryObservation>().await.unwrap().rows(), sorted(fixture.ancestry.clone()).as_slice());
        assert_eq!(lease.read::<FunctionTraitObservation>().await.unwrap().rows(), sorted(fixture.functions.clone()).as_slice());
        assert_eq!(lease.read::<PublicNameObservation>().await.unwrap().rows(), sorted(fixture.public.clone()).as_slice());
        assert_eq!(lease.read::<ExportOrigin>().await.unwrap().rows(), sorted(fixture.origins.clone()).as_slice());
        assert_eq!(lease.read::<DependencyModuleObservation>().await.unwrap().rows(), sorted(fixture.dependencies.clone()).as_slice());
        assert_eq!(lease.read::<ProviderModule>().await.unwrap().rows(), sorted(fixture.modules.values().cloned().collect()).as_slice());
        assert_eq!(lease.read::<SymbolSequenceMember>().await.unwrap().rows().len(), 2);
        lease.release().await.unwrap();
        store.retire(generation).await.unwrap();
    }
}
