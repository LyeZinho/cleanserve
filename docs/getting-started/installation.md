# Installation

CleanServe is a zero-config PHP development server written in Rust. It simplifies PHP development by managing PHP versions, extensions, and certificates automatically.

## System Requirements

CleanServe runs on Linux, macOS, and Windows. Depending on your installation method, you need the following tools:

- **Quick Install:** `curl` and `unzip`.
- **Building from Source:** Rust toolchain (Cargo 1.70+).
- **PHP Storage:** PHP binaries are stored in `~/.cleanserve/bin/php-{version}/`.
- **Certificates:** Local TLS certificates are stored in `~/.cleanserve/certs/`.

## Quick Install

The easiest way to install CleanServe on Linux and macOS is via the shell script:

```bash
curl -fsSL https://raw.githubusercontent.com/LyeZinho/cleanserve/main/install.sh | sh
```

The script downloads the pre-built binary for your architecture and adds it to your system path.

## Building from Source

If you prefer to build CleanServe manually, you can clone the repository and use Cargo:

```bash
git clone https://github.com/LyeZinho/cleanserve
cd cleanserve
cargo build --release
```

Once complete, the binary will be available at `./target/release/cleanserve`. You can move it to a directory in your `$PATH`.

## Docker Installation

CleanServe can be used within Docker environments. You can reference the provided `Dockerfile` and `docker-compose.yml` in the repository root for containerized development.

```bash
docker-compose up -d
```

> **Note:** When running in Docker, ensure the document root and PHP configuration in `cleanserve.json` match your container paths.

## Verifying Installation

To confirm CleanServe is installed correctly, run the version command:

```bash
cleanserve --version
```

The output should show version `0.1.0`.

## Next Steps

Once installed, proceed to the [Quick Start Guide](./quick-start.md) to initialize your first project.
