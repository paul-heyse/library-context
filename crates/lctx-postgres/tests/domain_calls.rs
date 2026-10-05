//! Native call/signature contract fixtures through real generated PostgreSQL lowerings.
use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, calls::*, conditions::*, input::*, source::*, *,
};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::DisposableDatabase;
use lctx_postgres::testing::Harness;
use std::sync::Arc;

#[path = "../../lctx-model/tests/fixtures/callers.rs"]
mod caller_fixture;
#[tokio::test]
async fn call_site_caller_ownership_is_checked_in_persisted_content() {
    use lctx_model::domain::lexical::*;
    let db = DisposableDatabase::start().await;
    let model = Arc::new(ValidatedModel::declared(facts_relations()).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    for form in ["module", "symbol", "class", "decorator"] {
        for mismatch in ["none", "provider", "context"] {
            let fixture = caller_fixture::fixture(form, mismatch);
            assert_eq!(
                fixture
                    .check(
                        &lctx_model::domain::validation::invariants_for::<ProviderCallSiteSupport>(
                        )[0]
                    )
                    .is_ok(),
                mismatch == "none"
            );
            let mut attempt = Harness::begin(
                &store,
                db.writer.clone(),
                lctx_model::domain::stages::Profile::Catalog,
                budget(),
            )
            .await
            .unwrap();
            macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(attempt.copy(&Batch::new(&model,fixture.rows::<$ty>(),&budget()).unwrap(),&budget()).await.unwrap();)+ }; }
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
                ProviderCoverage,
                Condition,
                ConditionNode,
                assumptions::AssumptionSet,
                AssertionQualification,
                SourceArtifact,
                ArtifactChunk,
                Occurrence,
                Module,
                ProviderModule,
                ProviderSymbol,
                ProviderCallable,
                CallOrigin,
                ProviderCallSite,
                ProviderCallSiteSupport,
                Evidence,
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
                LexicalResolutionSupport
            );
            attempt.seal().await.unwrap();
            let result = attempt.validate(&budget()).await;
            assert_eq!(
                result.is_ok(),
                mismatch == "none",
                "{form}/{mismatch}: {result:?}"
            );
            if mismatch == "none" {
                attempt.publish().await.unwrap();
                store.retire(attempt.generation()).await.unwrap();
            } else {
                assert!(attempt.publish().await.is_err());
                attempt.abort().await.unwrap();
            }
        }
    }
}

