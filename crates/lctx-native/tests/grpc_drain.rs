//! Actual installed-fixture wire controls. The server lifecycle belongs to the
//! fixture/maintenance owner. In particular, a lost QueryEnd stays unknown even
//! if later reads, owned session detach, and the Rust test body succeed.
use lctx_model::domain::{
    ContentHash, ModelError,
    completion::{Completion, RemoteState, StorageState},
};
use lctx_native::{control::RetryCancellation, signal::SignalRegistration};
use rpc::surreal_db_service_client::SurrealDbServiceClient;
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    time::Duration,
};
use surrealdb::types::Value;
use surrealdb_protocol::proto::{rpc::v1 as rpc, v1 as proto};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Client = SurrealDbServiceClient<tonic::transport::Channel>;
type Stream = tonic::Streaming<rpc::QueryResponse>;

struct Wire {
    client: Client,
    context: rpc::RequestContext,
    cancellation: RetryCancellation,
    signal: SignalRegistration,
    table: String,
    sequence: u64,
    session: Option<String>,
    generation: String,
    recovery: PathBuf,
}
#[derive(Default)]
struct Observed {
    values: BTreeMap<u32, Vec<Value>>,
    errors: Vec<serde_json::Value>,
    end: bool,
    stream_error: bool,
}
impl Wire {
    async fn open() -> Result<Self> {
        let path = std::env::var("LCTX_SURREAL_TEST_CONFIG")?;
        let config: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
        let generation: ContentHash = serde_json::from_value(config["service_generation"].clone())?;
        assert_eq!(generation.hex(), std::env::var("LCTX_SURREAL_GENERATION")?);
        let cancellation = RetryCancellation::default();
        let signal =
            SignalRegistration::retry("PD02 installed fixture wire owner", cancellation.clone())?;
        let table = format!(
            "pd02_{}",
            lctx_native::control::fresh_identity("grpc-wire-control")?.hex()
        );
        let endpoint = config["grpc_endpoint"]
            .as_str()
            .ok_or("fixture gRPC endpoint")?
            .replacen("grpcs://", "https://", 1)
            .replacen("grpc://", "http://", 1);
        let client = Client::connect(endpoint).await?;
        let context = rpc::RequestContext::default();
        let recovery = PathBuf::from(std::env::var("LCTX_PD02_RECOVERY_DIRECTORY")?);
        if !recovery.is_absolute() || recovery.canonicalize()? != recovery {
            return Err("physical attachment recovery directory required".into());
        }
        let mut wire = Self {
            client,
            context,
            cancellation,
            signal,
            table,
            sequence: 0,
            session: None,
            generation: generation.hex(),
            recovery,
        };
        wire.persist("attach_session", b"attach_session")?;
        let attached = wire
            .client
            .attach_session(rpc::AttachSessionRequest::default())
            .await?
            .into_inner();
        assert!(
            attached.created,
            "this control owns a newly allocated session"
        );
        let allocated = attached.session.ok_or("missing allocated session ID")?;
        if allocated.bytes.len() != 16 {
            return Err("invalid allocated session UUID".into());
        }
        let encoded = hex::encode(&allocated.bytes);
        wire.session = Some(format!(
            "{}-{}-{}-{}-{}",
            &encoded[..8],
            &encoded[8..12],
            &encoded[12..16],
            &encoded[16..20],
            &encoded[20..]
        ));
        wire.context.session = Some(allocated);
        let ns = config["namespace"].as_str().ok_or("fixture namespace")?;
        let db = config["database"].as_str().ok_or("fixture database")?;
        println!(
            "{}",
            json!({"pd02":"owner", "generation":generation.hex(),
            "namespace":ns,"database":db,"session":wire.session,"table":wire.table,
            "attachment":std::env::var("LCTX_FIXTURE_ATTACHMENT")?})
        );
        wire.persist("signin", b"signin")?;
        wire.client
            .signin(rpc::SigninRequest {
                context: Some(wire.context.clone()),
                access_method: Some(rpc::AccessMethod {
                    method: Some(rpc::access_method::Method::User(rpc::UserCredentials {
                        namespace: ns.into(),
                        database: db.into(),
                        username: config["admin_user"].as_str().ok_or("fixture user")?.into(),
                        password: config["admin_password"]
                            .as_str()
                            .ok_or("fixture password")?
                            .into(),
                        ..Default::default()
                    })),
                }),
            })
            .await?;
        let some = |s: &str| rpc::NullableString {
            value: Some(rpc::nullable_string::Value::Some(s.into())),
        };
        wire.persist("use", b"validation namespace and database")?;
        wire.client
            .r#use(rpc::UseRequest {
                context: Some(wire.context.clone()),
                namespace: Some(some(ns)),
                database: Some(some(db)),
            })
            .await?;
        let setup = wire
            .query(
                "define_table",
                format!("DEFINE TABLE {} SCHEMALESS", wire.table),
                None,
            )
            .await?;
        setup.checked()?;
        Ok(wire)
    }
    fn persist(&mut self, policy: &str, payload: &[u8]) -> Result<String> {
        let effects = match policy {
            "normal_commit" => json!({"normal":"present"}),
            "before_poll" => json!({"before_poll":"absent"}),
            "inflight_transaction" => json!({"inflight":"absent"}),
            "committed_before_disconnect" => json!({"committed":"present","later":"absent"}),
            "deadline" => json!({"deadline":"absent"}),
            "attach_session"
            | "signin"
            | "use"
            | "detach_session"
            | "define_table"
            | "session_read"
            | "table_rows"
            | "table_info"
            | "remove_table"
            | "delivery_backpressure"
            | "no_output_window"
            | "late_live_query" => json!({}),
            _ => return Err("unknown finite PD02 reconciliation policy".into()),
        };
        let request = format!("{}:{}", self.table, self.sequence);
        let path = self
            .recovery
            .join(format!("{}-{}.json", &self.table[5..], self.sequence));
        let body = json!({"schema":1,"kind":"PD02-wire-request",
            "attachment":std::env::var("LCTX_FIXTURE_ATTACHMENT")?, "generation":self.generation,
            "namespace":"library_context","database":"validation", "table":self.table,
            "process":{"pid":std::process::id(),"start_ticks":fs::read_to_string("/proc/self/stat")?
                .rsplit_once(") ").and_then(|(_,tail)|tail.split_whitespace().nth(19))
                .ok_or("wire owner process start ticks")?.parse::<u64>()?},
            "sequence":self.sequence,"session":self.session,"policy":policy,"effects":effects,
            "live":"absent_after_join","session_policy":"volatile_named_session_after_join",
            "sql_blake3":ContentHash::of(payload).hex()});
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(&serde_json::to_vec_pretty(&body)?)?;
        file.sync_all()?;
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o400))?;
        file.sync_all()?;
        fs::File::open(&self.recovery)?.sync_all()?;
        self.sequence += 1;
        Ok(request)
    }
    async fn start(
        &mut self,
        policy: &str,
        sql: String,
        timeout: Option<proto::Duration>,
    ) -> Result<(String, Stream)> {
        if self.cancellation.stopped() {
            return Err("wire owner cancelled before new request".into());
        }
        let request = self.persist(policy, sql.as_bytes())?;
        let mut context = self.context.clone();
        context.timeout = timeout;
        // SQL contains only this test's exact table/record identities, never
        // credentials. Retain it so maintenance can reconcile the unknown run.
        println!(
            "{}",
            json!({"pd02":"request","request":request,"session":context.session,"sql":sql})
        );
        let stream = self
            .client
            .query(rpc::QueryRequest {
                context: Some(context),
                query: sql,
                max_batch_records: 1,
                ..Default::default()
            })
            .await?
            .into_inner();
        Ok((request, stream))
    }
    async fn query(
        &mut self,
        policy: &str,
        sql: String,
        timeout: Option<proto::Duration>,
    ) -> Result<Observed> {
        let (request, mut stream) = self.start(policy, sql, timeout).await?;
        let mut observed = Observed::default();
        while let Some(frame) = stream.message().await? {
            observed.frame(&request, frame)?;
        }
        if !observed.end && !observed.stream_error {
            return Err(format!("{request}: missing terminal before transport EOF").into());
        }
        println!(
            "{}",
            json!({"pd02":"terminal","request":request,"remote":"confirmed",
            "kind":if observed.end {"query_end"} else {"stream_error"},"errors":observed.errors})
        );
        Ok(observed)
    }
    async fn abandon(
        &mut self,
        case: &str,
        sql: String,
        batches: usize,
        delay: Duration,
    ) -> Result<()> {
        let (request, mut stream) = self.start(case, sql, None).await?;
        if case != "before_poll" {
            let first = stream.message().await?.ok_or("missing QueryBegin")?;
            if !matches!(first.frame, Some(rpc::query_response::Frame::Begin(_))) {
                return Err("first wire frame was not QueryBegin".into());
            }
            let mut observed = Observed::default();
            let mut seen = 0;
            while seen < batches {
                let frame = stream
                    .message()
                    .await?
                    .ok_or("wire ended before selected batch")?;
                if matches!(frame.frame, Some(rpc::query_response::Frame::Batch(_))) {
                    seen += 1;
                }
                observed.frame(&request, frame)?;
                if !observed.errors.is_empty() || observed.end {
                    return Err("query ended before selected abandonment".into());
                }
            }
        }
        // The delay provides an overlap window, not an acknowledgement that an
        // executor reached a particular internal instruction or transaction.
        tokio::time::sleep(delay).await;
        drop(stream);
        println!(
            "{}",
            json!({"pd02":"abandoned","case":case,"request":request,
            "session":self.context.session,"table":self.table,"remote":"unknown"})
        );
        Ok(())
    }
    async fn finish(mut self, mut completion: Completion, outcome: Result<()>) -> Result<()> {
        if let Err(error) = &outcome {
            completion.remote = RemoteState::Unknown;
            completion.storage.push(StorageState::Orphan(format!(
                "PD02 owned table {}",
                self.table
            )));
            completion.step(
                "PD02 wire control",
                Err(ModelError::Invalid(error.to_string())),
            );
        }
        self.persist("detach_session", b"detach_session")?;
        completion.step(
            "PD02 owned named session detach",
            self.client
                .detach_session(rpc::DetachSessionRequest {
                    context: Some(self.context.clone()),
                })
                .await
                .map(|_| ())
                .map_err(ModelError::codec),
        );
        let result = self.signal.complete(&completion).await;
        completion.step("PD02 native signal receipt", result);
        println!(
            "{}",
            json!({"pd02":"completion","table":self.table,"session":self.context.session,
            "remote":if completion.remote==RemoteState::Unknown {"unknown"} else {"confirmed"},
            "failures":completion.failures.iter().map(|f|f.error.to_string()).collect::<Vec<_>>()})
        );
        outcome?;
        if !completion.failures.is_empty() {
            return Err("PD02 owner cleanup failed; see retained receipt".into());
        }
        Ok(())
    }
}
impl Observed {
    fn frame(&mut self, request: &str, response: rpc::QueryResponse) -> Result<()> {
        if self.end || self.stream_error {
            return Err("wire frame after terminal".into());
        }
        match response.frame {
            Some(rpc::query_response::Frame::Begin(_)) => {}
            Some(rpc::query_response::Frame::Batch(batch)) => {
                println!(
                    "{}",
                    json!({"pd02":"batch","request":request,
                    "statement":batch.query_index,"batch":batch.batch_index,"kind":batch.kind,"error":batch.error})
                );
                if let Some(error) = batch.error {
                    self.errors.push(serde_json::to_value(error)?);
                }
                if let Some(rpc::query_batch_frame::Payload::Values(values)) = batch.payload {
                    for value in values.values {
                        self.values
                            .entry(batch.query_index)
                            .or_default()
                            .push(Value::try_from(value)?);
                    }
                }
            }
            Some(rpc::query_response::Frame::End(_)) => {
                self.end = true;
            }
            Some(rpc::query_response::Frame::Error(error)) => {
                self.stream_error = true;
                self.errors.push(serde_json::to_value(error)?);
            }
            None => return Err("unrecognized wire frame".into()),
        }
        Ok(())
    }
    fn checked(&self) -> Result<()> {
        if !self.errors.is_empty() {
            return Err(format!("query errors: {}", json!(self.errors)).into());
        }
        Ok(())
    }
    fn first(&self) -> Result<&Value> {
        self.checked()?;
        self.values
            .values()
            .flatten()
            .next()
            .ok_or_else(|| "missing query value".into())
    }
}

