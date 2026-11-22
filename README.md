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

Run the official installer (needs `curl` or `wget`, plus `tar`):

```bash
curl -fsSL https://raw.githubusercontent.com/LyeZinho/cleanserve/main/install.sh | sh
```

The download is verified against the release's `SHA256SUMS`. Useful options (append `-s -- <option>` when piping):

```bash
sh install.sh --version v0.3.1     # a specific release
sh install.sh --dir ~/bin          # custom directory (default: /usr/local/bin, else ~/.local/bin)
sh install.sh --docker             # also install Docker (optional)
sh install.sh --purge              # also delete ~/.cleanserve (PHP runtimes, caches, certificates)
sh install.sh --uninstall          # remove the binary (add --purge to remove its data too)
```

**Upgrading from an old version:** the installer checks every CleanServe it finds (the install directory and your `PATH`). Any copy older than the minimum compatible version (`MIN_COMPATIBLE` at the top of `install.sh`) is deleted and the new one installed, with no migration. Nothing is touched until the new download is verified and known to run, files that are not CleanServe are left alone, and your data in `~/.cleanserve` is kept unless you pass `--purge`. When a release breaks compatibility with earlier installs, raise `MIN_COMPATIBLE`.

Building from source instead (e.g. after changing the code):

```bash
sh scripts/install-local.sh        # release build -> ~/.local/bin   (--debug for a debug build)
```

Then check your setup:

```bash
cleanserve doctor                  # PHP, Composer and (optional) Docker
```

**Docker is optional.** It is only used by `cleanserve stack` (MySQL, Redis, …) and as a last-resort fallback to run Composer. Install it any time with `cleanserve setup docker`.

### Composer

`cleanserve composer <args>` picks the first thing that works: `composer.phar` run by the project's PHP (downloaded once, SHA-256 verified) → a `composer` on your `PATH` → the `composer:2` Docker image.

```bash
cleanserve composer install
cleanserve composer require monolog/monolog
cleanserve composer --version
```

-----

## 🧩 Modular stack (not XAMPP)

Enable only the services you need per project — MySQL, Redis, Mailpit, phpMyAdmin — under `.cleanserve/stack/`:

```bash
cleanserve stack list
cleanserve stack add mysql
cleanserve stack up
cleanserve stack status
```

Removing CleanServe from a project tears down containers, volumes, and local images:

```bash
cleanserve uninstall
```

Legacy alias: `cleanserve package add mysql` → same as `stack add`.

-----

## 🛠 How to Use

### 1\. Initialize a Project

Navigate to your PHP project folder and run:

```bash
cleanserve init
```

This will create a `cleanserve.json` file based on your `composer.json`.

To scaffold a new app (Yii, Laravel, Symfony, …) without a system PHP:

```bash
cleanserve composer create-project yiisoft/app your_project
cd your_project
cleanserve up
```

### 2\. Spin Up the Server

```bash
cleanserve up
```

The server will be running at `https://localhost:8080` (self-signed certificate: accept it once in the browser, or use `cleanserve up --http-only` for plain `http://`).

- A new project from `cleanserve init` has a working `public/index.php`; `cleanserve init --quickstart` creates a small static site plus a JSON endpoint at `/api/example`.
- All HTTP methods (GET, POST, PUT, PATCH, DELETE, OPTIONS, HEAD) reach your PHP app with the original `Host` and `X-Forwarded-*` headers.
- The `up` dashboard is one screen: your URL, a live request log (time, method, status, duration) and, on demand, server logs. Keys: `↑↓`/`PgUp`/`PgDn` scroll (auto-follow resumes with `End`), `c` copy the selected request, `o` open the site, `l` toggle logs, `m` more actions (Docker stack), `q` quit. Use `--no-tui` for plain output (CI, pipes).
- The PHP server runs on a free local port; its request log is written to `.cleanserve/php-server.log`.
- Only one server runs per project. If the port is busy, `up` tells you and exits; use `--port` to pick another.

### 3\. Stop the Server

Press `q` / `Ctrl+C` in the dashboard (or with `--no-tui`), or from another terminal:

```bash
cleanserve down
```

### 4\. Switch PHP Version

```bash
cleanserve use 8.5
```

### 5\. Update CleanServe

```bash
cleanserve update --check   # is a newer CleanServe release available?
cleanserve update --self    # update CleanServe: download, verify (SHA-256), install, roll back on failure
cleanserve update           # install the PHP version from cleanserve.json (or 8.4)
cleanserve update -v 8.4    # install a specific PHP version
```

Updates are verified against the `SHA256SUMS` published with each GitHub release. Set `GITHUB_TOKEN` if you hit the anonymous GitHub API rate limit.

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
  * **Runtime:** Official PHP (NTS binaries from [php-runtimes](https://github.com/LyeZinho/php-runtimes)) cached in `~/.cleanserve/php`. The runtime repo is a [git submodule](./docs/contributing/php-runtimes-submodule.md) at `php-runtimes/`.

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

## 🤝 Contributing

CleanServe is a community-focused project. Feel free to open Issues or submit Pull Requests. See the [Contributing Guide](docs/contributing/development-setup.md) for setup instructions.

1.  Fork the project.
2.  Create your Feature Branch (`git checkout -b feature/AmazingFeature`).
3.  Commit your changes (`git commit -m 'Add some AmazingFeature'`).
4.  Push to the Branch (`git push origin feature/AmazingFeature`).
5.  Open a Pull Request.

-----

**CleanServe** — *Because PHP development should be clean, fast, and fun again.*
