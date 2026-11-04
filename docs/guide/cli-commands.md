# CLI Command Reference

CleanServe provides a set of commands to manage your local PHP environment and project lifecycle.

## init

Initializes a new CleanServe project in the current directory.

```bash
cleanserve init [--name NAME] [--php VERSION]
```

This command scans your `composer.json` (if present) to generate a `cleanserve.json` configuration file. It reads the package name and `require.php` constraint, and detects Laravel, Symfony, Yii, WordPress, Drupal, and CodeIgniter for the document root.

For projects with several apps (Yii2 Advanced), `init` writes an `apps` map instead of creating a synthetic `public/` directory:

```json
{
  "apps": {
    "frontend": { "root": "frontend/web", "framework": "yii" },
    "backend":  { "root": "backend/web",  "framework": "yii" }
  }
}
```

When `apps` is empty, CleanServe auto-detects the layout on every `up`.

**Arguments:**
- `--name`: The project name. Defaults to the name in `composer.json` or the directory name.
- `--php`: The PHP version to use. Defaults to a version compatible with `require.php`, or **8.4**.

## up

Starts the development server.

```bash
cleanserve up [--port PORT]
```

By default, the server listens on port **8080**. It also starts a Hot Module Replacement (HMR) WebSocket server on the next available port (default **8081**).

`up` serves every app found in the project. Yii2 Advanced gets two apps, `frontend` and `backend`, each with its own PHP worker; requests are routed by hostname:

```text
https://frontend.serve -> frontend/web
https://backend.serve  -> backend/web
```

Single-app and vanilla projects keep using `https://localhost`.

**Arguments:**
- `--port`: The port to listen on.

## down

Stops the development server and all associated PHP worker processes.

```bash
cleanserve down
```

## use

Switches the PHP version for the current project.

```bash
cleanserve use VERSION
```

This updates the `engine.php` field in your `cleanserve.json` file.

**Example:**
```bash
cleanserve use 8.5
```

## list

Lists PHP versions from the php-runtimes manifest and marks which ones are already installed.

```bash
cleanserve list
cleanserve list --installed
```

Installed versions are stored in `~/.cleanserve/php/` (project-local `.cleanserve/php/` is still detected).

## update

Downloads and installs a PHP version into the shared cache.

```bash
cleanserve update [--version VERSION]
```

**Arguments:**
- `--version`: The PHP version to download (e.g., `8.4` or `8.4.19`). If omitted, uses `engine.php` from `cleanserve.json`, or 8.4.
- `--check`: Check for CleanServe application updates instead of installing PHP.

## composer

Runs Composer with the PHP version assigned to the project. Composer itself is downloaded automatically if it is not on the machine.

```bash
cleanserve composer ARGS...
```

`create-project` does **not** require an existing `cleanserve.json`. After the project is created, CleanServe runs `init` in the new directory.

**Example:**
```bash
cleanserve composer install
cleanserve composer require laravel/framework
cleanserve composer create-project yiisoft/app your_project
```

## php

Runs the managed PHP CLI with the same binary the server uses.

```bash
cleanserve php -v
cleanserve php artisan migrate
```

## setupdomain

Configures local development domains for the apps in `cleanserve.json`.

```bash
cleanserve setupdomain
cleanserve setupdomain --remove
```

The command uses `server.domain_suffix` (default `serve`) and routes `*.serve` to a small DNS responder started by `cleanserve up`. With `systemd-resolved`, it writes a drop-in and asks for sudo once. On systems without it, a managed block is written to `/etc/hosts`.

Remove everything created by the command with:

```bash
cleanserve setupdomain --remove
```

If `server.port` is `443`, the command also grants the binary `CAP_NET_BIND_SERVICE` so it can bind the privileged port.

## stack

Manages Docker-backed development services such as MySQL, Redis, and Mailpit.

```bash
cleanserve stack add mysql
cleanserve stack add redis 7.2
cleanserve stack list
cleanserve stack up
cleanserve stack down
cleanserve stack status
cleanserve stack logs mysql
cleanserve stack remove mysql
```

Services are stored in `cleanserve.json` and the generated compose file lives in `.cleanserve/stack/docker-compose.yml`. Data is kept under `.cleanserve/stack/data/<service>/`.

`cleanserve up` automatically starts the services marked as `enabled`.

> **Docker is required.** The installer checks for Docker and the Compose plugin before installing CleanServe. Set `CLEANSERVE_SKIP_DOCKER_CHECK=1` to bypass the check in CI.

Available services: `mysql`, `mariadb`, `postgres`, `redis`, `mailpit`, `phpmyadmin`, `adminer`.
