//! Native incidence eligibility is independent of whether immutable bytes change.
//! These controls use the explicitly bound campaign fixture and logical attempt owners.
use super::*;
use futures::{FutureExt, future::BoxFuture};
use lctx_compiler_store::compiler::NativeCompilerStore;
use lctx_model::domain::{
    admission::Frontier,
    completed::{ContributionSpec, OutputProfile},
    completion::{self, Completion, LocalState, RemoteState, ThreadPanic},
    resources::{DEFAULT_MEMORY_BYTES, ResourceBudget},
    stages::Profile,
};

struct Fixture {
    client: Arc<Surreal<Client>>,
    first: Arc<NativeCompilerStore>,
    second: Arc<NativeCompilerStore>,
}
async fn run_case(
    label: &'static str,
    run: impl for<'a> FnOnce(&'a Fixture) -> BoxFuture<'a, Result<(), ModelError>>,
) {
    let config = crate::RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native campaign fixture"),
    ))
    .unwrap();
    // A native unit-test crate and the compiler-store dev dependency have distinct
    // native type identities; each adapter reads its own runtime contract.
    let compiler_config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").unwrap(),
    ))
    .unwrap();
    let begin = || async {
        let budget = ResourceBudget::fixed(DEFAULT_MEMORY_BYTES).unwrap();
        let factory =
            lctx_compiler_store::compiler::recovery_factory(&compiler_config, budget).unwrap();
        NativeCompilerStore::begin(&compiler_config, Frontier::Facts, &factory)
            .await
            .unwrap()
    };
    let first = begin().await;
    let second = begin().await;
    let client = crate::reader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        first.namespace().as_str(),
        first.database().as_str(),
    )
    .await
    .unwrap();
    let cancellation = RetryCancellation::default();
    let stop = cancellation.clone();
    let signals = config.signal_owner(label, move || stop.cancel()).unwrap();
    let scope = RecoveryScope::new(
        client.clone(),
        ResourceBudget::fixed(DEFAULT_MEMORY_BYTES).unwrap(),
        cancellation,
        signals.pending_recipient().unwrap(),
        RecoveryTarget {
            generation: config.service_generation,
            namespace: config.namespace.as_str().into(),
            database: config.database.as_str().into(),
            endpoint: Some(config.endpoint.clone()),
        },
    )
    .unwrap();
    let fixture = Fixture {
        client: client.clone(),
        first,
        second,
    };
    let result = std::panic::AssertUnwindSafe(with_recovery_scope(scope.clone(), run(&fixture)))
        .catch_unwind()
        .await
        .unwrap_or_else(|panic| Err(ModelError::Cause(Box::new(ThreadPanic::new(label, panic)))));
    let mut terminal = Completion::default();
    let conclusive = result
        .as_ref()
        .err()
        .is_none_or(ModelError::permits_storage_cleanup);
    if conclusive && scope.pending_count().unwrap() == 0 {
        terminal.step(
            "incidence control client close",
            client.invalidate().await.map_err(ModelError::codec),
        );
    } else {
        // Neither an assertion nor a lost response grants native-executor drainage.
        terminal.local = LocalState::Outstanding;
        terminal.remote = RemoteState::Unknown;
    }
    terminal.merge(fixture.first.abandon_finality_report().await);
    terminal.merge(fixture.second.abandon_finality_report().await);
    let observed = signals.complete(&terminal).await;
    terminal.step("incidence control signal owner", observed);
    completion::complete(result, terminal).unwrap();
}
fn external(name: &str) -> (RecordId, Value) {
    let target = lctx_model::domain::graph::Target::External {
        provider: lctx_model::domain::graph::EntityId(ContentHash::of(
            b"incidence-control-provider",
        )),
        context: lctx_model::domain::graph::EntityId(ContentHash::of(b"incidence-control-context")),
        name: name.into(),
        reason: lctx_model::domain::normalized::entities::EntityReason::ProviderExternal,
    };
    let id = crate::reader::target_id(target.clone());
    let mut row = Object::new();
    row.insert("id", id.clone());
    row.insert(
        "canonical",
        surrealdb::types::Bytes::from(serde_json::to_vec(&target).unwrap()),
    );
    (id, Value::Object(row))
}
async fn read_row(client: &Surreal<Client>, id: &RecordId) -> Result<Value, ModelError> {
    let mut response = client
        .query("SELECT * FROM $record")
        .bind(("record", id.clone()))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let rows: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
    let [row] = rows.as_slice() else {
        return Err(ModelError::Conflict("incidence control expected one row"));
    };
    Ok(row.clone())
}
async fn physical(client: &Surreal<Client>, id: &RecordId) -> Result<Value, ModelError> {
    read_row(client, &guard_id(&Value::RecordId(id.clone()))?).await
}
fn revision(row: &Value) -> Result<i64, ModelError> {
    authorization::int_field(
        row.as_object()
            .ok_or(ModelError::Schema("incidence control guard"))?,
        "revision",
    )
}

