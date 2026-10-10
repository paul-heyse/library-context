//! Owned loopback SurrealDB 0.13.1 protocol peer. This injects faults, not engine behavior.
//! Real SDK 3.3.0 file export consumes these frames; no replacement export validator is used.
use futures::{Stream, StreamExt};
use rpc::surreal_db_service_server::{SurrealDbService, SurrealDbServiceServer};
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use surrealdb_protocol::proto::{rpc::v1 as rpc, v1 as proto};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpListener, TcpStream},
    sync::{Notify, mpsc, oneshot, watch},
};
use tonic::{
    Request, Response, Status,
    transport::{Server, server::Connected},
};

pub const PARTIAL: &[u8] = b"-- provisional SDK export bytes\n";
#[derive(Clone, Copy, Debug)]
pub enum Fault {
    Success,
    LateEngineError,
    LateTaskError,
    MissingTrailer,
    ByteCountMismatch,
    TransportClose,
}
#[derive(Clone, Copy, Debug)]
pub enum QueryFault { Success, LateStatementError, OuterError, MissingOuterEnd, LateTransportError, LatePayload }
type Frames<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

pub struct Fixture {
    pub endpoint: String,
    terminal: Arc<Notify>,
    pub provisional: Arc<Notify>,
    pub events: Arc<Mutex<Vec<&'static str>>>,
    socket_stop: watch::Sender<bool>,
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<Result<(), tonic::transport::Error>>>,
}
impl Fixture {
    pub async fn start(fault: Fault) -> Self { Self::start_inner(fault, None).await }
    pub async fn start_query(fault: QueryFault) -> Self { Self::start_inner(Fault::Success, Some(fault)).await }
    async fn start_inner(fault: Fault, query_fault: Option<QueryFault>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("grpc://{}", listener.local_addr().unwrap());
        let terminal = Arc::new(Notify::new());
        let provisional = Arc::new(Notify::new());
        let events = Arc::new(Mutex::new(Vec::new()));
        let (socket_stop, sockets) = watch::channel(false);
        let incoming =
            tokio_stream::wrappers::TcpListenerStream::new(listener).map(move |stream| {
                let mut stopped = sockets.clone();
                stream.map(|stream| Socket {
                    stream,
                    stop: Box::pin(async move {
                        let _ = stopped.wait_for(|stop| *stop).await;
                    }),
                })
            });
        let service = Peer {
            fault,
            terminal: terminal.clone(),
            provisional: provisional.clone(), events: events.clone(), query_fault,
        };
        let (shutdown, stop) = oneshot::channel();
        let task = tokio::spawn(
            Server::builder()
                .add_service(SurrealDbServiceServer::new(service))
                .serve_with_incoming_shutdown(incoming, async {
                    let _ = stop.await;
                }),
        );
        Self {
            endpoint,
            terminal, provisional, events,
            socket_stop,
            shutdown: Some(shutdown),
            task: Some(task),
        }
    }
    pub fn release_terminal(&self) {
        self.terminal.notify_one();
    }
    // Interrupt the actual accepted TCP connections, after the test observes partial SDK bytes.
    pub fn disconnect(&self) {
        self.socket_stop.send(true).unwrap();
    }
    pub async fn close(mut self) {
        self.terminal.notify_waiters();
        let _ = self.socket_stop.send(true);
        let _ = self.shutdown.take().unwrap().send(());
        let mut task = self.task.take().unwrap();
        match tokio::time::timeout(std::time::Duration::from_secs(5), &mut task).await {
            Ok(result) => result.unwrap().unwrap(),
            Err(error) => {
                task.abort();
                let _ = task.await;
                panic!("owned protocol peer did not drain: {error}");
            }
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.socket_stop.send(true);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

// Socket interruption is local to this fault peer; it does not alter SDK or production reads.
struct Socket {
    stream: TcpStream,
    stop: Pin<Box<dyn std::future::Future<Output = ()> + Send>>,
}
impl Connected for Socket {
    type ConnectInfo = <TcpStream as Connected>::ConnectInfo;
    fn connect_info(&self) -> Self::ConnectInfo {
        self.stream.connect_info()
    }
}
fn disconnected() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::ConnectionReset,
        "injected export transport disconnect",
    )
}
impl AsyncRead for Socket {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.stop.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Err(disconnected()));
        }
        Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}
