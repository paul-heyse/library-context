//! Optional graph algorithms run against completed native compiler projections and explicit effects.
use crate::catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile, *};
async fn run(
    profile: Profile,
    selected: bool,
    vectors_available: bool,
    extra_layers: bool,
    layer_only: bool,
) {
    let settings = analysis::settings::AnalyticsConfiguration {
        module_prefixes: vec!["api".into()],
        public_roots: vec!["api".into()],
        configured_seeds: vec!["api.alpha".into(), "api.missing".into()],
        depth: 2,
        vertices: 512,
        arcs: 2048,
        witnesses: 3,
        brief_budget: 8,
        communities: selected,
        pagerank: selected,
        fca: selected,
        rca: selected,
        knn: selected && extra_layers && !layer_only,
        type_layer: selected && extra_layers,
        mention_layer: selected && extra_layers,
        knn_layer: selected && extra_layers,
    };
    let provider = ContractEmbedder::new(vectors_available);
    let vectors_requested = settings.knn || settings.knn_layer;
    let fixture = catalog_runtime::compile(
        "analytic_optional",
        profile,
        Frontier::Catalog,
        settings.clone(),
        if vectors_requested {
            Some(&provider)
        } else {
            None
        },
    )
    .await;
    if selected && vectors_available {
        replay_controls(&fixture).await;
    }
    let results: Vec<(i16, bool, i16, i16)> = catalog_runtime::query(
        &fixture,
        "SELECT method,selected,status,stop FROM analytic_technique_results",
    )
    .await;
    assert!(!results.is_empty());
    assert_eq!(results.len() % 5, 0);
    assert!(results.iter().all(|r| r.1
        == (selected
            && (extra_layers && !layer_only
                || r.0 != analysis::AnalysisMethod::Neighbours.code()))));
    if !selected {
        assert!(
            results
                .iter()
                .all(|r| r.2 == analysis::AnalysisStatus::NotRequested.code())
        );
        let ranks: i64 =
            catalog_runtime::one(&fixture, "SELECT count(*) FROM analytic_rank_scores").await;
        assert_eq!(ranks, 0);
    } else {
        let ranks: i64 =
            catalog_runtime::one(&fixture, "SELECT count(*) FROM analytic_rank_scores").await;
        assert!(ranks >= 7);
        let runs: i64 =
            catalog_runtime::one(&fixture, "SELECT count(*) FROM analytic_community_runs").await;
        assert!(runs >= 40);
        let scopes: i64 =
            catalog_runtime::one(&fixture, "SELECT count(*) FROM analytic_concept_scopes").await;
        assert!(scopes > 0);
        let parameter_inputs: Vec<(String, i64)> = catalog_runtime::query(&fixture, "SELECT CAST(a.parameter_name AS VARCHAR), count(DISTINCT i.entity) FROM analytic_attributes a JOIN analytic_incidences i ON i.attribute=a.id WHERE a.kind=0 GROUP BY a.parameter_name").await;
        assert!(
            parameter_inputs
                .iter()
                .any(|(name, n)| name == "flag" && *n == 1),
            "defaulted formal must contribute its source parameter attribute"
        );
        assert!(
            parameter_inputs
                .iter()
                .any(|(name, n)| name == "value" && *n == 8),
            "all eight declared value parameters must contribute independently"
        );
        let typed_parameters: i64 = catalog_runtime::one(&fixture, "SELECT count(DISTINCT i.entity) FROM analytic_incidences i JOIN analytic_attributes a ON a.id=i.attribute JOIN analytic_incidence_sources p ON p.id=i.source WHERE a.kind=1 AND p.kind=1").await;
        assert_eq!(
            typed_parameters, 8,
            "parameter type subjects attach to formals rather than their containers"
        );
        // Independent fixture oracle: only alpha/beta/gamma declare LeftValue. Its actual
        // native final/record traits enrich those parameter ports, with no RightValue negatives.
        let traits: Vec<(String, i16, i16, i16)> = catalog_runtime::query(&fixture, "SELECT DISTINCT m.name, a.typeclasstrait_role, a.typeclasstrait_basis, a.typeclasstrait_trait_kind FROM analytic_attributes a JOIN analytic_incidences i ON i.attribute=a.id JOIN structural_public_candidates p ON p.entity=i.entity JOIN catalog_members m ON m.id=p.member WHERE a.kind=8 ORDER BY 1,2,3,4").await;
        let expected: Vec<_> = ["alpha", "beta", "gamma"]
            .into_iter()
            .map(|name| (name.to_owned(), 0_i16, 1_i16, 0_i16))
            .collect();
        assert_eq!(
            traits, expected,
            "metadata incidence must match the independently enumerated declared-port universe"
        );
        let records: Vec<String> = catalog_runtime::query(&fixture, "SELECT DISTINCT m.name FROM analytic_attributes a JOIN analytic_incidences i ON i.attribute=a.id JOIN structural_public_candidates p ON p.entity=i.entity JOIN catalog_members m ON m.id=p.member JOIN record_options o ON o.id=a.typeclassrecord_options WHERE a.kind=9 AND a.typeclassrecord_role=0 AND o.frozen ORDER BY 1").await;
        assert_eq!(records, ["alpha", "beta", "gamma"]);
        let uncertain: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM analytic_type_metadata_selections WHERE status=1 AND abstract_absence_known IS NULL").await;
        assert!(
            uncertain > 0,
            "builtin class metadata is unavailable rather than a negative trait"
        );
        let known_absence: i64 = catalog_runtime::one(
            &fixture,
            "SELECT count(*) FROM analytic_type_metadata_selections WHERE abstract_absence_known",
        )
        .await;
        assert_eq!(known_absence, 0);
        let captures: Vec<(String, i16, Option<bool>, i16)> = catalog_runtime::query(&fixture, "SELECT DISTINCT m.name, a.capturedependence_origin, a.capturedependence_mutable, a.capturedependence_timing FROM analytic_attributes a JOIN analytic_incidences i ON i.attribute=a.id JOIN structural_public_candidates p ON p.entity=i.entity JOIN catalog_members m ON m.id=p.member WHERE a.kind=15 AND a.capturedependence_name='GLOBAL' ORDER BY 1").await;
        assert_eq!(
            captures,
            [("isolate".to_owned(), 1_i16, None, 0_i16)],
            "global capture dependence has unknown mutation and snapshot time"
        );
        let decorators: Vec<(String, String)> = catalog_runtime::query(&fixture, "SELECT DISTINCT m.name,r.name FROM analytic_attributes a JOIN analytic_incidences i ON i.attribute=a.id JOIN analytic_incidence_sources src ON src.id=i.source JOIN structural_public_candidates p ON p.entity=i.entity JOIN catalog_members m ON m.id=p.member JOIN reference_entity_assessments ra ON ra.id=src.resolveddecorator_assessment JOIN reference_observations r ON r.id=ra.reference WHERE a.kind=7 AND src.kind=7 ORDER BY 1,2").await;
        assert_eq!(
            decorators
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>(),
            [
                ("isolate".to_owned(), "_marker".to_owned()),
                ("isolate".to_owned(), "deprecated".to_owned())
            ]
            .into_iter()
            .collect(),
            "only declared decorator heads supply their resolved identities"
        );
        let selected_decorators: i64 = catalog_runtime::one(
            &fixture,
            "SELECT count(*) FROM analytic_decorator_selections WHERE status=0",
        )
        .await;
        assert!(selected_decorators > 0);
        if settings.type_layer {
            let pairs: Vec<i64> = catalog_runtime::query(
                &fixture,
                "SELECT count(*) FROM analytic_layer_pairs WHERE layer=2 GROUP BY frame",
            )
            .await;
            assert!(!pairs.is_empty());
            assert!(
                pairs.iter().all(|count| *count == 6),
                "two release-class triples supply exactly six pairs; builtin int cannot add a release-class edge"
            );
        }
        let handoffs: i64 = catalog_runtime::one(
            &fixture,
            "SELECT count(*) FROM analytic_incidence_sources WHERE kind=4",
        )
        .await;
        if handoffs == 0 {
            let attempts: Vec<(Option<i16>, i16, i16, i16, i64)> = catalog_runtime::query(&fixture, "SELECT sig.role,b.authority,b.authority_reason,b.outcome,count(*) FROM call_binding_attempts b LEFT JOIN signature_observations sig ON sig.id=b.signature GROUP BY 1,2,3,4 ORDER BY 1,2,3,4").await;
            let effective: Vec<(i16, i16, i16, i16, i16, i16, i64)> = catalog_runtime::query(&fixture, "SELECT identity,identity_reason,signatures,signature_reason,descriptor,descriptor_reason,count(*) FROM effective_callable_assessments GROUP BY 1,2,3,4,5,6 ORDER BY 1,2,3,4,5,6").await;
            let coverage: Vec<(i16, i16, Option<String>, i64)> = catalog_runtime::query(&fixture, "SELECT family,status,diagnostic,count(*) FROM provider_coverage GROUP BY 1,2,3 ORDER BY 1,2,3").await;
            let a0: Vec<(Option<i16>, i64)> = catalog_runtime::query(
                &fixture,
                "SELECT reason,count(*) FROM structural_handoff_assessments GROUP BY 1 ORDER BY 1",
            )
            .await;
            eprintln!(
                "handoff attempts {attempts:?}; effective {effective:?}; coverage {coverage:?}; A0 {a0:?}"
            );
        }
        assert!(handoffs > 0, "RCA must retain exact A0 handoff occurrences");
        if settings.knn && !vectors_available {
            assert!(
                results
                    .iter()
                    .filter(|r| r.0 == analysis::AnalysisMethod::Neighbours.code())
                    .all(|r| matches!(r.2, 1 | 2))
            );
        }
    }
    let selectors: i64 =
        catalog_runtime::one(&fixture, "SELECT count(*) FROM analytic_public_selectors").await;
    assert!(
        selectors > 2,
        "configured two seed paths cannot shrink all public candidates"
    );
    let public: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(*) FROM structural_public_candidates WHERE in_subsystem",
    )
    .await;
    assert_eq!(selectors, public);
    if selected && !extra_layers {
        let isolated: i64 = catalog_runtime::one(
            &fixture,
            "SELECT count(*) FROM analytic_universe_members WHERE excluded_isolate",
        )
        .await;
        assert!(isolated >= 1);
        let co_use: i64 = catalog_runtime::one(
            &fixture,
            "SELECT count(*) FROM analytic_layer_pairs WHERE layer=1 AND count=1",
        )
        .await;
        assert!(
            co_use >= 1,
            "distinct call sites in one official scope must co-occur"
        );
    }
    if layer_only {
        let ordinary:i64=catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM analytic_neighbours)+(SELECT count(*) FROM analytic_document_neighbours)+(SELECT count(*) FROM analytic_community_labels)").await;
        assert_eq!(ordinary, 0);
        let layer: i64 =
            catalog_runtime::one(&fixture, "SELECT count(*) FROM analytic_layer_neighbours").await;
        assert!(layer > 0);
        assert!(
            results
                .iter()
                .filter(|r| r.0 == analysis::AnalysisMethod::Neighbours.code())
                .all(|r| !r.1 && r.2 == analysis::AnalysisStatus::NotRequested.code())
        );
    }
}
use cpg_core::embedding_service::{EmbedFuture, Embedder};
struct ContractEmbedder {
    inner: cpg_core::embedding_service::FakeEmbedder,
    specification: embedding::Spec,
    available: bool,
}
impl ContractEmbedder {
    fn new(available: bool) -> Self {
        let inner = cpg_core::embedding_service::FakeEmbedder::new();
        let mut specification = inner.spec().clone();
        specification.model = "lctx-analytic-contract-unit-vectors".into();
        specification.revision = "unit-e0-v1".into();
        Self {
            inner,
            specification,
            available,
        }
    }
}
impl Embedder for ContractEmbedder {
    fn spec(&self) -> &embedding::Spec {
        &self.specification
    }
    fn endpoint(&self) -> &str {
        self.inner.endpoint()
    }
    fn document_tokenizer(
        &self,
    ) -> Option<std::sync::Arc<dyn lctx_model::domain::retrieval::partition::Tokenizer>> {
        self.inner.document_tokenizer()
    }
    fn count_tokens<'a>(&'a self, text: &'a str) -> EmbedFuture<'a, usize> {
        if self.available {
            self.inner.count_tokens(text)
        } else {
            Box::pin(async {
                Err(cpg_core::CoreError::EmbeddingService(
                    "contract unavailable".into(),
                ))
            })
        }
    }
    fn embed<'a>(&'a self, text: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>> {
        Box::pin(async move {
            if !self.available {
                return Err(cpg_core::CoreError::EmbeddingService(
                    "contract unavailable".into(),
                ));
            }
            Ok(text
                .iter()
                .map(|_| {
                    let mut vector = vec![0.0; self.specification.dimensions as usize];
                    vector[0] = 1.0;
                    vector
                })
                .collect())
        })
    }
}
#[tokio::test]
async fn default_off_is_explicit_in_both_profiles() {
    for p in Profile::ALL {
        run(p, false, false, false, false).await;
    }
}
#[tokio::test]
async fn selected_algorithms_publish_actual_nominal_results() {
    run(Profile::Catalog, true, true, true, false).await;
}
#[tokio::test]
async fn selected_vector_failures_remain_visible() {
    run(Profile::Behavioral, true, false, true, false).await;
}

