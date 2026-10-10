//! Actual persistent gRPC completion and full canonical physical readback controls.
use lctx_model::domain::{
    graph::{Assertion, Entity, FamilyHasher, GraphFamily, Manifest},
    input::{Package, Release},
    *,
};
use lctx_surrealdb::surrealdb::types::{RecordId, Value, Variables};
use lctx_surrealdb::{Loader, NativeReader};
#[path = "fixtures/scoped.rs"]
mod scoped;

#[tokio::test]
async fn prepared_canonical_reads_select_exact_payloads_at_declared_result_positions() {
    let config = scoped::config();
    let nonce = lctx_surrealdb::control::fresh_identity("prepared-canonical-control").unwrap();
    let package = Package {
        name: format!("prepared-{}", nonce.hex()),
    };
    let release = Release {
        package: package.id(),
        version: "1".into(),
    };
    let input = input::InputRevision { manifest: nonce };
    let artifact = source::SourceArtifact {
        input: input.id(),
        path: "prepared.py".into(),
        content: ContentHash::of(b"x"),
        byte_len: 1,
    };
    let mut other_artifact = artifact.clone();
    other_artifact.path = "unrelated.py".into();
    let assertion = Assertion::from_record(input::InputDistribution {
        input: input.id(),
        release: release.id(),
        role: input::DistributionRole::FirstParty,
    })
    .unwrap();
    let selected = scoped::reader(
        &config,
        &[
            Entity::from(package.clone()),
            Entity::from(artifact.clone()),
        ],
        std::slice::from_ref(&assertion),
    )
    .await
    .unwrap();
    let unrelated = scoped::reader(
        &config,
        &[
            Entity::from(Package {
                name: format!("unrelated-{}", nonce.hex()),
            }),
            Entity::from(other_artifact),
        ],
        &[],
    )
    .await
    .unwrap();
    let result = async {
        let keyed = selected
            .records::<source::SourceArtifact>(lctx_surrealdb::RecordSelection::Keys(vec![
                *artifact.id().bytes(),
            ]))
            .await?;
        let mut entities = selected.record_stream_prepared::<source::SourceArtifact>(
            "true",
            Variables::new(),
            vec!["LET $canonical_prepared_marker = true".into()],
            "semantic_key",
        )?;
        let entity = entities.next().await?;
        let entity_end = entities.next().await?;
        drop(entities);
        let mut assertions = selected.record_stream_prepared::<input::InputDistribution>(
            "true",
            Variables::new(),
            vec!["LET $canonical_prepared_marker = true".into()],
            "semantic_key",
        )?;
        let assertion = assertions.next().await?;
        let assertion_end = assertions.next().await?;
        drop(assertions);
        let mut failed = selected.record_stream_prepared::<Package>(
            "true",
            Variables::new(),
            vec!["THROW 'expected canonical preparation failure'".into()],
            "semantic_key",
        )?;
        let refused = failed.next().await.is_err();
        failed.drain_transport().await?;
        let sticky = failed.next().await.is_err();
        drop(failed);
        Ok::<_, ModelError>((
            keyed,
            entity,
            entity_end,
            assertion,
            assertion_end,
            refused,
            sticky,
        ))
    }
    .await;
    let mut completion = completion::Completion::default();
    completion.step(
        "prepared canonical selected fixture",
        selected.close().await,
    );
    completion.step(
        "prepared canonical unrelated fixture",
        unrelated.close().await,
    );
    let (keyed, entity, entity_end, actual_assertion, assertion_end, refused, sticky) =
        completion::complete(result, completion).unwrap();
    assert_eq!(keyed, vec![artifact.clone()]);
    assert_eq!(entity, Some(artifact));
    assert!(entity_end.is_none());
    assert_eq!(
        actual_assertion,
        Some(lctx_surrealdb::codec::assertion_record(&assertion).unwrap())
    );
    assert!(assertion_end.is_none());
    assert!(refused);
    assert!(sticky);
}

#[tokio::test]
async fn prepared_native_results_preserve_nested_variants_and_all_terminals() {
    use lctx_surrealdb::{prepared::PreparedQuery, surrealdb::types::Object};
    let config = scoped::config();
    let client = lctx_surrealdb::compiler::check_installation(&config)
        .await
        .unwrap();
    let reader = NativeReader::private(client);
    let mut row = Object::new();
    row.insert("id", RecordId::new("vector", "prepared-native-control"));
    row.insert(
        "dependencies",
        vec![
            RecordId::new("entity", "first"),
            RecordId::new("compiler_record", "second"),
        ],
    );
    row.insert("nested", vec![vec![1i64, 2], vec![3, 4]]);
    let expected = vec![row];
    let mut vars = Variables::new();
    vars.insert("rows", expected.clone());
    let prepared = PreparedQuery::new(
        vars.clone(),
        vec!["LET $selected=$rows".into()],
        vec!["RETURN $selected".into()],
    )
    .unwrap();
    assert_eq!(
        reader
            .query_prepared_native::<Vec<Object>>(prepared)
            .await
            .unwrap(),
        expected
    );
    let failed = PreparedQuery::new(
        vars.clone(),
        vec!["THROW 'native-preparation-failure'".into()],
        vec!["RETURN $rows".into()],
    )
    .unwrap();
    assert!(
        reader
            .query_prepared_native::<Vec<Object>>(failed)
            .await
            .is_err(),
        "preparation failure rejects native result exposure"
    );
    let missing = PreparedQuery::from_sql("RETURN $rows".into(), vars, 2, vec![1]).unwrap();
    assert!(
        reader
            .query_prepared_native::<Vec<Object>>(missing)
            .await
            .is_err(),
        "native conversion cannot bypass the terminal inventory"
    );
}

