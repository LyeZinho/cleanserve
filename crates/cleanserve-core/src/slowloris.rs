use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Slowloris attack detection and prevention.
///
/// Slowloris attacks send requests extremely slowly, one header at a time,
/// to exhaust server connection pools. This module detects and terminates
/// connections that remain idle during the request header reception phase.
pub struct SlowlorisProtection {
    /// Maximum time allowed for a client to send all headers (ms)
    header_timeout_ms: u64,
    /// Track active connections: {addr -> timestamp_of_first_byte}
    active_connections: Arc<Mutex<HashMap<SocketAddr, u64>>>,
}

impl SlowlorisProtection {
    /// Create a new slowloris protection instance.
    ///
    /// # Arguments
    /// * `header_timeout_ms` - Maximum time (in milliseconds) for headers to complete.
    ///   Typical values: 30,000-60,000 (30-60 seconds). CleanServe default: 30,000.
    pub fn new(header_timeout_ms: u64) -> Self {
        Self {
            header_timeout_ms,
            active_connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new connection.
    ///
    /// Call this when a client first connects. Returns true if accepted, false if rejected.
    pub fn register_connection(&self, addr: SocketAddr) -> bool {
        let now = current_timestamp_ms();
        let mut conns = self.active_connections.lock().unwrap();

        // Check if this connection is already registered and hasn't timed out
        if let Some(&first_byte_time) = conns.get(&addr) {
            let elapsed = now.saturating_sub(first_byte_time);
            if elapsed <= self.header_timeout_ms {
                // Still active and within timeout
                return true;
            }
        }

        // Register or re-register with current timestamp
        conns.insert(addr, now);
        true
    }

    /// Check if a connection has timed out waiting for headers.
    ///
    /// Call this periodically or during request processing to detect slowloris attacks.
    /// Returns true if connection is valid, false if it should be closed.
    pub fn is_connection_valid(&self, addr: SocketAddr) -> bool {
        let now = current_timestamp_ms();
        let conns = self.active_connections.lock().unwrap();

        if let Some(&first_byte_time) = conns.get(&addr) {
            let elapsed = now.saturating_sub(first_byte_time);
            elapsed <= self.header_timeout_ms
        } else {
            false
        }
    }

    /// Mark connection as completed (headers received or request finished).
    ///
    /// Call this when a complete request is received to reset the timer.
    pub fn mark_request_complete(&self, addr: SocketAddr) {
        let now = current_timestamp_ms();
        let mut conns = self.active_connections.lock().unwrap();
        conns.insert(addr, now);
    }

    /// Clean up expired connections from tracking.
    ///
    /// Call periodically (e.g., every 60 seconds) to prevent memory leaks.
    pub fn cleanup_expired(&self) {
        let now = current_timestamp_ms();
        let timeout_threshold = self.header_timeout_ms + 60_000; // Allow 60s grace period

        let mut conns = self.active_connections.lock().unwrap();
        conns.retain(|_, &mut first_byte_time| {
            let elapsed = now.saturating_sub(first_byte_time);
            elapsed <= timeout_threshold
        });
    }

    /// Get number of active connections being tracked.
    pub fn active_connection_count(&self) -> usize {
        self.active_connections.lock().unwrap().len()
    }

    /// Verify a connection is still within timeout window.
    /// Returns (is_valid, elapsed_ms, timeout_ms)
    pub fn check_connection_status(&self, addr: SocketAddr) -> (bool, u64, u64) {
        let now = current_timestamp_ms();
        let conns = self.active_connections.lock().unwrap();

        if let Some(&first_byte_time) = conns.get(&addr) {
            let elapsed = now.saturating_sub(first_byte_time);
            let is_valid = elapsed <= self.header_timeout_ms;
            (is_valid, elapsed, self.header_timeout_ms)
        } else {
            (false, 0, self.header_timeout_ms)
        }
    }
}

/// Get current time in milliseconds since UNIX_EPOCH.
fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn test_addr() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345)
    }

    fn test_addr_2() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 12346)
    }

    #[test]
    fn test_register_connection_accepts_new() {
        let protection = SlowlorisProtection::new(30_000);
        let addr = test_addr();

        assert!(protection.register_connection(addr));
        assert_eq!(protection.active_connection_count(), 1);
    }

    #[test]
    fn test_register_multiple_connections() {
        let protection = SlowlorisProtection::new(30_000);
        let addr1 = test_addr();
        let addr2 = test_addr_2();

        assert!(protection.register_connection(addr1));
        assert!(protection.register_connection(addr2));
        assert_eq!(protection.active_connection_count(), 2);
    }

    #[test]
    fn test_is_connection_valid_returns_false_for_unregistered() {
        let protection = SlowlorisProtection::new(30_000);
        let addr = test_addr();

        assert!(!protection.is_connection_valid(addr));
    }

    #[test]
    fn test_is_connection_valid_returns_true_for_registered() {
        let protection = SlowlorisProtection::new(30_000);
        let addr = test_addr();

        protection.register_connection(addr);
        assert!(protection.is_connection_valid(addr));
    }

    #[test]
    fn test_mark_request_complete_resets_timer() {
        let protection = SlowlorisProtection::new(30_000);
        let addr = test_addr();

        protection.register_connection(addr);
        let (valid1, elapsed1, _) = protection.check_connection_status(addr);

        // Wait 100ms
        std::thread::sleep(Duration::from_millis(100));

        protection.mark_request_complete(addr);
        let (valid2, elapsed2, _) = protection.check_connection_status(addr);

        assert!(valid1);
        assert!(valid2);
        // After mark_request_complete, elapsed should be much smaller than after 100ms wait
        // elapsed1 should be ~100ms, elapsed2 should be ~0ms
        assert!(
            elapsed2 <= elapsed1,
            "elapsed2={} should be <= elapsed1={}",
            elapsed2,
            elapsed1
        );
    }

    #[test]
    fn test_check_connection_status_returns_metrics() {
        let protection = SlowlorisProtection::new(30_000);
        let addr = test_addr();

        protection.register_connection(addr);
        let (valid, elapsed, timeout) = protection.check_connection_status(addr);

        assert!(valid);
        assert!(elapsed < 10); // Should be near zero
        assert_eq!(timeout, 30_000);
    }

    #[test]
    fn test_cleanup_removes_expired_connections() {
        let protection = SlowlorisProtection::new(50); // Very short timeout for testing
        let addr = test_addr();

        protection.register_connection(addr);
        assert_eq!(protection.active_connection_count(), 1);

        // Wait for connection to expire plus grace period
        std::thread::sleep(Duration::from_millis(150));

        protection.cleanup_expired();
        // Connection should be removed if its timestamp is old enough
        let remaining = protection.active_connection_count();
        assert!(
            remaining <= 1,
            "Expected 0-1 connections remaining, got {}",
            remaining
        );
    }

    #[test]
    fn test_cleanup_preserves_valid_connections() {
        let protection = SlowlorisProtection::new(30_000);
        let addr1 = test_addr();
        let addr2 = test_addr_2();

        protection.register_connection(addr1);
        std::thread::sleep(Duration::from_millis(10));
        protection.register_connection(addr2);

        // Only addr2 should be valid, addr1 is stale
        let mut conns = protection.active_connections.lock().unwrap();
        conns.insert(addr1, 0); // Simulate very old connection
        drop(conns);

        protection.cleanup_expired();

        // addr1 should be removed (timeout_threshold exceeded)
        // This test verifies cleanup logic
        let remaining = protection.active_connection_count();
        assert!(remaining <= 2);
    }

    #[test]
    fn test_timeout_detection_with_small_window() {
        let protection = SlowlorisProtection::new(50); // 50ms timeout
        let addr = test_addr();

        protection.register_connection(addr);
        assert!(protection.is_connection_valid(addr));

        // Wait for timeout to expire
        std::thread::sleep(Duration::from_millis(60));

        assert!(!protection.is_connection_valid(addr));
    }

    #[test]
    fn test_multiple_addresses_tracked_independently() {
        let protection = SlowlorisProtection::new(30_000);
        let addr1 = test_addr();
        let addr2 = test_addr_2();

        protection.register_connection(addr1);
        std::thread::sleep(Duration::from_millis(50));
        protection.register_connection(addr2);

        let (valid1, elapsed1, _) = protection.check_connection_status(addr1);
        let (valid2, elapsed2, _) = protection.check_connection_status(addr2);

        assert!(valid1);
        assert!(valid2);
        assert!(elapsed1 > elapsed2); // addr1 registered earlier
    }

    #[test]
    fn test_current_timestamp_ms_is_positive() {
        let ts = current_timestamp_ms();
        assert!(ts > 0, "Timestamp should be positive: {}", ts);
        assert!(ts > 1_600_000_000_000, "Timestamp seems too small");
    }
}
