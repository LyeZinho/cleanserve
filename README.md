# 🌀 CleanServe

> **The Zero-Burden PHP Runtime & Development Server.**

[](https://www.rust-lang.org/)
[](https://www.php.net/)
[](https://www.google.com/search?q=LICENSE)

**CleanServe** is a micro-server and runtime manager for PHP written in **Rust**. It was created to eliminate the "miserable chore" of configuring Apache, Nginx, or FPM in development environments, bringing the *Zero Config* experience of Node/Bun to the PHP ecosystem.

-----

## 🚀 Why CleanServe?

Setting up a modern PHP environment shouldn't require a PhD in `.conf` files. CleanServe solves three main pain points:

1.  **Annoying Configuration:** Forget VHosts, folder permissions, and manual `php.ini` editing.
2.  **Version Management:** Switch from PHP 8.2 to 8.5 instantly without cluttering your OS.
3.  **Slow Feedback:** Smart Hot Reload (HMR) that updates CSS without page refreshes and restarts PHP workers upon saving.

-----

## ✨ Features

  * 📦 **Portable PHP:** Automatically downloads and manages isolated PHP binaries.
  * 🔄 **Smart Hot Reload:**
      * **Logic:** Restarts workers when `.php` files change.
      * **Style:** Injects CSS via WebSocket without refresh (Vite-style).
  * 🎼 **Composer Native:** Runs Composer commands using the exact PHP version assigned to the project.
  * 🛠 **Zero Config:** Automatically detects your `public/index.php` and configures necessary extensions via `cleanserve.json`.
  * 🔒 **Auto HTTPS:** Generates local SSL certificates automatically.

-----

## 📥 Installation

Simply run the official installer (requires `curl` and `unzip`):

```bash
curl -fsSL https://raw.githubusercontent.com/LyeZinho/cleanserve/main/install.sh | sh
```

-----

## 🛠 How to Use

### 1\. Initialize a Project

Navigate to your PHP project folder and run:

```bash
cleanserve init
```

This will create a `cleanserve.json` file based on your `composer.json`.

### 2\. Spin Up the Server

```bash
cleanserve up
```

The server will be running at `https://localhost:8080`.

### 3\. Switch PHP Version

```bash
cleanserve use 8.5
```

-----

## 📄 The `cleanserve.json` File

The brain of your development environment:

```json
{
  "name": "my-project",
  "engine": {
    "php": "8.4",
    "extensions": ["pdo_mysql", "gd", "intl"],
    "display_errors": true
  },
  "server": {
    "root": "public/",
    "port": 8080,
    "hot_reload": true
  }
}
```

-----

## 🏗 Architecture (The Stack)

  * **Core:** Written in **Rust** for maximum performance and portability (static binary).
  * **Proxy:** An ultra-lightweight reverse proxy that intercepts requests and injects the reload script only into `text/html` responses.
  * **Watcher:** Native OS monitoring to detect file changes with zero latency.
  * **Runtime:** Official PHP (NTS binaries) managed in isolation within `~/.cleanserve`.

-----

## 📚 Documentation

Full documentation is available in the [`docs/`](docs/README.md) directory:

  * [**Getting Started**](docs/getting-started/installation.md) - Installation, quick start, and configuration
  * [**User Guide**](docs/guide/cli-commands.md) - CLI commands, PHP management, hot reload, framework support
  * [**Architecture**](docs/architecture/overview.md) - Internal design, proxy, worker pool, FastCGI, VFS
  * [**Security**](docs/security/overview.md) - Rate limiting, path traversal protection, request validation, TLS
  * [**API Reference**](docs/api/configuration-reference.md) - Full configuration reference
  * [**Contributing**](docs/contributing/development-setup.md) - Development setup and architecture guide

-----

## ✅ Definition of Done (DoD)

For a feature to be considered stable in CleanServe, it must:

  - [ ] Pass regression tests on **Linux, macOS, and Windows**.
  - [ ] Add less than **5ms** of latency to the Proxy.
  - [ ] Require no external dependencies (Shared Libs) other than the `cleanserve` binary.
  - [ ] Be fully documented in the CLI `--help` command.

-----

## 🤝 Contributing

CleanServe is a community-focused project. Feel free to open Issues or submit Pull Requests. See the [Contributing Guide](docs/contributing/development-setup.md) for setup instructions.

1.  Fork the project.
2.  Create your Feature Branch (`git checkout -b feature/AmazingFeature`).
3.  Commit your changes (`git commit -m 'Add some AmazingFeature'`).
4.  Push to the Branch (`git push origin feature/AmazingFeature`).
5.  Open a Pull Request.

-----

**CleanServe** — *Because PHP development should be clean, fast, and fun again.*