impl AsyncWrite for Socket {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        if self.stop.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Err(disconnected()));
        }
        Pin::new(&mut self.stream).poll_write(cx, bytes)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if self.stop.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Err(disconnected()));
        }
        Pin::new(&mut self.stream).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}
struct Peer {
    fault: Fault,
    terminal: Arc<Notify>,
    provisional: Arc<Notify>,
    events: Arc<Mutex<Vec<&'static str>>>,
    query_fault: Option<QueryFault>,
}
fn checked_context(context: Option<rpc::RequestContext>, transaction: bool) -> Result<(), Status> {
    let context = context.ok_or(Status::invalid_argument("missing same-session context"))?;
    if context.session.as_ref().map(|id| id.bytes.as_ref()) != Some(&[1u8; 16][..]) ||
        (transaction && context.transaction.as_ref().map(|id| id.bytes.as_ref()) != Some(&[2u8; 16][..])) {
        return Err(Status::invalid_argument("wrong session/transaction identity"));
    }
    Ok(())
}
#[tonic::async_trait]
impl SurrealDbService for Peer {
    async fn get_capabilities(
        &self,
        _: Request<rpc::GetCapabilitiesRequest>,
    ) -> Result<Response<rpc::GetCapabilitiesResponse>, Status> {
        Ok(Response::new(rpc::GetCapabilitiesResponse {
            capabilities: Some(rpc::ServerCapabilities {
                server_version: "3.3.0-injected-export-fixture".into(),
                ..Default::default()
            }),
        }))
    }
    async fn attach_session(
        &self,
        _: Request<rpc::AttachSessionRequest>,
    ) -> Result<Response<rpc::AttachSessionResponse>, Status> {
        Ok(Response::new(rpc::AttachSessionResponse {
            session: Some(proto::Uuid {
                bytes: vec![1u8; 16].into(),
            }),
            created: true,
        }))
    }
    async fn detach_session(
        &self,
        _: Request<rpc::DetachSessionRequest>,
    ) -> Result<Response<rpc::DetachSessionResponse>, Status> {
        Ok(Response::new(Default::default()))
    }
    async fn r#use(
        &self,
        _: Request<rpc::UseRequest>,
    ) -> Result<Response<rpc::UseResponse>, Status> {
        Ok(Response::new(rpc::UseResponse {
            namespace: "injected_export".into(),
            database: "fixture".into(),
        }))
    }
    async fn signin(
        &self,
        _: Request<rpc::SigninRequest>,
    ) -> Result<Response<rpc::SigninResponse>, Status> {
        Ok(Response::new(rpc::SigninResponse {
            tokens: Some(rpc::Tokens {
                access: "fixture-session-token".into(),
                ..Default::default()
            }),
        }))
    }
    async fn invalidate(
        &self,
        request: Request<rpc::InvalidateRequest>,
    ) -> Result<Response<rpc::InvalidateResponse>, Status> {
        if self.query_fault.is_some() { checked_context(request.into_inner().context, false)?; self.events.lock().unwrap().push("invalidate"); }
        Ok(Response::new(Default::default()))
    }
    type ExportSurqlStream = Frames<rpc::ExportSurqlResponse>;
    async fn export_surql(
        &self,
        _: Request<rpc::ExportSurqlRequest>,
    ) -> Result<Response<Self::ExportSurqlStream>, Status> {
        use rpc::export_surql_response::Frame;
        let (sender, receiver) = mpsc::channel(1);
        let terminal = self.terminal.clone();
        let fault = self.fault;
        tokio::spawn(async move {
            if sender
                .send(Ok(rpc::ExportSurqlResponse {
                    frame: Some(Frame::Chunk(rpc::DataChunk {
                        data: PARTIAL.into(),
                    })),
                }))
                .await
                .is_err()
            {
                return;
            }
            // The test releases this only after SDK file bytes prove the first frame arrived.
            tokio::select! { _ = terminal.notified() => {}, _ = sender.closed() => return }
            let frame = match fault {
                Fault::Success => Frame::Trailer(rpc::DataTrailer {
                    bytes: PARTIAL.len() as u64,
                    blake3: String::new(),
                }),
                Fault::ByteCountMismatch => Frame::Trailer(rpc::DataTrailer {
                    bytes: PARTIAL.len() as u64 + 1,
                    blake3: String::new(),
                }),
                Fault::LateEngineError => Frame::Error(proto::SurrealError::new(
                    proto::ErrorKind::Internal,
                    "injected late engine export error",
                )),
                Fault::LateTaskError => {
                    let _ = sender
                        .send(Err(Status::internal("injected late export task failure")))
                        .await;
                    return;
                }
                Fault::MissingTrailer | Fault::TransportClose => return,
            };
            let _ = sender
                .send(Ok(rpc::ExportSurqlResponse { frame: Some(frame) }))
                .await;
        });
        Ok(Response::new(Box::pin(
            tokio_stream::wrappers::ReceiverStream::new(receiver),
        )))
    }
    async fn health(
        &self,
        _: Request<rpc::HealthRequest>,
    ) -> Result<Response<rpc::HealthResponse>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    async fn reset_session(
        &self,
        _: Request<rpc::ResetSessionRequest>,
    ) -> Result<Response<rpc::ResetSessionResponse>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    async fn set_variable(
        &self,
        _: Request<rpc::SetVariableRequest>,
    ) -> Result<Response<rpc::SetVariableResponse>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    async fn unset_variable(
        &self,
        _: Request<rpc::UnsetVariableRequest>,
    ) -> Result<Response<rpc::UnsetVariableResponse>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    async fn signup(
        &self,
        _: Request<rpc::SignupRequest>,
    ) -> Result<Response<rpc::SignupResponse>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    async fn authenticate(
        &self,
        _: Request<rpc::AuthenticateRequest>,
    ) -> Result<Response<rpc::AuthenticateResponse>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    async fn refresh_tokens(
        &self,
        _: Request<rpc::RefreshTokensRequest>,
    ) -> Result<Response<rpc::RefreshTokensResponse>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    async fn revoke_tokens(
        &self,
        _: Request<rpc::RevokeTokensRequest>,
    ) -> Result<Response<rpc::RevokeTokensResponse>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    async fn begin_transaction(
        &self,
        request: Request<rpc::BeginTransactionRequest>,
    ) -> Result<Response<rpc::BeginTransactionResponse>, Status> {
        checked_context(request.into_inner().context, false)?;
        self.events.lock().unwrap().push("begin");
        Ok(Response::new(rpc::BeginTransactionResponse { transaction: Some(proto::Uuid { bytes: vec![2u8; 16].into() }) }))
    }
    async fn commit_transaction(
        &self,
        _: Request<rpc::CommitTransactionRequest>,
    ) -> Result<Response<rpc::CommitTransactionResponse>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    async fn cancel_transaction(
        &self,
        request: Request<rpc::CancelTransactionRequest>,
    ) -> Result<Response<rpc::CancelTransactionResponse>, Status> {
        checked_context(request.into_inner().context, true)?;
        self.events.lock().unwrap().push("cancel");
        Ok(Response::new(Default::default()))
    }
    async fn run(&self, _: Request<rpc::RunRequest>) -> Result<Response<rpc::RunResponse>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    async fn kill(
        &self,
        _: Request<rpc::KillRequest>,
    ) -> Result<Response<rpc::KillResponse>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    type QueryStream = Frames<rpc::QueryResponse>;
    async fn query(
        &self,
        request: Request<rpc::QueryRequest>,
    ) -> Result<Response<Self::QueryStream>, Status> {
        let fault = self.query_fault.ok_or(Status::unimplemented("export fault fixture only"))?;
        checked_context(request.into_inner().context, true)?;
        self.events.lock().unwrap().push("query");
        let (sender, receiver) = mpsc::channel(8);
        let terminal = self.terminal.clone(); let provisional = self.provisional.clone(); let events = self.events.clone();
        tokio::spawn(async move {
            use rpc::query_response::Frame;
            let send = |frame| sender.send(Ok(rpc::QueryResponse { frame: Some(frame) }));
            let batch = |kind, error| rpc::QueryBatchFrame {
                query_index: 0, batch_index: 0, kind: kind as i32,
                statement_kind: rpc::QueryStatementKind::Other as i32, stats: None, error,
                payload: Some(rpc::query_batch_frame::Payload::Values(rpc::ValueBatch {
                    values: vec![proto::Value::try_from(lctx_surrealdb::surrealdb::types::Value::Null).unwrap()],
                })),
            };
            if send(Frame::Begin(rpc::QueryBegin { statement_count: 1, ..Default::default() })).await.is_err() { return; }
            let first = if matches!(fault, QueryFault::LateStatementError) { rpc::QueryResponseKind::Batched } else { rpc::QueryResponseKind::BatchedFinal };
            if send(Frame::Batch(batch(first, None))).await.is_err() { return; }
            if !matches!(fault, QueryFault::LateStatementError | QueryFault::OuterError | QueryFault::MissingOuterEnd) {
                if send(Frame::End(rpc::QueryEnd { result_count: 1, ..Default::default() })).await.is_err() { return; }
            }
            provisional.notify_one();
            tokio::select! { _ = terminal.notified() => {}, _ = sender.closed() => return }
            match fault {
                QueryFault::Success | QueryFault::MissingOuterEnd => {},
                QueryFault::LateStatementError => {
                    let mut final_batch = batch(rpc::QueryResponseKind::BatchedFinal, Some(proto::SurrealError::new(proto::ErrorKind::Internal, "injected late selected statement failure")));
                    final_batch.batch_index = 1; final_batch.payload = None;
                    let _ = send(Frame::Batch(final_batch)).await;
                    let _ = send(Frame::End(rpc::QueryEnd { result_count: 1, ..Default::default() })).await;
                },
                QueryFault::OuterError => { let _ = send(Frame::Error(proto::SurrealError::new(proto::ErrorKind::Internal, "injected selected outer failure"))).await; },
                QueryFault::LateTransportError => { let _ = sender.send(Err(Status::unavailable("injected selected late physical status"))).await; },
                QueryFault::LatePayload => { let _ = send(Frame::Batch(batch(rpc::QueryResponseKind::BatchedFinal, None))).await; },
            }
            events.lock().unwrap().push("physical-tail-sent");
        });
        Ok(Response::new(Box::pin(tokio_stream::wrappers::ReceiverStream::new(receiver))))
    }
    type SubscribeStream = Frames<rpc::SubscribeResponse>;
    async fn subscribe(
        &self,
        _: Request<rpc::SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    type ExportDirectoryStream = Frames<rpc::ExportDirectoryResponse>;
    async fn export_directory(
        &self,
        _: Request<rpc::ExportDirectoryRequest>,
    ) -> Result<Response<Self::ExportDirectoryStream>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    type ExportMlModelStream = Frames<rpc::ExportMlModelResponse>;
    async fn export_ml_model(
        &self,
        _: Request<rpc::ExportMlModelRequest>,
    ) -> Result<Response<Self::ExportMlModelStream>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    async fn import_surql(
        &self,
        _: Request<tonic::Streaming<rpc::ImportSurqlRequest>>,
    ) -> Result<Response<rpc::ImportSurqlResponse>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
    async fn import_ml_model(
        &self,
        _: Request<tonic::Streaming<rpc::ImportMlModelRequest>>,
    ) -> Result<Response<rpc::ImportMlModelResponse>, Status> {
        Err(Status::unimplemented("export fault fixture only"))
    }
}
