//! Cooperative process signals are owned by the actual native lifetime, including
//! direct test processes. A listener never drops an execution future or claims
//! remote finality; only the enclosing owner's typed Completion can settle it.
use crate::control::RetryCancellation;
use lctx_model::domain::{ModelError, completion::{Completion, LocalState, RemoteState, StorageState}};
use std::{fs::{self, OpenOptions}, io::Write, path::PathBuf, sync::{Arc, Mutex, Weak, atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering}}};
use tokio::{sync::Notify, task::JoinHandle};

struct Receipt { path: PathBuf, body: serde_json::Value }
struct Listener { stop: Arc<Notify>, task: tokio::sync::Mutex<Option<JoinHandle<()>>> }
static PROCESS_STOPPED: AtomicBool = AtomicBool::new(false);
static PROCESS_SIGNAL: AtomicU8 = AtomicU8::new(0);
type Cancellation = dyn Fn() + Send + Sync;
static CANCELLATIONS: Mutex<Vec<Weak<Cancellation>>> = Mutex::new(Vec::new());
fn process_stop(observed: u8) {
    let callbacks = {
        let mut registrations = CANCELLATIONS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if PROCESS_STOPPED.load(Ordering::Acquire) { return; }
        PROCESS_SIGNAL.store(observed, Ordering::Release);
        PROCESS_STOPPED.store(true, Ordering::Release);
        let callbacks = registrations.iter().filter_map(Weak::upgrade).collect::<Vec<_>>();
        registrations.retain(|registration| registration.strong_count() != 0);
        callbacks
    };
    // A faulty callback must not prevent cancellation of unrelated accepted
    // owners. Preserve its original panic after all live callbacks were invoked.
    let mut panic = None;
    for cancel in callbacks {
        if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cancel())) {
            if panic.is_none() { panic = Some(payload); }
        }
    }
    if let Some(payload) = panic { std::panic::resume_unwind(payload); }
}
/// Retain this owner through actual session invalidation/final owner completion,
/// not a mid-operation flush/drain. Drop preserves the started receipt as unknown.
pub struct SignalRegistration {
    listener: Listener,
    cancel: Arc<dyn Fn() + Send + Sync>,
    signal: Arc<AtomicU8>,
    receipt: Option<Receipt>,
    completed: tokio::sync::OnceCell<Result<(), Arc<ModelError>>>,
}
impl SignalRegistration {
    pub fn retry(owner: &'static str, cancellation: RetryCancellation) -> Result<Self, ModelError> {
        Self::new(owner, move || cancellation.cancel())
    }
    pub fn new(owner: &'static str, cancel: impl Fn() + Send + Sync + 'static) -> Result<Self, ModelError> {
        Self::start(owner, cancel, true)
    }
    /// CLI fence installed before synchronous capture/preparation. Actual native
    /// owners report their own receipts; this listener claims no native finality.
    pub fn process(cancel: impl Fn() + Send + Sync + 'static) -> Result<Self, ModelError> {
        Self::start("CLI process signal fence", cancel, false)
    }
    pub fn process_stopped() -> bool { PROCESS_STOPPED.load(Ordering::Acquire) }
    fn start(owner: &'static str, cancel: impl Fn() + Send + Sync + 'static, record: bool) -> Result<Self, ModelError> {
        let runtime = tokio::runtime::Handle::try_current().map_err(ModelError::codec)?;
        // Synchronous installation precedes the first native admission/query. Each
        // real owner receives the broadcast signal; CLI installation is unnecessary.
        #[cfg(unix)]
        let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .map_err(ModelError::codec)?;
        #[cfg(unix)]
        let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .map_err(ModelError::codec)?;
        #[cfg(not(unix))]
        return Err(ModelError::Invalid("native process signals require Unix".into()));
        let receipt = if record { Receipt::begin(owner)? } else { None };
        let cancel: Arc<Cancellation> = Arc::new(cancel);
        let stopped = {
            let mut registrations = CANCELLATIONS.lock().map_err(|_| ModelError::Conflict("native signal registration"))?;
            registrations.retain(|registration| registration.strong_count() != 0);
            registrations.push(Arc::downgrade(&cancel));
            Self::process_stopped()
        };
        if stopped { cancel(); }
        let signal = Arc::new(AtomicU8::new(0));
        let stop = Arc::new(Notify::new());
        let (stopping, received) = (stop.clone(), signal.clone());
        let task = runtime.spawn(async move {
            #[cfg(unix)]
            let observed = tokio::select! {
                _ = interrupt.recv() => 2,
                _ = terminate.recv() => 15,
                _ = stopping.notified() => 0,
            };
            if observed != 0 {
                received.store(observed, Ordering::Release);
                process_stop(observed);
            }
        });
        Ok(Self { listener: Listener { stop, task: tokio::sync::Mutex::new(Some(task)) }, cancel,
            signal, receipt, completed: tokio::sync::OnceCell::new() })
    }
    pub fn received(&self) -> Option<&'static str> {
        let signal = self.signal.load(Ordering::Acquire);
        match if signal == 0 { PROCESS_SIGNAL.load(Ordering::Acquire) } else { signal } {
            2 => Some("SIGINT"), 15 => Some("SIGTERM"), _ => None
        }
    }
    /// Record actual owner finality after owned work, remote reconciliation and
    /// permitted cleanup; signals and process exit are separate observations.
    pub async fn complete(&self, completion: &Completion) -> Result<(), ModelError> {
        self.completed.get_or_init(|| async {
            self.listener.stop.notify_one();
            // Await by reference: cancellation releases this lock, but leaves the
            // sole actual join handle with the listener for a later completion.
            let mut task = self.listener.task.lock().await;
            let joined = task.as_mut().ok_or_else(|| Arc::new(ModelError::Conflict("native signal finalization owner lost")))?.await;
            task.take();
            joined.map_err(|error| Arc::new(ModelError::codec(error)))?;
            if let Some(receipt) = &self.receipt {
                receipt.finish(completion, self.received()).map_err(Arc::new)?;
            }
            Ok(())
        }).await.clone().map_err(ModelError::SharedCause)
    }
}
impl Drop for SignalRegistration {
    fn drop(&mut self) {
        if self.completed.get().is_none() {
            (self.cancel)();
            // This is only a signal-listener task, never native execution. Leave
            // started receipt intact: runtime/drop is not a Completion report.
            if let Some(task) = self.listener.task.get_mut().take() { task.abort(); }
        }
    }
}
impl Receipt {
    fn begin(owner: &'static str) -> Result<Option<Self>, ModelError> {
        let Some(directory) = std::env::var_os("LCTX_NATIVE_DRAIN_DIRECTORY") else { return Ok(None); };
        let directory = PathBuf::from(directory);
        if !directory.is_absolute() || directory.canonicalize().map_err(ModelError::codec)? != directory
            || !directory.is_dir() {
            return Err(ModelError::Conflict("native completion receipt directory"));
        }
        let attachment = std::env::var("LCTX_FIXTURE_ATTACHMENT").map_err(ModelError::codec)?;
        let generation = std::env::var("LCTX_SURREAL_GENERATION").map_err(ModelError::codec)?;
        if attachment.is_empty() || !attachment.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            || generation.len() != 64 || !generation.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(ModelError::Conflict("native completion receipt binding"));
        }
        let pid = std::process::id();
        let stat = fs::read_to_string("/proc/self/stat").map_err(ModelError::codec)?;
        let start_ticks = stat.rsplit_once(") ").and_then(|(_, tail)| tail.split_whitespace().nth(19))
            .ok_or(ModelError::Conflict("native process start identity"))?
            .parse::<u64>().map_err(ModelError::codec)?;
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!("{pid}-{start_ticks}-{sequence}.json"));
        let body = serde_json::json!({"schema":1,"kind":"native-process-owner","owner":owner,
            "attachment":attachment,"generation":generation,"process":{"pid":pid,"start_ticks":start_ticks},
            "registration":sequence,"state":"started","signal":null,"completion":null});
        let receipt = Self { path, body };
        receipt.write(&receipt.body, false)?;
        Ok(Some(receipt))
    }
    fn finish(&self, completion: &Completion, signal: Option<&str>) -> Result<(), ModelError> {
        let mut body = self.body.clone();
        body["state"] = serde_json::json!("complete");
        body["signal"] = serde_json::json!(signal);
        body["completion"] = serde_json::json!({
            "local":match completion.local {LocalState::Terminal=>"terminal",LocalState::Outstanding=>"outstanding"},
            "remote":match completion.remote {RemoteState::Confirmed=>"confirmed",RemoteState::Unknown=>"unknown"},
            "storage":completion.storage.iter().map(|state| match state {
                StorageState::Removed(identity)=>serde_json::json!({"state":"removed","identity":identity}),
                StorageState::Orphan(identity)=>serde_json::json!({"state":"orphan","identity":identity}),
            }).collect::<Vec<_>>(),
            "committed":completion.committed.iter().map(|effect| serde_json::json!({"kind":effect.kind,"identity":effect.identity})).collect::<Vec<_>>(),
            "failures":completion.failures.iter().map(|failure| serde_json::json!({"step":failure.step,"error":failure.error.to_string()})).collect::<Vec<_>>(),
        });
        self.write(&body, true)
    }
    fn write(&self, body: &serde_json::Value, replace: bool) -> Result<(), ModelError> {
        let path = if replace { self.path.with_extension("pending") } else { self.path.clone() };
        let mut options = OpenOptions::new(); options.write(true).create_new(true);
        #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
        let mut file = options.open(&path).map_err(ModelError::codec)?;
        let bytes = serde_json::to_vec(body).map_err(ModelError::codec)?;
        file.write_all(&bytes).map_err(ModelError::codec)?;
        file.sync_all().map_err(ModelError::codec)?;
        if replace { fs::rename(&path, &self.path).map_err(ModelError::codec)?; }
        fs::File::open(self.path.parent().ok_or(ModelError::Conflict("native receipt parent"))?)
            .and_then(|file| file.sync_all()).map_err(ModelError::codec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{process::{Command, Stdio}, time::{Duration, Instant}};

    #[tokio::test(flavor = "current_thread")]
    async fn cancelled_listener_join_retains_actual_owner_for_completion_retry() {
        let cancelled=Arc::new(AtomicBool::new(false));
        let observed=cancelled.clone();
        let owner=SignalRegistration::process(move||{observed.store(true,Ordering::Release);}).unwrap();
        let completion=Completion::default();
        // The current-thread listener has not run yet. Polling completion reaches
        // its real pending JoinHandle await without relying on timing or a mock.
        let mut first=Box::pin(owner.complete(&completion));
        assert!(futures::poll!(first.as_mut()).is_pending());
        drop(first);
        assert!(owner.completed.get().is_none());
        assert!(owner.listener.task.lock().await.is_some(),"cancelled initializer must retain the sole actual listener handle");
        owner.complete(&completion).await.unwrap();
        assert!(matches!(owner.completed.get(),Some(Ok(()))));
        assert!(owner.listener.task.lock().await.is_none(),"only acknowledged listener join consumes its handle");
        owner.complete(&completion).await.unwrap();
        drop(owner);
        assert!(!cancelled.load(Ordering::Acquire),"acknowledged retry completed the real owner");
    }

    #[test]
    #[ignore = "invoked only by the subprocess ownership control"]
    fn direct_signal_child() {
        let mode = std::env::var("LCTX_SIGNAL_CONTROL_MODE").expect("owned subprocess control");
        let directory = PathBuf::from(std::env::var_os("LCTX_NATIVE_DRAIN_DIRECTORY").unwrap());
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
            let cancellation = RetryCancellation::default();
            let owner = SignalRegistration::retry("direct-native-test", cancellation.clone()).unwrap();
            fs::write(directory.join("ready"), b"installed before admission").unwrap();
            tokio::time::timeout(Duration::from_secs(10), cancellation.cancelled()).await.unwrap();
            assert_eq!(owner.received(), Some("SIGINT"));
            if mode == "drop" { drop(owner); return; }
            let mut completion = Completion::default();
            if mode == "unknown" {
                completion.remote = RemoteState::Unknown;
                completion.committed("committed before signal", "exact-operation");
                completion.cleanup("retained-native-hold", Err(ModelError::Conflict("recovery required")));
            }
            owner.complete(&completion).await.unwrap();
            owner.complete(&completion).await.unwrap();
            let later = RetryCancellation::default();
            let registration = SignalRegistration::retry("later admission after process stop", later.clone()).unwrap();
            assert!(later.stopped());
            registration.complete(&completion).await.unwrap();
        });
    }
    struct Child(std::process::Child);
    impl Drop for Child { fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); } }
    #[test]
    fn actual_direct_process_sigint_activates_same_token_and_preserves_completion() {
        // No service/transaction is substituted here: this establishes only the
        // actual test-process signal entrypoint and typed receipt protocol.
        for mode in ["complete", "unknown", "drop"] {
            let directory = tempfile::tempdir().unwrap();
            let mut child = Child(Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "signal::tests::direct_signal_child", "--ignored", "--nocapture"])
                .env("LCTX_SIGNAL_CONTROL_MODE", mode)
                .env("LCTX_NATIVE_DRAIN_DIRECTORY", directory.path())
                .env("LCTX_FIXTURE_ATTACHMENT", "owned-signal-control")
                .env("LCTX_SURREAL_GENERATION", "a".repeat(64))
                .stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap());
            let until = Instant::now() + Duration::from_secs(10);
            while !directory.path().join("ready").exists() {
                assert!(child.0.try_wait().unwrap().is_none(), "child exited before signal owner");
                assert!(Instant::now() < until, "signal owner did not start");
                std::thread::sleep(Duration::from_millis(10));
            }
            fs::remove_file(directory.path().join("ready")).unwrap();
            assert!(Command::new("kill").arg("-INT").arg(child.0.id().to_string()).status().unwrap().success());
            assert!(child.0.wait().unwrap().success());
            let files = fs::read_dir(directory.path()).unwrap().map(|row| row.unwrap().path()).collect::<Vec<_>>();
            assert_eq!(files.len(), if mode=="drop" {1} else {2});
            let receipt: serde_json::Value = files.iter().map(|path| serde_json::from_slice::<serde_json::Value>(&fs::read(path).unwrap()).unwrap())
                .find(|row| row["owner"]=="direct-native-test").unwrap();
            assert_eq!(receipt["kind"], "native-process-owner");
            assert_eq!(receipt["process"]["pid"], child.0.id());
            if mode == "drop" {
                assert_eq!(receipt["state"], "started");
                assert!(receipt["completion"].is_null());
            } else {
                assert_eq!(receipt["state"], "complete");
                assert_eq!(receipt["signal"], "SIGINT");
                assert_eq!(receipt["completion"]["local"], "terminal");
                assert_eq!(receipt["completion"]["remote"], if mode=="unknown" {"unknown"} else {"confirmed"});
                if mode=="unknown" {
                    assert_eq!(receipt["completion"]["committed"][0]["identity"], "exact-operation");
                    assert_eq!(receipt["completion"]["storage"][0]["state"], "orphan");
                    assert_eq!(receipt["completion"]["failures"][0]["step"], "private storage cleanup");
                }
            }
        }
    }
}
