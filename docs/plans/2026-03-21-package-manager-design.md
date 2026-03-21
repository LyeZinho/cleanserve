# Package Manager + Auto-Updater Design

**Date**: 2026-03-21  
**Status**: Approved  
**Priority**: High

---

## Executive Summary

CleanServe will gain two major capabilities:

1. **Package Manager** - Download and manage standalone tools (MySQL, phpMyAdmin, Redis, etc) per-project with optional proxy integration
2. **Auto-Updater** - Self-updating binary that always installs the latest release

These features streamline development environments by centralizing tool management and keeping CleanServe current.

---

## 1. Architecture

### 1.1 Package Manager Architecture

```
User Request (cleanserve package add mysql 8.0)
        │
        ▼
┌────────────────────────────────┐
│ Package Manager (new CLI cmd)  │
│ - parse args                   │
│ - validate package exists      │
└────────────┬───────────────────┘
             │
             ▼
┌────────────────────────────────┐
│ PackageRegistry                │
│ - load manifest (built-in)     │
│ - load manifest (custom)       │
│ - merge & deduplicate          │
└────────────┬───────────────────┘
             │
             ▼
┌────────────────────────────────┐
│ PackageDownloader              │
│ - check cache ~/.cleanserve/   │
│ - validate checksum (SHA256)   │
│ - download if missing          │
│ - extract/install              │
└────────────┬───────────────────┘
             │
             ▼
┌────────────────────────────────┐
│ ProjectPackageManager          │
│ - update cleanserve.json       │
│ - create symlinks              │
│ - log operation                │
└────────────────────────────────┘
```

### 1.2 Auto-Updater Architecture

```
User: cleanserve update  OR  curl install.sh | sh
        │
        ▼
┌────────────────────────────────┐
│ UpdateChecker                  │
│ - get current version          │
│ - fetch latest from GitHub API │
│ - compare semver               │
└────────────┬───────────────────┘
             │ (if newer available)
             ▼
┌────────────────────────────────┐
│ BinaryDownloader               │
│ - download latest release      │
│ - validate checksum (SHA256)   │
│ - temp extraction              │
└────────────┬───────────────────┘
             │
             ▼
┌────────────────────────────────┐
│ Installation                   │
│ - backup current binary        │
│ - install new binary           │
│ - verify (--version)           │
│ - cleanup on failure           │
└────────────────────────────────┘
```

---

## 2. Package Manager Specification

### 2.1 CLI Commands

```bash
# Add/remove packages
cleanserve package add <name> [version]      # Adds to cleanserve.json, downloads
cleanserve package remove <name>             # Removes from cleanserve.json, keeps cache
cleanserve package uninstall <name> --purge  # Removes and deletes cache

# List and info
cleanserve package list                      # All available packages from manifest
cleanserve package installed                 # Installed in current project
cleanserve package info <name>               # Details about package

# Lifecycle
cleanserve package start <name>              # Start package manually
cleanserve package stop <name>               # Stop package manually
cleanserve package status                    # Status of all packages
```

### 2.2 cleanserve.json Integration

**Before:**
```json
{
  "name": "my-project",
  "engine": { "php": "8.4" },
  "server": { "root": "public/", "port": 8080 }
}
```

**After:**
```json
{
  "name": "my-project",
  "engine": { "php": "8.4" },
  "server": { "root": "public/", "port": 8080 },
  "packages": {
    "mysql": "8.0",
    "phpmyadmin": {
      "version": "5.2",
      "path": "/admin",
      "enabled": true
    },
    "redis": {
      "version": "7.0",
      "enabled": false
    }
  }
}
```

### 2.3 File Structure

