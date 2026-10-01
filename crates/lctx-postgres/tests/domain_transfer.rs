//! Model contract fixtures, not a qualified behavioral producer.
#[path = "../../lctx-model/tests/fixtures/analysis_support.rs"]
mod analysis_fixture;
#[path = "fixtures/analysis_support.rs"]
mod stored_analysis;
use lctx_model::domain::{
    artifact::*,
    assertion::*,
    attribution::*,
    calls::*,
    conditions::*,
    input::*,
    normalized::entities::EntityRef,
    source::*,
    transfer::{local::*, *},
    value::*,
    *,
};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::DisposableDatabase;
use lctx_postgres::testing::Harness;
use std::sync::Arc;

#[tokio::test]
async fn transfer_control_selection_survive_postgres_and_cross_scope_call_site_refuses() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(transfer_model());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let bytes = b"x y";
    let input = InputRevision::from_entries(vec![
        ManifestEntry {
            path: "x.py".into(),
            content: ContentHash::of(bytes),
            byte_len: 3,
        },
        ManifestEntry {
            path: "other.py".into(),
            content: ContentHash::of(b"z"),
            byte_len: 1,
        },
    ])
    .unwrap();
    let origin = InputOrigin::Tree {
        label: "transfer-contract".into(),
    };
    let acquisition = InputAcquisition {
        input: input.id(),
        origin: origin.id(),
    };
    let source = SourceArtifact::from_bytes(input.id(), "x.py".into(), bytes).unwrap();
    let other = SourceArtifact::from_bytes(input.id(), "other.py".into(), b"z").unwrap();
    let scope = CoverageScope::Artifact {
        artifact: source.id(),
    };
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
        [FactFamily::Flow],
    )
    .unwrap();
    let surface = ProviderSurface {
        provider: provider.id(),
        family: FactFamily::Flow,
        name: "flow".into(),
    };
    let module = ProviderModule::Bundled {
        provider: provider.id(),
        bundle: ModuleBundle::Typeshed,
        name: "x".into(),
    };
    let symbol = ProviderSymbol {
        provider: provider.id(),
        context: context.id(),
        module: module.id(),
        native_key: "f".into(),
        name: "f".into(),
        kind: SymbolKind::Function,
    };
    let source_module = Module {
        source: source.id(),
        qualified_name: "x".into(),
    };
    let entity = EntityRef::Module {
        module: source_module.id(),
    };
    let occurrences: Vec<_> = (0..3)
        .map(|i| Occurrence {
            source: source.id(),
            start: i,
            end: i + 1,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Read,
            structural_path: vec![i as i32],
        })
        .collect();
    let other_site = Occurrence {
        source: other.id(),
        start: 0,
        end: 1,
        ..occurrences[0].clone()
    };
    let roots: Vec<_> = occurrences
        .iter()
        .map(|o| PlaceRoot::Occurrence { occurrence: o.id() })
        .collect();
    let path = AccessPath::empty();
    let places: Vec<_> = roots
        .iter()
        .map(|r| Place {
            root: r.id(),
            path: path.id(),
        })
        .collect();
    let predicate = Predicate::Truthy;
    let other_root = PlaceRoot::Occurrence {
        occurrence: other_site.id(),
    };
    let other_place = Place {
        root: other_root.id(),
        path: path.id(),
    };
    for boundary in 0..3 {
        let atom = EvaluationAtom {
            evaluation: occurrences[1].id(),
            context: context.id(),
            predicate: predicate.id(),
            operand: Some(if boundary == 2 {
                other_place.id()
            } else {
                places[1].id()
            }),
        };
        let diagram = Diagram::from_atom(atom.id());
        let (condition, nodes) = diagram.records();
        let qualification = AssertionQualification {
            context: context.id(),
            scope: scope.id(),
            condition: condition.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let influence = ControlInfluence {
            qualification: qualification.id(),
            input: places[1].id(),
            atom: atom.id(),
            evaluation: atom.evaluation,
        };
        let evidence = Evidence::Occurrence {
            occurrence: occurrences[0].id(),
        };
        let influence_evidence = Evidence::Occurrence {
            occurrence: atom.evaluation,
        };
        let foundation = analysis_fixture::SupportFixture::new(
            input.id(),
            run.id(),
            surface.id(),
            evidence.id(),
            &qualification,
            &diagram,
            occurrences[0].id(),
            places[0].id(),
        );
        let control_foundation = analysis_fixture::SupportFixture::new(
            input.id(),
            run.id(),
            surface.id(),
            influence_evidence.id(),
            &qualification,
            &diagram,
            atom.evaluation,
            influence.input,
        );
        let control_support = ControlSupport {
            assertion: influence.id(),
            source: control_foundation.derived.id(),
        };
        let key = TransferKey {
            owner: entity.id(),
            input: places[0].id(),
            output: places[2].id(),
            context: context.id(),
            scope: scope.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
            kind: TransferKind::Identity,
            call_site: Some(if boundary == 1 {
                other_site.id()
            } else {
                occurrences[0].id()
            }),
            provenance: ProvenanceClass::FlowLocal,
        };
        let branch = TransferBranch::new(
            key.clone(),
            qualification.clone(),
            diagram.clone(),
            &budget(),
        )
        .unwrap();
        let alternative = branch.alternative();
        let selection = branch
            .selection(&influence, &qualification)
            .unwrap()
            .unwrap();
        let support = TransferSupport {
            assertion: alternative.id(),
            source: foundation.derived.id(),
        };
        let mut g_h = Harness::begin(
            &store,
            writer.clone(),
            lctx_model::domain::stages::Profile::Behavioral,
            budget(),
        )
        .await
        .unwrap();
        let g = g_h.generation();
        let mut native_inventory =
            lctx_model::domain::analysis::native::NativeInventory::new(&budget());
        macro_rules! copy { ($($row:expr),+ $(,)?) => { $(stored_analysis::copy(&g_h, &model, vec![$row.clone()], &mut native_inventory, &budget()).await.unwrap();)+ }; }
        copy!(
            input,
            origin,
            acquisition,
            source,
            other,
            scope,
            context,
            provider,
            run,
            surface,
            module,
            source_module,
            entity,
            symbol,
            path,
            predicate,
            atom,
            condition,
            qualification,
            influence,
            evidence,
            influence_evidence,
            control_support,
            key,
            alternative,
            selection,
            support,
            other_site,
            other_root,
            other_place
        );
        macro_rules! copies { ($($rows:expr),+ $(,)?) => { $(stored_analysis::copy(&g_h, &model, $rows.clone(), &mut native_inventory, &budget()).await.unwrap();)+ }; }
        copy!(
            foundation.parameters,
            foundation.definition,
            foundation.invocation
        );
        copies!(
            vec![foundation.use_.clone(), control_foundation.use_.clone()],
            vec![
                foundation.observation.clone(),
                control_foundation.observation.clone()
            ],
            vec![
                foundation.support.clone(),
                control_foundation.support.clone()
            ],
            vec![
                foundation.subject.clone(),
                control_foundation.subject.clone()
            ],
            vec![
                foundation.proposition.clone(),
                control_foundation.proposition.clone()
            ],
            vec![
                foundation.derivation.clone(),
                control_foundation.derivation.clone()
            ],
            vec![
                foundation.native.clone(),
                foundation.derived.clone(),
                control_foundation.native.clone(),
                control_foundation.derived.clone()
            ],
            foundation
                .members
                .iter()
                .chain(&control_foundation.members)
                .cloned()
                .collect::<Vec<_>>()
        );
        copies!(families, occurrences, roots, places, nodes);
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
            &Batch::new(
                &model,
                ArtifactChunk::split(&other, b"z").unwrap().collect(),
                &budget(),
            )
            .unwrap(),
            &budget(),
        )
        .await
        .unwrap();
        stored_analysis::finish(&g_h, &model, native_inventory, &budget())
            .await
            .unwrap();
        g_h.seal().await.unwrap();
        if boundary != 0 {
            let error = g_h.validate(&budget()).await.unwrap_err();
            assert!(
                matches!(error, Error::Model(_)) && error.to_string().contains("scope"),
                "{error}"
            );
            assert!(g_h.publish().await.is_err());
            g_h.abort().await.unwrap();
        } else {
            g_h.validate(&budget()).await.unwrap();
            g_h.publish().await.unwrap();
            let mut lease = store.pin(&reader, g, budget()).await.unwrap();
            assert_eq!(lease.read::<TransferKey>().await.unwrap().rows(), &[key]);
            assert_eq!(
                lease.read::<TransferAlternative>().await.unwrap().rows(),
                &[alternative]
            );
            assert_eq!(
                lease.read::<TransferSupport>().await.unwrap().rows(),
                &[support]
            );
            assert_eq!(
                lease.read::<ControlInfluence>().await.unwrap().rows(),
                std::slice::from_ref(&influence)
            );
            assert_eq!(
                lease.read::<ControlSupport>().await.unwrap().rows(),
                std::slice::from_ref(&control_support)
            );
            assert_eq!(
                lease.read::<Selection>().await.unwrap().rows(),
                std::slice::from_ref(&selection)
            );
            let derivation: (String, String, Vec<u8>) =
                sqlx::query_as(sqlx::AssertSqlSafe(format!(
                    "SELECT rule,conclusion_relation,conclusion_id FROM {}.derivations WHERE source_relation='{}'",
                    g.schema(), Selection::NAME
                )))
                .fetch_one(&reader)
                .await
                .unwrap();
            assert_eq!(
                derivation,
                (
                    "control_selects_transfer".into(),
                    Selection::NAME.into(),
                    selection.id().bytes().to_vec()
                )
            );
            let premises: Vec<(String,String,Vec<u8>)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT role,premise_relation,premise_id FROM {}.derivation_premises WHERE source_relation='{}' ORDER BY role",g.schema(),Selection::NAME))).fetch_all(&reader).await.unwrap();
            assert_eq!(
                premises,
                vec![
                    (
                        "alternative".into(),
                        TransferAlternative::NAME.into(),
                        branch.alternative().id().bytes().to_vec()
                    ),
                    (
                        "influence".into(),
                        ControlInfluence::NAME.into(),
                        influence.id().bytes().to_vec()
                    )
                ]
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

/// This conformance control owns only Local transfer/control contracts, not later composition or
/// entry replay. Keep its nominal declaration closure independent of future publication owners.
fn transfer_model() -> ValidatedModel {
    let mut relations = facts_relations();
    relations.extend(analysis::early_relations());
    relations.extend(analysis::local::relations());
    relations.extend(transfer::local::relations());
    relations.extend([
        Relation::of::<ControlInfluence>(),
        Relation::of::<ControlSupport>(),
        Relation::of::<Selection>(),
    ]);
    relations.extend(normalized::coverage::relations());
    relations.extend([
        Relation::of::<normalized::entities::CallableEntity>(),
        Relation::of::<normalized::entities::ClassEntity>(),
        Relation::of::<normalized::entities::ParameterEntity>(),
        Relation::of::<normalized::entities::FieldEntity>(),
        Relation::of::<EntityRef>(),
    ]);
    ValidatedModel::validate(relations).unwrap()
}
