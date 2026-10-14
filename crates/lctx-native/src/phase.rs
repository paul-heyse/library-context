//! Coarse runtime observations. Only the owner of actual terminal work may finish a phase.
use std::{
    sync::LazyLock,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

static NEXT_PHASE: AtomicU64 = AtomicU64::new(1);
static PHASE_CALLSITE: tracing::callsite::DefaultCallsite =
    tracing::callsite::DefaultCallsite::new(&PHASE_METADATA);
static PHASE_METADATA: tracing::Metadata<'static> = tracing::Metadata::new(
    "phase",
    "lctx_phase",
    tracing::Level::INFO,
    Some(file!()),
    Some(line!()),
    Some(module_path!()),
    tracing::field::FieldSet::new(
        &[
            "phase",
            "phase_id",
            "status",
            "elapsed_ms",
            "invocation_id",
            "process_id",
            "process_start_ticks",
        ],
        tracing::callsite::Identifier(&PHASE_CALLSITE),
    ),
    tracing::metadata::Kind::EVENT,
);

struct ProcessContext {
    invocation_id: Option<String>,
    process_id: u32,
    process_start_ticks: Option<u64>,
}
static PROCESS_CONTEXT: LazyLock<ProcessContext> = LazyLock::new(|| ProcessContext {
    invocation_id: std::env::var("LCTX_EVIDENCE_INVOCATION")
        .ok()
        .filter(|id| valid_invocation(id)),
    process_id: std::process::id(),
    process_start_ticks: process_start_ticks(),
});
fn valid_invocation(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}
fn process_start_ticks() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        // One process-lifetime observation; never reopen procfs for each event.
        std::fs::read_to_string("/proc/self/stat")
            .ok()?
            .rsplit_once(") ")?
            .1
            .split_whitespace()
            .nth(19)?
            .parse()
            .ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Observe only an actual pending wait. Ready work adds no events; interruption
/// retains the begin without asserting that the underlying owner is terminal.
pub async fn wait<F: std::future::Future>(name: &'static str, future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut phase = None;
    std::future::poll_fn(|context| match future.as_mut().poll(context) {
        std::task::Poll::Pending => {
            if phase.is_none() {
                phase = Some(Phase::begin(name));
            }
            std::task::Poll::Pending
        }
        std::task::Poll::Ready(result) => {
            if let Some(phase) = phase.take() {
                phase.finish(Terminal::Returned);
            }
            std::task::Poll::Ready(result)
        }
    })
    .await
}

