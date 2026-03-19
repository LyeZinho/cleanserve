# PHP Version Management

CleanServe automates the process of downloading, installing, and switching between PHP versions. This allows you to run different projects on different PHP versions without global installation conflicts.

## Installation and Storage

When you initialize a project or update a version, CleanServe downloads the appropriate Non-Thread Safe (NTS) PHP binaries.

### Storage Location
All binaries are stored in your home directory:
`~/.cleanserve/bin/php-{version}/`

### Platform-Specific Downloads
- **Windows**: CleanServe downloads ZIP packages from `windows.php.net` and extracts them to `php-{version}/php.exe`.
- **Unix/Linux/macOS**: CleanServe downloads `tar.gz` packages from `php.net` and extracts them to `php-{version}/bin/php`.

## Managing Versions

### List Installed Versions
To see which PHP versions are currently installed on your system:

```bash
cleanserve list
```

### Download a New Version
To install a specific version:

```bash
cleanserve update --version 8.4
```

### Switch Project PHP Version
You can change the version used by the current project by running:

```bash
cleanserve use 8.4
```

This updates the `engine.version` property in `cleanserve.json`. Each project maintains its own isolated version reference.

## Configuring PHP Extensions

PHP extensions are managed via the `cleanserve.json` configuration file. The server automatically enables these extensions in the temporary `php.ini` generated for each project.

```json
{
  "engine": {
    "version": "8.4",
    "extensions": ["mbstring", "openssl", "curl", "pdo_mysql"]
  }
}
```

Extensions listed in the `extensions` array will be loaded when the server starts.

For more details on configuration, see the [Configuration Guide](../getting-started/configuration.md).