async fn predicate(client: &Surreal<Client>, target: &RecordId) -> Result<Value, ModelError> {
    let mut response = client
        .query("LET $id=type::record('native_predicate',[$target]); SELECT * FROM ONLY $id")
        .bind(("target", target.clone()))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    response.take(1).map_err(ModelError::codec)
}
async fn stripes(client: &Surreal<Client>, owner: &RecordId) -> Result<Vec<Object>, ModelError> {
    let mut response = client.query("SELECT * FROM native_attempt_stripe WITH INDEX attempt_stripes WHERE attempt=$owner ORDER BY slot")
        .bind(("owner", owner.clone())).await.map_err(ModelError::codec)?
        .check().map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    assert_eq!(rows.len(), coordination::ADMISSION_STRIPES);
    Ok(rows)
}
async fn incidence_snapshot(
    client: &Surreal<Client>,
    records: Vec<RecordId>,
    owner: &RecordId,
) -> Result<Object, ModelError> {
    let guards = records
        .iter()
        .map(|id| guard_id(&Value::RecordId(id.clone())))
        .collect::<Result<Vec<_>, _>>()?;
    let mut response = client.query("LET $predicates=$records.map(|$record| type::record('native_predicate',[$record])); RETURN {records:(SELECT * FROM $records ORDER BY id),guards:(SELECT * FROM $guards ORDER BY id),predicates:(SELECT * FROM $predicates ORDER BY id),stripes:(SELECT * FROM native_attempt_stripe WITH INDEX attempt_stripes WHERE attempt=$owner ORDER BY slot)}")
        .bind(("records", records)).bind(("guards", guards)).bind(("owner", owner.clone()))
        .await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let Value::Object(snapshot) = response.take::<Value>(1).map_err(ModelError::codec)? else {
        return Err(ModelError::Schema("native incidence snapshot object"));
    };
    Ok(snapshot)
}