**Global cache** (~/.cleanserve/):
```
~/.cleanserve/
├── packages-manifest.json         # Built-in package definitions
├── packages-config.json           # User custom packages (optional)
├── tools/
│   ├── mysql/
│   │   ├── 8.0/
│   │   │   ├── bin/
│   │   │   ├── lib/
│   │   │   └── share/
│   │   └── 5.7/
│   ├── phpmyadmin/
│   │   └── 5.2/
│   └── redis/
│       └── 7.0/
├── logs/
│   ├── packages.log              # All package operations
│   └── mysql-8.0.log             # Per-package logs
└── backups/
    └── cleanserve.2026-03-21     # Updater backups
```

**Per-project** (.cleanserve/):
```
.cleanserve/
├── pages/
├── php/
├── tools/                        # Symlinks to ~/.cleanserve/tools/
│   ├── mysql -> ~/.cleanserve/tools/mysql/8.0
│   ├── phpmyadmin -> ~/.cleanserve/tools/phpmyadmin/5.2
│   └── redis -> ~/.cleanserve/tools/redis/7.0
└── logs/
    ├── packages-runtime.log      # Start/stop events
    └── mysql-runtime.log
```

### 2.4 Packages Manifest Format

**Built-in** (in codebase, updated per release):
```json
{
  "version": "1.0",
  "packages": {
    "mysql": {
      "name": "MySQL Community Server",
      "description": "Open-source relational database",
      "homepage": "https://www.mysql.com/",
      "versions": {
        "8.0": {
          "downloads": {
            "linux-x64": {
              "url": "https://dev.mysql.com/.../mysql-8.0.35-linux-x86_64-glibc2.17.tar.xz",
              "checksum": "sha256:abc123...",
              "format": "tar.xz"
            },
            "darwin-x64": { "url": "...", "checksum": "..." },
            "darwin-arm64": { "url": "...", "checksum": "..." }
          },
          "default_port": 3306,
          "executable": "bin/mysqld",
          "health_check": "bin/mysqladmin -u root ping",
          "requires": ["libc6"],
          "env_vars": {
            "MYSQL_UNIX_PORT": "{package_dir}/mysql.sock",
            "MYSQL_DATADIR": "{package_dir}/data"
          }
        }
      }
    },
    "phpmyadmin": {
      "name": "phpMyAdmin",
      "description": "Web interface for MySQL management",
      "homepage": "https://www.phpmyadmin.net/",
      "versions": {
        "5.2": {
          "downloads": {
            "all": {
              "url": "https://files.phpmyadmin.net/.../phpMyAdmin-5.2-all-languages.tar.gz",
              "checksum": "sha256:def456..."
            }
          },
          "executable": "index.php",
          "requires": ["php", "mysql"],
          "proxy_path": "/admin",
          "server_type": "http",
          "default_port": 8081
        }
      }
    },
    "redis": { ... },
    "postgresql": { ... }
  }
}
```

**Custom** (~/.cleanserve/packages-config.json):
```json
{
  "packages": {
    "my-tool": {
      "name": "My Custom Tool",
      "versions": {
        "1.0": {
          "downloads": {
            "linux-x64": {
              "url": "https://example.com/tool-1.0.tar.gz",
              "checksum": "sha256:xyz789..."
            }
          }
        }
      }
    }
  }
}
```

### 2.5 Security Considerations

**Checksum Validation:**
- All downloads must match SHA256 checksums from manifest
- Fail hard if checksum doesn't match (don't skip)
- Store checksums in manifest, not dynamic fetch

**Sandboxing:**
- Each package in separate directory
- No symlink traversal outside ~/.cleanserve/
- Validate all paths (no `../` escapes)

**Permission Handling:**
- Downloaded files get restrictive permissions (700 for dirs, 500 for executables)
- Validate executable bit only on known executables
- Never auto-chmod user files

**Manifest Integrity:**
- Verify manifest integrity with checksum
- Pin manifest version in code
- Allow override via environment for testing

**Logging:**
- All operations logged with timestamps
- Include checksum validation results
- Log download sources and sizes

---

## 3. Auto-Updater Specification

