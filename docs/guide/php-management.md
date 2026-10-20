# PHP Version Management

CleanServe downloads, caches, and switches PHP runtimes so each project can pin a version without touching the system PHP.

## Installation and Storage

Binaries come from the [php-runtimes](https://github.com/LyeZinho/php-runtimes) manifest (`linux-x64` NTS builds today).

### Storage Location

New installs go to a **shared cache**:

`~/.cleanserve/php/php-{version}/`

`$CLEANSERVE_HOME` overrides the `~/.cleanserve` prefix.

CleanServe also still finds runtimes that were previously downloaded into a project:

`.cleanserve/php/php-{version}/`

The php-runtimes tarball layout is detected automatically (`php`, `bin/php`, or `linux-x64/php-8.4.19`).

### Version Queries

`engine.php` in `cleanserve.json` can be a minor version (`8.4`) or an exact patch (`8.4.19`):

- `8.4` uses the newest installed `8.4.x`
- `8.4.19` uses that patch only

## Managing Versions

### List Available and Installed Versions

```bash
cleanserve list
cleanserve list --installed
cleanserve list --refresh
```

### Download a Version

```bash
cleanserve update --version 8.4
```

`cleanserve update` with no flags installs the version from `cleanserve.json` (or 8.4). Use `cleanserve update --check` to look for CleanServe itself.

### Switch Project PHP Version

```bash
cleanserve use 8.4
```

This updates `engine.php` in `cleanserve.json` and downloads the runtime if needed.

## Composer and PHP CLI

Composer and the PHP CLI always use the managed binary:

```bash
cleanserve composer install
cleanserve composer create-project yiisoft/app your_project
cleanserve php -v
cleanserve php vendor/bin/phpunit
```

If Composer is not installed on the machine, CleanServe downloads `composer.phar` into `~/.cleanserve/tools/`.

## Configuring PHP Extensions

PHP extensions are declared in `cleanserve.json`:

```json
{
  "engine": {
    "php": "8.4",
    "extensions": ["mbstring", "openssl", "curl", "pdo_mysql"]
  }
}
```

For more details on configuration, see the [Configuration Guide](../getting-started/configuration.md).
