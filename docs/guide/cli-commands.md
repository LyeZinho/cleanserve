# CLI Command Reference

CleanServe provides a set of commands to manage your local PHP environment and project lifecycle.

## init

Initializes a new CleanServe project in the current directory.

```bash
cleanserve init [--name NAME] [--php VERSION]
```

This command scans your `composer.json` (if present) to generate a `cleanserve.json` configuration file. If no PHP version is specified, it defaults to **8.4**.

**Arguments:**
- `--name`: The project name. Defaults to the directory name or the name in `composer.json`.
- `--php`: The PHP version to use for this project.

## up

Starts the development server.

```bash
cleanserve up [--port PORT]
```

By default, the server listens on port **8080**. It also starts a Hot Module Replacement (HMR) WebSocket server on the next available port (default **8081**).

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

This updates the `engine.version` field in your `cleanserve.json` file.

**Example:**
```bash
cleanserve use 8.5
```

## list

Lists all PHP versions currently installed and managed by CleanServe.

```bash
cleanserve list
```

Installed versions are stored in `~/.cleanserve/bin/`.

## update

Downloads and installs a specific PHP version.

```bash
cleanserve update [--version VERSION]
```

**Arguments:**
- `--version`: The PHP version to download (e.g., `8.4.1`). If omitted, it checks for updates to the current version.

## composer

Runs Composer commands using the PHP version assigned to the project.

```bash
cleanserve composer ARGS...
```

This ensures that Composer uses the exact same PHP environment as your development server, preventing version mismatches during dependency installation.

**Example:**
```bash
cleanserve composer install
cleanserve composer require laravel/framework
```
