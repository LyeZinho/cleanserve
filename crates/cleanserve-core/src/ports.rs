use std::net::TcpListener;

/// Returns true if `127.0.0.1:port` can be bound (port is free).
pub fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// First free port in `[preferred, preferred + max_attempts)`.
pub fn find_available_port(preferred: u16, max_attempts: u16) -> Option<u16> {
    for offset in 0..max_attempts {
        let port = preferred.saturating_add(offset);
        if port == 0 || port > 65534 {
            break;
        }
        if is_port_available(port) {
            return Some(port);
        }
    }
    None
}

/// Pick HTTP + HMR ports. When `needs_hmr`, `hmr` is `main + 1` and both must be free.
pub fn find_server_ports(preferred: u16, needs_hmr: bool) -> Option<(u16, u16)> {
    const SCAN: u16 = 128;
    for offset in 0..SCAN {
        let main = preferred.saturating_add(offset);
        if main == 0 || main > 65533 {
            break;
        }
        if !is_port_available(main) {
            continue;
        }
        let hmr = main + 1;
        if needs_hmr && !is_port_available(hmr) {
            continue;
        }
        return Some((main, hmr));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn find_server_ports_skips_blocked_main() {
        let blocker = TcpListener::bind(("127.0.0.1", 18080)).unwrap();
        let (main, hmr) = find_server_ports(18080, true).expect("should find alternate ports");
        assert_ne!(main, 18080);
        assert_eq!(hmr, main + 1);
        drop(blocker);
    }
}
