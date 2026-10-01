//! Independent controls over actual extracted and published catalog records.
use arrow_array::RecordBatch;
use cpg_core::embedding_service::{EmbedFuture, Embedder, FakeEmbedder, Spec};
use cpg_schema::{
    Id, Table, catalog::*, selection::catalog::*, serving_projection::projected_rows, wire::*,
};
use std::{collections::BTreeMap, path::Path};

pub async fn check(
    ctx: &datafusion::prelude::SessionContext,
    snapshot: Id,
    generation: &cpg_core::bundle::Generation,
    root: &Path,
) {
    domain_scan_order(ctx, snapshot).await;
    let tables: BTreeMap<String, Vec<RecordBatch>> = cpg_schema::bundle::files(0)
        .iter()
        .map(|f| {
            let reader = arrow_ipc::reader::FileReader::try_new(
                std::fs::File::open(generation.dir.join(format!("{}.arrow", f.name))).unwrap(),
                None,
            )
            .unwrap();
            (
                f.name.into(),
                reader.collect::<Result<Vec<_>, _>>().unwrap(),
            )
        })
        .collect();
    let mut inputs = Inputs::default();
    macro_rules! load {
        ($field:ident,$table:ty) => {
            inputs.$field = projected_rows::<$table>(&tables).unwrap();
        };
    }
    load!(members, CatalogMembers);
    inputs.domains = projected_rows::<CatalogSelectionDomains>(&tables)
        .unwrap()
        .into_iter()
        .map(DomainInput::try_from)
        .collect::<Result<_, _>>()
        .unwrap();
    load!(bindings, CatalogBindings);
    load!(signatures, CatalogSignatures);
    load!(parameters, CatalogParameters);
    load!(types, CatalogTypes);
    load!(type_args, CatalogTypeArgs);
    load!(type_observations, CatalogTypeObservations);
    load!(surfaces, CatalogSurfaces);
    load!(configurations, CatalogConfigurations);
    load!(field_links, CatalogFieldLinks);
    let oversized = Inputs {
        members: vec![inputs.members[0].clone(); 200_001],
        ..Inputs::default()
    };
    assert!(
        PreparedCatalog::new(&oversized)
            .err()
            .unwrap()
            .to_string()
            .contains("resource_refused")
    );
    let mut projected = inputs.clone();
    for row in &mut projected.domains {
        let mut detail: DomainDetail = serde_json::from_str(&row.detail).unwrap();
        detail.domains.clear();
        row.detail = serde_json::to_string(&detail).unwrap();
    }
    let unloaded = PreparedCatalog::new(&projected).unwrap();
    assert_eq!(
        unloaded.classify(&Selection::default()).unwrap().len(),
        inputs.members.len()
    );
    assert!(
        unloaded
            .classify(
                &serde_json::from_value(
                    serde_json::json!({"requirements":[{"predicate":"member_kind","kind":"class"}]})
                )
                .unwrap()
            )
            .unwrap_err()
            .to_string()
            .contains("domain was not loaded")
    );
    let prepared = PreparedCatalog::new(&inputs).unwrap();
    // Projection changes transport only: states and original canonical domain witnesses agree.
    for requirement in [
        serde_json::json!({"predicate":"declares_parameter","name":"flag"}),
        serde_json::json!({"predicate":"parameter_kind","name":"flag","kind":"positional"}),
        serde_json::json!({"predicate":"parameter_required","name":"flag","required":false}),
        serde_json::json!({"predicate":"parameter_default_state","name":"flag","state":"literal"}),
        serde_json::json!({"predicate":"parameter_default","name":"flag","value":{"kind":"bool","value":false}}),
        serde_json::json!({"predicate":"facet_membership","facet":"async","value":"true"}),
        serde_json::json!({"predicate":"deployment_declaration","field":"launch","name":"python -m catalogpkg"}),
        serde_json::json!({"predicate":"deployment_declaration","field":"configuration","name":"transport"}),
    ] {
        let request: Selection =
            serde_json::from_value(serde_json::json!({"requirements":[requirement]})).unwrap();
        let required = request.requirements[0].input_domain();
        let mut narrow = inputs.clone();
        for row in &mut narrow.domains {
            let mut detail: DomainDetail = serde_json::from_str(&row.detail).unwrap();
            detail.domains.retain(|d| Some(d.domain) == required);
            row.detail = serde_json::to_string(&detail).unwrap();
        }
        // The schema's declared dependencies must preserve answers and all witness identities.
        let dependencies = request.requirements[0].dependencies();
        macro_rules! omit {
            ($field:ident, $name:literal) => {
                if !dependencies.contains(&$name) {
                    narrow.$field.clear();
                }
            };
        }
        omit!(bindings, "catalog_bindings");
        omit!(signatures, "catalog_signatures");
        omit!(parameters, "catalog_parameters");
        omit!(type_observations, "catalog_type_observations");
        omit!(types, "catalog_types");
        omit!(type_args, "catalog_type_args");
        omit!(surfaces, "catalog_surfaces");
        omit!(configurations, "catalog_configurations");
        omit!(field_links, "catalog_field_links");
        let actual = PreparedCatalog::new(&narrow)
            .unwrap()
            .classify(&request)
            .unwrap();
        let expected = prepared.classify(&request).unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }

    for requirement in [
        serde_json::json!({"predicate":"parameter_type","name":"flag","type":{"operator":"canonical_term","term":Id([253;16])}}),
        serde_json::json!({"predicate":"configuration_relationship","name":"left","kind":"exact_storage","target":{"kind":"parameter","signature":Id([253;16]),"ordinal":0}}),
        serde_json::json!({"predicate":"relationship","role":"declares","target":{"kind":"member","member":Id([253;16])},"fidelity":"source_fact"}),
    ] {
        let request =
            serde_json::from_value(serde_json::json!({"requirements":[requirement]})).unwrap();
        assert!(
            prepared
                .classify(&request)
                .unwrap_err()
                .to_string()
                .contains("operand is absent")
        );
    }
    let classify = |path: &str, terms: serde_json::Value| {
        let selection: Selection =
            serde_json::from_value(serde_json::json!({"requirements":terms})).unwrap();
        prepared
            .classify(&selection)
            .unwrap()
            .into_iter()
            .find(|c| c.access_path == path)
            .unwrap()
    };
    assert_eq!(
        classify(
            "catalogpkg.ordinary",
            serde_json::json!([{"predicate":"declares_parameter","name":"flag"}])
        )
        .outcome,
        SelectionOutcome::Supported
    );
    assert_eq!(
        classify(
            "catalogpkg.ordinary",
            serde_json::json!([{"predicate":"declares_parameter","name":"absent"}])
        )
        .outcome,
        SelectionOutcome::Contradicted
    );
    assert_eq!(classify("catalogpkg.ordinary",serde_json::json!([{"predicate":"parameter_default","name":"flag","value":{"kind":"bool","value":false}}])).outcome,SelectionOutcome::Supported);
    assert_ne!(classify("catalogpkg.ordinary",serde_json::json!([{"predicate":"parameter_default","name":"flag","value":{"kind":"int","value":0}}])).outcome,SelectionOutcome::Supported);
    assert_eq!(classify("catalogpkg.wrapper",serde_json::json!([{"predicate":"parameter_type","name":"function","type":{"operator":"category","category":"class_instance"}}])).outcome,SelectionOutcome::Unresolved);
    assert_eq!(classify("catalogpkg.Options.read_left",serde_json::json!([{"predicate":"declares_configuration_field","name":"left"},{"predicate":"configuration_scope","scope":"object"},{"predicate":"configuration_owner","path":"catalogpkg.Options"}])).outcome,SelectionOutcome::Supported);
    assert_ne!(
        classify(
            "catalogpkg.Options.read_left",
            serde_json::json!([{"predicate":"declares_configuration_field","name":"right"}])
        )
        .outcome,
        SelectionOutcome::Supported
    );
    for quantifier in ["any_applicable", "all_applicable"] {
        assert_eq!(classify("catalogpkg.wrapped",serde_json::json!([{"predicate":"public_path","path":"not.this.path","quantifier":quantifier}])).outcome,SelectionOutcome::Contradicted);
        assert_eq!(classify("catalogpkg.wrapped",serde_json::json!([{"predicate":"member_kind","kind":"class","quantifier":quantifier}])).outcome,SelectionOutcome::Contradicted);
    }
    assert_ne!(classify("catalogpkg.RetainedOptions",serde_json::json!([{"predicate":"configuration_owner","path":"catalogpkg.ReboundOptions"}])).outcome,SelectionOutcome::Supported);
    assert_eq!(classify("catalogpkg.Options.read_left",serde_json::json!([{"predicate":"member_kind","kind":"method"},{"predicate":"invocation_form","form":"method"}])).outcome,SelectionOutcome::Supported);
    let member = inputs
        .members
        .iter()
        .find(|m| m.access_path == "catalogpkg.Options.read_left")
        .unwrap();
    let link = inputs
        .field_links
        .iter()
        .find(|l| l.kind == "exact_reader" && l.reader_node_id == member.operation_node_id)
        .unwrap();
    assert_eq!(classify("catalogpkg.Options",serde_json::json!([{"predicate":"configuration_relationship","name":"left","kind":"exact_reader","target":{"kind":"node","node":link.reader_node_id.unwrap()}}])).outcome,SelectionOutcome::Supported);
    if let Some(formal) = link.formal_node_id {
        assert_ne!(classify("catalogpkg.Options",serde_json::json!([{"predicate":"configuration_relationship","name":"left","kind":"exact_reader","target":{"kind":"node","node":formal}}])).outcome,SelectionOutcome::Supported);
    }
    let options = inputs
        .members
        .iter()
        .find(|m| m.access_path == "catalogpkg.Options")
        .unwrap();
    let declarations = inputs
        .bindings
        .iter()
        .filter(|b| b.member_id == options.member_id)
        .filter_map(|b| b.declaration_node_id)
        .collect::<std::collections::BTreeSet<_>>();
    let mut shadowed = inputs.clone();
    let rebound = inputs
        .members
        .iter()
        .find(|m| m.access_path == "catalogpkg.ReboundOptions")
        .unwrap();
    let mut old = inputs
        .bindings
        .iter()
        .find(|b| b.member_id == options.member_id && b.declaration_node_id.is_some())
        .unwrap()
        .clone();
    old.member_id = rebound.member_id;
    old.role = "shadowed_source".into();
    old.binding_id = Id([239; 16]);
    shadowed.bindings.push(old);
    let request=serde_json::from_value(serde_json::json!({"requirements":[{"predicate":"configuration_owner","path":"catalogpkg.ReboundOptions"}]})).unwrap();
    let rows = PreparedCatalog::new(&shadowed)
        .unwrap()
        .classify(&request)
        .unwrap();
    assert_ne!(
        rows.iter()
            .find(|r| r.access_path == "catalogpkg.Options")
            .unwrap()
            .outcome,
        SelectionOutcome::Supported,
        "shadowed class is not the owner at its rebound public path"
    );
    let generated = inputs
        .field_links
        .iter()
        .find(|l| {
            declarations.contains(&l.class_node_id)
                && l.formal_node_id.is_none()
                && l.kind == "exact_storage"
        })
        .unwrap();
    let field = inputs
        .configurations
        .iter()
        .find(|f| f.field_id == generated.field_id)
        .unwrap();
    assert_eq!(classify("catalogpkg.Options",serde_json::json!([{"predicate":"configuration_relationship","name":field.name,"kind":"exact_storage","target":{"kind":"parameter","signature":generated.signature_id,"ordinal":generated.ordinal}}])).outcome,SelectionOutcome::Supported);
    assert!(
        inputs
            .types
            .iter()
            .any(|t| t.literal_json.as_deref() == Some("\"http\""))
    );
    assert!(
        inputs
            .types
            .iter()
            .any(|t| t.literal_json.as_deref() == Some("true"))
    );
    assert!(
        inputs
            .types
            .iter()
            .any(|t| t.literal_json.as_deref() == Some("7"))
    );
    let all = prepared.classify(&Selection::default()).unwrap();
    assert_eq!(all.len(), inputs.members.len());
    assert!(all.iter().all(|m| m.outcome == SelectionOutcome::Supported));
    assert_ne!(
        all.iter()
            .find(|m| m.access_path == "catalogpkg.ordinary")
            .unwrap()
            .member_id,
        all.iter()
            .find(|m| m.access_path == "catalogpkg.alias")
            .unwrap()
            .member_id
    );
    let units =
        cpg_core::retrieval::derive(&cpg_core::retrieval::load(ctx).await.unwrap()).unwrap();
    assert!(units.iter().any(
        |u| u.subjects.contains(&cpg_schema::retrieval::Subject::Member(
            PublicMemberId::from_storage(member.member_id)
        ))
    ));
    assert_eq!(
        serde_json::to_value(&units).unwrap(),
        serde_json::to_value(
            cpg_core::retrieval::derive(&cpg_core::retrieval::load(ctx).await.unwrap()).unwrap()
        )
        .unwrap()
    );
    let mut render_inputs = cpg_core::retrieval::load(ctx).await.unwrap();
    if let Some(mut alternate) = render_inputs.fields.first().cloned() {
        alternate.source_fact_id = Id([198; 16]);
        alternate.default_text = Some("alternative declaration".into());
        render_inputs.fields.push(alternate);
    }
    let expected = cpg_core::retrieval::derive(&render_inputs).unwrap();
    render_inputs.members.reverse();
    render_inputs.bindings.reverse();
    render_inputs.signatures.reverse();
    render_inputs.constructors.reverse();
    render_inputs.parameters.reverse();
    render_inputs.fields.reverse();
    render_inputs.evidence.reverse();
    render_inputs.artifacts.reverse();
    render_inputs.spans.reverse();
    render_inputs.scenarios.reverse();
    render_inputs.deployments.reverse();
    render_inputs.associations.reverse();
    assert_eq!(
        expected,
        cpg_core::retrieval::derive(&render_inputs).unwrap(),
        "rendering ignores canonical row order including alternative declarations"
    );
    for forge_subject in [true, false] {
        let mut forged = tables.clone();
        let batches = forged.get_mut("retrieval_units").unwrap();
        let batch = batches.iter_mut().find(|b| b.num_rows() > 0).unwrap();
        let details = batch
            .column_by_name("detail")
            .unwrap()
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .unwrap();
        let mut values = details
            .iter()
            .map(|s| s.unwrap().to_owned())
            .collect::<Vec<_>>();
        let mut unit: cpg_schema::retrieval::Unit = serde_json::from_str(&values[0]).unwrap();
        if forge_subject {
            let other = inputs
                .members
                .iter()
                .find(|m| {
                    !unit
                        .subjects
                        .contains(&cpg_schema::retrieval::Subject::Member(
                            PublicMemberId::from_storage(m.member_id),
                        ))
                })
                .unwrap();
            unit.subjects = vec![cpg_schema::retrieval::Subject::Member(
                PublicMemberId::from_storage(other.member_id),
            )];
        } else {
            unit.text.push_str(" forged");
        }
        values[0] = serde_json::to_string(&unit).unwrap();
        let mut columns = batch.columns().to_vec();
        columns[2] = std::sync::Arc::new(arrow_array::StringArray::from(values));
        *batch = RecordBatch::try_new(batch.schema(), columns).unwrap();
        assert!(
            cpg_schema::retrieval::validate_projection(&forged)
                .unwrap_err()
                .to_string()
                .contains("canonical catalog rendering")
        );
    }
    // Two independently specified retrieval artifacts reuse exactly the same canonical snapshot.
    let a = SpecEmbedder::new("first");
    let b = SpecEmbedder::new("second");
    let aa = cpg_core::retrieval::prepare(ctx, snapshot, Some(&a), None)
        .await
        .unwrap();
    let ga = cpg_core::bundle::build_with_retrieval(ctx, &root.join("two-specs"), Some(&aa))
        .await
        .unwrap();
    let ab = cpg_core::retrieval::prepare(ctx, snapshot, Some(&b), None)
        .await
        .unwrap();
    let gb = cpg_core::bundle::build_with_retrieval(ctx, &root.join("two-specs"), Some(&ab))
        .await
        .unwrap();
    assert_ne!(ga.key, gb.key);
    assert_eq!(ga.manifest["snapshot_id"], gb.manifest["snapshot_id"]);
    for name in [
        "catalog_members",
        "catalog_selection_domains",
        "catalog_parameters",
    ] {
        assert_eq!(ga.manifest["files"][name], gb.manifest["files"][name]);
    }
    cpg_core::bundle::verify(&ga.dir).unwrap();
    cpg_core::bundle::verify(&gb.dir).unwrap();
    let replay_space = tempfile::tempdir_in(root).unwrap();
    let replay_root = replay_space.path().join("replay");
    let saved = cpg_core::retrieval::save(&aa, &replay_root, snapshot).unwrap();
    let restored = cpg_core::retrieval::restore(&replay_root, snapshot)
        .unwrap()
        .unwrap();
    let cold = cpg_core::bundle::build_with_retrieval(ctx, &root.join("cold"), Some(&restored))
        .await
        .unwrap();
    assert_eq!(ga.key, cold.key);
    // Same snapshot and specification can select a newly admitted materialization.
    let unavailable =
        cpg_core::retrieval::prepare(ctx, snapshot, Some(&TokenUnavailable(&a)), None)
            .await
            .unwrap();
    let other = cpg_core::retrieval::save(&unavailable, &replay_root, snapshot).unwrap();
    assert_ne!(other, saved);
    let restored = cpg_core::retrieval::restore(&replay_root, snapshot)
        .unwrap()
        .unwrap();
    let unavailable_generation =
        cpg_core::bundle::build_with_retrieval(ctx, &root.join("unavailable"), Some(&restored))
            .await
            .unwrap();
    assert_ne!(unavailable_generation.key, ga.key);
    let mut foreign = aa.batches.clone();
    let receipt = foreign.get_mut("retrieval_receipt").unwrap();
    let text = receipt
        .column(0)
        .as_any()
        .downcast_ref::<arrow_array::StringArray>()
        .unwrap()
        .value(0);
    let mut detail: serde_json::Value = serde_json::from_str(text).unwrap();
    detail["snapshot_id"] = Id([237; 16]).hex().into();
    *receipt = RecordBatch::try_new(
        receipt.schema(),
        vec![std::sync::Arc::new(arrow_array::StringArray::from(vec![
            detail.to_string(),
        ]))],
    )
    .unwrap();
    let foreign = cpg_core::retrieval::Artifacts {
        batches: foreign,
        spec: aa.spec.clone(),
    };
    assert!(
        cpg_core::bundle::build_with_retrieval(ctx, &root.join("foreign-snapshot"), Some(&foreign))
            .await
            .err()
            .unwrap()
            .to_string()
            .contains("snapshot identity")
    );
    cpg_core::retrieval::save(&aa, &replay_root, snapshot).unwrap();
    std::fs::write(saved.join("retrieval_units.arrow"), b"corrupt").unwrap();
    assert!(cpg_core::retrieval::restore(&replay_root, snapshot).is_err());
    let mut forged = tables.clone();
    let mut rows = projected_rows::<CatalogSelectionDomains>(&tables).unwrap();
    rows[0].domain_id = Id([199; 16]);
    replace::<CatalogSelectionDomains>(&mut forged, &rows);
    assert!(cpg_schema::selection::catalog::validate_projection(&forged).is_err());
    // Missing type observations are unknown even with complete declaration domains.
    let mut missing = inputs;
    missing.type_observations.clear();
    let selected=PreparedCatalog::new(&missing).unwrap().classify(&serde_json::from_value(serde_json::json!({"requirements":[{"predicate":"parameter_type","name":"flag","type":{"operator":"category","category":"class_instance"}}]})).unwrap()).unwrap();
    assert_eq!(
        selected
            .iter()
            .find(|c| c.access_path == "catalogpkg.ordinary")
            .unwrap()
            .outcome,
        SelectionOutcome::Unresolved
    );
}
fn replace<T: Table>(tables: &mut BTreeMap<String, Vec<RecordBatch>>, rows: &[T::Row]) {
    let batch = T::to_batch(rows).unwrap();
    let indices = (1..batch.num_columns()).collect::<Vec<_>>();
    let projected = batch.project(&indices).unwrap();
    let schema = tables[T::NAME][0].schema();
    tables.insert(
        T::NAME.into(),
        vec![RecordBatch::try_new(schema, projected.columns().to_vec()).unwrap()],
    );
}
struct SpecEmbedder {
    spec: Spec,
    fake: FakeEmbedder,
}
impl SpecEmbedder {
    fn new(task: &str) -> Self {
        let fake = FakeEmbedder::new();
        let mut spec = fake.spec().clone();
        spec.query_task = task.into();
        Self { spec, fake }
    }
}
impl Embedder for SpecEmbedder {
    fn endpoint(&self) -> &str {"fixture://spec"}
    fn spec(&self) -> &Spec {
        &self.spec
    }
    fn count_tokens<'a>(&'a self, text: &'a str) -> EmbedFuture<'a, usize> {
        self.fake.count_tokens(text)
    }
    fn embed<'a>(&'a self, texts: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>> {
        self.fake.embed(texts)
    }
}

