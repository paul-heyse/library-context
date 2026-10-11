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
    let span =
        tracing::debug_span!(target: "lctx_phase", "compiler_cpu", producer, placement=?placement);
    let _entered = span.enter();
    // This phase records synchronous execution returning, not semantic success of a generic T.
    let started = std::time::Instant::now();
    tracing::debug!(target: "lctx_phase", phase="borrowed_cpu", producer, status="begin");
    let result = match placement {
        CpuPlacement::BlockingRegion => tokio::task::block_in_place(run),
        CpuPlacement::Inline => run(),
    };
    tracing::debug!(target: "lctx_phase", phase="borrowed_cpu", producer, status="returned",
        elapsed_ms=started.elapsed().as_secs_f64() * 1000.0);
    result
}

#[cfg(test)]
mod tests {
    use super::borrowed_cpu;
    use std::{io::{self, Write}, sync::{Arc, Mutex}};

    #[derive(Clone, Default)]
    struct Output(Arc<Mutex<Vec<u8>>>);
    impl Write for Output {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> { Ok(()) }
    }

    #[test]
    fn root_detail_requires_debug_while_coarse_terminals_remain_visible() {
        for (filter, detailed) in [("warn,lctx_phase=info", false), ("warn,lctx_phase=debug", true)] {
            let output = Output::default();
            let writer = output.clone();
            let subscriber = tracing_subscriber::fmt()
                .with_ansi(false).without_time()
                .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
                .with_writer(move || writer.clone()).finish();
            tracing::subscriber::with_default(subscriber, || {
                let phase = lctx_native::phase::Phase::begin("coarse_stage");
                assert_eq!(borrowed_cpu("selected_root", || 42), 42);
                phase.finish(lctx_native::phase::Terminal::Failed);
                lctx_native::phase::Phase::begin("drain").finish(lctx_native::phase::Terminal::Cancelled);
            });
            let text = String::from_utf8(output.0.lock().unwrap().clone()).unwrap();
            assert!(text.contains("coarse_stage") && text.contains("status=\"failed\""), "{text}");
            assert!(text.contains("drain") && text.contains("status=\"cancelled\""), "{text}");
            assert_eq!(text.contains("borrowed_cpu"), detailed, "{text}");
            assert_eq!(text.contains("selected_root"), detailed, "{text}");
        }
    }
}
