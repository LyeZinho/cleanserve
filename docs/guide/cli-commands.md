# CLI Command Reference

CleanServe provides a set of commands to manage your local PHP environment and project lifecycle.

## init

Initializes a new CleanServe project in the current directory.

```bash
cleanserve init [--name NAME] [--php VERSION]
```

This command scans your `composer.json` (if present) to generate a `cleanserve.json` configuration file. It reads the package name and `require.php` constraint, and detects Laravel, Symfony, Yii, WordPress, Drupal, and CodeIgniter for the document root.

**Arguments:**
- `--name`: The project name. Defaults to the name in `composer.json` or the directory name.
- `--php`: The PHP version to use. Defaults to a version compatible with `require.php`, or **8.4**.

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