struct TokenUnavailable<'a>(&'a SpecEmbedder);
impl Embedder for TokenUnavailable<'_> {
    fn endpoint(&self) -> &str {"fixture://unavailable"}
    fn spec(&self) -> &Spec {
        self.0.spec()
    }
    fn count_tokens<'a>(&'a self, _: &'a str) -> EmbedFuture<'a, usize> {
        Box::pin(async {
            Err(cpg_core::CoreError::EmbeddingService(
                "tokenizer unavailable control".into(),
            ))
        })
    }
    fn embed<'a>(&'a self, texts: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>> {
        self.0.embed(texts)
    }
}

/// Delta/DataFusion scans have no implicit ordering. A domain digest must bind the same
/// contexts and complete evidence set regardless of partition or batch arrival order.
async fn domain_scan_order(ctx: &datafusion::prelude::SessionContext, snapshot: Id) {
    let mut facts = cpg_core::catalog::load_facts(ctx).await.unwrap();
    let mut evidence = cpg_core::evidence::load(ctx).await.unwrap();
    let public = cpg_core::sql::fetch(
        ctx,
        &cpg_schema::public::public_paths(),
        cpg_core::sql::Params::new().texts("roots", ["catalogpkg"]),
    )
    .await
    .unwrap();
    let mut catalog =
        cpg_core::catalog::derive_contracts(&facts, snapshot, &["catalogpkg".into()], &public)
            .unwrap();
    catalog.contextual = cpg_core::evidence::PreparedEvidence::new(&facts, &evidence)
        .derive(snapshot, &catalog)
        .unwrap();
    let expected =
        cpg_core::catalog_domains::derive(&facts, &evidence, &catalog, snapshot).unwrap();
    facts.names.reverse();
    facts.source.reverse();
    facts.releases.reverse();
    facts.declarations.reverse();
    facts.candidates.reverse();
    evidence.coverage.reverse();
    catalog.members.reverse();
    catalog.bindings.reverse();
    catalog.signatures.reverse();
    catalog.constructors.reverse();
    catalog.configurations.reverse();
    catalog.field_links.reverse();
    catalog.surfaces.reverse();
    catalog.contextual.associations.reverse();
    let actual = cpg_core::catalog_domains::derive(&facts, &evidence, &catalog, snapshot).unwrap();
    let keyed = |rows: Vec<CatalogSelectionDomainsRow>| {
        rows.into_iter()
            .map(|row| (row.member_id, (row.domain_id, row.detail)))
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(
        keyed(expected),
        keyed(actual),
        "scan order changed canonical domains"
    );
}
