# Architecture Overview

CleanServe is a zero-config PHP development server written in Rust, designed for performance, security, and a modern developer experience. It utilizes a multi-crate workspace architecture to separate concerns between the core logic, proxy server, CLI, and specialized utilities.

## Workspace Structure

The project is organized into 9 specialized Rust crates:

*   `cleanserve-shared`: Contains common types and error handling logic. The central `CleanServeError` enum defines variants for `Config`, `Download`, `Io`, `PhpWorker`, and `Server`.
*   `cleanserve-core`: The heart of the application. It handles business logic, configuration management, PHP binary lifecycle, framework detection, FastCGI communication, the error overlay, and the dynamic worker pool.
*   `cleanserve-cli`: The command-line interface built with `clap`. It provides subcommands for `init`, `up`, `down`, `use`, `list`, `update`, and `composer`.
*   `cleanserve-proxy`: A high-performance HTTP reverse proxy powered by `hyper`. It includes an HMR (Hot Module Replacement) WebSocket server using `tokio-tungstenite`.
*   `cleanserve-watcher`: A file system watcher using `notify` and a debouncer (100ms) to trigger HMR events.
*   `cleanserve-preload`: Generates PHP OpCache preload scripts by parsing `composer.json` PSR-4 autoload definitions.
*   `cleanserve-inliner`: Converts `.env` files into PHP constants for production readiness.
*   `cleanserve-vfs`: A Virtual File System supporting memory backends, symlink caching, and ZIP archive serving.
*   `cleanserve-bundle`: A PHAR bundler used to create standalone, portable PHP applications.

## Request Flow

When a request enters CleanServe, it passes through several layers before reaching a PHP worker:

```text
[ Client ]
    |
    v
[ Proxy Server (hyper) ]
    |
    |-- Security Pipeline --|
    |   1. Slowloris Check  |
    |   2. Rate Limiter     |
    |   3. Validator        |
    |   4. Path Traversal   |
    |   5. Static Blacklist |
    |-----------------------|
    |
    v
[ Routing Logic ]
    |
    |-- Static File Serving (if file exists)
    |
    v
[ PHP Worker Pool (FastCGI) ]
```

1.  **Proxy Server**: Receives the raw TCP connection and upgrades it to HTTP/1.1 or HTTP/2.
2.  **Security Pipeline**: Validates the request against common attacks and limits.
3.  **Routing Logic**: Determines if the request matches a physical file on disk or should be handled by PHP.
4.  **Worker Pool**: Assigns the request to an available PHP worker via the FastCGI protocol.

## Key Dependencies

CleanServe leverages the following libraries:

*   `tokio`: Asynchronous runtime for all I/O operations.
*   `hyper`: Low-level HTTP implementation.
*   `clap`: Command-line argument parsing.
*   `serde`: Serialization and deserialization of configuration files.
*   `reqwest`: HTTP client for downloading PHP binaries.
*   `notify`: Cross-platform file system notifications.
*   `rcgen`: Generation of self-signed certificates for local TLS.

## Further Reading

*   [Proxy Server Details](proxy-server.md)
*   [Worker Pool Management](worker-pool.md)
*   [FastCGI Protocol Implementation](fastcgi.md)
*   [Virtual File System](vfs.md)
*   [Bundling and Deployment](bundle.md)
