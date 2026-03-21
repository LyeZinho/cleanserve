# Design Doc: CleanServe Documentation Update (Package Manager & Auto-Updater)

**Date:** 2026-03-21
**Status:** Validated
**Context:** Creating 3 new/updated documentation files for the CleanServe ecosystem.

## Goals
- Provide comprehensive guide for the new Package Manager (`cleanserve package`).
- Document the Auto-Updater secure lifecycle (`cleanserve update`).
- Update the CLI command reference to include all new subcommands.
- Maintain a "Zero-Burden" tone while providing technical depth for power users.

## 1. File: `docs/guide/package-manager.md`
**Structure:**
- **Overview:** Introduction to integrated service management.
- **Quick Start:**
    - `list`: Show available packages.
    - `info <pkg>`: Detailed metadata.
    - `add <pkg>`: Dependency injection to project.
    - `start/stop/status`: Runtime control.
- **Available Packages:** Detailed table (MySQL 8.0, Redis 7.0, phpMyAdmin 5.2).
- **Examples:**
    - "The Full Stack": Adding PHP, MySQL, and Redis concurrently.
    - "Verifying Integrity": Checksum and isolation details.
- **Troubleshooting:**
    - Checksum failures.
    - Permission issues in `~/.cleanserve/tools`.

## 2. File: `docs/guide/auto-updater.md`
**Structure:**
- **Overview:** Secure self-update mechanism.
- **Command Reference:**
    - `--check`: Version discovery.
    - `--force`: Re-installation/Repair.
    - `update <version>`: PHP version management.
- **Security Lifecycle (5-Step):**
    1. Check (API)
    2. Download (Stream)
    3. Verify (SHA256)
    4. Backup (Timestamped)
    5. Install (Atomic)
- **Rollback:** Restoring from `~/.cleanserve/backups/`.
- **Automated Updates:** `CLEANSERVE_AUTO_UPDATE` env var in installation script.
- **Troubleshooting:** Stuck downloads, verification errors.

## 3. File: `docs/guide/cli-commands.md` (Update)
**Structure:**
- Preserve existing `init`, `up`, `down`, `use`, `list`, `composer` sections.
- **New Section: `package`**: Document all subcommands with syntax and examples.
- **Revised Section: `update`**: Detail the dual role (binary update vs. PHP version download).

## Success Criteria
- Files meet ~200 line requirement where applicable.
- Real command output examples included.
- Warning blocks for security-critical operations (e.g., manual binary replacement).
- All links and references are valid.
