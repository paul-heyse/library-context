#[macro_use]
extern crate tracing;

mod rpc {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[path = "QUERY_DRAIN_PATH"]
    mod query_drain;

    pub struct RpcState {
        drains: Arc<query_drain::QueryDrains>,
        fenced: tokio::sync::Notify,
        forced: tokio::sync::Notify,
        swept: AtomicUsize,
    }
    pub fn close_grpc_queries(state: &RpcState) {
        state.drains.close();
        state.fenced.notify_one();
    }
    // The harness substitutes only the database/WebSocket sweeps. The actual
    // registry, admission fence, signal wrapper, Tokio and Axum Handle run here.
    pub async fn graceful_shutdown(state: Arc<RpcState>) -> Result<(), usize> {
        close_grpc_queries(&state);
        let result = state.drains.wait().await;
        state.swept.fetch_add(1, Ordering::Relaxed);
        result
    }
    pub fn shutdown(state: &Arc<RpcState>) { state.forced.notify_one(); }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::ntw::signals;
        use tokio_util::sync::CancellationToken;
        use uuid::Uuid;

        fn state() -> Arc<RpcState> {
            Arc::new(RpcState { drains: query_drain::QueryDrains::new(),
                fenced: tokio::sync::Notify::new(), forced: tokio::sync::Notify::new(),
                swept: AtomicUsize::new(0) })
        }

        #[tokio::test]
        async fn actual_forced_shutdown_wrapper_joins_execution_before_sweep_and_return() {
            let state = state();
            let permit = state.drains.accept(Uuid::new_v4(), Arc::new(|| {})).unwrap();
            let (send, receive) = tokio::sync::oneshot::channel();
            drop(permit.handoff(move |permit| async move {
                receive.await.unwrap(); permit.complete();
            }));
            let cancelled = CancellationToken::new();
            let shutdown = signals::graceful_shutdown(Arc::clone(&state), cancelled.clone(),
                axum_server::Handle::new());
            cancelled.cancel();
            state.fenced.notified().await;
            state.forced.notified().await;
            assert!(!shutdown.is_finished(), "forced transport close must still join finality");
            assert_eq!(state.swept.load(Ordering::Relaxed), 0);
            send.send(()).unwrap();
            shutdown.await.unwrap().unwrap();
            assert_eq!(state.swept.load(Ordering::Relaxed), 1);
        }

        #[tokio::test]
        async fn actual_shutdown_wrapper_propagates_unresolved_finalizer_error() {
            let state = state();
            let permit = state.drains.accept(Uuid::new_v4(), Arc::new(|| {})).unwrap();
            assert!(permit.handoff(|_permit| async { panic!("lost finalizer"); }).await.is_err());
            let cancelled = CancellationToken::new();
            cancelled.cancel();
            let shutdown = signals::graceful_shutdown(state, cancelled, axum_server::Handle::new());
            let error = shutdown.await.unwrap().unwrap_err();
            assert!(error.to_string().contains("1 execution finalizers"));
        }
    }
}

mod telemetry { pub fn shutdown() {} }
mod ntw {
    pub const LOG: &str = "lctx_registry_control";
    #[path = "SIGNALS_PATH"]
    pub mod signals;
}
