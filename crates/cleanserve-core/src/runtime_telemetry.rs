use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const MAX_LOG_LINES: usize = 200;

pub struct RuntimeTelemetry {
    pub requests: AtomicU64,
    log: Mutex<VecDeque<String>>,
}

impl RuntimeTelemetry {
    pub fn new() -> Self {
        Self {
            requests: AtomicU64::new(0),
            log: Mutex::new(VecDeque::new()),
        }
    }

    pub fn global() -> &'static RuntimeTelemetry {
        static INSTANCE: OnceLock<RuntimeTelemetry> = OnceLock::new();
        INSTANCE.get_or_init(RuntimeTelemetry::new)
    }

    pub fn record_request(&self) {
        self.requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn push_log(&self, line: impl Into<String>) {
        let mut log = self.log.lock().unwrap_or_else(|e| e.into_inner());
        log.push_back(line.into());
        while log.len() > MAX_LOG_LINES {
            log.pop_front();
        }
    }

    pub fn snapshot_logs(&self) -> Vec<String> {
        self.log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .cloned()
            .collect()
    }

    pub fn request_count(&self) -> u64 {
        self.requests.load(Ordering::Relaxed)
    }
}

impl Default for RuntimeTelemetry {
    fn default() -> Self {
        Self::new()
    }
}
