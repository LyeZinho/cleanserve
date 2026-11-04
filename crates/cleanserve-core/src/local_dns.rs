//! Tiny DNS responder for `*.serve` development domains.
//!
//! `cleanserve setupdomain` routes the configured suffix to this server via
//! systemd-resolved, so every subdomain resolves to loopback without editing
//! `/etc/hosts` again.

use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::UdpSocket;
use tracing::info;

pub const DEFAULT_DNS_PORT: u16 = 5353;

/// Answer A queries for `<anything>.<suffix>` with 127.0.0.1.
///
/// Non-matching names and other record types are answered with NXDOMAIN / no
/// data so the resolver can fall back cleanly.
pub async fn serve(suffix: &str) -> anyhow::Result<()> {
    let suffix = suffix.trim().trim_start_matches('.').to_ascii_lowercase();
    if suffix.is_empty() {
        anyhow::bail!("Cannot start local DNS without a domain suffix");
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], DEFAULT_DNS_PORT));
    let socket = UdpSocket::bind(addr).await?;
    info!(
        "🌐 Local DNS for *.{} listening on {}",
        suffix, addr
    );

    let mut buf = [0u8; 1500];
    loop {
        let (len, peer) = socket.recv_from(&mut buf).await?;
        if let Some(response) = build_response(&buf[..len], &suffix) {
            let _ = socket.send_to(&response, peer).await;
        }
    }
}

fn build_response(query: &[u8], suffix: &str) -> Option<Vec<u8>> {
    if query.len() < 12 {
        return None;
    }

    let id = &query[0..2];
    let flags = u16::from_be_bytes([query[2], query[3]]);
    let rd = (flags & 0x0100) != 0;
    let (name, qtype, question_end) = parse_question(query)?;
    let matches = name == suffix || name.ends_with(&format!(".{}", suffix));

    let mut response = Vec::with_capacity(question_end + 16);
    response.extend_from_slice(id);

    let (rcode, answer_count) = if !matches {
        (3u16, 0u16) // NXDOMAIN
    } else if qtype == 1 {
        (0u16, 1u16) // A
    } else {
        (0u16, 0u16) // NOERROR / no data
    };

    let mut resp_flags = 0x8000u16; // QR
    resp_flags |= 0x0400; // AA
    if rd {
        resp_flags |= 0x0100;
    }
    resp_flags |= 0x0080; // RA
    resp_flags |= rcode & 0x000f;
    response.extend_from_slice(&resp_flags.to_be_bytes());
    response.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT
    response.extend_from_slice(&answer_count.to_be_bytes());
    response.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT
    response.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT

    response.extend_from_slice(&query[12..question_end]);

    if answer_count == 1 {
        response.extend_from_slice(&[0xc0, 0x0c]); // pointer to question name
        response.extend_from_slice(&1u16.to_be_bytes()); // A
        response.extend_from_slice(&1u16.to_be_bytes()); // IN
        response.extend_from_slice(&30u32.to_be_bytes()); // TTL
        response.extend_from_slice(&4u16.to_be_bytes()); // RDLENGTH
        response.extend_from_slice(&Ipv4Addr::LOCALHOST.octets());
    }

    Some(response)
}

fn parse_question(query: &[u8]) -> Option<(String, u16, usize)> {
    let mut pos = 12;
    let mut labels = Vec::new();

    loop {
        let len = *query.get(pos)? as usize;
        pos += 1;
        if len == 0 {
            break;
        }
        if len & 0xc0 != 0 || pos + len > query.len() {
            return None;
        }
        let label = std::str::from_utf8(&query[pos..pos + len]).ok()?;
        labels.push(label.to_ascii_lowercase());
        pos += len;
    }

    let qtype = u16::from_be_bytes([*query.get(pos)?, *query.get(pos + 1)?]);
    let question_end = pos + 4;
    if question_end > query.len() {
        return None;
    }

    Some((labels.join("."), qtype, question_end))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(name: &str, qtype: u16) -> Vec<u8> {
        let mut q = vec![0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
        for label in name.split('.') {
            q.push(label.len() as u8);
            q.extend_from_slice(label.as_bytes());
        }
        q.push(0);
        q.extend_from_slice(&qtype.to_be_bytes());
        q.extend_from_slice(&1u16.to_be_bytes());
        q
    }

    #[test]
    fn answers_subdomain_with_loopback() {
        let response = build_response(&query("frontend.serve", 1), "serve").unwrap();
        assert_eq!(&response[0..2], &[0x12, 0x34]);
        assert_eq!(u16::from_be_bytes([response[6], response[7]]), 1);
        assert_eq!(&response[response.len() - 4..], &[127, 0, 0, 1]);
    }

    #[test]
    fn rejects_other_domains() {
        let response = build_response(&query("example.com", 1), "serve").unwrap();
        assert_eq!(response[3] & 0x0f, 3); // NXDOMAIN
        assert_eq!(u16::from_be_bytes([response[6], response[7]]), 0);
    }
}