#[tokio::test]
async fn baseline_community_scope_preserves_excluded_isolates_and_official_scope_pairs() {
    run(Profile::Catalog, true, true, false, false).await;
}

#[tokio::test]
async fn nearest_community_layer_preserves_unrequested_public_neighbours() {
    run(Profile::Catalog, true, true, true, true).await;
}

async fn replay_controls(fixture: &catalog_runtime::Fixture) {
    let workspace = &fixture.workspace;
    let b = workspace.budget();
    let completed = workspace.completed_relations().unwrap();
    let access = workspace
        .inputs(
            "analytic-contract-control",
            Profile::Catalog,
            completed.iter().map(|r| r.name()),
        )
        .unwrap();
    let checked = workspace.admit_semantics(Profile::Catalog).await.unwrap();
    let graphs = cpg_core::analysis_graphs::PreparedGraphs::load(
        &access,
        workspace,
        &checked,
        workspace.model(),
        &[projection::ProjectionName::CallableInvocation]
            .into_iter()
            .collect(),
    )
    .await
    .unwrap();
    let mut d = analytics::build::Data::new(b);
    let mut seen = std::collections::BTreeSet::new();
    for input in analytics::build::Data::validation_inputs() {
        if !seen.insert((input.name(), input.prefix())) {
            continue;
        }
        if let Ok(relation) = workspace.input_relation(&input) {
            for batch in relation.batches().unwrap() {
                d.visit_input(&input, &batch.unwrap()).unwrap();
            }
        }
    }
    let mut c = analytics::frames::Context::new(b);
    let sources =
        analysis::sources::CapturedSources::capture(access.profile(), access.snapshots(), b)
            .unwrap();
    let mut out = analytics::Output::new(b);
    for sf in d.structural.frames.iter() {
        let parent = d.structural_invocations.get(sf.invocation).unwrap();
        for method in analytics::build::METHODS {
            let def = analytics::build::definition(d.configuration().unwrap(), method)
                .unwrap()
                .1;
            let parents = analytics::frames::parents(&d, sf, method).unwrap();
            let (inv, inputs, _, _) = analysis::analytic::Invocation::admitted(
                parent.input,
                parent.context,
                def.id(),
                None,
                parents.iter().map(Record::id),
                &sources,
                [analysis::ProjectionDefinition::builtin(
                    projection::ProjectionName::CallableInvocation,
                )
                .id()],
                b,
            )
            .unwrap();
            for p in parents {
                c.sources.insert(p).unwrap();
            }
            for i in inputs {
                c.inputs.insert(i).unwrap();
            }
            c.invocations.insert(inv).unwrap();
        }
        let f = analytics::AnalyticFrame {
            structural: sf.id(),
            configuration: d.configuration().unwrap().id(),
        };
        let g = graphs
            .graph(
                &access,
                workspace,
                projection::normalization::ProjectionKey {
                    input: parent.input,
                    context: parent.context,
                    name: projection::ProjectionName::CallableInvocation,
                },
            )
            .unwrap();
        out.extend(analytics::build::produce(&d, &f, &c.invocations, g, b).unwrap())
            .unwrap();
    }
    for r in out.results.iter() {
        c.outcomes.insert(analytics::frames::outcome(r)).unwrap();
    }
    analytics::frames::verify(&d, &c, &out, b).unwrap();
    assert!(
        out.attributes.iter().all(|a| !matches!(
            a,
            analytics::Attribute::NativeSignature { .. }
                | analytics::Attribute::NativeParameterType { .. }
                | analytics::Attribute::NativeReturnType { .. }
                | analytics::Attribute::NativeDeprecation { .. }
        )),
        "default source/declared policy must not silently include native variants"
    );
    // Exercise the public optional projection against actual completed workspace/native predecessors.
    // The eight explicit fixture callables provide the independent entity oracle below.
    let policy = analytics::policy::AttributePolicy {
        signature_roles: analytics::policy::SignatureRoles::AllAvailable,
        receiver: analytics::policy::ReceiverPolicy::ExcludeSourceBoundPreserveNative,
    };
    let mut native_context = analytics::frames::Context::new(b);
    let mut native_output = analytics::Output::new(b);
    for sf in d.structural.frames.iter() {
        let parent = d.structural_invocations.get(sf.invocation).unwrap();
        for method in analytics::build::METHODS {
            let (parameters, definition) = analytics::build::definition_with_attribute_policy(
                d.configuration().unwrap(),
                method,
                policy,
            )
            .unwrap();
            assert_ne!(
                definition.id(),
                analytics::build::definition(d.configuration().unwrap(), method)
                    .unwrap()
                    .1
                    .id()
            );
            let (_, include_receiver) = analytics::build::definition_with_attribute_policy(
                d.configuration().unwrap(),
                method,
                analytics::policy::AttributePolicy {
                    receiver: analytics::policy::ReceiverPolicy::IncludeSourceBoundPreserveNative,
                    ..policy
                },
            )
            .unwrap();
            assert_ne!(
                definition.id(),
                include_receiver.id(),
                "receiver policy participates in persisted definition identity"
            );
            d.parameters.insert(parameters).unwrap();
            d.definitions.insert(definition.clone()).unwrap();
            let parents = analytics::frames::parents(&d, sf, method).unwrap();
            let (invocation, inputs, _, _) = analysis::analytic::Invocation::admitted(
                parent.input,
                parent.context,
                definition.id(),
                None,
                parents.iter().map(Record::id),
                &sources,
                [analysis::ProjectionDefinition::builtin(
                    projection::ProjectionName::CallableInvocation,
                )
                .id()],
                b,
            )
            .unwrap();
            for row in parents {
                native_context.sources.insert(row).unwrap();
            }
            for row in inputs {
                native_context.inputs.insert(row).unwrap();
            }
            native_context.invocations.insert(invocation).unwrap();
        }
        let frame = analytics::AnalyticFrame {
            structural: sf.id(),
            configuration: d.configuration().unwrap().id(),
        };
        let graph = graphs
            .graph(
                &access,
                workspace,
                projection::normalization::ProjectionKey {
                    input: parent.input,
                    context: parent.context,
                    name: projection::ProjectionName::CallableInvocation,
                },
            )
            .unwrap();
        native_output
            .extend(
                analytics::build::produce_with_policy(
                    &d,
                    &frame,
                    &native_context.invocations,
                    graph,
                    b,
                    policy,
                )
                .unwrap(),
            )
            .unwrap();
    }
    for result in native_output.results.iter() {
        native_context
            .outcomes
            .insert(analytics::frames::outcome(result))
            .unwrap();
    }
    analytics::frames::verify_with_policy(&d, &native_context, &native_output, b, policy).unwrap();
    assert!(
        analytics::frames::verify(&d, &native_context, &native_output, b).is_err(),
        "optional output cannot replay under the default policy"
    );
    let native_names: std::collections::BTreeSet<_> = native_output
        .incidences
        .iter()
        .filter(|i| {
            matches!(
                native_output.attributes.get(i.attribute),
                Some(analytics::Attribute::NativeReturnType {
                    role: calls::SignatureRole::EffectiveTyped,
                    ..
                })
            )
        })
        .flat_map(|i| {
            d.structural
                .public
                .iter()
                .filter(move |p| p.entity == i.entity)
                .map(|p| d.members.get(p.member).unwrap().name.clone())
        })
        .collect();
    assert_eq!(
        native_names,
        [
            "alpha", "beta", "delta", "epsilon", "gamma", "identity", "isolate", "zeta"
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        "actual effective native return ports must cover exactly the hand-enumerated public callable set"
    );
    let native_members = &d.members;
    let deprecated: std::collections::BTreeSet<_> = native_output
        .incidences
        .iter()
        .filter_map(|i| match native_output.attributes.get(i.attribute) {
            Some(analytics::Attribute::NativeDeprecation {
                role: calls::SignatureRole::EffectiveTyped,
                basis: class_metadata::MetadataBasis::NativeEffective,
                availability: types::CallableDeprecation::Deprecated,
                message,
            }) => Some((i.entity, message.clone())),
            _ => None,
        })
        .flat_map(|(entity, message)| {
            d.structural
                .public
                .iter()
                .filter(move |p| p.entity == entity)
                .map(move |p| {
                    (
                        native_members.get(p.member).unwrap().name.clone(),
                        message.clone(),
                    )
                })
        })
        .collect();
    assert_eq!(
        deprecated,
        [(
            "isolate".to_owned(),
            Some("  use the next isolate  ".to_owned())
        )]
        .into_iter()
        .collect(),
        "deprecation must use the live native payload without trimming or spelling inference"
    );
    // The same production replay operates on native predecessors plus serialized/hydrated graph.
    let target = out.ranks.iter().next().unwrap().target;
    for corruption in 0..if out.neighbours.is_empty() && out.layer_neighbours.is_empty() {
        6
    } else {
        7
    } {
        let mut forged = analytics::Output::new(b);
        macro_rules! copy{($($f:ident:$t:ty,)*)=>{$(for r in out.$f.iter(){forged.$f.insert(r.clone()).unwrap();})*};}
        lctx_model::analytic_outputs!(copy);
        analytics::frames::verify(&d, &c, &forged, b).unwrap();
        match corruption {
            0 => {
                forged.ranks = normalized::Rows::new(b);
                for r in out.ranks.iter() {
                    let mut r = r.clone();
                    if r.target == target {
                        r.score = FiniteF64::new(r.score.get() + 0.125).unwrap();
                    }
                    forged.ranks.insert(r).unwrap();
                }
            }
            1 => {
                forged.universe = normalized::Rows::new(b);
                for r in out.universe.iter() {
                    let mut r = r.clone();
                    if r.entity == target {
                        r.graph = !r.graph;
                    }
                    forged.universe.insert(r).unwrap();
                }
            }
            2 => {
                forged.selectors = normalized::Rows::new(b);
                let removed = out.selectors.iter().next().unwrap().id();
                for r in out.selectors.iter().filter(|r| r.id() != removed) {
                    forged.selectors.insert(r.clone()).unwrap();
                }
                forged.ranks = normalized::Rows::new(b);
                for r in out.ranks.iter().filter(|r| r.target != target) {
                    forged.ranks.insert(r.clone()).unwrap();
                }
            }
            3 => {
                forged.results = normalized::Rows::new(b);
                for r in out.results.iter() {
                    let mut r = r.clone();
                    if r.method == analysis::AnalysisMethod::PageRank {
                        r.selected = false;
                        r.status = analysis::AnalysisStatus::NotRequested;
                        r.stop = analytics::Stop::NotRequested;
                    }
                    forged.results.insert(r).unwrap();
                }
            }
            4 => {
                forged.partitions = normalized::Rows::new(b);
                let removed = out.partitions.iter().next().unwrap().id();
                for r in out.partitions.iter().filter(|r| r.id() != removed) {
                    forged.partitions.insert(r.clone()).unwrap();
                }
            }
            5 => {
                forged.objects = normalized::Rows::new(b);
                let removed = out.objects.iter().next().unwrap().id();
                for r in out.objects.iter().filter(|r| r.id() != removed) {
                    forged.objects.insert(r.clone()).unwrap();
                }
            }
            _ if !out.neighbours.is_empty() => {
                forged.neighbours = normalized::Rows::new(b);
                let replaced = out.neighbours.iter().next().unwrap().id();
                for r in out.neighbours.iter() {
                    let mut r = r.clone();
                    if r.id() == replaced {
                        assert_ne!(r.query_use, r.target_use);
                        r.query_use = r.target_use;
                    }
                    forged.neighbours.insert(r).unwrap();
                }
            }
            _ => {
                forged.layer_neighbours = normalized::Rows::new(b);
                let replaced = out.layer_neighbours.iter().next().unwrap().id();
                for r in out.layer_neighbours.iter() {
                    let mut r = r.clone();
                    if r.id() == replaced {
                        assert_ne!(r.query_use, r.target_use);
                        r.query_use = r.target_use;
                    }
                    forged.layer_neighbours.insert(r).unwrap();
                }
            }
        }
        assert!(
            analytics::frames::verify(&d, &c, &forged, b).is_err(),
            "forgery {corruption} accepted"
        );
    }
}

