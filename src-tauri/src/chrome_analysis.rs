//! Shared cancel flag for in-flight Chrome analysis requests.

use std::sync::atomic::{AtomicBool, Ordering};

static CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn begin_chrome_analysis() {
    CANCEL_REQUESTED.store(false, Ordering::SeqCst);
}

pub fn cancel_chrome_analysis() {
    CANCEL_REQUESTED.store(true, Ordering::SeqCst);
}

pub fn check_chrome_analysis_cancelled() -> Result<(), String> {
    if CANCEL_REQUESTED.load(Ordering::SeqCst) {
        return Err("Cancelled".to_string());
    }
    Ok(())
}
