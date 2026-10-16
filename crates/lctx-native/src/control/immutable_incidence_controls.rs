//! Native incidence eligibility is independent of whether immutable bytes change.
//! These controls use the explicitly bound campaign fixture and logical attempt owners.
use super::*;
use futures::{FutureExt, future::BoxFuture};
use lctx_model::domain::{
    admission::Frontier,
    completed::{ContributionSpec, OutputProfile},
    completion::{self, Completion, LocalState, RemoteState, ThreadPanic},
    resources::{DEFAULT_MEMORY_BYTES, ResourceBudget},
    stages::Profile,
};
use lctx_compiler_store::compiler::NativeCompilerStore;

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
    )).unwrap();
    // A native unit-test crate and the compiler-store dev dependency have distinct
    // native type identities; each adapter reads its own runtime contract.
    let compiler_config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").unwrap(),
    )).unwrap();
    let begin = || async {
        let budget = ResourceBudget::fixed(DEFAULT_MEMORY_BYTES).unwrap();
        let factory = lctx_compiler_store::compiler::recovery_factory(&compiler_config, budget).unwrap();
        NativeCompilerStore::begin(&compiler_config, Frontier::Facts, &factory).await.unwrap()
    };
    let first = begin().await;
    let second = begin().await;
    let client = crate::reader::connect(
        &config.endpoint, &config.writer_credentials(),
        first.namespace().as_str(), first.database().as_str(),
    ).await.unwrap();
    let cancellation = RetryCancellation::default();
    let stop = cancellation.clone();
    let signals = config.signal_owner(label, move || stop.cancel()).unwrap();
    let scope = RecoveryScope::new(
        client.clone(), ResourceBudget::fixed(DEFAULT_MEMORY_BYTES).unwrap(), cancellation,
        signals.pending_recipient().unwrap(), RecoveryTarget {
            generation: config.service_generation,
            namespace: config.namespace.as_str().into(),
            database: config.database.as_str().into(),
            endpoint: Some(config.endpoint.clone()),
        },
    ).unwrap();
    let fixture = Fixture { client: client.clone(), first, second };
    let result = std::panic::AssertUnwindSafe(with_recovery_scope(scope.clone(), run(&fixture)))
        .catch_unwind().await.unwrap_or_else(|panic| {
            Err(ModelError::Cause(Box::new(ThreadPanic::new(label, panic))))
        });
    let mut terminal = Completion::default();
    let conclusive = result.as_ref().err().is_none_or(ModelError::permits_storage_cleanup);
    if conclusive && scope.pending_count().unwrap() == 0 {
        terminal.step("incidence control client close", client.invalidate().await.map_err(ModelError::codec));
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
        provider: lctx_model::domain::graph::EntityId(ContentHash::of(b"incidence-control-provider")),
        context: lctx_model::domain::graph::EntityId(ContentHash::of(b"incidence-control-context")),
        name: name.into(),
        reason: lctx_model::domain::normalized::entities::EntityReason::ProviderExternal,
    };
    let id = crate::reader::target_id(target.clone());
    let mut row = Object::new();
    row.insert("id", id.clone());
    row.insert("canonical", surrealdb::types::Bytes::from(serde_json::to_vec(&target).unwrap()));
    (id, Value::Object(row))
}
async fn read_row(client: &Surreal<Client>, id: &RecordId) -> Result<Value, ModelError> {
    let mut response = client.query("SELECT * FROM $record")
        .bind(("record", id.clone())).await.map_err(ModelError::codec)?
        .check().map_err(ModelError::codec)?;
    let rows: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
    let [row] = rows.as_slice() else { return Err(ModelError::Conflict("incidence control expected one row")); };
    Ok(row.clone())
}
async fn physical(client: &Surreal<Client>, id: &RecordId) -> Result<Value, ModelError> {
    read_row(client, &guard_id(&Value::RecordId(id.clone()))?).await
}
fn revision(row: &Value) -> Result<i64, ModelError> {
    authorization::int_field(row.as_object().ok_or(ModelError::Schema("incidence control guard"))?, "revision")
}