async fn physical_row(reader: &NativeReader<()>, bindings: Variables) -> Value {
    let mut rows = reader
        .query_stream("SELECT * FROM $id", bindings, 1)
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert!(rows.next().await.unwrap().is_none());
    row
}
fn assert_compact_index_plan(plan: &str, index: &str) {
    let scans = plan
        .lines()
        .filter(|line| line.trim_start().starts_with("IndexScan "))
        .collect::<Vec<_>>();
    assert_eq!(scans.len(), 1, "one equality driver: {plan}");
    assert!(
        scans[0].contains(&format!("index: {index},")),
        "required driver: {plan}"
    );
    assert!(
        scans[0].contains("access: ["),
        "single equality prefix: {plan}"
    );
    for forbidden in [
        "TableScan",
        "UnionIndexScan",
        "Union",
        "Sort",
        "TopK",
        "Aggregate",
        "Group",
        "Distinct",
        "Collect",
        "Materialize",
    ] {
        assert!(
            !plan
                .lines()
                .any(|line| line.trim_start().starts_with(forbidden)),
            "whole-match operator {forbidden}: {plan}"
        );
    }
}
async fn candidate_plan_controls(
    native: &NativeReader<()>,
    fixture: &scoped::ScopedFixture,
    release: &Release,
    expected_members: usize,
) {
    let native_operation_budget = lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    use lctx_surrealdb::prepared::{PreparedQuery, scope_constant};
    let record = analytics::QualityStep {
        run: serde_json::from_value(
            serde_json::to_value(&fixture.store.attempt().0[..16]).unwrap(),
        )
        .unwrap(),
        ordinal: 0,
        value: FiniteF64::new(0.5).unwrap(),
    };
    let relation = Relation::of::<analytics::QualityStep>();
    let contribution = fixture
        .store
        .begin_contribution(completed::ContributionSpec {
            captured_binding: None,
            producer: "candidate-plan-quality".into(),
            profile: stages::Profile::Catalog,
            model: ContentHash::of(b"candidate-plan-model"),
            implementation: ContentHash::of(b"candidate-plan-code"),
            configuration: None,
            inputs: vec![],
            outputs: std::collections::BTreeSet::from([relation.name().into()]),
        })
        .await
        .unwrap();
    fixture
        .store
        .write_batch(
            &contribution,
            &relation,
            &analytics::QualityStep::encode(std::slice::from_ref(&record)).unwrap(),
        )
        .await
        .unwrap();
    fixture
        .store
        .complete_contribution(contribution,
            stages::ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &std::collections::BTreeMap::new(), &native_operation_budget)
        .await
        .unwrap();
    for (table, relation, field, value) in [
        (
            "entity",
            Release::NAME,
            "package",
            lctx_surrealdb::loader::json_value(serde_json::to_value(release.package).unwrap())
                .unwrap(),
        ),
        (
            "assertion",
            input::InputDistribution::NAME,
            "release",
            lctx_surrealdb::loader::json_value(serde_json::to_value(release.id()).unwrap())
                .unwrap(),
        ),
        (
            "compiler_record",
            analytics::QualityStep::NAME,
            "run",
            lctx_surrealdb::loader::json_value(serde_json::to_value(record.run).unwrap()).unwrap(),
        ),
    ] {
        let mut bindings = Variables::new();
        bindings.insert("relation", relation);
        bindings.insert("value", value);
        let preparation = vec![format!(
            "LET $__compiler_scope = {}",
            scope_constant("$relation", field, "$value")
        )];
        let query = format!(
            "SELECT semantic_type AS relation,semantic_key,id AS node FROM {table} WITH INDEX by_scope WHERE scope_keys CONTAINS $__compiler_scope AND semantic_type=$relation"
        );
        let plan: String = native
            .query_prepared(
                PreparedQuery::new(
                    bindings.clone(),
                    preparation.clone(),
                    vec![format!("EXPLAIN {query}")],
                )
                .unwrap(),
            )
            .await
            .unwrap();
        assert_compact_index_plan(&plan, "by_scope");
        let mut rows = native
            .stream_prepared(PreparedQuery::new(bindings, preparation, vec![query]).unwrap())
            .unwrap();
        assert!(rows.next().await.unwrap().is_some());
        assert!(rows.next().await.unwrap().is_none());
    }
    let owner = RecordId::new("compiler_contribution", fixture.contribution.hex());
    let mut bindings = Variables::new();
    bindings.insert("owner", owner);
    bindings.insert("relation", Release::NAME);
    for (query, expected_rows) in [
        (
            "SELECT relation,semantic_key,node FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner AND relation=$relation",
            1,
        ),
        (
            "SELECT node FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner",
            expected_members,
        ),
    ] {
        let plan: String = native
            .query(format!("EXPLAIN {query}"), bindings.clone())
            .await
            .unwrap();
        assert_compact_index_plan(&plan, "contribution_rows");
        let mut rows = native.query_stream(query, bindings.clone(), 1).unwrap();
        let mut observed = 0;
        while rows.next().await.unwrap().is_some() {
            observed += 1;
        }
        assert_eq!(observed, expected_rows, "exact owned membership: {query}");
    }
}

