//! Probe: nextest per-test env, live side channel, SIGTERM grace handling.
use std::{io::Write, os::fd::AsRawFd, sync::atomic::{AtomicI32, Ordering}, time::Duration};

static SIDE_FD: AtomicI32 = AtomicI32::new(-1);
extern "C" fn on_term(_: libc::c_int) {
    let msg = b"signal=SIGTERM handled in-process\n";
    let fd = SIDE_FD.load(Ordering::Relaxed);
    if fd >= 0 { unsafe { libc::write(fd, msg.as_ptr().cast(), msg.len()); } }
}

fn side_file(name: &str) -> std::fs::File {
    let dir = std::env::var("PROBE_DIR").unwrap();
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(format!("{dir}/{name}.side")).unwrap();
    for key in ["NEXTEST_RUN_ID", "NEXTEST_TEST_NAME", "NEXTEST_ATTEMPT_ID", "NEXTEST_BINARY_ID", "NEXTEST_PROFILE", "NEXTEST_EXECUTION_MODE", "NEXTEST_TEST_GLOBAL_SLOT", "NEXTEST_ATTEMPT"] {
        writeln!(f, "{key}={}", std::env::var(key).unwrap_or_else(|_| "<unset>".into())).unwrap();
    }
    writeln!(f, "pid={} pgid={}", std::process::id(), unsafe { libc::getpgrp() }).unwrap();
    f
}

fn heartbeat(mut f: std::fs::File, secs: u64) {
    for i in 0..secs * 5 {
        writeln!(f, "heartbeat={i}").unwrap();
        eprintln!("stderr heartbeat {i}");
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[test]
fn handles_term_and_lingers() {
    let f = side_file("handles_term");
    SIDE_FD.store(f.as_raw_fd(), Ordering::Relaxed);
    unsafe { libc::signal(libc::SIGTERM, on_term as *const () as libc::sighandler_t); }
    heartbeat(f, 60);
}

#[test]
fn default_term_dies() {
    heartbeat(side_file("default_term"), 60);
}

#[test]
fn quick_pass() {
    eprintln!("quick stderr");
}
