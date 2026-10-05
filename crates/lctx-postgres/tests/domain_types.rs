//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[path = "../../lctx-model/tests/fixtures/types.rs"]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{
    artifact::*,
    assertion::*,
    attribution::*,
    calls::{ProviderModule, ProviderSymbol, Signature, SignatureParameter, SignatureSupport},
    conditions::*,
    input::*,
    lexical::*,
    source::*,
    types::*,
    value::Literal,
    *,
};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::DisposableDatabase;
use lctx_postgres::testing::Harness;
use std::sync::Arc;

#[tokio::test]
async fn structural_types_and_recursive_variable_restrictions_roundtrip_without_namespace_erasure()
{
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(ValidatedModel::declared(facts_relations()).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let mut cases = vec![
        (false, false, Fidelity::NativeStructural, None),
        (true, false, Fidelity::NativeStructural, None),
        (false, true, Fidelity::NativeStructural, None),
        (false, true, Fidelity::DisplayOnly, None),
    ];
    for nested in [false, true] {
        for mismatch in ["none", "provider", "context", "kind"] {
            cases.push((
                false,
                false,
                Fidelity::NativeStructural,
                Some((mismatch, nested)),
            ));
        }
    }
    for port in [
        "residual:bound",
        "residual:overload",
        "residual:generic_parameter",
        "residual:generic_body",
    ] {
        for truncated in [false, true] {
            for fidelity in [Fidelity::DisplayOnly, Fidelity::NativeStructural] {
                cases.push((false, false, fidelity, Some((port, truncated))));
            }
        }
    }
    for (foreign, opaque, fidelity, callable) in cases {
        let mut fixture = Fixture::new(foreign);
        fixture.vocabulary();
        fixture.native_signature_ports("none");
        let (fields, body) = fixture.records();
        if opaque {
            fixture.opaque(true, true, fidelity);
        }
        if let Some((mismatch, nested)) = callable {
            if let Some(port) = mismatch.strip_prefix("residual:") {
                fixture.residual_envelope(port, nested, fidelity, false);
            } else {
                fixture.named_callable(mismatch, nested);
            }
        }
        let valid = !foreign
            && (!opaque || fidelity == Fidelity::DisplayOnly)
            && callable.is_none_or(|(mismatch, _)| {
                mismatch == "none"
                    || (mismatch.starts_with("residual:") && fidelity == Fidelity::DisplayOnly)
            });
        assert!(
            fixture
                .base
                .rows::<TypeTerm>()
                .iter()
                .any(|t| t.id() == fixture.term.id())
        );
        assert!(
            fixture
                .base
                .rows::<TypeVariable>()
                .contains(&fixture.variable)
        );
        // Exercise the same in-memory check before the independent persisted-content execution.
        assert_eq!(
            fixture.base.check(&lctx_model::domain::validation::invariants_for::<TypeSupport>()[0]).is_ok()
                && fixture
                    .base
                    .check(&lctx_model::domain::validation::invariants_for::<TypePresentationSupport>()[0])
                    .is_ok(),
            valid
        );
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
            generation_h.copy(&Batch::new(&model,fixture.base.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap();
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
            ProviderModule,
            CoverageScope,
            ProviderCoverage,
            Condition,
            ConditionNode,
            assumptions::AssumptionSet,
            AssertionQualification,
            SourceArtifact,
            ArtifactChunk,
            Occurrence,
            LexicalScope,
            BindingEvent,
            LexicalTarget,
            LexicalScopeObservation,
            LexicalScopeSupport,
            BindingObservation,
            BindingSupport,
            ReferenceObservation,
            ReferenceSupport,
            LexicalResolution,
            LexicalResolutionSupport,
            Evidence,
            ProviderSymbol,
            Literal,
            TypeVariable,
            Signature,
            SignatureSupport,
            SignatureParameter,
            NativeSignatureObservation,
            NativeSignatureSupport,
            SignatureTypeSubject,
            SignatureTypeObservation,
            SignatureTypeSupport,
            TypeTerm,
            TypeSequence,
            TypeSequenceMember,
            CallableParameterList,
            CallableParameter,
            TypedDictFieldList,
            TypedDictField,
            RecordFieldObservation,
            RecordFieldSupport,
            FunctionBodyObservation,
            FunctionBodySupport,
            TypeObservation,
            TypeSupport,
            TypePresentation,
            TypePresentationSupport,
            TypeVariableRestriction,
            TypeRestrictionSupport
        );
        generation_h.seal().await.unwrap();
        if !valid {
            assert!(matches!(
                generation_h.validate(&budget()).await,
                Err(Error::Model(_))
            ));
            assert!(generation_h.publish().await.is_err());
            generation_h.abort().await.unwrap();
        } else {
            generation_h.validate(&budget()).await.unwrap();
            generation_h.publish().await.unwrap();
            let mut lease = store.pin(&reader, generation, budget()).await.unwrap();
            assert_eq!(
                lease.read::<TypeObservation>().await.unwrap().rows(),
                fixture.base.rows::<TypeObservation>()
            );
            assert_eq!(
                lease
                    .read::<TypeVariableRestriction>()
                    .await
                    .unwrap()
                    .rows(),
                fixture.base.rows::<TypeVariableRestriction>()
            );
            assert_eq!(
                lease.read::<Literal>().await.unwrap().rows(),
                fixture.base.rows::<Literal>()
            );
            // Record-field flags and every structural form round-trip.
            let mut expected = fields.clone();
            expected.sort_by_key(Record::id);
            assert_eq!(
                lease.read::<RecordFieldObservation>().await.unwrap().rows(),
                expected.as_slice()
            );
            assert_eq!(
                lease
                    .read::<FunctionBodyObservation>()
                    .await
                    .unwrap()
                    .rows(),
                std::slice::from_ref(&body)
            );
            assert_eq!(
                lease.read::<TypeTerm>().await.unwrap().rows(),
                fixture.base.rows::<TypeTerm>()
            );
            assert_eq!(
                lease
                    .read::<CallableParameter>()
                    .await
                    .unwrap()
                    .rows()
                    .len(),
                fixture.base.rows::<CallableParameter>().len()
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