#[tokio::test]
#[ignore = "requires exclusive just service maintenance --native-clients"]
async fn root_native_query_cancellation_drains_before_fresh_checked_query() {
    assert!(
        std::env::var_os("LCTX_SURREAL_MAINTENANCE_TOKEN").is_some(),
        "run through explicit native-client maintenance"
    );
    let installer_path = std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG")
        .expect("validation maintenance installer configuration");
    let installer =
        lctx_surrealdb::RuntimeConfig::read(std::path::Path::new(&installer_path)).unwrap();
    assert_eq!(
        installer.authentication,
        lctx_surrealdb::AuthenticationScope::Root
    );
    assert_eq!(installer.database.as_str(), "validation");
    let client = lctx_surrealdb::compiler::check_installation(&installer)
        .await
        .unwrap();
    let native = NativeReader::private(client.clone());
    // The first provisional row proves real native dispatch. Keep the stream owned
    // outside the polling task: task cancellation does not claim remote query abort.
    let rows = std::sync::Arc::new(tokio::sync::Mutex::new(
        native
            .query_stream(
                "SELECT * FROM [{id:entity:entered}]; SLEEP 1s; SELECT * FROM [{id:entity:late}];",
                Variables::new(),
                3,
            )
            .unwrap(),
    ));
    let (entered, started) = tokio::sync::oneshot::channel();
    let active_rows = rows.clone();
    let active = tokio::spawn(async move {
        let mut rows = active_rows.lock().await;
        let row = rows
            .next()
            .await
            .unwrap()
            .expect("actual provisional native row");
        assert_eq!(
            row.as_object().unwrap().get("id"),
            Some(&Value::RecordId(RecordId::new("entity", "entered")))
        );
        entered.send(()).unwrap();
        rows.next().await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), started)
        .await
        .unwrap()
        .unwrap();
    if active.is_finished() {
        panic!(
            "native operation ended before polling cancellation: {:?}",
            active.await
        );
    }
    active.abort();
    assert!(active.await.unwrap_err().is_cancelled());
    // Explicitly observe all three terminals and outer EOF before a fresh query.
    // The controlled one-second suspension leaves production runtime limits unchanged.
    rows.lock().await.drain_transport().await.unwrap();
    rows.lock().await.drain_transport().await.unwrap();
    let mut fresh = native
        .query_stream("RETURN [{id:entity:fresh}]", Variables::new(), 1)
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        assert!(fresh.next().await.unwrap().is_some());
        assert!(fresh.next().await.unwrap().is_none());
        assert!(fresh.next().await.unwrap().is_none());
    })
    .await
    .expect("fresh checked query completes after explicit cancelled-polling drainage");
    client.invalidate().await.unwrap();
}

