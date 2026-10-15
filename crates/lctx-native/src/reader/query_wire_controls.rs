//! Tableless finite-query controls on the explicitly borrowed installed fixture.
//! They never install a schema, mutate application content or abandon a remote stream.
use super::*;
use lctx_model::domain::{
    completion::{Completion, RemoteState},
    resources::ResourceBudget,
};
use surrealdb::method::BatchItem;
use surrealdb_protocol::proto::{rpc::v1 as rpc, v1 as proto};

fn accepted(
    reader: &NativeReader<()>,
    stream: impl futures::Stream<Item = surrealdb::Result<BatchItem>> + Send + 'static,
    statements: usize,
    selected: usize,
) -> NativeValueOwner {
    let budget = reader.scratch_budget().unwrap();
    let charge = budget
        .reserve("S09 actual wire accumulation", std::mem::size_of::<Value>())
        .unwrap();
    NativeValueOwner::new(
        stream,
        statements,
        selected,
        budget,
        charge,
        QueryLease::admit(reader.operations.clone(), reader.client.clone(), None).unwrap(),
    )
}

#[tokio::test]
async fn accepted_finite_query_wire_shapes_cancellation_drop_and_terminal_errors()
-> Result<(), ModelError> {
    let config = crate::RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").ok_or(ModelError::Conflict(
            "installed query wire fixture required",
        ))?,
    ))?;
    let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let signal = crate::signal::SignalRegistration::new("S09 finite accepted-query wire owner", {
        let flag = flag.clone();
        move || flag.store(true, std::sync::atomic::Ordering::Release)
    })?;
    let client = connect(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
    )
    .await?;
    let budget = ResourceBudget::fixed(16 << 20)?;
    let reader = NativeReader::private(client.clone())
        .with_budget(&budget)
        .with_read_cancellation(flag.clone());
    let mut session_context = None;
    let result=async {
        let session:surrealdb::types::Uuid=reader.query_native("RETURN session::id()",Variables::new()).await?;
        session_context=Some(rpc::RequestContext{session:Some(proto::Uuid::from_uuid(session.into_inner())),..Default::default()});
        for value in [Value::None,Value::Null,7i64.into_value(),surrealdb::types::object!{answer:7i64}.into_value(),
            vec![vec![Value::None,Value::Null].into_value()].into_value(),RecordId::new("s09","query").into_value(),Bytes::from(vec![0u8,255]).into_value()] {
            let mut vars=Variables::new();vars.insert("value",value.clone());
            let mut buffered=client.query("RETURN $value").bind(vars.clone()).await.map_err(sdk_error)?.check().map_err(sdk_error)?;
            assert_eq!(buffered.num_statements(),1);
            let expected:Value=buffered.take(0).map_err(sdk_error)?;
            let actual:Value=reader.query_native("RETURN $value",vars).await?;
            assert_eq!(actual,expected);assert_eq!(actual,value);
            assert_eq!(budget.reserved(),0);
        }
        for sql in ["SELECT * FROM []","SELECT * FROM [{answer:7}]","SELECT * FROM ONLY [{answer:7}]","RETURN [[1,NONE],NULL]"] {
            let mut buffered=client.query(sql).await.map_err(sdk_error)?.check().map_err(sdk_error)?;
            let expected:Value=buffered.take(0).map_err(sdk_error)?;
            assert_eq!(reader.query_native::<Value>(sql,Variables::new()).await?,expected);
        }
        let prepared=crate::prepared::PreparedQuery::new(Variables::new(),vec!["SELECT VALUE 19 FROM [0]".into()],vec!["RETURN 42".into()])?;
        assert_eq!(reader.query_prepared_native::<Value>(prepared).await?,42i64.into_value());
        // The first delivered preparation item acknowledges actual remote execution.
        // Keep it in the exact owner, then cancel/drop before the selected result.
        for cancel in [true,false] {
            let local=NativeReader::private(client.clone()).with_budget(&budget);
            let mut stream=client.query("SELECT VALUE session::id() FROM [0]; RETURN NONE; RETURN 42").stream_batches().map_err(sdk_error)?;
            let first=stream.next().await.ok_or(ModelError::Schema("S09 missing acknowledgement"))?;
            assert!(matches!(first,Ok(BatchItem::Rows{index:0,..})|Ok(BatchItem::Value{index:0,..})));
            let mut owner=accepted(&local,futures::stream::once(async move{first}).chain(stream),3,2);
            if cancel {
                let cancelled=Arc::new(std::sync::atomic::AtomicBool::new(true));
                let error=owner.collect(Some(cancelled)).await.err().ok_or(ModelError::Schema("S09 cancellation accepted output"))?;
                assert!(error.to_string().contains("cancelled"));assert!(error.permits_storage_cleanup());
            } else {drop(owner);}
            local.close().await?;
            assert_eq!(local.operations.lock().unwrap().active,0);
            assert_eq!(budget.reserved(),0);
        }
        let local=NativeReader::private(client.clone()).with_budget(&budget);
        let mut vars=Variables::new();vars.insert("rows",vec![surrealdb::types::object!{value:"row"};2048]);
        let mut stream=client.query("SELECT * FROM $rows").bind(vars).stream_batches().map_err(sdk_error)?;
        let first=stream.next().await.ok_or(ModelError::Schema("S09 missing row batch"))?;
        assert!(matches!(&first,Ok(BatchItem::Rows{index:0,values}) if !values.is_empty()));
        drop(accepted(&local,futures::stream::once(async move{first}).chain(stream),1,0));
        local.close().await?;assert_eq!(budget.reserved(),0);
        let local=NativeReader::private(client.clone()).with_budget(&budget);
        let mut stream=client.query("SELECT VALUE session::id() FROM [0]; RETURN NONE; THROW 'S09 late statement failure'").stream_batches().map_err(sdk_error)?;
        let first=stream.next().await.ok_or(ModelError::Schema("S09 missing late-error acknowledgement"))?;
        drop(accepted(&local,futures::stream::once(async move{first}).chain(stream),3,2));
        let error=local.close().await.err().ok_or(ModelError::Schema("S09 abandoned late error lost"))?;
        assert!(error.to_string().contains("S09 late statement failure"));assert!(error.permits_storage_cleanup());
        assert!(local.check_read_admission().is_err(),"confirmed late error still closes reader admission");
        assert_eq!(local.operations.lock().unwrap().active,0);assert_eq!(budget.reserved(),0);
        // Observe actual QueryEnd counts and physical tonic EOF independently of the SDK.
        let mut wire=rpc::surreal_db_service_client::SurrealDbServiceClient::connect(config.endpoint.replacen("grpc://","http://",1)).await.map_err(ModelError::codec)?;
        let context=session_context.clone().ok_or(ModelError::Conflict("S09 original session context"))?;
        let mut stream=wire.query(rpc::QueryRequest{context:Some(context.clone()),query:"SELECT * FROM []; RETURN NONE; RETURN NULL".into(),max_batch_records:1,..Default::default()}).await.map_err(ModelError::codec)?.into_inner();
        let mut terminals=0;let mut ended=false;
        while let Some(response)=stream.message().await.map_err(ModelError::codec)? {
            assert!(!ended,"payload after QueryEnd");
            match response.frame {
                Some(rpc::query_response::Frame::Begin(begin))=>assert_eq!(begin.statement_count,3),
                Some(rpc::query_response::Frame::Batch(batch))=>{
                    assert!(batch.error.is_none());
                    if batch.kind==rpc::QueryResponseKind::Single as i32 || batch.kind==rpc::QueryResponseKind::BatchedFinal as i32 {assert_eq!(batch.query_index,terminals);terminals+=1;}
                },
                Some(rpc::query_response::Frame::End(end))=>{assert_eq!(end.result_count,3);assert_eq!(terminals,3);ended=true;},
                _=>return Err(ModelError::Schema("S09 normal wire frame")),
            }
        }
        assert!(ended,"physical EOF must follow logical QueryEnd");
        let malformed=wire.query(rpc::QueryRequest{context:Some(context),query:"SELECT * FROM".into(),..Default::default()}).await;
        match malformed {
            Ok(response)=>{
                let mut stream=response.into_inner();let mut failed=false;
                while let Some(response)=stream.next().await {
                    match response {
                        Ok(response)=>match response.frame {
                            Some(rpc::query_response::Frame::Error(_))=>failed=true,
                            Some(rpc::query_response::Frame::Batch(batch)) if batch.error.is_some()=>failed=true,
                            _=>{},
                        },
                        Err(_)=>failed=true,
                    }
                }
                assert!(failed,"malformed query needs a wire failure before physical EOF");
            },
            Err(status)=>assert_ne!(status.code(),tonic::Code::Ok),
        }
        assert_eq!(reader.query_native::<surrealdb::types::Uuid>("RETURN session::id()",Variables::new()).await?,session);
        Ok(())
    }.await;
    let mut completion = Completion::default();
    if let Err(error) = &result {
        if !error.permits_storage_cleanup() {
            completion.remote = RemoteState::Unknown;
        }
        completion.step(
            "S09 actual wire control",
            Err(ModelError::Invalid(error.to_string())),
        );
    }
    completion.step("S09 accepted query reader close", reader.close().await);
    if completion.remote == RemoteState::Confirmed {
        completion.step(
            "S09 original session invalidation",
            client.invalidate().await.map_err(sdk_error),
        );
        if let Some(context) = session_context {
            let detach = async {
                let mut wire = rpc::surreal_db_service_client::SurrealDbServiceClient::connect(
                    config.endpoint.replacen("grpc://", "http://", 1),
                )
                .await
                .map_err(ModelError::codec)?;
                wire.detach_session(rpc::DetachSessionRequest {
                    context: Some(context),
                })
                .await
                .map_err(ModelError::codec)?;
                Ok(())
            }
            .await;
            completion.step("S09 original named session detach acknowledgement", detach);
        }
    } else {
        std::mem::forget(reader);
        std::mem::forget(client);
    }
    completion.step(
        "S09 native owner receipt",
        signal.complete(&completion).await,
    );
    lctx_model::domain::completion::complete(result, completion)
}

