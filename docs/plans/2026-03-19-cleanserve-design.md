# CleanServe Design Document

**Date:** 2026-03-19  
**Status:** Approved

## Overview

CleanServe is a micro-server and runtime manager for PHP written in Rust. It eliminates the need for Apache, Nginx, or FPM configuration in development environments, bringing a "Zero Config" experience to PHP.

## Architecture

### Workspace Crate Structure

```
cleanserve/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── cleanserve-core/    # Core logic, config, PHP manager
│   ├── cleanserve-proxy/    # Hyper HTTP proxy, WebSocket
│   ├── cleanserve-watcher/ # File watching with notify
│   ├── cleanserve-cli/     # Clap CLI entrypoint
│   └── cleanserve-shared/  # Shared types/utilities
├── docs/
│   └── plans/              # Implementation plans
└── README.md
```

### Crate Responsibilities

| Crate | Responsibility |
|-------|----------------|
| `cleanserve-core` | Config parsing (cleanserve.json), PHP version manager, php.ini generation |
| `cleanserve-proxy` | Hyper-based reverse proxy, request routing, WebSocket server |
| `cleanserve-watcher` | File system monitoring with debouncing |
| `cleanserve-cli` | Clap CLI commands (init, up, use, composer, etc.) |
| `cleanserve-shared` | Shared types, error types, logging utilities |

## Key Technical Decisions

### 1. HTTP Framework: Hyper (Low-Level)
- **Rationale:** Minimal overhead for <5ms latency target
- **Trade-off:** Less ergonomic than Axum/Actix, but faster
- **Usage:** Direct `hyper::Server` with custom service

### 2. PHP Execution: PHP-FPM Style (Persistent Worker)
- PHP process stays alive, handles multiple requests
- Signal-based restart on file changes (SIGTERM/SIGKILL)
- FastCGI-like communication via stdin/stdout pipes
- **Why:** Faster than CGI (no process spawn per request)

### 3. SSL: Self-Signed + Trust-on-First-Use
- Generate self-signed certificates on first run
- Store fingerprint for trust-on-first-use pattern
- User accepts browser warning once per session
- Certificate stored in `~/.cleanserve/certs/`

### 4. File Watching: notify + Async Debouncing
- Use `notify` crate for cross-platform file watching
- Custom debouncer using `tokio::time::interval`
- Separate watch tasks for:
  - `.php` files → restart worker
  - `.css/.js` files → trigger WebSocket injection
- **Why:** notify handles OS-specific quirks, debounce prevents event storms

### 5. PHP Downloads: php.net + CDN Mirrors
- Source: `https://windows.php.net/downloads/releases/php-[version]-Win32-vs[compiler]-x64.zip`
- Mirror fallback list for reliability
- Store in `~/.cleanserve/bin/php-[version]/`
- Extract and manage multiple versions simultaneously

## Data Flow

```
Browser ──HTTPS──> CleanServe Proxy (Hyper)
                              │
                              ├── Static files ──────────────────────────┐
                              │                                          │
                              └── PHP Worker (FastCGI-like)              │
                                         │                               │
                                         └── PHP CGI ──> index.php        │
                                                                          │
                                          <──── WebSocket (CSS injection)
```

### Request Flow
1. Browser sends HTTPS request to proxy
2. Proxy inspects path:
   - Static file → serve directly
   - PHP file → forward to PHP worker
3. PHP worker processes request via CGI pipes
4. Response returned through proxy
5. If HTML response → inject hot reload script

## cleanserve.json Schema

```json
{
  "name": "string",
  "engine": {
    "php": "string (semver)",
    "extensions": ["string"],
    "display_errors": "boolean",
    "memory_limit": "string (optional, e.g. '128M')"
  },
  "server": {
    "root": "string (default: 'public/')",
    "port": "number (default: 8080)",
    "hot_reload": "boolean (default: true)",
    "ssl": {
      "enabled": "boolean (default: true)",
      "auto_generate": "boolean (default: true)"
    }
  }
}
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `cleanserve init` | Initialize project (create cleanserve.json) |
| `cleanserve up` | Start development server |
| `cleanserve down` | Stop development server |
| `cleanserve use <version>` | Switch PHP version |
| `cleanserve composer <args>` | Run Composer with project's PHP |
| `cleanserve php <args>` | Run PHP CLI with project's version |
| `cleanserve list` | List available PHP versions |
| `cleanserve update` | Download latest PHP versions |

## Implementation Phases

### Phase 1: Foundation
- [ ] Initialize Rust workspace
- [ ] Implement `cleanserve-shared` (types, errors)
- [ ] Implement `cleanserve-core`:
  - Config parsing (serde_json)
  - PHP version detection
  - PHP download manager
  - php.ini generator
- [ ] Implement `cleanserve-cli`:
  - Clap setup with subcommands
  - `init` command
  - `use` command
  - `list` command

### Phase 2: Server Core
- [ ] Implement `cleanserve-proxy`:
  - Hyper server setup
  - Request routing
  - PHP CGI handler
  - Static file serving
- [ ] Implement PHP worker lifecycle
- [ ] SSL certificate generation

### Phase 3: Hot Reload
- [ ] Implement `cleanserve-watcher`:
  - File watching with notify
  - Event debouncing
  - Separate handlers for PHP/CSS/JS
- [ ] WebSocket server in proxy
- [ ] Client-side HMR script injection
- [ ] CSS injection without page refresh

### Phase 4: Ecosystem
- [ ] Composer integration
- [ ] Extension manager
- [ ] Dashboard/logging improvements
- [ ] Installation scripts

## Error Handling

### Error Types (shared)
```rust
pub enum CleanServeError {
    ConfigParseError(String),
    PhpNotFound(String),
    DownloadError(String),
    ProxyError(String),
    WatcherError(String),
}
```

### CLI Error Output
- Human-readable messages with context
- Exit codes: 1 (general), 2 (config), 3 (network), 4 (PHP)

## Testing Strategy

1. **Unit Tests:** Each crate tests its own logic
2. **Integration Tests:** CLI commands with mock scenarios
3. **Hot Reload Tests:** File watcher behavior verification
4. **Cross-Platform:** CI/CD on Linux, macOS, Windows

## Performance Targets

| Metric | Target |
|--------|--------|
| Proxy latency overhead | <5ms |
| Hot reload detection | <100ms |
| PHP worker restart | <500ms |
| Cold start (no PHP) | Network-dependent |

## Distribution

### Binary
- Static binary via `cargo build --release`
- Target: x86_64-unknown-linux-musl, x86_64-apple-darwin, x86_64-pc-windows-msvc

### Installation Script
```bash
curl -fsSL https://get.cleanserve.dev | sh
```

Detects OS, downloads binary, adds to PATH, optionally downloads initial PHP.