#[tokio::test]
async fn call_signature_membership_support_ownership_and_readback() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(ValidatedModel::declared(facts_relations()).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let bytes = b"f(1)";
    let input = InputRevision::from_entries(vec![ManifestEntry {
        path: "example.py".into(),
        content: ContentHash::of(bytes),
        byte_len: bytes.len() as i64,
    }])
    .unwrap();
    let origin = InputOrigin::Tree {
        label: "contract-fixture".into(),
    };
    let acquisition = InputAcquisition {
        input: input.id(),
        origin: origin.id(),
    };
    let source = SourceArtifact::from_bytes(input.id(), "example.py".into(), bytes).unwrap();
    let site = Occurrence {
        source: source.id(),
        start: 0,
        end: 4,
        syntax_kind: SyntaxKind::ExprCall,
        role: OccurrenceRole::Call,
        structural_path: vec![0],
    };
    let scope = CoverageScope::Input { input: input.id() };
    let context = AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec!["$input".into()],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"cfg"),
        environment_digest: input.manifest,
        lock_digest: None,
    };
    let provider = Provider {
        tool: "contract-fixture".into(),
        revision: "one".into(),
        build_digest: ContentHash::of(b"build"),
    };
    let (run, families) = ProviderRun::new(
        provider.id(),
        context.id(),
        input.id(),
        context.config_digest,
        [FactFamily::Calls, FactFamily::Signatures],
    )
    .unwrap();
    let (condition, nodes) = Diagram::always().records();
    let qualification = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
        context: context.id(),
        scope: scope.id(),
        condition: condition.id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    let module = ProviderModule::Bundled {
        provider: provider.id(),
        bundle: ModuleBundle::Typeshed,
        name: "example".into(),
    };
    let symbol = ProviderSymbol {
        provider: provider.id(),
        context: context.id(),
        module: module.id(),
        native_key: "fixture:f".into(),
        name: "f".into(),
        kind: SymbolKind::Function,
    };
    let shapes = vec![
        ParameterShape {
            name: Some("value".into()),
            kind: ParameterKind::PositionalOnly,
            required: true,
        },
        ParameterShape {
            name: Some("kwargs".into()),
            kind: ParameterKind::VarKeyword,
            required: false,
        },
    ];
    let (signature, parameters) = Signature::new(
        &qualification,
        lctx_model::domain::calls::SignatureRole::Source,
        None,
        symbol.id(),
        0,
        SignatureForm::List,
        &shapes,
    )
    .unwrap();
    let destination = CallDestination::Resolved {
        symbol: symbol.id(),
    };
    let channel = CallChannel::Direct;
    let receiver = Receiver::None;
    let target = CallTarget {
        qualification: qualification.id(),
        site: site.id(),
        destination: destination.id(),
        channel: channel.id(),
        phase: CallPhase::Call,
        receiver: receiver.id(),
        implicit: false,
        origin: CallOrigin::explicit(),
        receiver_class: None,
        passing: Some(ReceiverPassing::NotPassed),
        class_method: None,
        static_method: None,
    };
    let (resolution, members) = CallResolution::new(
        &qualification,
        site.id(),
        CallOrigin::explicit(),
        channel.id(),
        CallPhase::Call,
        true,
        std::slice::from_ref(&target),
    )
    .unwrap();
    let explicit = CallOrigin::new(&[]).unwrap().0;
    // `for` at the same site: an implicit call event whose native unresolved reason is kept.
    let (for_iter, for_iter_steps) = CallOrigin::new(&[(OriginStep::ForIter, None)]).unwrap();
    let unresolved = CallDestination::Unresolved {
        reason: ObligationKind::UnresolvedTarget,
        native: Some(PysaUnresolvedReason::Mixed),
    };
    let unknown = Receiver::Unknown {
        reason: ObligationKind::AmbiguousBinding,
    };
    let implicit_target = CallTarget {
        origin: for_iter.id(),
        destination: unresolved.id(),
        receiver: unknown.id(),
        passing: None,
        ..target.clone()
    };
    let (implicit_resolution, implicit_members) = CallResolution::new(
        &qualification,
        site.id(),
        for_iter.id(),
        channel.id(),
        CallPhase::Call,
        false,
        std::slice::from_ref(&implicit_target),
    )
    .unwrap();
    let module_row = Module {
        source: source.id(),
        qualified_name: "example".into(),
    };
    let acquired = ProviderModule::Acquired {
        module: module_row.id(),
    };
    let caller = ProviderCallable::ModuleBody {
        provider: provider.id(),
        context: context.id(),
        module: acquired.id(),
    };
    let sites = vec![
        ProviderCallSite {
            qualification: qualification.id(),
            site: site.id(),
            origin: explicit.id(),
            kind: PysaSiteKind::Regular,
            caller: caller.id(),
            callee: PysaCalleeKind::Call,
            is_attribute: None,
        },
        ProviderCallSite {
            qualification: qualification.id(),
            site: site.id(),
            origin: for_iter.id(),
            kind: PysaSiteKind::ArtificialCall,
            caller: caller.id(),
            callee: PysaCalleeKind::Call,
            is_attribute: None,
        },
    ];
    let call_surface = ProviderSurface {
        provider: provider.id(),
        family: FactFamily::Calls,
        name: "native-target".into(),
    };
    let signature_surface = ProviderSurface {
        provider: provider.id(),
        family: FactFamily::Signatures,
        name: "native-signature".into(),
    };
    let evidence = Evidence::Occurrence {
        occurrence: site.id(),
    };
    let signature_support = SignatureSupport {
        assertion: signature.id(),
        run: run.id(),
        surface: signature_surface.id(),
        evidence: evidence.id(),
        origin: Origin::AnalyzerAssertion,
        mode: ExtractionMode::NativeTraversal,
        fidelity: Fidelity::NativeStructural,
    };
    let target_support = CallTargetSupport {
        assertion: target.id(),
        run: run.id(),
        surface: call_surface.id(),
        evidence: evidence.id(),
        origin: Origin::AnalyzerAssertion,
        mode: ExtractionMode::NativeTraversal,
        fidelity: Fidelity::NativeStructural,
    };
    let resolution_support = CallResolutionSupport {
        assertion: resolution.id(),
        run: run.id(),
        surface: call_surface.id(),
        evidence: evidence.id(),
        origin: Origin::AnalyzerAssertion,
        mode: ExtractionMode::NativeTraversal,
        fidelity: Fidelity::NativeStructural,
    };
    let implicit_supports = (
        CallTargetSupport {
            assertion: implicit_target.id(),
            ..target_support.clone()
        },
        CallResolutionSupport {
            assertion: implicit_resolution.id(),
            ..resolution_support.clone()
        },
    );
    let site_supports: Vec<_> = sites
        .iter()
        .map(|row| ProviderCallSiteSupport {
            assertion: row.id(),
            run: run.id(),
            surface: call_surface.id(),
            evidence: evidence.id(),
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::ReportDecode,
            fidelity: Fidelity::NativeStructural,
        })
        .collect();
    for wrong_provider in [false, true] {
        let mut g_h = Harness::begin(
            &store,
            writer.clone(),
            lctx_model::domain::stages::Profile::Catalog,
            budget(),
        )
        .await
        .unwrap();
        let g = g_h.generation();
        macro_rules! copy { ($($row:expr),+ $(,)?) => { $(g_h.copy(&Batch::new(&model,vec![$row.clone()], &budget()).unwrap(), &budget()).await.unwrap();)+ }; }
        copy!(
            assumptions::AssumptionSet::empty(),
            input,
            origin,
            acquisition,
            source,
            site,
            scope,
            context,
            provider,
            run,
            condition,
            qualification,
            module,
            symbol,
            signature,
            destination,
            channel,
            receiver,
            explicit,
            target,
            resolution,
            call_surface,
            signature_surface,
            evidence,
            signature_support,
            resolution_support
        );
        g_h.copy(
            &Batch::new(
                &model,
                ArtifactChunk::split(&source, bytes).unwrap().collect(),
                &budget(),
            )
            .unwrap(),
            &budget(),
        )
        .await
        .unwrap();
        g_h.copy(
            &Batch::new(&model, nodes.clone(), &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
        g_h.copy(
            &Batch::new(&model, families.clone(), &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
        g_h.copy(
            &Batch::new(&model, shapes.clone(), &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
        g_h.copy(
            &Batch::new(&model, parameters.clone(), &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
        g_h.copy(
            &Batch::new(
                &model,
                [members.clone(), implicit_members.clone()].concat(),
                &budget(),
            )
            .unwrap(),
            &budget(),
        )
        .await
        .unwrap();
        copy!(
            for_iter,
            module_row,
            acquired,
            caller,
            unresolved,
            unknown,
            implicit_target,
            implicit_resolution,
            implicit_supports.0,
            implicit_supports.1
        );
        g_h.copy(
            &Batch::new(&model, for_iter_steps.clone(), &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
        g_h.copy(
            &Batch::new(&model, sites.clone(), &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
        g_h.copy(
            &Batch::new(&model, site_supports.clone(), &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
        if wrong_provider {
            let other = Provider {
                tool: "other-namespace".into(),
                ..provider.clone()
            };
            let (other_run, other_families) = ProviderRun::new(
                other.id(),
                context.id(),
                input.id(),
                context.config_digest,
                [FactFamily::Calls],
            )
            .unwrap();
            let other_surface = ProviderSurface {
                provider: other.id(),
                ..call_surface.clone()
            };
            let support = CallTargetSupport {
                run: other_run.id(),
                surface: other_surface.id(),
                ..target_support.clone()
            };
            copy!(other, other_run, other_surface, support);
            g_h.copy(
                &Batch::new(&model, other_families, &budget()).unwrap(),
                &budget(),
            )
            .await
            .unwrap();
        } else {
            copy!(target_support);
        }
        g_h.seal().await.unwrap();
        if wrong_provider {
            let error = g_h.validate(&budget()).await.unwrap_err();
            assert!(
                matches!(error, Error::Model(_))
                    && (error.to_string().contains("different provider")
                        || error.to_string().contains("own provider and context")),
                "{error}"
            );
            assert!(g_h.publish().await.is_err());
            g_h.abort().await.unwrap();
        } else {
            g_h.validate(&budget()).await.unwrap();
            g_h.publish().await.unwrap();
            let mut lease = store.pin(&reader, g, budget()).await.unwrap();
            assert_eq!(
                lease.read::<Signature>().await.unwrap().rows(),
                std::slice::from_ref(&signature)
            );
            assert_eq!(
                lease.read::<SignatureParameter>().await.unwrap().rows(),
                Batch::new(&model, parameters.clone(), &budget())
                    .unwrap()
                    .rows()
            );
            fn sorted<R: Record>(mut rows: Vec<R>) -> Vec<R> {
                rows.sort_by_key(Record::id);
                rows
            }
            assert_eq!(
                lease.read::<CallTarget>().await.unwrap().rows(),
                sorted(vec![target.clone(), implicit_target.clone()]).as_slice()
            );
            assert_eq!(
                lease.read::<CallResolution>().await.unwrap().rows(),
                sorted(vec![resolution.clone(), implicit_resolution.clone()]).as_slice()
            );
            assert_eq!(
                lease
                    .read::<CallResolutionMember>()
                    .await
                    .unwrap()
                    .rows()
                    .len(),
                2
            );
            assert_eq!(
                lease.read::<CallDestination>().await.unwrap().rows(),
                sorted(vec![destination.clone(), unresolved.clone()]).as_slice(),
                "the native reason is kept"
            );
            assert_eq!(
                lease.read::<ProviderCallSite>().await.unwrap().rows(),
                sorted(sites.clone()).as_slice()
            );
            assert_eq!(
                lease.read::<CallOriginStep>().await.unwrap().rows(),
                for_iter_steps.as_slice()
            );
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