/// Invoked only by the parent below, with the parent's real fixture binding.
#[tokio::test]
#[ignore = "owned direct-process native signal child"]
async fn direct_native_signal_child() -> Result<(), ModelError> {
    let ready = std::path::PathBuf::from(
        std::env::var_os("LCTX_NATIVE_SIGNAL_READY")
            .ok_or(ModelError::Conflict("owned direct signal child required"))?,
    );
    let config = crate::RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").ok_or(ModelError::Conflict(
            "installed direct signal fixture required",
        ))?,
    ))?;
    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let wake = Arc::new(tokio::sync::Notify::new());
    let signal = crate::signal::SignalRegistration::new("D10 direct native query signal owner", {
        let cancelled = cancelled.clone();
        let wake = wake.clone();
        move || {
            cancelled.store(true, std::sync::atomic::Ordering::Release);
            wake.notify_one();
        }
    })?;
    let client = connect(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
    )
    .await?;
    let budget = ResourceBudget::fixed(16 << 20)?;
    let reader = NativeReader::private(client.clone()).with_budget(&budget);
    let mut context = None;
    let result = async {
        let session: surrealdb::types::Uuid = reader
            .query_native("RETURN session::id()", Variables::new())
            .await?;
        context = Some(rpc::RequestContext {
            session: Some(proto::Uuid::from_uuid(session.into_inner())),
            ..Default::default()
        });
        let mut stream = client
            .query("SELECT VALUE session::id() FROM [0]; RETURN NONE; RETURN 42")
            .stream_batches()
            .map_err(sdk_error)?;
        let first = stream.next().await.ok_or(ModelError::Schema(
            "direct signal remote acknowledgement missing",
        ))?;
        if !matches!(
            first,
            Ok(BatchItem::Rows { index: 0, .. }) | Ok(BatchItem::Value { index: 0, .. })
        ) {
            return Err(ModelError::Schema("direct signal first remote frame"));
        }
        // Retain the accepted physical stream before exposing readiness. The OS
        // signal affects delivery; collect still consumes all terminals and EOF.
        let mut owner = accepted(
            &reader,
            futures::stream::once(async move { first }).chain(stream),
            3,
            2,
        );
        {
            use std::{io::Write, os::unix::fs::OpenOptionsExt};
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&ready)
                .map_err(ModelError::codec)?;
            file.write_all(b"remote first frame acknowledged; accepted owner retained")
                .map_err(ModelError::codec)?;
            file.sync_all().map_err(ModelError::codec)?;
        }
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let notified = wake.notified();
                if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                    break;
                }
                notified.await;
            }
        })
        .await
        .map_err(ModelError::codec)?;
        let error =
            owner
                .collect(Some(cancelled.clone()))
                .await
                .err()
                .ok_or(ModelError::Schema(
                    "direct signal delivered cancelled result",
                ))?;
        if !error.to_string().contains("cancelled") || !error.permits_storage_cleanup() {
            return Err(error);
        }
        reader.close().await?;
        assert_eq!(reader.operations.lock().unwrap().active, 0);
        assert_eq!(budget.reserved(), 0);
        assert!(
            reader.check_read_admission().is_err(),
            "drained reader remains closed"
        );
        Ok(())
    }
    .await;
    let mut completion = Completion::default();
    if let Err(error) = &result {
        if !error.permits_storage_cleanup() {
            completion.remote = RemoteState::Unknown;
        }
        completion.step(
            "D10 actual native signal control",
            Err(ModelError::Invalid(error.to_string())),
        );
    }
    completion.step("D10 accepted native query close", reader.close().await);
    if completion.remote == RemoteState::Confirmed {
        completion.step(
            "D10 session invalidation",
            client.invalidate().await.map_err(sdk_error),
        );
        if let Some(context) = context {
            completion.step(
                "D10 named session detach acknowledgement",
                async {
                    let mut wire = rpc::surreal_db_service_client::SurrealDbServiceClient::connect(
                        config.endpoint.replacen("grpc://", "http://", 1),
                    )
                    .await
                    .map_err(ModelError::codec)?;
                    wire.detach_session(rpc::DetachSessionRequest {
                        context: Some(context),
                    })
                    .await
                    .map_err(ModelError::codec)?;
                    Ok(())
                }
                .await,
            );
        }
    } else {
        std::mem::forget(reader);
        std::mem::forget(client);
    }
    completion.step(
        "D10 native owner receipt",
        signal.complete(&completion).await,
    );
    lctx_model::domain::completion::complete(result, completion)
}

