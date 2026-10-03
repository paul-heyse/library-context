//! The shared coordinator for native provider pumps and computed normalization stages.
use lctx_model::domain::{
    ModelError,
    stages::{Execution, Stage, StageAccess},
};

/// Operational telemetry is outside all semantic records and content digests.
#[derive(Debug, serde::Serialize)]
pub struct StageMeasurement {
    pub stage: &'static str,
    pub elapsed_ms: u128,
    pub sampled_peak_rss_bytes: Option<usize>,
    pub reservation_peak_bytes: Option<usize>,
    pub reservation_end_bytes: Option<usize>,
    pub outcome: &'static str,
}
fn process_rss() -> Option<usize> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix("VmRSS:").and_then(|value| {
            value
                .split_whitespace()
                .next()?
                .parse::<usize>()
                .ok()?
                .checked_mul(1024)
        })
    })
}
/// Borrowed CPU work drains synchronously; no access or reservation escapes the closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuPlacement {
    BlockingRegion,
    Inline,
}
pub fn cpu_placement() -> CpuPlacement {
    if tokio::runtime::Handle::try_current()
        .is_ok_and(|h| h.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread)
    {
        CpuPlacement::BlockingRegion
    } else {
        CpuPlacement::Inline
    }
}
pub fn borrowed_cpu<T>(stage: &'static str, run: impl FnOnce() -> T) -> T {
    let placement = cpu_placement();
    let span = tracing::info_span!("borrowed_cpu", stage, placement=?placement);
    let _entered = span.enter();
    match placement {
        CpuPlacement::BlockingRegion => tokio::task::block_in_place(run),
        CpuPlacement::Inline => run(),
    }
}
struct Sampler {
    stop: tokio::sync::watch::Sender<bool>,
    task: Option<tokio::task::JoinHandle<Option<usize>>>,
}
impl Sampler {
    fn start() -> Self {
        let (stop, mut receiver) = tokio::sync::watch::channel(false);
        let task = tokio::spawn(async move {
            let mut peak = process_rss();
            loop {
                if *receiver.borrow() {
                    break;
                }
                tokio::select! {
                    result = receiver.changed() => { if result.is_err() || *receiver.borrow() { break; } },
                    _ = tokio::time::sleep(std::time::Duration::from_millis(20)) => {
                        if let Some(rss) = process_rss() { peak = Some(peak.unwrap_or(0).max(rss)); }
                    }
                }
            }
            if let Some(rss) = process_rss() {
                peak = Some(peak.unwrap_or(0).max(rss));
            }
            peak
        });
        Self {
            stop,
            task: Some(task),
        }
    }
    async fn finish(mut self) -> Result<Option<usize>, ModelError> {
        self.stop.send_replace(true);
        self.task
            .take()
            .expect("sampler task")
            .await
            .map_err(ModelError::codec)
    }
}
impl Drop for Sampler {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}