#[tokio::test(flavor = "multi_thread")]
async fn immutable_bulk_incidence_preserves_equal_replay_new_holders_and_context_free_attempt_cuts()
{
    run_case("bulk immutable exact incidence", |fixture| async move {
        let first = fixture.first.attempt();
        let second = fixture.second.attempt();
        let owner1 = RecordId::new("native_attempt", first.hex());
        let owner2 = RecordId::new("native_attempt", second.hex());
        let owner1_before = physical(&fixture.client, &owner1).await?;
        let owner2_before = physical(&fixture.client, &owner2).await?;
        let (target, row) = external(&format!("{}-bulk-holder", first.hex()));
        let install = |attempt, owner, row| ImmutableWindows::new(vec![row], Some(attempt), None, Some(owner)).next().unwrap();
        install_window(&fixture.client, Some(first), install(first, owner1.clone(), row.clone())?, None).await?;
        let first_guard = physical(&fixture.client, &target).await?;
        let first_predicate = predicate(&fixture.client, &target).await?;
        assert_eq!(revision(&first_guard)?, 1, "vacant guard creation supplies revision one; the first hold must not increment it again");
        assert_eq!(revision(&first_predicate)?, 1, "row and its first hold share one changed-target increment");
        install_window(&fixture.client, Some(first), install(first, owner1.clone(), row.clone())?, None).await?;
        assert_eq!(physical(&fixture.client, &target).await?, first_guard);
        assert_eq!(predicate(&fixture.client, &target).await?, first_predicate);
        install_window(&fixture.client, Some(second), install(second, owner2.clone(), row.clone())?, None).await?;
        let next_guard = physical(&fixture.client, &target).await?;
        assert_eq!(revision(&next_guard)?, revision(&first_guard)?+1, "a new holder advances the existing endpoint guard once");
        assert_eq!(revision(&predicate(&fixture.client, &target).await?)?, revision(&first_predicate)?+1);
        assert_eq!(next_guard.as_object().unwrap().get("incarnation"), first_guard.as_object().unwrap().get("incarnation"));
        assert_eq!(read_row(&fixture.client, &target).await?, row);
        for owner in [&owner1, &owner2] {
            assert_eq!(revision(&predicate(&fixture.client, &hold_record_id(owner, &target)?).await?)?, 1);
        }
        assert_eq!(physical(&fixture.client, &owner1).await?, owner1_before);
        assert_eq!(physical(&fixture.client, &owner2).await?, owner2_before);
        let before1 = stripes(&fixture.client, &owner1).await?;
        let before2 = stripes(&fixture.client, &owner2).await?;
        // An installation-owned hold legitimately addresses distinct attempts.
        // Unlike admitted windows, both endpoints must retain complete cuts.
        hold(&fixture.client, None, owner2.clone(), vec![owner1.clone()]).await?;
        for (owner, before) in [(&owner1, before1), (&owner2, before2)] {
            for (after, before) in stripes(&fixture.client, owner).await?.iter().zip(before) {
                assert_eq!(authorization::int_field(after, "revision")?, authorization::int_field(&before, "revision")?+1);
            }
        }
        Ok(())
    }.boxed()).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn immutable_bulk_incidence_tail_maxima_and_collision_roll_back_every_address() {
    run_case("bulk immutable late rollback", |fixture| async move {
        let first = fixture.first.attempt();
        let second = fixture.second.attempt();
        let owner1 = RecordId::new("native_attempt", first.hex());
        let owner2 = RecordId::new("native_attempt", second.hex());
        for damage in ["predicate_max", "guard_max", "collision"] {
            let (tail, row) = external(&format!("{}-bulk-tail-{damage}", first.hex()));
            ensure_rows(&fixture.client, Some(first), vec![row.clone()]).await?;
            hold(&fixture.client, Some(first), owner1.clone(), vec![tail.clone()]).await?;
            let (fresh, fresh_row) = external(&format!("{}-bulk-prefix-{damage}", second.hex()));
            let mut wanted = row.as_object().unwrap().clone();
            if damage == "collision" {
                wanted.insert("canonical", surrealdb::types::Bytes::from(b"different full immutable bytes".to_vec()));
            }
            let (mut sql, mut bindings) = immutable_row_plan(Some(second), None, vec![fresh_row, Value::Object(wanted)])?
                .ok_or(ModelError::Schema("bulk immutable control rows"))?;
            let (_, holds, hold_bindings) = hold_plan(Some(second), vec![(owner2.clone(), fresh.clone()), (owner2.clone(), tail.clone())])?
                .ok_or(ModelError::Schema("bulk immutable control holds"))?;
            merge_plan_bindings(&mut bindings, hold_bindings)?;
            sql = join_immutable_holds(&sql, &holds);
            // These owned mutations occur AFTER the real wrapper's expectation
            // checks and roll back with this transaction. They reveal the body's
            // predicate/guard overflow checks rather than a prior wrapper refusal.
            let prefix = match damage {
                "predicate_max" => "LET $control_predicate=type::record('native_predicate',[$control_target]); UPSERT $control_predicate SET revision=9223372036854775807 RETURN NONE; ",
                "guard_max" => "UPDATE $control_guard SET revision=9223372036854775807 RETURN NONE; ",
                _ => "",
            };
            bindings.insert("control_target", tail.clone());
            bindings.insert("control_guard", guard_id(&Value::RecordId(tail.clone()))?);
            let records = vec![fresh.clone(), tail.clone(), hold_record_id(&owner2, &fresh)?, hold_record_id(&owner2, &tail)?, owner2.clone()];
            let before = incidence_snapshot(&fixture.client, records.clone(), &owner2).await?;
            let error = effect_for_owner(&fixture.client, EffectOwner::OpenAttempt(second), &format!("{prefix}{sql}"), bindings).await.unwrap_err();
            let expected = match damage {
                "predicate_max" => "native predicate revision exhausted",
                "guard_max" => "native changed incidence guard invalid",
                _ => "native immutable address collision",
            };
            assert!(error.to_string().contains(expected), "{damage}: {error}");
            assert!(error.permits_storage_cleanup(), "known rollback must settle its actual request: {error}");
            assert_eq!(incidence_snapshot(&fixture.client, records, &owner2).await?, before,
                "{damage}: rows, holds, physical/predicate revisions and attempt participation all roll back");
        }
        Ok(())
    }.boxed()).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn reused_window_writer_refuses_after_complete_owner_cut() {
    run_case("reused immutable writer owner cut", |fixture| async move {
        let attempt = fixture.first.attempt();
        let owner = RecordId::new("native_attempt", attempt.hex());
        let (first_id, first_row) = external(&format!("{}-before-cut", attempt.hex()));
        let mut writer = WindowWriter::new(&fixture.client, Some(attempt), None);
        let first = ImmutableWindows::new(vec![first_row.clone()], Some(attempt), None, Some(owner.clone()))
            .next().unwrap()?;
        writer.install(first).await?;
        close_attempt(&fixture.client, attempt, "abandoned").await?;
        let (late_id, late_row) = external(&format!("{}-after-cut", attempt.hex()));
        let late = ImmutableWindows::new(vec![late_row], Some(attempt), None, Some(owner))
            .next().unwrap()?;
        let error = writer.install(late).await.unwrap_err();
        assert!(error.permits_storage_cleanup(), "known late refusal: {error}");
        assert_eq!(read_row(&fixture.client, &first_id).await?, first_row);
        let mut response = fixture.client.query("SELECT * FROM $record; SELECT * FROM native_hold WITH INDEX object_holds WHERE object=$record")
            .bind(("record", late_id)).await.map_err(ModelError::codec)?
            .check().map_err(ModelError::codec)?;
        assert!(response.take::<Vec<Value>>(0).map_err(ModelError::codec)?.is_empty());
        assert!(response.take::<Vec<Value>>(1).map_err(ModelError::codec)?.is_empty());
        Ok(())
    }.boxed()).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn equal_foreign_attempt_hold_refuses_without_incidence_mutation() {
    run_case("equal foreign hold incidence", |fixture| {
        async move {
            let (target, row) =
                external(&format!("{}-foreign-hold", fixture.second.attempt().hex()));
            let owner = RecordId::new("native_attempt", fixture.second.attempt().hex());
            ensure_rows(&fixture.client, Some(fixture.second.attempt()), vec![row]).await?;
            hold(
                &fixture.client,
                Some(fixture.second.attempt()),
                owner.clone(),
                vec![target.clone()],
            )
            .await?;
            let before = physical(&fixture.client, &target).await?;
            // Same-owner equal replay remains legal and cannot churn the endpoint guard.
            hold(
                &fixture.client,
                Some(fixture.second.attempt()),
                owner.clone(),
                vec![target.clone()],
            )
            .await?;
            assert_eq!(physical(&fixture.client, &target).await?, before);
            let error = hold(
                &fixture.client,
                Some(fixture.first.attempt()),
                owner.clone(),
                vec![target.clone()],
            )
            .await
            .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("native unrelated attempt incidence"),
                "{error}"
            );
            assert!(
                error.permits_storage_cleanup(),
                "acknowledged refusal must retain its true finality: {error}"
            );
            assert_eq!(physical(&fixture.client, &target).await?, before);
            let existing = read_row(&fixture.client, &hold_record_id(&owner, &target)?).await?;
            assert_eq!(
                existing.as_object().unwrap().get("owner"),
                Some(&Value::RecordId(owner))
            );
            Ok(())
        }
        .boxed()
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn equal_foreign_attempt_immutable_row_refuses_without_incidence_mutation() {
    run_case("equal foreign immutable incidence", |fixture| {
        async move {
            let identity = ContentHash::of(b"foreign immutable incidence control");
            let contribution = fixture
                .second
                .begin_contribution(ContributionSpec {
                    captured_binding: None,
                    producer: "incidence-control".into(),
                    profile: OutputProfile::Specific(Profile::Catalog),
                    model: identity,
                    implementation: identity,
                    configuration: None,
                    inputs: Vec::new(),
                    outputs: Default::default(),
                })
                .await?;
            let record = RecordId::new("compiler_contribution", contribution.hex());
            let row = read_row(&fixture.client, &record).await?;
            let before = physical(&fixture.client, &record).await?;
            ensure_rows(
                &fixture.client,
                Some(fixture.second.attempt()),
                vec![row.clone()],
            )
            .await?;
            assert_eq!(physical(&fixture.client, &record).await?, before);
            let error = ensure_rows(
                &fixture.client,
                Some(fixture.first.attempt()),
                vec![row.clone()],
            )
            .await
            .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("native unrelated attempt incidence"),
                "{error}"
            );
            assert!(error.permits_storage_cleanup(), "{error}");
            assert_eq!(read_row(&fixture.client, &record).await?, row);
            assert_eq!(physical(&fixture.client, &record).await?, before);
            Ok(())
        }
        .boxed()
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn immutable_insert_into_active_absent_guard_advances_its_revision() {
    run_case("active absent immutable incidence", |fixture| {
        async move {
            let (record, row) =
                external(&format!("{}-active-absent", fixture.first.attempt().hex()));
            // Mutable operation admission may earn an active physical guard before a row
            // exists, just as retirement root registration can. It grants no immutable bytes.
            guarded_effect(
                &fixture.client,
                Some(fixture.first.attempt()),
                vec![record.clone()],
                "RETURN true",
                Variables::new(),
            )
            .await?;
            let before = physical(&fixture.client, &record).await?;
            assert_eq!(
                before.as_object().unwrap().get("phase"),
                Some(&Value::String("active".into()))
            );
            let mut response = fixture
                .client
                .query("SELECT VALUE id FROM $record")
                .bind(("record", record.clone()))
                .await
                .map_err(ModelError::codec)?
                .check()
                .map_err(ModelError::codec)?;
            assert!(
                response
                    .take::<Vec<RecordId>>(0)
                    .map_err(ModelError::codec)?
                    .is_empty()
            );
            ensure_rows(
                &fixture.client,
                Some(fixture.first.attempt()),
                vec![row.clone()],
            )
            .await?;
            let after = physical(&fixture.client, &record).await?;
            assert_eq!(revision(&after)?, revision(&before)? + 1);
            assert_eq!(
                after.as_object().unwrap().get("incarnation"),
                before.as_object().unwrap().get("incarnation")
            );
            assert_eq!(read_row(&fixture.client, &record).await?, row);
            let owner = RecordId::new("native_attempt", fixture.first.attempt().hex());
            hold(
                &fixture.client,
                Some(fixture.first.attempt()),
                owner,
                vec![record],
            )
            .await?;
            Ok(())
        }
        .boxed()
    })
    .await;
}
