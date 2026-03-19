# Bundling and Deployment

CleanServe provides a comprehensive bundling system to create standalone, portable PHP applications. This system integrates several specialized crates to package code, configuration, and even the PHP binary into a single, redistributable package.

## Components of the Bundling System

The bundling process leverages three key components:

### PHAR Bundler (`cleanserve-bundle`)

The `cleanserve-bundle` crate is responsible for orchestrating the final build. It includes:

*   **Builder**: Manages the compilation and assembly of the bundle.
*   **Config**: Handles bundle-specific configuration options.
*   **PHAR Modules**: Specialized logic for creating and optimizing PHP Archive (PHAR) files.

### Inliner (`cleanserve-inliner`)

The `cleanserve-inliner` utility handles environmental configuration by converting `.env` files into native PHP constants. This ensures that application settings are embedded directly into the code, eliminating the need for external configuration files at runtime.

### Preloader (`cleanserve-preload`)

To maximize performance, the `cleanserve-preload` crate generates an OpCache preload script. It parses the `composer.json` PSR-4 autoload definitions to identify frequently used classes and functions, allowing PHP to load them into memory when the server starts.

## The Bundling Process

When creating a standalone application, CleanServe performs several steps:

1.  **Framework Detection**: Identifies the PHP framework and its requirements.
2.  **Asset Collection**: Gathers all application code, static assets, and vendor dependencies.
3.  **Environment Inlining**: Converts `.env` settings into PHP constants using `cleanserve-inliner`.
4.  **Preload Generation**: Creates a `preload.php` script based on `composer.json` using `cleanserve-preload`.
5.  **PHAR Creation**: Packages the application code and optimized configurations into a single PHAR file.
6.  **Final Assembly**: Bundles a static PHP binary, the PHAR file, and custom `php.ini` settings into the final executable.

## Resulting Artifact

The final output is a single, portable executable that includes everything required to run the application:

*   **Static PHP Binary**: A pre-compiled PHP binary for the target platform.
*   **Application PHAR**: The complete application code, inlined environment, and pre-compiled assets.
*   **Optimized Configuration**: Fine-tuned `php.ini` and `preload.php` settings.

This architecture ensures that CleanServe applications are truly "zero-config" and can be deployed with minimal effort across different environments.

## Further Reading

*   [Architecture Overview](overview.md)
*   [Virtual File System](vfs.md)
*   [Proxy Server Details](proxy-server.md)