/// Check the executable declaration before any stage effect, then measure its acknowledged
/// completion. Both native and computed stages use this boundary; neither can forge a receipt.
pub async fn run_declared_stage<'e, 's, T>(
    execution: &'e mut Execution<'s>,
    declaration: &Stage,
    run: impl AsyncFnOnce(StageAccess<'e, 's>) -> Result<T, ModelError>,
    observe: &mut impl FnMut(StageMeasurement),
) -> Result<T, ModelError> {
    run_stage(execution, declaration, None, run, observe).await
}
pub async fn run_declared_stage_with_resources<'e, 's, T>(
    execution: &'e mut Execution<'s>,
    declaration: &Stage,
    budget: &lctx_model::domain::resources::ResourceBudget,
    run: impl AsyncFnOnce(StageAccess<'e, 's>) -> Result<T, ModelError>,
    observe: &mut impl FnMut(StageMeasurement),
) -> Result<T, ModelError> {
    run_stage(execution, declaration, Some(budget), run, observe).await
}
async fn run_stage<'e, 's, T>(
    execution: &'e mut Execution<'s>,
    declaration: &Stage,
    budget: Option<&lctx_model::domain::resources::ResourceBudget>,
    run: impl AsyncFnOnce(StageAccess<'e, 's>) -> Result<T, ModelError>,
    observe: &mut impl FnMut(StageMeasurement),
) -> Result<T, ModelError> {
    let scheduled = execution
        .schedule()
        .stages()
        .iter()
        .find(|s| s.name == declaration.name)
        .ok_or_else(|| ModelError::Invalid("executable stage is not scheduled".into()))?;
    if scheduled.digest() != declaration.digest() {
        return Err(ModelError::Invalid(
            "executable declaration differs from schedule".into(),
        ));
    }
    let access = execution.begin(declaration.name)?;
    if let Some(budget) = budget {
        budget.reset_stage_peak();
    }
    let started = std::time::Instant::now();
    let sampler = Sampler::start();
    let result = run(access).await;
    let peak = sampler.finish().await?;
    observe(StageMeasurement {
        stage: declaration.name,
        elapsed_ms: started.elapsed().as_millis(),
        sampled_peak_rss_bytes: peak,
        reservation_peak_bytes: budget.and_then(|b| b.stage_peak()),
        reservation_end_bytes: budget.map(|b| b.reserved()),
        outcome: if result.is_ok() { "passed" } else { "failed" },
    });
    result
}

#[cfg(test)]
mod cpu_tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    #[test]
    fn outside_runtime_is_explicit_inline() {
        assert_eq!(cpu_placement(), CpuPlacement::Inline);
        let borrowed = String::from("borrowed");
        assert_eq!(borrowed_cpu("test", || borrowed.len()), 8);
    }
    #[tokio::test]
    async fn current_thread_fallback_preserves_borrows_without_panicking() {
        assert_eq!(cpu_placement(), CpuPlacement::Inline);
        let mut n = 0;
        borrowed_cpu("test", || n += 1);
        assert_eq!(n, 1);
        Sampler::start().finish().await.unwrap();
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn independent_heartbeat_progresses_during_borrowed_cpu_and_sampler_exits() {
        let heartbeat = Arc::new(AtomicBool::new(false));
        let task_flag = heartbeat.clone();
        let task = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            task_flag.store(true, Ordering::Release);
        });
        let sampler = Sampler::start();
        let borrowed = String::from("retained");
        borrowed_cpu("test", || {
            assert_eq!(cpu_placement(), CpuPlacement::BlockingRegion);
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while !heartbeat.load(Ordering::Acquire) {
                assert!(
                    std::time::Instant::now() < deadline,
                    "heartbeat suspended by CPU kernel"
                );
                std::thread::yield_now();
            }
            assert_eq!(borrowed, "retained");
        });
        task.await.unwrap();
        sampler.finish().await.unwrap();
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn cancelled_borrowed_work_drains_before_releasing_its_charge_and_sampler_stops() {
        use lctx_model::domain::resources::ResourceBudget;
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let charged_budget = budget.clone();
        let release = Arc::new(AtomicBool::new(false));
        let worker_release = release.clone();
        let drained = Arc::new(AtomicBool::new(false));
        let worker_drained = drained.clone();
        let (entered, started) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _charge = charged_budget.reserve("cpu-drain-control", 4096).unwrap();
            borrowed_cpu("drain-control", || {
                entered.send(()).unwrap();
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
                while !worker_release.load(Ordering::Acquire) {
                    assert!(std::time::Instant::now() < deadline);
                    std::thread::yield_now();
                }
                worker_drained.store(true, Ordering::Release);
            });
            tokio::task::yield_now().await;
        });
        started.await.unwrap();
        task.abort();
        assert_eq!(budget.reserved(), 4096);
        assert!(!drained.load(Ordering::Acquire));
        release.store(true, Ordering::Release);
        let _ = task.await;
        assert!(drained.load(Ordering::Acquire));
        assert_eq!(budget.reserved(), 0);
        let mut sampler = Sampler::start();
        let sampling = sampler.task.take().unwrap();
        drop(sampler);
        tokio::time::timeout(std::time::Duration::from_secs(2), sampling)
            .await
            .unwrap()
            .unwrap();
    }
}
