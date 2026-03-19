# Architecture Guide

CleanServe is built with a modular architecture split across 9 specialized crates.

## Workspace Structure

All crates are located under the `crates/` directory.

### Dependency Graph

- `cleanserve-shared`: The base crate containing shared error types and utilities.
- `cleanserve-core`: Manages configuration, PHP execution, security modules, FastCGI integration, and worker pools. Depends on `cleanserve-shared`.
- `cleanserve-proxy`: The core HTTP server and HMR (Hot Module Replacement) logic. Depends on `cleanserve-core`.
- `cleanserve-cli`: The command line interface and entry point. Depends on `cleanserve-core`, `cleanserve-proxy`, and `cleanserve-watcher`.
- `cleanserve-watcher`: File system monitoring and event debouncing. Independent.
- `cleanserve-vfs`: Virtual filesystem supporting memory and zip backends. Independent.
- `cleanserve-bundle`: Logic for bundling standalone applications. Depends on `cleanserve-vfs`.
- `cleanserve-inliner`: CSS and JS resource inlining. Independent.
- `cleanserve-preload`: PHP preloading functionality. Independent.

## Feature-to-File Mapping

Locate the relevant files for specific features:

- New CLI commands: `crates/cleanserve-cli/src/commands/`
- Framework detection: `crates/cleanserve-core/src/framework.rs`
- Security modules: `crates/cleanserve-core/src/` (security.rs, path_traversal.rs, etc.)
- Proxy and HMR: `crates/cleanserve-proxy/src/server.rs`
- File watching: `crates/cleanserve-watcher/src/watcher.rs`
- Configuration: `crates/cleanserve-core/src/config.rs`

## Coding Conventions

### Error Handling

- Use the `thiserror` crate for library-level error definitions.
- Use `anyhow` for high-level errors within the CLI.

### Core Libraries

- Async Runtime: `tokio`
- HTTP Implementation: `hyper`
- Logging: `tracing`

### Platform Support

Isolate platform-specific code using configuration attributes:

- `#[cfg(windows)]` for Windows-specific logic.
- `#[cfg(unix)]` for Unix-like systems.

### Testing

Write unit tests in the same file as the implementation within a `#[cfg(test)]` module.

## Contribution Workflow

1. Fork the repository.
2. Create a dedicated feature branch for your changes.
3. Ensure all tests pass by running `cargo test`.
4. Submit a Pull Request with a clear description of the modifications.