#[tokio::test]
async fn grpc_normal_eof_commits_and_preserves_exact_named_session() -> Result<()> {
    let mut wire = Wire::open().await?;
    let outcome = async {
        let before = wire
            .query("session_read", "RETURN session::id()".into(), None)
            .await?
            .first()?
            .clone();
        wire.query(
            "normal_commit",
            format!(
                "BEGIN; CREATE {}:normal SET phase='committed'; COMMIT",
                wire.table
            ),
            None,
        )
        .await?
        .checked()?;
        let row = wire
            .query(
                "table_rows",
                format!("SELECT * FROM ONLY {}:normal", wire.table),
                None,
            )
            .await?;
        assert!(row.first()?.as_object().is_some());
        let after = wire
            .query("session_read", "RETURN session::id()".into(), None)
            .await?
            .first()?
            .clone();
        assert_eq!(before, after, "normal EOF keeps the client's named session");
        wire.query("remove_table", format!("REMOVE TABLE {}", wire.table), None)
            .await?
            .checked()?;
        Ok(())
    }
    .await;
    wire.finish(Completion::default(), outcome).await
}

#[tokio::test]
async fn grpc_abandoned_runs_report_exact_unknown_owners_and_checked_observations() -> Result<()> {
    let mut wire = Wire::open().await?;
    // These are intentionally protected after test exit. Only the dedicated
    // maintenance owner's joined server registry/recovery can release them.
    let mut completion = Completion {
        remote: RemoteState::Unknown,
        ..Default::default()
    };
    completion.storage.push(StorageState::Orphan(format!(
        "PD02 owned table {}",
        wire.table
    )));
    let outcome = async {
        wire.abandon(
            "before_poll",
            format!("BEGIN; CREATE {}:before_poll; SLEEP 2s; COMMIT", wire.table),
            0,
            Duration::ZERO,
        )
        .await?;
        wire.abandon(
            "inflight_transaction",
            format!("BEGIN; CREATE {}:inflight; SLEEP 2s; COMMIT", wire.table),
            0,
            Duration::from_millis(250),
        )
        .await?;
        wire.abandon(
            "committed_before_disconnect",
            format!(
                "CREATE {}:committed; SLEEP 2s; CREATE {}:later",
                wire.table, wire.table
            ),
            1,
            Duration::ZERO,
        )
        .await?;
        // The retained payload exceeds HTTP/2's ordinary receive window. Pause
        // after its first batch to exercise a delivery-blocked producer; exact
        // channel occupancy is established only by the server's native controls.
        let payload = serde_json::to_string(&vec!["x".repeat(1024); 2048])?;
        wire.abandon(
            "delivery_backpressure",
            format!("RETURN {payload}; SLEEP 2s"),
            1,
            Duration::from_millis(100),
        )
        .await?;
        wire.abandon(
            "no_output_window",
            "SLEEP 2s; RETURN 42".into(),
            0,
            Duration::from_millis(100),
        )
        .await?;
        wire.abandon(
            "late_live_query",
            format!("LIVE SELECT * FROM {}; SLEEP 2s", wire.table),
            1,
            Duration::ZERO,
        )
        .await?;
        // Follow-up reads report observations only; they do not upgrade any
        // abandoned request to confirmed remote completion.
        tokio::time::sleep(Duration::from_secs(3)).await;
        let rows = wire
            .query("table_rows", format!("SELECT id FROM {}", wire.table), None)
            .await?;
        rows.checked()?;
        println!(
            "{}",
            json!({"pd02":"owned_rows_observed","table":wire.table,"rows":rows.values})
        );
        let info = wire
            .query("table_info", format!("INFO FOR TABLE {}", wire.table), None)
            .await?;
        let lives = info
            .first()?
            .as_object()
            .and_then(|row| row.get("lives"))
            .and_then(Value::as_object)
            .ok_or("table live-query inventory")?;
        assert!(
            lives.is_empty(),
            "abandoned LIVE query must be reconciled by the server owner"
        );
        let deadline = wire
            .query(
                "deadline",
                format!("BEGIN; CREATE {}:deadline; SLEEP 2s; COMMIT", wire.table),
                Some(proto::Duration {
                    seconds: 0,
                    nanos: 200_000_000,
                }),
            )
            .await?;
        assert!(
            !deadline.errors.is_empty(),
            "the exact server deadline must remain effective"
        );
        let rows = wire
            .query("table_rows", format!("SELECT id FROM {}", wire.table), None)
            .await?;
        rows.checked()?;
        println!(
            "{}",
            json!({"pd02":"after_deadline_rows_observed","table":wire.table,"rows":rows.values})
        );
        Ok(())
    }
    .await;
    wire.finish(completion, outcome).await
}
