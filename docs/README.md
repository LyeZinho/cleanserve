# CleanServe Documentation

Welcome to the CleanServe documentation. This guide covers everything from installation and basic usage to the internal architecture and security model.

## Getting Started

New to CleanServe? Start here.

- [Installation](getting-started/installation.md) - System requirements and installation methods
- [Quick Start](getting-started/quick-start.md) - Initialize a project and start the server
- [Configuration](getting-started/configuration.md) - The `cleanserve.json` file reference

## User Guide

Day-to-day usage and features.

- [CLI Commands](guide/cli-commands.md) - Full command reference for the `cleanserve` binary
- [PHP Management](guide/php-management.md) - Automatic PHP downloads, version switching, and isolation
- [Hot Reload](guide/hot-reload.md) - File watching, HMR WebSocket, and CSS injection
- [Framework Support](guide/framework-support.md) - Auto-detection for Laravel, Symfony, WordPress, and more
- [Docker](guide/docker.md) - Production and development Docker setups

## Architecture

How CleanServe works under the hood.

- [Overview](architecture/overview.md) - Workspace structure and request flow
- [Proxy Server](architecture/proxy-server.md) - The Hyper-based reverse proxy and HMR injection
- [Worker Pool](architecture/worker-pool.md) - Dynamic PHP process management and scaling
- [FastCGI](architecture/fastcgi.md) - Native FastCGI protocol implementation
- [Virtual File System](architecture/vfs.md) - Pluggable VFS with memory and zip backends
- [Bundling](architecture/bundle.md) - Standalone PHP application packaging

## Security

Defense layers and threat mitigation.

- [Overview](security/overview.md) - Security architecture and request pipeline
- [Rate Limiting](security/rate-limiting.md) - Sliding window algorithm and IP tracking
- [Path Traversal](security/path-traversal.md) - Path normalization and escape protection
- [Static Blacklist](security/static-blacklist.md) - Sensitive file blocking
- [Slowloris Protection](security/slowloris-protection.md) - Connection tracking and header timeouts
- [Request Validation](security/request-validation.md) - Payload limits and header enforcement
- [SSL/TLS](security/ssl-tls.md) - Automatic certificate generation and TLS 1.3

## API Reference

- [Configuration Reference](api/configuration-reference.md) - All configuration fields, defaults, and environment variables

## Contributing

- [Development Setup](contributing/development-setup.md) - From clone to running server
- [Architecture Guide](contributing/architecture-guide.md) - Crate map, conventions, and where to make changes
