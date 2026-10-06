//! Borrowed CPU work stays owned by the caller and drains before its input buffers are released.
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
pub fn borrowed_cpu<T>(producer: &'static str, run: impl FnOnce() -> T) -> T {
    let placement = cpu_placement();
    let span = tracing::info_span!("compiler_cpu", producer, placement=?placement);
    let _entered = span.enter();
    match placement {
        CpuPlacement::BlockingRegion => tokio::task::block_in_place(run),
        CpuPlacement::Inline => run(),
    }
}