#[test]
fn actual_direct_native_sigint_and_sigterm_drain_remote_queries() {
    use std::{
        process::Command,
        time::{Duration, Instant},
    };
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let directory = std::path::PathBuf::from(
        std::env::var_os("LCTX_NATIVE_DRAIN_DIRECTORY")
            .expect("actual fixture receipt owner required"),
    );
    assert!(std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").is_some());
    for (name, flag) in [("SIGINT", "-INT"), ("SIGTERM", "-TERM")] {
        let ready = directory.join(format!("direct-native-ready-{}-{name}", std::process::id()));
        let mut child = Child(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "reader::query_wire_controls::direct_native_signal_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("LCTX_NATIVE_SIGNAL_READY", &ready)
                .spawn()
                .unwrap(),
        );
        let until = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "native child exited before first remote frame"
            );
            assert!(
                Instant::now() < until,
                "native child did not acknowledge first remote frame"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            Command::new("kill")
                .arg(flag)
                .arg(child.0.id().to_string())
                .status()
                .unwrap()
                .success()
        );
        let until = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            assert!(
                Instant::now() < until,
                "native signal child did not join after remote drain"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(status.success(), "direct native {name} control failed");
        let receipts = std::fs::read_dir(&directory)
            .unwrap()
            .filter_map(|entry| {
                let path = entry.unwrap().path();
                if path.extension().is_none_or(|extension| extension != "json") {
                    return None;
                }
                let row: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
                (row["owner"] == "D10 direct native query signal owner"
                    && row["process"]["pid"] == child.0.id())
                .then_some(row)
            })
            .collect::<Vec<_>>();
        assert_eq!(receipts.len(), 1);
        let receipt = &receipts[0];
        assert_eq!(receipt["state"], "complete");
        assert_eq!(receipt["signal"], name);
        assert_eq!(receipt["completion"]["local"], "terminal");
        assert_eq!(receipt["completion"]["remote"], "confirmed");
        assert_eq!(receipt["completion"]["storage"], serde_json::json!([]));
        assert_eq!(receipt["completion"]["failures"], serde_json::json!([]));
        std::fs::remove_file(ready).unwrap();
    }
}