#[tokio::test(flavor = "multi_thread")]
async fn equal_foreign_attempt_hold_refuses_without_incidence_mutation() {
    run_case("equal foreign hold incidence", |fixture| async move {
        let (target, row) = external(&format!("{}-foreign-hold", fixture.second.attempt().hex()));
        let owner = RecordId::new("native_attempt", fixture.second.attempt().hex());
        ensure_rows(&fixture.client, Some(fixture.second.attempt()), vec![row]).await?;
        hold(&fixture.client, Some(fixture.second.attempt()), owner.clone(), vec![target.clone()]).await?;
        let before = physical(&fixture.client, &target).await?;
        // Same-owner equal replay remains legal and cannot churn the endpoint guard.
        hold(&fixture.client, Some(fixture.second.attempt()), owner.clone(), vec![target.clone()]).await?;
        assert_eq!(physical(&fixture.client, &target).await?, before);
        let error = hold(&fixture.client, Some(fixture.first.attempt()), owner.clone(), vec![target.clone()])
            .await.unwrap_err();
        assert!(error.to_string().contains("native unrelated attempt incidence"), "{error}");
        assert!(error.permits_storage_cleanup(), "acknowledged refusal must retain its true finality: {error}");
        assert_eq!(physical(&fixture.client, &target).await?, before);
        let existing = read_row(&fixture.client, &hold_record_id(&owner, &target)?).await?;
        assert_eq!(existing.as_object().unwrap().get("owner"), Some(&Value::RecordId(owner)));
        Ok(())
    }.boxed()).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn equal_foreign_attempt_immutable_row_refuses_without_incidence_mutation() {
    run_case("equal foreign immutable incidence", |fixture| async move {
        let identity = ContentHash::of(b"foreign immutable incidence control");
        let contribution = fixture.second.begin_contribution(ContributionSpec {
            captured_binding: None,
            producer: "incidence-control".into(),
            profile: OutputProfile::Specific(Profile::Catalog),
            model: identity,
            implementation: identity,
            configuration: None,
            inputs: Vec::new(),
            outputs: Default::default(),
        }).await?;
        let record = RecordId::new("compiler_contribution", contribution.hex());
        let row = read_row(&fixture.client, &record).await?;
        let before = physical(&fixture.client, &record).await?;
        ensure_rows(&fixture.client, Some(fixture.second.attempt()), vec![row.clone()]).await?;
        assert_eq!(physical(&fixture.client, &record).await?, before);
        let error = ensure_rows(&fixture.client, Some(fixture.first.attempt()), vec![row.clone()])
            .await.unwrap_err();
        assert!(error.to_string().contains("native unrelated attempt incidence"), "{error}");
        assert!(error.permits_storage_cleanup(), "{error}");
        assert_eq!(read_row(&fixture.client, &record).await?, row);
        assert_eq!(physical(&fixture.client, &record).await?, before);
        Ok(())
    }.boxed()).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn immutable_insert_into_active_absent_guard_advances_its_revision() {
    run_case("active absent immutable incidence", |fixture| async move {
        let (record, row) = external(&format!("{}-active-absent", fixture.first.attempt().hex()));
        // Mutable operation admission may earn an active physical guard before a row
        // exists, just as retirement root registration can. It grants no immutable bytes.
        guarded_effect(&fixture.client, Some(fixture.first.attempt()), vec![record.clone()],
            "RETURN true", Variables::new()).await?;
        let before = physical(&fixture.client, &record).await?;
        assert_eq!(before.as_object().unwrap().get("phase"), Some(&Value::String("active".into())));
        let mut response = fixture.client.query("SELECT VALUE id FROM $record")
            .bind(("record", record.clone())).await.map_err(ModelError::codec)?
            .check().map_err(ModelError::codec)?;
        assert!(response.take::<Vec<RecordId>>(0).map_err(ModelError::codec)?.is_empty());
        ensure_rows(&fixture.client, Some(fixture.first.attempt()), vec![row.clone()]).await?;
        let after = physical(&fixture.client, &record).await?;
        assert_eq!(revision(&after)?, revision(&before)? + 1);
        assert_eq!(after.as_object().unwrap().get("incarnation"), before.as_object().unwrap().get("incarnation"));
        assert_eq!(read_row(&fixture.client, &record).await?, row);
        let owner = RecordId::new("native_attempt", fixture.first.attempt().hex());
        hold(&fixture.client, Some(fixture.first.attempt()), owner, vec![record]).await?;
        Ok(())
    }.boxed()).await;
}