#[tokio::test]
async fn terminal_success_is_required_and_sparse_scope_corruption_is_rejected() {
    let native_read_budget = lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let config = scoped::config();
    let client = lctx_surrealdb::compiler::check_installation(&config)
        .await
        .unwrap();
    let nonce = tempfile::NamedTempFile::new().unwrap();
    let scope = ContentHash::of(nonce.path().to_string_lossy().as_bytes());
    let native = NativeReader::private(client.clone());
    let prepared = lctx_surrealdb::prepared::PreparedQuery::new(
        Variables::new(),
        vec!["LET $constant=7".into()],
        vec!["RETURN [$constant]".into()],
    )
    .unwrap();
    assert_eq!(
        native.query_prepared::<Vec<i64>>(prepared).await.unwrap(),
        vec![7]
    );
    assert!(
        native
            .query::<Vec<i64>>("LET $constant=7; RETURN [$constant]", Variables::new())
            .await
            .is_err(),
        "multiple statements require explicit result positions"
    );
    let mut rows = native
        .query_stream(
            "SELECT * FROM [{id:entity:first}]; THROW 'late-terminal-control';",
            Variables::new(),
            2,
        )
        .unwrap();
    assert!(
        rows.next().await.unwrap().is_some(),
        "earlier rows are provisional"
    );
    assert!(
        rows.next().await.is_err(),
        "later statement failure must reject the query"
    );
    assert!(
        rows.next().await.is_err(),
        "a failed stream cannot subsequently claim completion"
    );
    let mut incomplete = native
        .query_stream("RETURN [{id:entity:first}]", Variables::new(), 2)
        .unwrap();
    assert!(incomplete.next().await.unwrap().is_some());
    assert!(
        incomplete.next().await.is_err(),
        "missing declared terminal is refused"
    );
    let mut oversized = native
        .query_stream(
            "RETURN [{id:entity:large,value:string::repeat('x',1048576)}]",
            Variables::new(),
            1,
        )
        .unwrap();
    let oversized_error = oversized.next().await.unwrap_err();
    assert!(
        matches!(
            oversized_error.primary(),
            Some(ModelError::Limit {
                owner: "native-stream",
                limit: "row bytes",
                observed,
                bound,
            }) if *bound == 1024 * 1024 && observed > bound
        ),
        "oversized native values fail before source expansion: {oversized_error}"
    );
    let repeated_error = oversized.next().await.unwrap_err();
    let (ModelError::SharedCause(first), ModelError::SharedCause(repeated)) =
        (&oversized_error, &repeated_error)
    else {
        panic!("failed reads retain their primary cause: {oversized_error}; {repeated_error}");
    };
    assert!(
        std::sync::Arc::ptr_eq(first, repeated),
        "a failed stream retains the same refusal"
    );

    let package = Package {
        name: format!("scope-fixture-{}", scope.hex()),
    };
    let release = Release {
        package: package.id(),
        version: "1".into(),
    };
    let mut bytes = (0..70000)
        .map(|position| (position % 251) as u8)
        .collect::<Vec<_>>();
    bytes[..32].copy_from_slice(&scope.0);
    let input = input::InputRevision::from_entries(vec![input::ManifestEntry {
        path: "original.py".into(),
        content: ContentHash::of(&bytes),
        byte_len: bytes.len() as i64,
    }])
    .unwrap();
    let artifact =
        source::SourceArtifact::from_bytes(input.id(), "original.py".into(), &bytes).unwrap();
    let original = graph::Original {
        source: graph::EntityId::of(artifact.id()),
        content: artifact.content,
        byte_len: bytes.len() as u64,
    };
    let assertion = Assertion::from_record(input::InputDistribution {
        input: input.id(),
        release: release.id(),
        role: input::DistributionRole::FirstParty,
    })
    .unwrap();
    let mut entities = vec![
        Entity::from(package),
        Entity::from(release.clone()),
        Entity::from(input),
        Entity::from(artifact),
        Entity::from(value::Literal::Bytes {
            value: EvidenceBytes(vec![0xff, 0, 0x80]),
        }),
    ];
    entities.sort_by_key(Entity::id);
    let fixture = scoped::reader(&config, &entities, std::slice::from_ref(&assertion))
        .await
        .unwrap();
    let native = &fixture.reader;
    let client = native.shared_client();
    let loader = Loader::for_attempt_views(
        client.clone(),
        fixture.store.attempt(),
        fixture.views.clone(),
    ).with_budget(&native_read_budget);
    loader.entities(&entities[..1]).await.unwrap();
    loader.ensure_entities(&entities).await.unwrap();
    loader.ensure_entities(&entities).await.unwrap();
    loader
        .ensure_assertions(&[assertion.clone(), assertion.clone()])
        .await
        .unwrap();
    loader
        .ensure_assertions(std::slice::from_ref(&assertion))
        .await
        .unwrap();
    assert_eq!(
        native
            .records::<Release>(lctx_surrealdb::RecordSelection::Scope {
                field: "package".into(),
                values: vec![serde_json::to_value(release.package).unwrap()]
            })
            .await
            .unwrap(),
        vec![release.clone()]
    );
    let expected_members = lctx_surrealdb::codec::entity_views(&entities)
        .unwrap()
        .len()
        + lctx_surrealdb::codec::assertion_views(std::slice::from_ref(&assertion))
            .unwrap()
            .len();
    candidate_plan_controls(native, &fixture, &release, expected_members).await;

    loader
        .original_stream(
            original.source.0,
            original.content,
            original.byte_len,
            &mut bytes.as_slice(),
        )
        .await
        .unwrap();
    loader
        .ensure_original_stream(
            original.source.0,
            original.content,
            original.byte_len,
            &mut bytes.as_slice(),
        )
        .await
        .unwrap();
    let ranges = [
        (original.source, 65534, 4),
        (original.source, 7, 3),
        (original.source, 65534, 4),
    ];
    assert_eq!(
        native.original_bytes_batch(&ranges).await.unwrap(),
        vec![
            bytes[65534..65538].to_vec(),
            bytes[7..10].to_vec(),
            bytes[65534..65538].to_vec()
        ]
    );
    let mut assertion_hasher = FamilyHasher::new(GraphFamily::Assertions);
    assert!(
        assertion_hasher
            .push(assertion.id().0, assertion.content())
            .unwrap()
    );
    let mut hasher = FamilyHasher::new(GraphFamily::Entities);
    for entity in &entities {
        assert!(hasher.push(entity.id().0, entity.content()).unwrap());
    }
    let manifest = Manifest {
        admission_contract: ContentHash::of(b"fixture admission contract"),
        format_version: graph::ARTIFACT_FORMAT_VERSION,
        completed_state: completed::CompletedStateIdentity {
            format_version: completed::STATE_FORMAT_VERSION,
            contributions: 0,
            memberships: 0,
            backing_rows: 0,
            content: ContentHash::of(b"fixture-empty-completed-state"),
        },
        frontier: admission::Frontier::Facts,
        profile: stages::Profile::Catalog,
        captures: vec![],
        semantic_contract: ContentHash::of(b"fixture"),
        producers: vec![],
        settings: ContentHash::of(b"fixture"),
        families: vec![hasher.finish(), assertion_hasher.finish()],
        required_outcomes: vec![],
        outcomes: vec![],
        originals: vec![original.clone()],
        projections: vec![],
        embeddings: vec![],
    };
    loader.reconcile(&manifest).await.unwrap();
    // Independent role readback must nominate all actual outgoing rows, rather than
    // only regenerated expected IDs. Both missing and unexpected roles are corruption.
    for (table, source) in [
        (
            "participant",
            lctx_surrealdb::loader::assertion_payload_id(&assertion).unwrap(),
        ),
        (
            "reference",
            lctx_surrealdb::loader::entity_payload_id(&Entity::from(release.clone())).unwrap(),
        ),
    ] {
        let mut variables = Variables::new();
        variables.insert("source", source);
        let mut roles: Vec<Value> = native
            .query_native(
                format!(
                    "SELECT * FROM {table} WITH INDEX outgoing WHERE in=$source ORDER BY id LIMIT 1"
                ),
                variables,
            )
            .await
            .unwrap();
        let saved = roles.pop().expect("fixture outgoing role");
        let id = saved.as_object().unwrap().get("id").unwrap().clone();
        let mut variables = Variables::new();
        variables.insert("id", id);
        variables.insert("saved", saved.clone());
        client
            .query("DELETE $id RETURN NONE")
            .bind(variables.clone())
            .await
            .unwrap()
            .check()
            .unwrap();
        let missing = loader.reconcile(&manifest).await;
        client
            .query("INSERT RELATION $saved RETURN NONE")
            .bind(variables)
            .await
            .unwrap()
            .check()
            .unwrap();
        assert!(missing.is_err(), "missing selected {table} must be refused");
        loader.reconcile(&manifest).await.unwrap();

        let mut extra = saved.as_object().unwrap().clone();
        let id = RecordId::new(
            table,
            lctx_surrealdb::control::fresh_identity("unexpected-role-control")
                .unwrap()
                .hex(),
        );
        extra.insert("id", id.clone());
        extra.insert("field", "__unexpected");
        let mut variables = Variables::new();
        variables.insert("extra", extra);
        variables.insert("id", id);
        client
            .query("INSERT RELATION $extra RETURN NONE")
            .bind(variables.clone())
            .await
            .unwrap()
            .check()
            .unwrap();
        let unexpected = loader.reconcile(&manifest).await;
        client
            .query("DELETE $id RETURN NONE")
            .bind(variables)
            .await
            .unwrap()
            .check()
            .unwrap();
        assert!(
            unexpected.is_err(),
            "unexpected selected {table} must be refused"
        );
        loader.reconcile(&manifest).await.unwrap();
    }
    // Same-key reuse compares the complete physical payload, not just the canonical digest.
    let mut assertion_bindings = Variables::new();
    assertion_bindings.insert(
        "id",
        lctx_surrealdb::loader::assertion_payload_id(&assertion).unwrap(),
    );
    let saved_assertion = physical_row(&native, assertion_bindings.clone()).await;
    assertion_bindings.insert("saved", saved_assertion.clone());
    client
        .query("UPDATE $id SET body.role=1")
        .bind(assertion_bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(
        loader
            .ensure_assertions(std::slice::from_ref(&assertion))
            .await
            .is_err()
    );
    assert!(loader.reconcile(&manifest).await.is_err());
    client
        .query("UPDATE $id CONTENT $saved")
        .bind(assertion_bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    let mut chunk_bindings = Variables::new();
    chunk_bindings.insert(
        "id",
        RecordId::new("original_chunk", format!("{}_0", original.source.0.hex())),
    );
    let saved_chunk = physical_row(&native, chunk_bindings.clone()).await;
    chunk_bindings.insert("saved", saved_chunk);
    client
        .query("UPDATE $id SET bytes=b\"00\"")
        .bind(chunk_bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(
        loader
            .ensure_original_stream(
                original.source.0,
                original.content,
                original.byte_len,
                &mut bytes.as_slice()
            )
            .await
            .is_err()
    );
    assert!(native.original_bytes_batch(&ranges).await.is_err());
    assert!(loader.reconcile(&manifest).await.is_err());
    client
        .query("UPDATE $id CONTENT $saved")
        .bind(chunk_bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    loader.reconcile(&manifest).await.unwrap();

    let mut bindings = Variables::new();
    bindings.insert(
        "id",
        lctx_surrealdb::loader::entity_payload_id(&Entity::from(release.clone())).unwrap(),
    );
    let before = physical_row(&native, bindings.clone()).await;
    bindings.insert("saved", before.clone());
    // The fixed envelope deliberately allows raw nested writes; admission still rejects them.
    client
        .query("UPDATE $id SET body.unexpected=1")
        .bind(bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(loader.reconcile(&manifest).await.is_err());
    client
        .query("UPDATE $id CONTENT $saved")
        .bind(bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    loader.reconcile(&manifest).await.unwrap();
    // Supplied scopes remain independently checked against the declared body.
    client
        .query("UPDATE $id SET scope_keys=['wrong'];")
        .bind(bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(loader.ensure_entities(&entities).await.is_err());
    assert!(loader.reconcile(&manifest).await.is_err());
    let after = physical_row(&native, bindings.clone()).await;
    assert_eq!(
        before.as_object().unwrap().get("canonical"),
        after.as_object().unwrap().get("canonical")
    );
    // Readback is read-only: the invalid persisted value remains after the refusal.
    assert_ne!(before, after);
    bindings.insert("saved", before.clone());
    client
        .query("UPDATE $id CONTENT $saved")
        .bind(bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    client
        .query("UPDATE $id SET canonical=b\"00\"")
        .bind(bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(matches!(
        native
            .records::<Release>(lctx_surrealdb::RecordSelection::Keys(vec![
                *release.id().bytes()
            ]))
            .await,
        Err(ModelError::Serving(serving::FailureKind::Corrupt))
    ));
    assert!(matches!(
        loader.reconcile(&manifest).await,
        Err(ModelError::Serving(serving::FailureKind::Corrupt))
    ));
    client
        .query("UPDATE $id CONTENT $saved")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    loader.reconcile(&manifest).await.unwrap();
    fixture.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn flexible_body_and_supplied_scopes_are_independently_reconstructed() {
    let native_read_budget = lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    use lctx_surrealdb::surrealdb::types::{Bytes, Object};
    let config = scoped::config();
    let client = lctx_surrealdb::compiler::check_installation(&config)
        .await
        .unwrap();
    let nonce = tempfile::NamedTempFile::new().unwrap();
    let scope = ContentHash::of(nonce.path().to_string_lossy().as_bytes());
    let mut original_bytes = vec![0xff, 0, 0x80];
    original_bytes.extend_from_slice(&scope.0);
    let input = input::InputRevision::from_entries(vec![input::ManifestEntry {
        path: "envelope.py".into(),
        content: ContentHash::of(&original_bytes),
        byte_len: original_bytes.len() as i64,
    }])
    .unwrap();
    let artifact =
        source::SourceArtifact::from_bytes(input.id(), "envelope.py".into(), &original_bytes)
            .unwrap();
    let original = graph::Original {
        source: graph::EntityId::of(artifact.id()),
        content: artifact.content,
        byte_len: original_bytes.len() as u64,
    };
    let entities = vec![
        Entity::from(value::Literal::Bytes {
            value: EvidenceBytes(original_bytes.clone()),
        }),
        Entity::from(Package {
            name: format!("envelope-control-{}", scope.hex()),
        }),
        Entity::from(input),
        Entity::from(artifact),
    ];
    let manifest_for = |entities: &[Entity]| {
        let mut ordered = entities.to_vec();
        ordered.sort_by_key(Entity::id);
        let mut entity_family = FamilyHasher::new(GraphFamily::Entities);
        for entity in ordered {
            entity_family.push(entity.id().0, entity.content()).unwrap();
        }
        Manifest {
            admission_contract: ContentHash::of(b"fixture-admission"),
            format_version: graph::ARTIFACT_FORMAT_VERSION,
            completed_state: completed::CompletedStateIdentity {
                format_version: completed::STATE_FORMAT_VERSION,
                contributions: 0,
                memberships: 0,
                backing_rows: 0,
                content: ContentHash::of(b"fixture-empty-state"),
            },
            frontier: admission::Frontier::Facts,
            profile: stages::Profile::Catalog,
            captures: vec![],
            semantic_contract: ContentHash::of(b"fixture"),
            producers: vec![],
            settings: ContentHash::of(b"fixture"),
            families: vec![
                entity_family.finish(),
                FamilyHasher::new(GraphFamily::Assertions).finish(),
            ],
            required_outcomes: vec![],
            outcomes: vec![],
            originals: vec![],
            projections: vec![],
            embeddings: vec![],
        }
    };
    Loader::for_views(client.clone(), vec![]).with_budget(&native_read_budget)
        .reconcile(&manifest_for(&[]))
        .await
        .unwrap();
    let fixture = scoped::reader(&config, &entities, &[]).await.unwrap();
    let client = fixture.reader.shared_client();
    let loader = Loader::for_attempt_views(
        client.clone(),
        fixture.store.attempt(),
        fixture.views.clone(),
    ).with_budget(&native_read_budget);
    loader
        .original_stream(
            original.source.0,
            original.content,
            original.byte_len,
            &mut original_bytes.as_slice(),
        )
        .await
        .unwrap();
    let mut manifest = manifest_for(&entities);
    manifest.originals = vec![original.clone()];
    loader.reconcile(&manifest).await.unwrap();
    let id = lctx_surrealdb::loader::entity_payload_id(&entities[0]).unwrap();
    let mut bindings = Variables::new();
    bindings.insert("id", id);
    let mut response = client
        .query("SELECT * FROM $id")
        .bind(bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    let mut rows: Vec<Value> = response.take(0).unwrap();
    let saved = rows.pop().unwrap();
    assert!(rows.is_empty());
    let saved = saved.as_object().unwrap();
    for defect in [
        "extra_body_field",
        "missing_inactive_null",
        "wrong_bytes",
        "wrong_sum_tag",
        "wrong_scope",
        "wrong_semantic_key",
    ] {
        let mut altered: Object = saved.clone();
        match defect {
            "wrong_scope" => {
                altered.insert("scope_keys", vec!["wrong"]);
            }
            "wrong_semantic_key" => {
                altered.insert("semantic_key", "wrong");
            }
            _ => {
                let Some(Value::Object(body)) = altered.get_mut("body") else {
                    panic!("literal body");
                };
                match defect {
                    "extra_body_field" => {
                        body.insert("unexpected", 1i64);
                    }
                    "missing_inactive_null" => {
                        assert_eq!(body.remove("string_value"), Some(Value::Null));
                    }
                    "wrong_bytes" => {
                        body.insert("bytes_value", Bytes::from(vec![9u8]));
                    }
                    "wrong_sum_tag" => {
                        body.insert(
                            Relation::of::<value::Literal>().sum().unwrap().tag,
                            i64::from(i16::MAX),
                        );
                    }
                    _ => unreachable!(),
                }
            }
        }
        bindings.insert("row", Value::Object(altered));
        client
            .query("UPDATE $id CONTENT $row")
            .bind(bindings.clone())
            .await
            .unwrap()
            .check()
            .unwrap();
        assert!(
            loader.reconcile(&manifest).await.is_err(),
            "independent reconstruction rejects {defect}"
        );
        bindings.insert("row", Value::Object(saved.clone()));
        client
            .query("UPDATE $id CONTENT $row")
            .bind(bindings.clone())
            .await
            .unwrap()
            .check()
            .unwrap();
        loader.reconcile(&manifest).await.unwrap();
    }
    let chunk_id = RecordId::new("original_chunk", format!("{}_0", original.source.0.hex()));
    let wrong_id = RecordId::new("original_chunk", format!("corrupted_{}", scope.hex()));
    let mut chunk_bindings = Variables::new();
    chunk_bindings.insert("id", chunk_id);
    chunk_bindings.insert("wrong", wrong_id.clone());
    let mut response = client
        .query("SELECT * FROM $id")
        .bind(chunk_bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    let rows: Vec<Value> = response.take(0).unwrap();
    let [saved_chunk] = rows.as_slice() else {
        panic!("one original chunk");
    };
    let mut wrong = saved_chunk.as_object().unwrap().clone();
    wrong.insert("id", wrong_id);
    chunk_bindings.insert("row", Value::Object(wrong));
    chunk_bindings.insert("saved", saved_chunk.clone());
    client
        .query("DELETE $id; INSERT INTO original_chunk $row;")
        .bind(chunk_bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(
        loader.reconcile(&manifest).await.is_err(),
        "original chunk identity is part of the complete physical envelope"
    );
    client
        .query("DELETE $wrong; INSERT INTO original_chunk $saved;")
        .bind(chunk_bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    loader.reconcile(&manifest).await.unwrap();
    fixture.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires exclusive just service maintenance --native-clients"]
async fn imported_extra_original_envelope_is_rejected_under_explicit_maintenance() {
    let native_read_budget = lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let installer_path = std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG")
        .expect("validation maintenance installer configuration");
    let installer =
        lctx_surrealdb::RuntimeConfig::read(std::path::Path::new(&installer_path)).unwrap();
    assert_eq!(installer.database.as_str(), "validation");
    let config = scoped::config();
    let client = lctx_surrealdb::compiler::check_installation(&config)
        .await
        .unwrap();
    let nonce = tempfile::NamedTempFile::new().unwrap();
    let scope = ContentHash::of(nonce.path().to_string_lossy().as_bytes());
    let mut original_bytes = vec![0xff, 0, 0x80];
    original_bytes.extend_from_slice(&scope.0);
    let input = input::InputRevision::from_entries(vec![input::ManifestEntry {
        path: "envelope.py".into(),
        content: ContentHash::of(&original_bytes),
        byte_len: original_bytes.len() as i64,
    }])
    .unwrap();
    let artifact =
        source::SourceArtifact::from_bytes(input.id(), "envelope.py".into(), &original_bytes)
            .unwrap();
    let original = graph::Original {
        source: graph::EntityId::of(artifact.id()),
        content: artifact.content,
        byte_len: original_bytes.len() as u64,
    };
    let entities = vec![
        Entity::from(value::Literal::Bytes {
            value: EvidenceBytes(original_bytes.clone()),
        }),
        Entity::from(Package {
            name: format!("envelope-control-{}", scope.hex()),
        }),
        Entity::from(input),
        Entity::from(artifact),
    ];
    let manifest_for = |entities: &[Entity]| {
        let mut ordered = entities.to_vec();
        ordered.sort_by_key(Entity::id);
        let mut entity_family = FamilyHasher::new(GraphFamily::Entities);
        for entity in ordered {
            entity_family.push(entity.id().0, entity.content()).unwrap();
        }
        Manifest {
            admission_contract: ContentHash::of(b"fixture-admission"),
            format_version: graph::ARTIFACT_FORMAT_VERSION,
            completed_state: completed::CompletedStateIdentity {
                format_version: completed::STATE_FORMAT_VERSION,
                contributions: 0,
                memberships: 0,
                backing_rows: 0,
                content: ContentHash::of(b"fixture-empty-state"),
            },
            frontier: admission::Frontier::Facts,
            profile: stages::Profile::Catalog,
            captures: vec![],
            semantic_contract: ContentHash::of(b"fixture"),
            producers: vec![],
            settings: ContentHash::of(b"fixture"),
            families: vec![
                entity_family.finish(),
                FamilyHasher::new(GraphFamily::Assertions).finish(),
            ],
            required_outcomes: vec![],
            outcomes: vec![],
            originals: vec![],
            projections: vec![],
            embeddings: vec![],
        }
    };
    Loader::for_views(client.clone(), vec![]).with_budget(&native_read_budget)
        .reconcile(&manifest_for(&[]))
        .await
        .unwrap();
    let fixture = scoped::reader(&config, &entities, &[]).await.unwrap();
    let client = fixture.reader.shared_client();
    let loader = Loader::for_attempt_views(
        client.clone(),
        fixture.store.attempt(),
        fixture.views.clone(),
    ).with_budget(&native_read_budget);
    loader
        .original_stream(
            original.source.0,
            original.content,
            original.byte_len,
            &mut original_bytes.as_slice(),
        )
        .await
        .unwrap();
    let mut manifest = manifest_for(&entities);
    manifest.originals = vec![original.clone()];
    loader.reconcile(&manifest).await.unwrap();

    let admin = lctx_surrealdb::reader::connect(
        &installer.endpoint,
        &installer.writer_credentials(),
        installer.namespace.as_str(),
        installer.database.as_str(),
    )
    .await
    .unwrap();
    let mut chunk_bindings = Variables::new();
    chunk_bindings.insert(
        "id",
        RecordId::new("original_chunk", format!("{}_0", original.source.0.hex())),
    );
    let mut response = client
        .query("SELECT * FROM $id")
        .bind(chunk_bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    let rows: Vec<Value> = response.take(0).unwrap();
    let [saved] = rows.as_slice() else {
        panic!("one owned original chunk");
    };
    chunk_bindings.insert("saved", saved.clone());
    // Simulate an imported auxiliary envelope with a separately declared extra field.
    admin
        .query("DEFINE FIELD extra ON original_chunk TYPE option<int>; UPDATE $id SET extra=1;")
        .bind(chunk_bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(
        loader.reconcile(&manifest).await.is_err(),
        "extra original envelope data cannot disappear in typed projection"
    );
    client
        .query("UPDATE $id CONTENT $saved;")
        .bind(chunk_bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    loader.reconcile(&manifest).await.unwrap();

    admin
        .query("REMOVE FIELD extra ON original_chunk;")
        .await
        .unwrap()
        .check()
        .unwrap();
    admin.invalidate().await.unwrap();
    fixture.close().await.unwrap();
}
