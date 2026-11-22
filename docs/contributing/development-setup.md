# Development Setup

This guide provides the steps to set up a local development environment for CleanServe.

## Prerequisites

- Rust (edition 2021)
- Git
- Docker (optional)

## Installation

Clone the repository **with submodules** (includes `php-runtimes/`):

```bash
git clone --recurse-submodules https://github.com/LyeZinho/cleanserve.git
cd cleanserve
```

If you already cloned without submodules:

```bash
git submodule update --init --recursive
```

See [php-runtimes submodule](./php-runtimes-submodule.md) for how to develop both repos together.

## Building

Compile the project in debug mode for local development:

```bash
cargo build
```

For production-ready binaries, use the release flag:

```bash
cargo build --release
```

## Running the Server

Initialize the configuration:

```bash
cargo run -- init
```

Start the development server:

```bash
cargo run -- up
```

### Debug Logging

Enable detailed logs by setting the `RUST_LOG` environment variable:

```bash
RUST_LOG=cleanserve=debug cargo run -- up
```

## Testing

Run the test suite across all crates in the workspace:

```bash
cargo test
```

## Docker Development

You can run the development environment using Docker Compose:

```bash
docker-compose up
```

This uses the `Dockerfile.dev` configuration.

## Workspace Overview

The project is organized into 9 crates located in the `crates/` directory:

- `cleanserve-shared`
- `cleanserve-core`
- `cleanserve-proxy`
- `cleanserve-cli`
- `cleanserve-watcher`
- `cleanserve-vfs`
- `cleanserve-bundle`
- `cleanserve-inliner`
- `cleanserve-preload`
