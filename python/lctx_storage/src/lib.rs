//! Canonical generation serving through one cancellation-safe asynchronous bridge.
mod serving;
use pyo3::prelude::*;
use std::sync::Once;
#[pymodule]
fn _storage(m:&Bound<'_,PyModule>)->PyResult<()> {
    static INIT:Once=Once::new();
    INIT.call_once(||{let mut builder=tokio::runtime::Builder::new_multi_thread();builder.worker_threads(2).max_blocking_threads(4).enable_all();pyo3_async_runtimes::tokio::init(builder);});
    serving::register(m)
}
