# Quick Start

CleanServe makes it easy to run PHP projects locally with minimal configuration. This guide shows how to initialize and run a PHP server in seconds.

## Project Setup

Navigate to your PHP project directory and initialize CleanServe:

```bash
cleanserve init
```

The command creates a `cleanserve.json` file in your project root. If a `composer.json` is present, it will automatically detect the project name and required PHP version.

### Initialization Options

Customize the project name or specify a PHP version during initialization:

```bash
cleanserve init --name my-project --php 8.3
```

By default, CleanServe uses PHP 8.4 if no version is specified.

## Running the Server

Start the development server with the following command:

```bash
cleanserve up
```

The server starts by default on `https://localhost:8080`.

### Server Options

Change the port or other settings via flags:

```bash
cleanserve up --port 9000
```

> **Note:** CleanServe also starts a Hot Module Replacement (HMR) WebSocket on `port + 1`. If the server is on 8080, the HMR port will be 8081.

## Version Management

CleanServe allows you to switch between PHP versions instantly.

### Switching PHP Versions

Change the current project's PHP version:

```bash
cleanserve use 8.5
```

This updates the `cleanserve.json` file and downloads the specified version if it's not already installed.

### Listing Installed Versions

View all PHP versions installed locally by CleanServe:

```bash
cleanserve list
```

## Composer Integration

CleanServe provides a wrapper for Composer to ensure it runs with the correct PHP version for your project.

```bash
cleanserve composer install
```

This uses the PHP binary managed by CleanServe, avoiding conflicts with system-wide PHP installations.

## Framework Auto-Detection

CleanServe automatically detects common PHP frameworks and configures itself accordingly. Supported frameworks include:

- Laravel
- Symfony
- WordPress
- CodeIgniter
- Drupal

## Stopping the Server

To stop the running server, use the following command:

```bash
cleanserve down
```

## Next Steps

For advanced configuration, refer to the [Configuration Guide](./configuration.md) to customize PHP extensions, document roots, and more.
