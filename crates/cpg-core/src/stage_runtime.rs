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
    let mut peak = process_rss();
    let work = run(access);
    tokio::pin!(work);
    let result = loop {
        tokio::select! {
            result = &mut work => break result,
            _ = tokio::time::sleep(std::time::Duration::from_millis(20)) => {
                if let Some(rss) = process_rss() { peak = Some(peak.unwrap_or(0).max(rss)); }
            }
        }
    };
    if let Some(rss) = process_rss() {
        peak = Some(peak.unwrap_or(0).max(rss));
    }
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
