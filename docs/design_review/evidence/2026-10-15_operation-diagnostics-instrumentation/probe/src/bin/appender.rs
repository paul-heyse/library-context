//! Probe: tracing-appender non_blocking vs direct append File under SIGTERM/panic;
//! per-layer filtering; FmtSpan::CLOSE busy/idle; static TRACE elision.
use std::{fs::OpenOptions, sync::Arc, time::Duration};
use tracing_subscriber::{fmt::format::FmtSpan, prelude::*, filter::{LevelFilter, Targets}};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = &args[1];
    let mode = args[2].as_str();
    let n: u64 = args[3].parse().unwrap();
    let nb_file = OpenOptions::new().create(true).append(true).open(format!("{dir}/nonblocking.jsonl")).unwrap();
    struct Slow(std::fs::File, bool);
    impl std::io::Write for Slow { fn write(&mut self, b: &[u8]) -> std::io::Result<usize> { if self.1 { std::thread::sleep(Duration::from_micros(20)); } self.0.write(b) } fn flush(&mut self) -> std::io::Result<()> { self.0.flush() } }
    let (nb, guard) = tracing_appender::non_blocking(Slow(nb_file, mode.contains("slow")));
    let dropped = nb.error_counter();
    let direct = Arc::new(OpenOptions::new().create(true).append(true).open(format!("{dir}/direct.jsonl")).unwrap());
    let phases = Targets::new().with_target("probe_phase", LevelFilter::TRACE).with_target("probe_span", LevelFilter::TRACE);
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().json().with_writer(nb).with_filter(phases.clone().with_target("probe_nb", LevelFilter::TRACE)))
        .with(tracing_subscriber::fmt::layer().json().with_span_events(FmtSpan::CLOSE).with_current_span(true).with_span_list(true)
              .with_writer(direct).with_filter(phases))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr).with_filter(LevelFilter::WARN));
    let _d = tracing::dispatcher::set_default(&subscriber.into());

    // Span timing: parent entered 20ms of own work + child 50ms busy; then 30ms idle (not entered).
    {
        let parent = tracing::info_span!(target: "probe_span", "parent");
        {
            let _p = parent.enter();
            std::thread::sleep(Duration::from_millis(20));
            let child = tracing::info_span!(target: "probe_span", "child");
            { let _c = child.enter(); std::thread::sleep(Duration::from_millis(50)); }
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    let mut trace_attempts = 0u64;
    for _ in 0..10 { tracing::trace!(target: "probe_phase", kind = "trace_level", "trace"); trace_attempts += 1; }
    tracing::info!(target: "probe_phase", kind = "info_only_in_files", "should not reach stderr");
    tracing::warn!(target: "probe_phase", kind = "warn_everywhere", "warn reaches stderr too");
    let nb_only = mode.contains("-nb");
    for seq in 0..n { if nb_only { tracing::info!(target: "probe_nb", seq, kind = "burst", "event"); } else { tracing::info!(target: "probe_phase", seq, kind = "burst", "event"); } }
    let mode = mode.split('-').next().unwrap();
    eprintln!("dropped_lines_at_end={}", dropped.dropped_lines());
    eprintln!("emitted={n} trace_attempts={trace_attempts} debug_assertions={}", cfg!(debug_assertions));
    match mode {
        "term" => unsafe { libc::raise(libc::SIGTERM); },
        "abort" => std::process::abort(),
        "panic" => panic!("probe panic with guard on stack"),
        _ => {}
    }
    drop(guard);
}
