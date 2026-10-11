//! Dedicated root-owned campaign client. Never opens an installed fixture.
use std::{io::Write, time::Duration};
use surrealdb_protocol::proto::rpc::v1 as rpc;
use rpc::surreal_db_service_client::SurrealDbServiceClient;

#[tokio::test]
async fn dedicated_grpc_campaign() {
    let endpoint = std::env::var("EFFICIENCY_GRPC_ENDPOINT").expect("dedicated endpoint");
    let sql = std::env::var("EFFICIENCY_GRPC_SQL").expect("explicit campaign SQL");
    let drop_after: usize = std::env::var("EFFICIENCY_GRPC_DROP_AFTER").unwrap_or("0".into()).parse().unwrap();
    let mut client = SurrealDbServiceClient::connect(endpoint).await.unwrap();
    let attached = client.attach_session(rpc::AttachSessionRequest::default()).await.unwrap().into_inner();
    let context = rpc::RequestContext { session: attached.session, ..Default::default() };
    client.signin(rpc::SigninRequest {
        context: Some(context.clone()),
        access_method: Some(rpc::AccessMethod { method: Some(rpc::access_method::Method::User(rpc::UserCredentials {
            username: std::env::var("EFFICIENCY_GRPC_USER").expect("dedicated user"),
            password: std::env::var("EFFICIENCY_GRPC_PASSWORD").expect("dedicated password"),
            ..Default::default()
        })) }),
    }).await.unwrap();
    let some = |s: &str| rpc::NullableString { value: Some(rpc::nullable_string::Value::Some(s.into())) };
    client.r#use(rpc::UseRequest { context: Some(context.clone()), namespace: Some(some("efficiency")), database: Some(some("disconnect")) }).await.unwrap();
    let mut stream = client.query(rpc::QueryRequest { context: Some(context.clone()), query: sql, max_batch_records: 1, ..Default::default() }).await.unwrap().into_inner();
    let mut count = 0;
    while let Some(response) = stream.message().await.unwrap() {
        count += 1;
        let kind = match response.frame { Some(rpc::query_response::Frame::Begin(_)) => "begin", Some(rpc::query_response::Frame::Batch(ref b)) => { println!("batch_index={} statement={} kind={} error={}", b.batch_index,b.query_index,b.kind,b.error.is_some()); "batch" }, Some(rpc::query_response::Frame::End(_)) => "end", Some(rpc::query_response::Frame::Error(_)) => "error", None => "none" };
        println!("frame={count} kind={kind}");
        std::io::stdout().flush().unwrap();
        if drop_after > 0 && count >= drop_after {
            let delay: u64 = std::env::var("EFFICIENCY_GRPC_DROP_DELAY_MS").unwrap_or("0".into()).parse().unwrap();
            tokio::time::sleep(Duration::from_millis(delay)).await;
            break;
        }
    }
    drop(stream);
    println!("stream_dropped frames={count}");
    std::io::stdout().flush().unwrap();
    tokio::time::sleep(Duration::from_secs(3)).await;
    client.detach_session(rpc::DetachSessionRequest { context: Some(context) }).await.unwrap();
    println!("session_detached");
}
