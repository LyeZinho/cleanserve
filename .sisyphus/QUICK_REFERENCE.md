# CleanServe Security Hardening - Quick Reference Card

## 📌 One-Page Cheat Sheet

### Current Status
```
✅ Foundation exists: SecurityHeaders, RateLimiter, SSL, Logging
❌ Integration missing: Rate limiter NOT CALLED, FastCGI unused, no validation
🔴 BLOCKING: PHP forwarding broken - only serves static files!
```

### 5 Security Layers to Implement

| Layer | What | Effort | Priority |
|-------|------|--------|----------|
| **Network** | Rate limiting, TLS 1.3, HTTPS, Slowloris | 6-8h | P0 |
| **Application** | Validation, blacklist, path traversal | 4-6h | P1 |
| **Isolation** | PHP sandbox, disable functions, open_basedir | 2-3h | P2 |
| **Secrets** | DotEnv masking, secure injection | 1-2h | P2 |
| **Audit** | Security event logging, JSON output | 1-2h | P2 |

### 8 Tasks (TDD - Test → Fail → Implement → Pass → Commit)

```
P0-1: Plug RateLimiter into request handler (2h)
  ├─ File: crates/cleanserve-proxy/src/rate_limit.rs (create)
  ├─ File: crates/cleanserve-proxy/src/server.rs (modify)
  └─ Result: Rate limiting active, 429 responses

P0-2: Add request validation (2h)
  ├─ File: crates/cleanserve-core/src/request_validator.rs (create)
  ├─ Check: Content-Length, Content-Type, header size
  └─ Result: 413 on oversized, 400 on bad content

P0-3: Enforce TLS 1.3 (1.5h)
  ├─ File: crates/cleanserve-proxy/src/tls_config.rs (create)
  ├─ Action: Force TLS 1.3, reject older versions
  └─ Result: HTTPS only with modern ciphers

P1-1: Static file blacklist (1.5h)
  ├─ File: crates/cleanserve-core/src/static_blacklist.rs (create)
  ├─ Block: .env, .git, .php in uploads
  └─ Result: 403 for blacklisted files

P1-2: Path traversal protection (1.5h)
  ├─ File: crates/cleanserve-core/src/path_normalizer.rs (create)
  ├─ Block: /../, %2e%2e, path escaping
  └─ Result: 400 for traversal attempts

P1-3: Slowloris protection (1.5h)
  ├─ File: crates/cleanserve-proxy/src/slowloris_protection.rs (create)
  ├─ Timeout: 60s idle → close connection
  └─ Result: DoS resistant

P2-1: PHP security lockdown (2-3h)
  ├─ File: crates/cleanserve-core/src/php_security_config.rs (create)
  ├─ Generate: Hardened php.ini with disable_functions
  ├─ Enforce: open_basedir, session cookies, restrictive settings
  └─ Result: PHP sandbox active

P2-2: Audit logging (1.5h)
  ├─ File: crates/cleanserve-core/src/audit_logger.rs (create)
  ├─ Log: Rate limits, path traversal, blacklist hits
  ├─ Format: JSON for SIEM integration
  └─ Result: [SECURITY] {...} logs

TOTAL: 16-22 hours
```

### Files to Create
```
crates/cleanserve-core/src/
  ├─ request_validator.rs
  ├─ static_blacklist.rs
  ├─ path_normalizer.rs
  ├─ php_security_config.rs
  └─ audit_logger.rs

crates/cleanserve-proxy/src/
  ├─ rate_limit.rs
  ├─ tls_config.rs
  └─ slowloris_protection.rs
```

### Files to Modify
```
crates/cleanserve-proxy/src/server.rs
  - Add rate limit check before request handling
  - Add request validation
  - Add blacklist check for static files
  - Add path normalization
  - Add audit logging

crates/cleanserve-core/src/lib.rs
  - Export all new modules

Cargo.toml
  - Add: urlencoding = "2"
```

### Quick Test Commands

```bash
# Run after each task
cargo test --lib
cargo test --test '*'
cargo build --release

# Functional verification
for i in {1..150}; do curl http://localhost:8080; done    # Expect 429 after 100
curl http://localhost:8080/../../etc/passwd               # Expect 400
curl http://localhost:8080/.env                           # Expect 403
curl -H "Content-Length: 999999999" http://localhost:8080 # Expect 413
```

### Configuration (cleanserve.json)

```json
{
  "security": {
    "mode": "production",
    "rate_limiting": {
      "enabled": true,
      "requests_per_second": 100,
      "window_seconds": 60
    },
    "request_validation": {
      "max_content_length": 10485760,
      "max_header_size": 51200
    },
    "tls": {
      "min_version": "1.3",
      "enforce_https": true
    },
    "slowloris_protection": {
      "enabled": true,
      "idle_timeout_secs": 60
    }
  }
}
```

### Success = 10 Checkboxes

- [ ] PHP forwarding works (FastCGI → php_worker)
- [ ] Rate limit returns 429
- [ ] Path traversal returns 400
- [ ] .env returns 403
- [ ] Oversized request returns 413
- [ ] Security events logged as JSON
- [ ] TLS 1.3 enforced
- [ ] All tests pass
- [ ] No performance regression
- [ ] Configuration via cleanserve.json

---

## 🚀 START HERE

1. Read: `.sisyphus/plans/security-hardening-production.md` (complete implementation plan)
2. Choose execution method (subagent-driven, parallel session, or manual)
3. Execute P0 tasks first (rate limiting, validation, TLS)
4. Run tests after each phase
5. Commit frequently

---

**Documents:**
- `.sisyphus/README.md` — Execution guide
- `.sisyphus/SECURITY_ANALYSIS.md` — Complete architecture analysis
- `.sisyphus/plans/security-hardening-production.md` — Full implementation plan

**Status:** Ready for implementation ✅
