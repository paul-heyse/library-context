//! Cooperative process signals are owned by the actual native lifetime, including
//! direct test processes. A listener never drops an execution future or claims
//! remote finality; only the enclosing owner's typed Completion can settle it.
use crate::control::RetryCancellation;
use lctx_model::domain::{
    ContentHash, ModelError,
    completion::{Completion, LocalState, RemoteState, StorageState},
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
    },
};
use tokio::{sync::Notify, task::JoinHandle};

/// Explicit existing run/storage owner binding for bare product entrypoints.
/// The caller owns the directory through actual native cleanup; no temp fallback.
#[derive(Clone)]
pub struct NativeReceiptBinding {
    pub directory: PathBuf,
    pub attachment: String,
    pub generation: ContentHash,
}

/// Authentication facts of the exact host which published an outstanding request.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingRequestHostOwner {
    pub receipt: PathBuf,
    pub pid: u32,
    pub start_ticks: u64,
    pub boot_id: String,
    pub pid_namespace: u64,
}
impl PendingRequestHostOwner {
    pub(crate) fn heap_bytes(&self) -> usize {
        self.receipt.capacity() + self.boot_id.capacity()
    }
}
/// A child capability of an actual started native process receipt, not a result.
#[derive(Clone)]
pub struct PendingRequestRecipient {
    directory: PathBuf,
    generation: ContentHash,
    host_owner: PendingRequestHostOwner,
    attachment: Option<String>,
    admission: Arc<Mutex<bool>>,
}
impl PendingRequestRecipient {
    /// Offline caller must separately authenticate original host drainage and
    /// exclusive maintenance. This capability grants only exact sidecar updates.
    pub(crate) fn for_recovery(
        host_owner: PendingRequestHostOwner,
        generation: ContentHash,
    ) -> Result<Self, ModelError> {
        let parent = host_owner
            .receipt
            .parent()
            .ok_or(ModelError::Conflict("native recovery receipt parent"))?;
        let stem = host_owner
            .receipt
            .file_stem()
            .ok_or(ModelError::Conflict("native recovery receipt name"))?;
        let directory = parent.with_extension("requests").join(stem);
        Ok(Self {
            directory,
            generation,
            host_owner,
            attachment: None,
            admission: Arc::new(Mutex::new(true)),
        })
    }
    pub(crate) fn binding(&self) -> Result<NativeReceiptBinding, ModelError> {
        if *self
            .admission
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery recipient"))?
        {
            return Err(ModelError::Conflict(
                "native recovery recipient admission closed",
            ));
        }
        Ok(NativeReceiptBinding {
            directory: self
                .host_owner
                .receipt
                .parent()
                .ok_or(ModelError::Conflict("native owner receipt parent"))?
                .to_path_buf(),
            attachment: self.attachment.clone().ok_or(ModelError::Conflict(
                "offline recovery recipient cannot admit owners",
            ))?,
            generation: self.generation,
        })
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn generation(&self) -> ContentHash {
        self.generation
    }
    pub fn host_owner(&self) -> &PendingRequestHostOwner {
        &self.host_owner
    }
    pub(crate) fn heap_bytes(&self) -> usize {
        self.directory.capacity()
            + self.host_owner.heap_bytes()
            + self.attachment.as_ref().map_or(0, String::capacity)
    }
    pub(crate) fn write(&self, name: &str, bytes: &[u8], replace: bool) -> Result<(), ModelError> {
        let closed = self
            .admission
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery recipient"))?;
        if *closed && !replace {
            return Err(ModelError::Conflict("native recovery admission closed"));
        }
        if name.len() != 64 || !name.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ModelError::Conflict("native pending request name"));
        }
        let parent = self
            .directory
            .parent()
            .ok_or(ModelError::Conflict("native recovery parent"))?;
        for directory in [parent, self.directory.as_path()] {
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(directory) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(ModelError::codec(error)),
            }
            if directory.canonicalize().map_err(ModelError::codec)? != directory {
                return Err(ModelError::Conflict("native recovery directory redirected"));
            }
        }
        if self.directory.canonicalize().map_err(ModelError::codec)? != self.directory {
            return Err(ModelError::Conflict("native recovery directory redirected"));
        }
        let final_path = self.directory.join(format!("{name}.json"));
        let mut file =
            tempfile::NamedTempFile::new_in(&self.directory).map_err(ModelError::codec)?;
        file.write_all(bytes).map_err(ModelError::codec)?;
        file.as_file().sync_all().map_err(ModelError::codec)?;
        if replace {
            file.persist(&final_path).map_err(ModelError::codec)?;
        } else {
            file.persist_noclobber(&final_path)
                .map_err(ModelError::codec)?;
        }
        fs::File::open(&self.directory)
            .and_then(|file| file.sync_all())
            .map_err(ModelError::codec)?;
        // Also persist newly created directories in their existing storage owner.
        let mut parent = self.directory.parent();
        for _ in 0..2 {
            if let Some(directory) = parent {
                fs::File::open(directory)
                    .and_then(|file| file.sync_all())
                    .map_err(ModelError::codec)?;
                parent = directory.parent();
            }
        }
        Ok(())
    }
    pub(crate) fn remove(&self, name: &str) -> Result<(), ModelError> {
        let _closed = self
            .admission
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery recipient"))?;
        match fs::remove_file(self.directory.join(format!("{name}.json"))) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(ModelError::codec(error)),
        }
        fs::File::open(&self.directory)
            .and_then(|file| file.sync_all())
            .map_err(ModelError::codec)
    }
    fn close_admission(&self) -> Result<(), ModelError> {
        let mut closed = self
            .admission
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery recipient"))?;
        *closed = true;
        match fs::read_dir(&self.directory) {
            Ok(mut entries) => {
                if entries
                    .next()
                    .transpose()
                    .map_err(ModelError::codec)?
                    .is_some()
                {
                    return Err(ModelError::Conflict(
                        "native owner still has pending recovery requests",
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(ModelError::codec(error)),
        }
        Ok(())
    }
}
struct Receipt {
    path: PathBuf,
    body: serde_json::Value,
    pending: PendingRequestRecipient,
}
struct Listener {
    stop: Arc<Notify>,
    task: tokio::sync::Mutex<Option<JoinHandle<()>>>,
}
static PROCESS_STOPPED: AtomicBool = AtomicBool::new(false);
static PROCESS_SIGNAL: AtomicU8 = AtomicU8::new(0);
type Cancellation = dyn Fn() + Send + Sync;
static CANCELLATIONS: Mutex<Vec<Weak<Cancellation>>> = Mutex::new(Vec::new());
fn process_stop(observed: u8) {
    let callbacks = {
        let mut registrations = CANCELLATIONS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if PROCESS_STOPPED.load(Ordering::Acquire) {
            return;
        }
        PROCESS_SIGNAL.store(observed, Ordering::Release);
        PROCESS_STOPPED.store(true, Ordering::Release);
        let callbacks = registrations
            .iter()
            .filter_map(Weak::upgrade)
            .collect::<Vec<_>>();
        registrations.retain(|registration| registration.strong_count() != 0);
        callbacks
    };
    // A faulty callback must not prevent cancellation of unrelated accepted
    // owners. Preserve its original panic after all live callbacks were invoked.
    let mut panic = None;
    for cancel in callbacks {
        if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cancel()))
            && panic.is_none()
        {
            panic = Some(payload);
        }
    }
    if let Some(payload) = panic {
        std::panic::resume_unwind(payload);
    }
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
    pub fn new(
        owner: &'static str,
        cancel: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self, ModelError> {
        Self::start(owner, cancel, true, None)
    }
    /// CLI fence installed before synchronous capture/preparation. Actual native
    /// owners report their own receipts; this listener claims no native finality.
    pub fn process(cancel: impl Fn() + Send + Sync + 'static) -> Result<Self, ModelError> {
        Self::start("CLI process signal fence", cancel, false, None)
    }
    pub fn with_receipt(
        owner: &'static str,
        cancel: impl Fn() + Send + Sync + 'static,
        binding: NativeReceiptBinding,
    ) -> Result<Self, ModelError> {
        Self::start(owner, cancel, true, Some(binding))
    }
    pub fn new_bound(
        owner: &'static str,
        cancel: impl Fn() + Send + Sync + 'static,
        directory: PathBuf,
        attachment: String,
        generation: ContentHash,
    ) -> Result<Self, ModelError> {
        Self::with_receipt(
            owner,
            cancel,
            NativeReceiptBinding {
                directory,
                attachment,
                generation,
            },
        )
    }
    pub fn pending_recipient(&self) -> Result<PendingRequestRecipient, ModelError> {
        self.receipt
            .as_ref()
            .map(|receipt| receipt.pending.clone())
            .ok_or(ModelError::Conflict(
                "native recovery requires an explicit receipt owner",
            ))
    }
    pub fn process_stopped() -> bool {
        PROCESS_STOPPED.load(Ordering::Acquire)
    }
    fn start(
        owner: &'static str,
        cancel: impl Fn() + Send + Sync + 'static,
        record: bool,
        binding: Option<NativeReceiptBinding>,
    ) -> Result<Self, ModelError> {
        let runtime = tokio::runtime::Handle::try_current().map_err(ModelError::codec)?;
        // Synchronous installation precedes the first native admission/query. Each
        // real owner receives the broadcast signal; CLI installation is unnecessary.
        #[cfg(unix)]
        let mut interrupt =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                .map_err(ModelError::codec)?;
        #[cfg(unix)]
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .map_err(ModelError::codec)?;
        #[cfg(not(unix))]
        return Err(ModelError::Invalid(
            "native process signals require Unix".into(),
        ));
        let receipt = if record {
            Receipt::begin(owner, binding)?
        } else {
            None
        };
        let cancel: Arc<Cancellation> = Arc::new(cancel);
        let stopped = {
            let mut registrations = CANCELLATIONS
                .lock()
                .map_err(|_| ModelError::Conflict("native signal registration"))?;
            registrations.retain(|registration| registration.strong_count() != 0);
            registrations.push(Arc::downgrade(&cancel));
            Self::process_stopped()
        };
        if stopped {
            cancel();
        }
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
        Ok(Self {
            listener: Listener {
                stop,
                task: tokio::sync::Mutex::new(Some(task)),
            },
            cancel,
            signal,
            receipt,
            completed: tokio::sync::OnceCell::new(),
        })
    }
    pub fn received(&self) -> Option<&'static str> {
        let signal = self.signal.load(Ordering::Acquire);
        match if signal == 0 {
            PROCESS_SIGNAL.load(Ordering::Acquire)
        } else {
            signal
        } {
            2 => Some("SIGINT"),
            15 => Some("SIGTERM"),
            _ => None,
        }
    }
    /// Record actual owner finality after owned work, remote reconciliation and
    /// permitted cleanup; signals and process exit are separate observations.
    pub async fn complete(&self, completion: &Completion) -> Result<(), ModelError> {
        if let Some(receipt) = &self.receipt {
            receipt.pending.close_admission()?;
        }
        self.completed
            .get_or_init(|| async {
                self.listener.stop.notify_one();
                // Await by reference: cancellation releases this lock, but leaves the
                // sole actual join handle with the listener for a later completion.
                let mut task = self.listener.task.lock().await;
                let joined = task
                    .as_mut()
                    .ok_or_else(|| {
                        Arc::new(ModelError::Conflict(
                            "native signal finalization owner lost",
                        ))
                    })?
                    .await;
                task.take();
                joined.map_err(|error| Arc::new(ModelError::codec(error)))?;
                if let Some(receipt) = &self.receipt {
                    receipt
                        .finish(completion, self.received())
                        .map_err(Arc::new)?;
                }
                Ok(())
            })
            .await
            .clone()
            .map_err(ModelError::SharedCause)
    }
}
impl Drop for SignalRegistration {
    fn drop(&mut self) {
        if self.completed.get().is_none() {
            (self.cancel)();
            // This is only a signal-listener task, never native execution. Leave
            // started receipt intact: runtime/drop is not a Completion report.
            if let Some(task) = self.listener.task.get_mut().take() {
                task.abort();
            }
        }
    }
}
impl Receipt {
    fn begin(
        owner: &'static str,
        explicit: Option<NativeReceiptBinding>,
    ) -> Result<Option<Self>, ModelError> {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        Self::begin_with_sequence(owner, explicit, &SEQUENCE)
    }
    fn begin_with_sequence(
        owner: &'static str,
        explicit: Option<NativeReceiptBinding>,
        sequence: &AtomicU64,
    ) -> Result<Option<Self>, ModelError> {
        let (directory, attachment, generation) = if let Some(binding) = explicit {
            (binding.directory, binding.attachment, binding.generation)
        } else {
            let Some(directory) = std::env::var_os("LCTX_NATIVE_DRAIN_DIRECTORY") else {
                return Ok(None);
            };
            let attachment = std::env::var("LCTX_FIXTURE_ATTACHMENT").map_err(ModelError::codec)?;
            let generation = std::env::var("LCTX_SURREAL_GENERATION").map_err(ModelError::codec)?;
            let bytes: [u8; 32] = hex::decode(generation)
                .map_err(ModelError::codec)?
                .try_into()
                .map_err(|_| ModelError::Conflict("native completion receipt generation"))?;
            (PathBuf::from(directory), attachment, ContentHash(bytes))
        };
        if !directory.is_absolute()
            || directory.canonicalize().map_err(ModelError::codec)? != directory
            || !directory.is_dir()
        {
            return Err(ModelError::Conflict("native completion receipt directory"));
        }
        if attachment.is_empty()
            || !attachment
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err(ModelError::Conflict("native completion receipt binding"));
        }
        let pid = std::process::id();
        let stat = fs::read_to_string("/proc/self/stat").map_err(ModelError::codec)?;
        let start_ticks = stat
            .rsplit_once(") ")
            .and_then(|(_, tail)| tail.split_whitespace().nth(19))
            .ok_or(ModelError::Conflict("native process start identity"))?
            .parse::<u64>()
            .map_err(ModelError::codec)?;
        let boot_id = fs::read_to_string("/proc/sys/kernel/random/boot_id")
            .map_err(ModelError::codec)?
            .trim()
            .to_owned();
        #[cfg(unix)]
        let pid_namespace = {
            use std::os::unix::fs::MetadataExt;
            fs::metadata("/proc/self/ns/pid")
                .map_err(ModelError::codec)?
                .ino()
        };
        #[cfg(not(unix))]
        let pid_namespace = 0;
        loop {
            let registration = sequence
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    value.checked_add(1)
                })
                .map_err(|_| ModelError::Conflict("native receipt registration exhausted"))?;
            let path = directory.join(format!("{pid}-{start_ticks}-{registration}.json"));
            let pending = PendingRequestRecipient {
                directory: directory.with_extension("requests").join(
                    path.file_stem()
                        .ok_or(ModelError::Conflict("native owner receipt name"))?,
                ),
                generation,
                host_owner: PendingRequestHostOwner {
                    receipt: path.clone(),
                    pid,
                    start_ticks,
                    boot_id: boot_id.clone(),
                    pid_namespace,
                },
                attachment: Some(attachment.clone()),
                admission: Arc::new(Mutex::new(false)),
            };
            let body = serde_json::json!({"schema":1,"kind":"native-process-owner","owner":owner,
            "attachment":attachment,"generation":generation.hex(),"process":{"pid":pid,"start_ticks":start_ticks},
            "registration":registration,"state":"started","signal":null,"completion":null,
            "pending_requests":pending.directory,"host_boot_id":boot_id,"host_pid_namespace":pid_namespace});
            let receipt = Self {
                path,
                body,
                pending,
            };
            // A process may load independently linked copies of this crate, each
            // with its own counter. The filesystem's create-new claim is the shared
            // allocation authority. Only an unclaimed candidate name is retried;
            // never replace/adopt an existing receipt or retry after publication.
            let file = match Self::create(&receipt.path) {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(ModelError::codec(error)),
            };
            receipt.write_created(&receipt.body, file, false)?;
            return Ok(Some(receipt));
        }
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
            "failures":completion.failures.iter().map(|failure| serde_json::json!({"step":failure.step,"error":failure.error.to_string(),"diagnostic":crate::diagnostic::failure(failure.step, &failure.error)})).collect::<Vec<_>>(),
        });
        self.write(&body, true)
    }
    fn write(&self, body: &serde_json::Value, replace: bool) -> Result<(), ModelError> {
        let path = if replace {
            self.path.with_extension("pending")
        } else {
            self.path.clone()
        };
        let file = Self::create(&path).map_err(ModelError::codec)?;
        self.write_created(body, file, replace)
    }
    fn create(path: &Path) -> std::io::Result<fs::File> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(path)
    }
    fn write_created(
        &self,
        body: &serde_json::Value,
        mut file: fs::File,
        replace: bool,
    ) -> Result<(), ModelError> {
        let bytes = serde_json::to_vec(body).map_err(ModelError::codec)?;
        file.write_all(&bytes).map_err(ModelError::codec)?;
        file.sync_all().map_err(ModelError::codec)?;
        if replace {
            fs::rename(self.path.with_extension("pending"), &self.path)
                .map_err(ModelError::codec)?;
        }
        fs::File::open(
            self.path
                .parent()
                .ok_or(ModelError::Conflict("native receipt parent"))?,
        )
        .and_then(|file| file.sync_all())
        .map_err(ModelError::codec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };

    fn local_binding(directory: &Path) -> NativeReceiptBinding {
        NativeReceiptBinding {
            directory: directory.to_path_buf(),
            attachment: "registration-control".into(),
            generation: ContentHash::of(b"registration control"),
        }
    }

    #[test]
    fn independent_registration_allocators_claim_names_without_replacing_receipts() {
        let directory = tempfile::tempdir().unwrap();
        let original = Receipt::begin_with_sequence(
            "existing owner",
            Some(local_binding(directory.path())),
            &AtomicU64::new(0),
        )
        .unwrap()
        .unwrap();
        let original_bytes = fs::read(&original.path).unwrap();
        // Model independently linked crate copies: each starts its own counter
        // at zero, but both must claim the same authenticated process namespace.
        let left = AtomicU64::new(0);
        let right = AtomicU64::new(0);
        let barrier = std::sync::Barrier::new(2);
        let (first, second) = std::thread::scope(|threads| {
            let first = threads.spawn(|| {
                barrier.wait();
                Receipt::begin_with_sequence(
                    "first independent owner",
                    Some(local_binding(directory.path())),
                    &left,
                )
                .unwrap()
                .unwrap()
            });
            let second = threads.spawn(|| {
                barrier.wait();
                Receipt::begin_with_sequence(
                    "second independent owner",
                    Some(local_binding(directory.path())),
                    &right,
                )
                .unwrap()
                .unwrap()
            });
            (first.join().unwrap(), second.join().unwrap())
        });
        assert_ne!(first.path, second.path);
        assert_eq!(fs::read(&original.path).unwrap(), original_bytes);
        let registrations = [&original, &first, &second].map(|receipt| {
            let body: serde_json::Value =
                serde_json::from_slice(&fs::read(&receipt.path).unwrap()).unwrap();
            assert_eq!(body["process"], original.body["process"]);
            assert_eq!(body["attachment"], original.body["attachment"]);
            assert_eq!(body["generation"], original.body["generation"]);
            assert_eq!(body["state"], "started");
            let registration = body["registration"].as_u64().unwrap();
            assert_eq!(
                receipt.path.file_name().unwrap().to_str().unwrap(),
                format!(
                    "{}-{}-{registration}.json",
                    body["process"]["pid"], body["process"]["start_ticks"]
                )
            );
            registration
        });
        assert_eq!(
            registrations
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>(),
            [0, 1, 2].into_iter().collect()
        );
    }

    #[test]
    fn finish_collision_is_not_registration_retry_or_completion() {
        let directory = tempfile::tempdir().unwrap();
        let receipt = Receipt::begin_with_sequence(
            "finish collision owner",
            Some(local_binding(directory.path())),
            &AtomicU64::new(0),
        )
        .unwrap()
        .unwrap();
        let original = fs::read(&receipt.path).unwrap();
        let pending = receipt.path.with_extension("pending");
        fs::write(&pending, b"unresolved prior publication").unwrap();
        assert!(receipt.finish(&Completion::default(), None).is_err());
        assert_eq!(fs::read(&receipt.path).unwrap(), original);
        assert_eq!(fs::read(&pending).unwrap(), b"unresolved prior publication");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }

    #[test]
    fn registration_exhaustion_refuses_without_wrapping_or_writing() {
        let directory = tempfile::tempdir().unwrap();
        let counter = AtomicU64::new(u64::MAX);
        assert!(matches!(
            Receipt::begin_with_sequence(
                "exhausted owner",
                Some(local_binding(directory.path())),
                &counter
            ),
            Err(ModelError::Conflict(
                "native receipt registration exhausted"
            ))
        ));
        assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    fn diagnostic_completion() -> Completion {
        use crate::control::{IssuanceEra, NativeAttemptError, NativeRequest, NativeRequestError};
        use lctx_model::domain::{ContentHash, Infrastructure, completion::OperationFailure};
        let issuance = IssuanceEra {
            generation: ContentHash([1; 32]),
            era: 7,
        };
        let request = NativeRequest {
            issuance,
            operation: ContentHash([2; 32]),
            request: ContentHash([3; 32]),
        };
        let mut nested = Completion::default();
        nested.step(
            "reconcile request",
            Err(ModelError::Cause(Box::new(NativeAttemptError {
                attempt: ContentHash([4; 32]),
                issuance,
                source: ModelError::infrastructure(Infrastructure::Unconfirmed, "unconfirmed"),
            }))),
        );
        let error = ModelError::Completion(Box::new(OperationFailure {
            primary: Some(Box::new(ModelError::Cause(Box::new(NativeRequestError {
                request,
                source: ModelError::Conflict("request refused"),
            })))),
            completion: nested,
        }));
        let mut completion = Completion::default();
        completion.committed("committed before cleanup", "exact-operation");
        completion.cleanup("held-storage", Err(error));
        completion.step(
            "capture before admission",
            Err(ModelError::Cause(Box::new(std::io::Error::other("opaque")))),
        );
        let mut deep = ModelError::Conflict("deep cause");
        for _ in 0..20 {
            deep = ModelError::SharedCause(Arc::new(deep));
        }
        completion.step("bounded cause control", Err(deep));
        completion
    }

    #[test]
    #[ignore = "invoked only by persisted receipt/host ownership controls"]
    fn diagnostic_receipt_child() {
        assert_eq!(
            std::env::var("LCTX_DIAGNOSTIC_RECEIPT_CONTROL").as_deref(),
            Ok("owned")
        );
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let owner = SignalRegistration::retry(
                    "diagnostic native control",
                    RetryCancellation::default(),
                )
                .unwrap();
                owner.complete(&diagnostic_completion()).await.unwrap();
            });
    }

    #[test]
    fn actual_persisted_receipt_retains_typed_causes_and_independent_outcomes() {
        let directory = tempfile::tempdir().unwrap();
        assert!(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "signal::tests::diagnostic_receipt_child",
                    "--ignored"
                ])
                .env("LCTX_DIAGNOSTIC_RECEIPT_CONTROL", "owned")
                .env("LCTX_NATIVE_DRAIN_DIRECTORY", directory.path())
                .env("LCTX_FIXTURE_ATTACHMENT", "owned-diagnostic-control")
                .env("LCTX_SURREAL_GENERATION", "a".repeat(64))
                .status()
                .unwrap()
                .success()
        );
        let paths = fs::read_dir(directory.path())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(paths.len(), 1);
        let body: serde_json::Value =
            serde_json::from_slice(&fs::read(paths[0].path()).unwrap()).unwrap();
        assert_eq!(body["completion"]["remote"], "unknown");
        assert_eq!(body["completion"]["local"], "terminal");
        assert_eq!(body["completion"]["storage"][0]["state"], "orphan");
        assert_eq!(
            body["completion"]["committed"][0]["identity"],
            "exact-operation"
        );
        let diagnostic = &body["completion"]["failures"][0]["diagnostic"];
        let causes = diagnostic["causes"].as_array().unwrap();
        let request = causes
            .iter()
            .find(|c| c["code"] == "native_request")
            .unwrap();
        assert_eq!(request["identities"][0]["value"], "02".repeat(32));
        assert_eq!(request["issuance"]["era"], 7);
        assert_eq!(request["issuance"]["generation"], "01".repeat(32));
        assert!(
            causes
                .iter()
                .any(|c| c["code"] == "native_attempt" && c["stage"] == "reconcile request")
        );
        assert_eq!(
            body["completion"]["failures"][2]["diagnostic"]["truncated"],
            true
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cancelled_listener_join_retains_actual_owner_for_completion_retry() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let observed = cancelled.clone();
        let owner = SignalRegistration::process(move || {
            observed.store(true, Ordering::Release);
        })
        .unwrap();
        let completion = Completion::default();
        // The current-thread listener has not run yet. Polling completion reaches
        // its real pending JoinHandle await without relying on timing or a mock.
        let mut first = Box::pin(owner.complete(&completion));
        assert!(futures::poll!(first.as_mut()).is_pending());
        drop(first);
        assert!(owner.completed.get().is_none());
        assert!(
            owner.listener.task.lock().await.is_some(),
            "cancelled initializer must retain the sole actual listener handle"
        );
        owner.complete(&completion).await.unwrap();
        assert!(matches!(owner.completed.get(), Some(Ok(()))));
        assert!(
            owner.listener.task.lock().await.is_none(),
            "only acknowledged listener join consumes its handle"
        );
        owner.complete(&completion).await.unwrap();
        drop(owner);
        assert!(
            !cancelled.load(Ordering::Acquire),
            "acknowledged retry completed the real owner"
        );
    }

    #[test]
    #[ignore = "invoked only by the subprocess ownership control"]
    fn direct_signal_child() {
        let mode = std::env::var("LCTX_SIGNAL_CONTROL_MODE").expect("owned subprocess control");
        let directory = PathBuf::from(std::env::var_os("LCTX_NATIVE_DRAIN_DIRECTORY").unwrap());
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let cancellation = RetryCancellation::default();
                let owner =
                    SignalRegistration::retry("direct-native-test", cancellation.clone()).unwrap();
                fs::write(directory.join("ready"), b"installed before admission").unwrap();
                tokio::time::timeout(Duration::from_secs(10), cancellation.cancelled())
                    .await
                    .unwrap();
                assert_eq!(owner.received(), Some("SIGINT"));
                if mode == "drop" {
                    drop(owner);
                    return;
                }
                let mut completion = Completion::default();
                if mode == "unknown" {
                    completion.remote = RemoteState::Unknown;
                    completion.committed("committed before signal", "exact-operation");
                    completion.cleanup(
                        "retained-native-hold",
                        Err(ModelError::Conflict("recovery required")),
                    );
                }
                owner.complete(&completion).await.unwrap();
                owner.complete(&completion).await.unwrap();
                let later = RetryCancellation::default();
                let registration =
                    SignalRegistration::retry("later admission after process stop", later.clone())
                        .unwrap();
                assert!(later.stopped());
                registration.complete(&completion).await.unwrap();
            });
    }
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    #[test]
    fn actual_direct_process_sigint_activates_same_token_and_preserves_completion() {
        // No service/transaction is substituted here: this establishes only the
        // actual test-process signal entrypoint and typed receipt protocol.
        for mode in ["complete", "unknown", "drop"] {
            let directory = tempfile::tempdir().unwrap();
            let mut child = Child(
                Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "signal::tests::direct_signal_child",
                        "--ignored",
                        "--nocapture",
                    ])
                    .env("LCTX_SIGNAL_CONTROL_MODE", mode)
                    .env("LCTX_NATIVE_DRAIN_DIRECTORY", directory.path())
                    .env("LCTX_FIXTURE_ATTACHMENT", "owned-signal-control")
                    .env("LCTX_SURREAL_GENERATION", "a".repeat(64))
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap(),
            );
            let until = Instant::now() + Duration::from_secs(10);
            while !directory.path().join("ready").exists() {
                assert!(
                    child.0.try_wait().unwrap().is_none(),
                    "child exited before signal owner"
                );
                assert!(Instant::now() < until, "signal owner did not start");
                std::thread::sleep(Duration::from_millis(10));
            }
            fs::remove_file(directory.path().join("ready")).unwrap();
            assert!(
                Command::new("kill")
                    .arg("-INT")
                    .arg(child.0.id().to_string())
                    .status()
                    .unwrap()
                    .success()
            );
            assert!(child.0.wait().unwrap().success());
            let files = fs::read_dir(directory.path())
                .unwrap()
                .map(|row| row.unwrap().path())
                .collect::<Vec<_>>();
            assert_eq!(files.len(), if mode == "drop" { 1 } else { 2 });
            let receipt: serde_json::Value = files
                .iter()
                .map(|path| {
                    serde_json::from_slice::<serde_json::Value>(&fs::read(path).unwrap()).unwrap()
                })
                .find(|row| row["owner"] == "direct-native-test")
                .unwrap();
            assert_eq!(receipt["kind"], "native-process-owner");
            assert_eq!(receipt["process"]["pid"], child.0.id());
            if mode == "drop" {
                assert_eq!(receipt["state"], "started");
                assert!(receipt["completion"].is_null());
            } else {
                assert_eq!(receipt["state"], "complete");
                assert_eq!(receipt["signal"], "SIGINT");
                assert_eq!(receipt["completion"]["local"], "terminal");
                assert_eq!(
                    receipt["completion"]["remote"],
                    if mode == "unknown" {
                        "unknown"
                    } else {
                        "confirmed"
                    }
                );
                if mode == "unknown" {
                    assert_eq!(
                        receipt["completion"]["committed"][0]["identity"],
                        "exact-operation"
                    );
                    assert_eq!(receipt["completion"]["storage"][0]["state"], "orphan");
                    assert_eq!(
                        receipt["completion"]["failures"][0]["step"],
                        "private storage cleanup"
                    );
                }
            }
        }
    }
}
