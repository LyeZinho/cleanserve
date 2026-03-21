# Design Doc: CleanServe API Reference Documentation

**Date**: 2026-03-21
**Topic**: API Reference for Package Manager and Auto-updater

## 1. Overview
The goal is to create two comprehensive API reference files for the CleanServe ecosystem. These files will target a developer audience, focusing on the Rust API, core data structures, and operational lifecycles.

## 2. Requirements
- **File 1**: `docs/api/package-manager-api.md` (~250 lines)
- **File 2**: `docs/api/auto-updater-api.md` (~250 lines)
- **Format**: Markdown with high-fidelity Rust code blocks.
- **Content**: Full type signatures, module descriptions, usage examples, error handling, and best practices.

## 3. Architecture & Components

### 3.1 Package Manager API (`package-manager-api.md`)
- **Core Types**:
    - `Package`: Root container for tool metadata.
    - `PackageVersion`: Version-specific configuration (ports, env, dependencies).
    - `DownloadInfo`: Platform-specific artifact pointers and checksums.
    - `PackageRuntime`: Active process state.
    - `RuntimeStatus`: Enum for lifecycle states (Stopped, Starting, Running, etc.).
- **Modules**:
    - `PackageRegistry`: Manifest loading (builtin + custom).
    - `PackageDownloader`: Secure artifact retrieval.
    - `PackageCache`: Filesystem layout at `~/.cleanserve/tools`.
    - `PackageLifecycle`: Runtime management and health checks.

### 3.2 Auto-updater API (`auto-updater-api.md`)
- **Core Types**:
    - `UpdateInfo`: Remote vs Local version comparison results.
    - `UpdateCheckerError`: Error scenarios for the update process.
- **Modules**:
    - `UpdateChecker`: SemVer comparison and GitHub API integration.
    - `BinaryDownloader`: Streaming binary downloads.
    - `UpdateInstaller`: Atomic installation pattern (Backup -> Swap -> Verify -> Cleanup).
- **Platform Support**: Explicit target triples for Linux, macOS, and Windows.

## 4. Error Handling Strategy
- `PackageManagerError`: Detailed messages for I/O, network, and checksum failures.
- `UpdateCheckerError`: Enum-based categorization for recovery and rollback.

## 5. Usage Examples
- **Package Manager**: Loading Redis, downloading, and starting a runtime.
- **Auto-updater**: Checking for updates and performing a safe atomic install.

## 6. Verification
- Validate all Rust types against the current `cleanserve-core` implementation.
- Ensure all module names and method signatures are accurate to the source code.
