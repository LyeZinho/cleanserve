use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

const MAX_LOG_LINES: usize = 200;
const MAX_REQUEST_LINES: usize = 500;

/// One handled HTTP request, as shown in the dashboard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestEntry {
    pub at: SystemTime,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration: Duration,
}

pub struct RuntimeTelemetry {
    pub requests: AtomicU64,
    tls_handshake_failures: AtomicU64,
    log: Mutex<VecDeque<String>>,
    requests_log: Mutex<VecDeque<RequestEntry>>,
}

impl RuntimeTelemetry {
    pub fn new() -> Self {
        Self {
            requests: AtomicU64::new(0),
            tls_handshake_failures: AtomicU64::new(0),
            log: Mutex::new(VecDeque::new()),
            requests_log: Mutex::new(VecDeque::new()),
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

    /// Record a finished request (oldest entries are dropped past a fixed cap).
    pub fn push_request(&self, entry: RequestEntry) {
        let mut log = self.requests_log.lock().unwrap_or_else(|e| e.into_inner());
        log.push_back(entry);
        while log.len() > MAX_REQUEST_LINES {
            log.pop_front();
        }
    }

    /// Browser or client rejected the self-signed certificate (rustls CertificateUnknown).
    pub fn record_tls_handshake_failure(&self) {
        let count = self.tls_handshake_failures.fetch_add(1, Ordering::Relaxed) + 1;
        if count == 1 {
            self.push_log(
                "TLS handshake rejected — open the site in your browser and accept the certificate, or run: cleanserve up --http-only",
            );
        } else if count == 10 || count % 100 == 0 {
            self.push_log(format!(
                "TLS handshake rejected ({} connections) — trust the localhost cert or use --http-only",
                count
            ));
        }
    }

    pub fn tls_handshake_failures(&self) -> u64 {
        self.tls_handshake_failures.load(Ordering::Relaxed)
    }

    /// Requests in chronological order (oldest first).
    pub fn snapshot_requests(&self) -> Vec<RequestEntry> {
        self.requests_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .cloned()
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str) -> RequestEntry {
        RequestEntry {
            at: SystemTime::now(),
            method: "GET".into(),
            path: path.into(),
            status: 200,
            duration: Duration::from_millis(3),
        }
    }

    #[test]
    fn requests_are_chronological_and_capped() {
        let t = RuntimeTelemetry::new();
        for i in 0..(MAX_REQUEST_LINES + 10) {
            t.push_request(entry(&format!("/{i}")));
        }
        let snap = t.snapshot_requests();
        assert_eq!(snap.len(), MAX_REQUEST_LINES);
        assert_eq!(snap.first().unwrap().path, "/10");
        assert_eq!(snap.last().unwrap().path, format!("/{}", MAX_REQUEST_LINES + 9));
    }

    #[test]
    fn counts_requests_and_logs() {
        let t = RuntimeTelemetry::new();
        t.record_request();
        t.record_request();
        t.push_log("hello");
        assert_eq!(t.request_count(), 2);
        assert_eq!(t.snapshot_logs(), vec!["hello".to_string()]);
    }
}