/// Preserve the typed refusal and the library's formatted reservation attribution
/// at the one boundary that creates the refusal. These numbers are not RSS.
pub fn resource_refusal(refusal: &lctx_model::domain::ModelError, cause: impl std::fmt::Display) {
    if let lctx_model::domain::ModelError::Resource {
        owner,
        requested,
        used,
        limit,
    } = refusal
    {
        let context = &*PROCESS_CONTEXT;
        tracing::warn!(target: "lctx_phase", observation="resource_refusal",
            invocation_id=context.invocation_id.as_deref(), process_id=context.process_id,
            process_start_ticks=context.process_start_ticks, owner, requested, used, limit,
            cause=%cause, "workspace memory reservation refused");
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Terminal {
    Passed,
    Failed,
    /// A generic synchronous leaf returned; its result's semantic status is owned by its caller.
    Returned,
    /// Use only after gracefully cancelled work has drained.
    Cancelled,
}
impl Terminal {
    fn name(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Returned => "returned",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Dropping this observation emits no terminal: an interrupted operation remains incomplete.
#[must_use = "finish only once the observed work is terminal; dropping leaves it incomplete"]
pub struct Phase {
    name: &'static str,
    id: u64,
    started: Instant,
    span: tracing::Span,
    dispatch: tracing::Dispatch,
}
impl Phase {
    pub fn begin(name: &'static str) -> Self {
        let phase = Self {
            name,
            id: NEXT_PHASE.fetch_add(1, Ordering::Relaxed),
            started: Instant::now(),
            span: tracing::Span::current(),
            dispatch: tracing::dispatcher::get_default(Clone::clone),
        };
        phase.emit("begin", None);
        phase
    }
    fn emit(&self, status: &str, elapsed_ms: Option<f64>) {
        // Scoped subscribers in composed binaries can observe different macro callsite
        // interest. Send this owner's observation to its exact captured dispatcher instead.
        // Register its fixed field inventory there, then honor both metadata and event filters.
        let _ = self.dispatch.register_callsite(&PHASE_METADATA);
        if !self.dispatch.enabled(&PHASE_METADATA) {
            return;
        }
        let context = &*PROCESS_CONTEXT;
        let values: [Option<&dyn tracing::field::Value>; 7] = [
            Some(&self.name),
            Some(&self.id),
            Some(&status),
            elapsed_ms
                .as_ref()
                .map(|value| value as &dyn tracing::field::Value),
            context
                .invocation_id
                .as_ref()
                .map(|value| value as &dyn tracing::field::Value),
            Some(&context.process_id),
            context
                .process_start_ticks
                .as_ref()
                .map(|value| value as &dyn tracing::field::Value),
        ];
        let fields = PHASE_METADATA.fields().value_set_all(&values);
        let event = tracing::Event::new_child_of(self.span.id(), &PHASE_METADATA, &fields);
        self.dispatch.event(&event);
    }
    pub fn finish(self, terminal: Terminal) {
        self.emit(
            terminal.name(),
            Some(self.started.elapsed().as_secs_f64() * 1000.0),
        );
    }
    pub fn finish_result<T, E>(self, result: &Result<T, E>) {
        self.finish(if result.is_ok() {
            Terminal::Passed
        } else {
            Terminal::Failed
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{self, Write},
        sync::{Arc, Mutex},
    };
    use tracing::{Instrument, instrument::WithSubscriber};

    #[derive(Clone, Default)]
    struct Output(Arc<Mutex<Vec<u8>>>);
    impl Write for Output {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl Output {
        fn dispatch(&self) -> tracing::Dispatch {
            let output = self.clone();
            tracing::Dispatch::new(
                tracing_subscriber::fmt()
                    .with_ansi(false)
                    .without_time()
                    .with_writer(move || output.clone())
                    .finish(),
            )
        }
        fn text(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    #[test]
    fn captured_dispatch_honors_phase_and_warning_filters() {
        let default = Output::default();
        let warn = Output::default();
        for (output, filter) in [(&default, "warn,lctx_phase=info"), (&warn, "warn")] {
            let writer = output.clone();
            let dispatch = tracing::Dispatch::new(
                tracing_subscriber::fmt()
                    .with_ansi(false)
                    .without_time()
                    .with_writer(move || writer.clone())
                    .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
                    .finish(),
            );
            tracing::dispatcher::with_default(&dispatch, || {
                tracing::info!(target:"non_phase_control", "unselected information");
                tracing::warn!(target:"non_phase_control", "selected warning");
                let phase = Phase::begin("filtered_phase");
                phase.finish_result(&Ok::<(), ()>(()));
            });
        }
        let default = default.text();
        let warn = warn.text();
        assert!(
            default.contains("selected warning") && !default.contains("unselected information"),
            "{default}"
        );
        let phases = default
            .lines()
            .filter(|line| line.contains("filtered_phase"))
            .collect::<Vec<_>>();
        assert_eq!(phases.len(), 2, "{default}");
        assert!(
            phases[0].contains("status=\"begin\"") && !phases[0].contains("elapsed_ms="),
            "{default}"
        );
        assert!(
            phases[1].contains("status=\"passed\"") && phases[1].contains("elapsed_ms="),
            "{default}"
        );
        assert!(
            warn.contains("selected warning")
                && !warn.contains("filtered_phase")
                && !warn.contains("unselected information"),
            "{warn}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_dispatchers_retain_attribution_and_incomplete_begins() {
        let left = Output::default();
        let right = Output::default();
        let left_task = tokio::spawn(
            async {
                let span = tracing::info_span!("journey", journey = "left");
                async {
                    let phase = Phase::begin("left_work");
                    tokio::task::yield_now().await;
                    phase.finish(Terminal::Passed);
                    let unfinished = Phase::begin("interrupted_work");
                    drop(unfinished);
                }
                .instrument(span)
                .await;
            }
            .with_subscriber(left.dispatch()),
        );
        let right_task = tokio::spawn(
            async {
                let span = tracing::info_span!("journey", journey = "right");
                async {
                    let phase = Phase::begin("right_work");
                    tokio::task::yield_now().await;
                    phase.finish_result(&Err::<(), _>("failed"));
                    Phase::begin("drained_work").finish(Terminal::Cancelled);
                    Phase::begin("generic_return").finish(Terminal::Returned);
                }
                .instrument(span)
                .await;
            }
            .with_subscriber(right.dispatch()),
        );
        left_task.await.unwrap();
        right_task.await.unwrap();
        let left = left.text();
        let right = right.text();
        assert!(
            left.contains("journey=\"left\"") && !left.contains("right_work"),
            "{left}"
        );
        assert!(
            right.contains("journey=\"right\"") && !right.contains("left_work"),
            "{right}"
        );
        assert!(
            left.contains("status=\"passed\"") && left.contains("elapsed_ms="),
            "{left}"
        );
        assert!(
            right.contains("status=\"failed\"") && right.contains("status=\"cancelled\""),
            "{right}"
        );
        assert!(right.contains("status=\"returned\""), "{right}");
        let incomplete = left
            .lines()
            .filter(|line| line.contains("interrupted_work"))
            .collect::<Vec<_>>();
        assert_eq!(incomplete.len(), 1, "{left}");
        assert!(
            incomplete[0].contains("status=\"begin\"") && !incomplete[0].contains("elapsed_ms=")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn abort_retains_begin_without_fabricating_a_terminal() {
        let output = Output::default();
        let (begun, observed) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(
            async move {
                let phase = Phase::begin("aborted_future");
                begun.send(()).unwrap();
                std::future::pending::<()>().await;
                phase.finish(Terminal::Passed);
            }
            .with_subscriber(output.dispatch()),
        );
        observed.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        let retained = output.text();
        let lines = retained
            .lines()
            .filter(|line| line.contains("aborted_future"))
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 1, "{retained}");
        assert!(lines[0].contains("status=\"begin\"") && !lines[0].contains("elapsed_ms="));
    }

    #[tokio::test]
    async fn pending_wait_has_one_pair_ready_wait_has_none_and_drop_stays_incomplete() {
        let output = Output::default();
        let (sender, receiver) = tokio::sync::oneshot::channel::<()>();
        let mut pending =
            Box::pin(wait("controlled_wait", receiver).with_subscriber(output.dispatch()));
        assert!(futures::poll!(&mut pending).is_pending());
        assert!(futures::poll!(&mut pending).is_pending());
        sender.send(()).unwrap();
        pending.await.unwrap();
        wait("ready_wait", async { 17 })
            .with_subscriber(output.dispatch())
            .await;
        let mut unfinished = Box::pin(
            wait("unfinished_wait", std::future::pending::<()>())
                .with_subscriber(output.dispatch()),
        );
        assert!(futures::poll!(&mut unfinished).is_pending());
        drop(unfinished);
        let text = output.text();
        let paired = text
            .lines()
            .filter(|line| line.contains("controlled_wait"))
            .collect::<Vec<_>>();
        assert_eq!(paired.len(), 2, "{text}");
        assert!(paired[0].contains("status=\"begin\""));
        assert!(paired[1].contains("status=\"returned\"") && paired[1].contains("elapsed_ms="));
        assert!(
            paired
                .iter()
                .all(|line| line.contains("phase_id=") && line.contains("process_id="))
        );
        assert!(!text.contains("ready_wait"));
        assert_eq!(
            text.lines()
                .filter(|line| line.contains("unfinished_wait"))
                .count(),
            1
        );
    }

    #[test]
    fn resource_observation_keeps_exact_typed_values_and_process_context() {
        let output = Output::default();
        let refusal = lctx_model::domain::ModelError::Resource {
            owner: "controlled-owner",
            requested: 17,
            used: 31,
            limit: 32,
        };
        tracing::dispatcher::with_default(&output.dispatch(), || {
            resource_refusal(&refusal, "largest admitted consumer: selected inventory")
        });
        let text = output.text();
        assert_eq!(text.lines().count(), 1, "{text}");
        for field in [
            "resource_refusal",
            "controlled-owner",
            "requested=17",
            "used=31",
            "limit=32",
            "process_id=",
            "largest admitted consumer",
        ] {
            assert!(text.contains(field), "missing {field}: {text}");
        }
        assert!(valid_invocation("0123456789abcdef0123456789abcdef"));
        assert!(!valid_invocation("not-an-invocation"));
    }
}