### 3.1 CLI Command

```bash
cleanserve update          # Check and update if available
cleanserve update --check  # Only check, don't update
cleanserve update --force  # Force update even if same version
```

### 3.2 Update Process

**Step 1: Check**
- Get current version from `cleanserve --version`
- Query GitHub API for latest release
- Compare semver
- If equal and no --force → exit with "Already up to date"

**Step 2: Download**
- Determine target platform (linux-x64, darwin-arm64, etc)
- Build download URL from GitHub API
- Download binary + checksum file
- Validate checksum

**Step 3: Backup**
- Find current binary location (in PATH)
- Copy to `~/.cleanserve/backups/cleanserve.{timestamp}`
- Verify backup integrity

**Step 4: Install**
- Write new binary to same location (with sudo if needed)
- Set permissions (755)
- Verify with `--version` and `--help`

**Step 5: Cleanup**
- Delete old checksums
- Keep last 3 backups, delete older
- Log operation

### 3.3 install.sh Changes

**Current:** Downloads latest and installs

**New:** Downloads latest, removes old if exists, installs

```bash
# NEW: Check if already installed
if command -v cleanserve >/dev/null 2>&1; then
  EXISTING_BIN=$(command -v cleanserve)
  info "Removing existing installation: $EXISTING_BIN"
  
  # Backup before removing
  BACKUP_DIR="$HOME/.cleanserve/backups"
  mkdir -p "$BACKUP_DIR"
  cp "$EXISTING_BIN" "$BACKUP_DIR/cleanserve.$(date +%s)"
  
  # Remove old binary
  rm "$EXISTING_BIN"
fi

# ... rest of existing logic
```

### 3.4 Safety Features

- **Rollback**: If new binary fails `--version` check, restore from backup
- **Atomic**: Move file atomically, don't partial-write
- **Permissions**: Verify binary is executable after install
- **Validation**: Always verify post-install before considering success

---

## 4. Implementation Plan

**Phase 1**: Core package manager (2-3 days)
- Create package registry data structure
- Implement PackageDownloader with checksum validation
- Add CLI commands (add, remove, list, info)
- Update cleanserve.json schema

**Phase 2**: Proxy integration + lifecycle (1-2 days)
- Implement package start/stop/status
- Integrate with proxy for HTTP packages
- Add package logs and monitoring

**Phase 3**: Auto-updater (1 day)
- Implement UpdateChecker
- Create update command
- Update install.sh
- Fallback and rollback logic

**Phase 4**: Testing + docs (1 day)
- Integration tests
- E2E test scenarios
- User documentation

---

## 5. Testing Strategy

**Unit Tests:**
- Manifest parsing (valid/invalid JSON)
- Checksum validation (match/mismatch)
- Version comparison (semver)
- Path validation (no traversal)

**Integration Tests:**
- Download → extract → verify flow
- cleanserve.json update
- Symlink creation/removal
- Package start/stop

**E2E Tests:**
- Fresh install with packages
- Update from old version
- Package removal and re-add
- Concurrent operations

**Security Tests:**
- Manifest tampering detection
- Checksum bypass attempts
- Path traversal attempts
- Symlink loop detection

---

## 6. Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| Network failure mid-download | Resume downloads, validate completeness |
| Corrupted binary download | Checksum validation before use |
| Permission denied on install | Fallback to ~/.local/bin |
| Symlink issues | Validate symlinks, clean broken ones |
| Manifest unavailable | Use cached manifest, fail gracefully |
| Package conflict | Namespace by version, isolate processes |

---

## 7. Success Criteria

- ✅ Package manager works end-to-end (add → download → symlink → use)
- ✅ Auto-updater maintains binary integrity
- ✅ All security validations in place (checksums, paths, permissions)
- ✅ 95%+ test coverage on critical paths
- ✅ Zero security regressions
- ✅ Documentation complete
- ✅ All tests pass before commit
