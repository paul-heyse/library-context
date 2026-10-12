//! Tableless finite-query controls on the explicitly borrowed installed fixture.
//! They never install a schema, mutate application content or abandon a remote stream.
use super::*;
use lctx_model::domain::{completion::{Completion, RemoteState}, resources::ResourceBudget};
use surrealdb::method::BatchItem;
use surrealdb_protocol::proto::{rpc::v1 as rpc, v1 as proto};

fn accepted(reader:&NativeReader<()>, stream:impl futures::Stream<Item=surrealdb::Result<BatchItem>>+Send+'static,
    statements:usize,selected:usize)->NativeValueOwner {
    let budget=reader.scratch_budget().unwrap();
    let charge=budget.reserve("S09 actual wire accumulation",std::mem::size_of::<Value>()).unwrap();
    NativeValueOwner::new(stream,statements,selected,budget,charge,
        QueryLease::admit(reader.operations.clone(),reader.client.clone(),None).unwrap())
}

#[tokio::test]
async fn accepted_finite_query_wire_shapes_cancellation_drop_and_terminal_errors()->Result<(),ModelError>{
    let config=crate::RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
        .ok_or(ModelError::Conflict("installed query wire fixture required"))?))?;
    let flag=Arc::new(std::sync::atomic::AtomicBool::new(false));
    let signal=crate::signal::SignalRegistration::new("S09 finite accepted-query wire owner",{
        let flag=flag.clone();move||flag.store(true,std::sync::atomic::Ordering::Release)
    })?;
    let client=connect(&config.endpoint,&config.writer_credentials(),config.namespace.as_str(),config.database.as_str()).await?;
    let budget=ResourceBudget::fixed(16<<20)?;
    let reader=NativeReader::private(client.clone()).with_budget(&budget).with_read_cancellation(flag.clone());
    let mut session_context=None;
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
    let mut completion=Completion::default();
    if let Err(error)=&result {if !error.permits_storage_cleanup(){completion.remote=RemoteState::Unknown;}completion.step("S09 actual wire control",Err(ModelError::Invalid(error.to_string())));}
    completion.step("S09 accepted query reader close",reader.close().await);
    if completion.remote==RemoteState::Confirmed {
        completion.step("S09 original session invalidation",client.invalidate().await.map_err(sdk_error));
        if let Some(context)=session_context {
            let detach=async {
                let mut wire=rpc::surreal_db_service_client::SurrealDbServiceClient::connect(config.endpoint.replacen("grpc://","http://",1)).await.map_err(ModelError::codec)?;
                wire.detach_session(rpc::DetachSessionRequest{context:Some(context)}).await.map_err(ModelError::codec)?;
                Ok(())
            }.await;
            completion.step("S09 original named session detach acknowledgement",detach);
        }
    } else {std::mem::forget(reader);std::mem::forget(client);}
    completion.step("S09 native owner receipt",signal.complete(&completion).await);
    lctx_model::domain::completion::complete(result,completion)
}
