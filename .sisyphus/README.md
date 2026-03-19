# CleanServe Production Security Hardening

## 📋 Documents

- **[SECURITY_ANALYSIS.md](./.SECURITY_ANALYSIS.md)** — Complete gap analysis with architecture overview
- **[plans/security-hardening-production.md](./plans/security-hardening-production.md)** — Detailed implementation plan with bite-sized tasks

## 🎯 Quick Summary

| What | Status | Priority | Effort |
|------|--------|----------|--------|
| **PHP Forwarding (BLOCKING)** | 🔴 Missing | P0 CRITICAL | 1-2h |
| **Rate Limiting Integration** | 🟡 Partial | P0 CRITICAL | 2h |
| **Request Validation** | 🔴 Missing | P0 CRITICAL | 2h |
| **TLS 1.3 Enforcement** | 🟡 Partial | P0 CRITICAL | 1.5h |
| **Static File Blacklist** | 🔴 Missing | P1 HIGH | 1.5h |
| **Path Traversal Protection** | 🔴 Missing | P1 HIGH | 1.5h |
| **Slowloris Protection** | 🔴 Missing | P1 HIGH | 1.5h |
| **PHP Security Lockdown** | 🔴 Missing | P2 MEDIUM | 2h |
| **Audit Logging** | 🟡 Partial | P2 MEDIUM | 1.5h |
| **ACME Integration** | 🔴 Missing | P3 NICE | 3-4h |

**Total Effort:** 16-22 hours (4-6 weeks part-time, 1-2 weeks full-time)

## 🚀 Recommended Execution Path

### Phase 0: Foundation (Do These First)
1. ✅ **Task P0-1**: Integrate Rate Limiter into HTTP pipeline
2. ✅ **Task P0-2**: Add Request Size Validation (Content-Length)
3. ✅ **Task P0-3**: Enforce TLS 1.3 and HTTPS redirect

**Result:** Network-level security active; rate limiting + size limits working

### Phase 1: Application Layer
4. ✅ **Task P1-1**: Static File Blacklist (.env, .git, .php in uploads)
5. ✅ **Task P1-2**: Path Traversal Protection with URL normalization
6. ✅ **Task P1-3**: Slowloris DoS protection (idle timeout)

**Result:** WAF-lite features active; OWASP Top 10 mitigation started

### Phase 2: Isolation & Auditoria
7. ✅ **Task P2-1**: PHP Security Lockdown (auto-generate php.ini)
8. ✅ **Task P2-2**: Structured Audit Logging (security events)

**Result:** Defense-in-depth active; forensics/compliance logging working

### Phase 3: Enterprise Features (Optional)
9. ⚠️ **Task P3-1**: ACME/Let's Encrypt Integration

**Result:** Zero-touch HTTPS deployment ready

## 💻 Quick Start

### Run Full Plan (Recommended)

```bash
# Option 1: Subagent-driven (this session)
# Have Claude execute tasks task-by-task with code review

# Option 2: Executing-plans (separate session)
# Create new session, pass plan as prompt
claude-opencode@cleanserve> execute .sisyphus/plans/security-hardening-production.md

# Option 3: Manual reference
# Read plan, implement independently
```

### Verify Implementation

```bash
# After each phase
cargo test --lib
cargo test --test '*'
cargo build --release

# Test rate limiting
for i in {1..150}; do curl http://localhost:8080; done
# Should see 429 after 100 requests

# Test path traversal blocking
curl http://localhost:8080/../../etc/passwd
# Should get 400

# Test .env blocking
curl http://localhost:8080/.env
# Should get 403

# Test TLS 1.3
openssl s_client -connect localhost:443 -tls1_3
# Should connect successfully
```

## 📁 Key Files to Understand

**Existing Security Foundation:**
- `crates/cleanserve-core/src/security.rs` — SecurityHeaders + RateLimiter
- `crates/cleanserve-core/src/ssl.rs` — SSL cert generation
- `crates/cleanserve-core/src/structured_logging.rs` — JSON logging

**Request Pipeline:**
- `crates/cleanserve-proxy/src/server.rs` — Main HTTP handler (WHERE TO INTEGRATE)
- `crates/cleanserve-core/src/fastcgi/mod.rs` — PHP-FPM forwarding (unused!)
- `crates/cleanserve-core/src/static_server.rs` — Static file serving

**To Create:**
- `crates/cleanserve-core/src/request_validator.rs`
- `crates/cleanserve-core/src/static_blacklist.rs`
- `crates/cleanserve-core/src/path_normalizer.rs`
- `crates/cleanserve-core/src/php_security_config.rs`
- `crates/cleanserve-core/src/audit_logger.rs`
- `crates/cleanserve-proxy/src/slowloris_protection.rs`
- `crates/cleanserve-proxy/src/tls_config.rs`

## 🔧 Configuration (cleanserve.json)

After implementation, configure via:

```json
{
  "security": {
    "mode": "production",
    "rate_limiting": {
      "enabled": true,
      "requests_per_second": 100
    },
    "tls": {
      "min_version": "1.3",
      "enforce_https": true
    }
  }
}
```

## 📊 Success Criteria

✅ All items checked = Production ready:

- [ ] PHP requests forward to PHP-FPM (FastCGI working)
- [ ] Rate limiter returns 429 when exceeded
- [ ] Path traversal returns 400
- [ ] .env returns 403
- [ ] Content-Length validation returns 413
- [ ] Security events logged as JSON
- [ ] TLS 1.3 enforced
- [ ] All tests pass
- [ ] No performance regression (each check <1ms)
- [ ] Configuration via cleanserve.json working

## 📝 Notes

- **TDD Approach**: Each task includes "write test → run test (fail) → implement → run test (pass) → commit"
- **Frequent Commits**: One commit per task, clean git history
- **DRY/YAGNI**: No premature optimization, implement what's specified
- **Performance**: Each security check designed to add <1ms latency
- **Documentation**: Plan includes exact code snippets (not pseudocode)

## 🤝 Execution Options

**This session (Subagent-Driven):**
```
I dispatch fresh subagent per task
→ You review code changes
→ Move to next task on approval
→ Full context maintained via session_id
```

**Separate session (Executing-Plans):**
```
1. Create new OpenCode session
2. Invoke: execute .sisyphus/plans/security-hardening-production.md
3. Batch execution with checkpoints
4. Parallel with other work
```

**Manual (Self-Driven):**
```
1. Read plan document completely
2. Implement tasks in order
3. Use exact code snippets provided
4. Run tests after each task
5. Commit frequently
```

---

**Start:** [Read the complete plan](./plans/security-hardening-production.md)  
**Questions?** See [SECURITY_ANALYSIS.md](./.SECURITY_ANALYSIS.md) for detailed architecture  
**Status:** Ready for implementation ✅
