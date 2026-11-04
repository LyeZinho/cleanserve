# Framework Support and Auto-Detection

CleanServe identifies and configures itself based on the project structure it detects. This automatic setup includes setting the correct entry point, creating necessary storage directories, and applying framework-specific PHP configurations.

## Supported Frameworks

### Laravel
- **Detection**: Found when `artisan` and `composer.json` are both present.
- **Entry Point**: `public/index.php`.
- **Auto-Config**: Creates `storage/` and `bootstrap/cache/` directories. Sets the session location to the storage directory and configures a **128MB** upload limit.

### Symfony
- **Detection**: Found when `bin/console` and `composer.json` are present.
- **Entry Point**: `public/index.php`.
- **Auto-Config**: Creates `var/` directory for logs and cache. Sets a **128MB** upload limit.

### Yii
- **Detection**: Found when `composer.json` references `yiisoft/` (Yii 3) or `yiisoft/yii2`, or when a `yii` console entry exists next to `composer.json`.
- **Entry Point**: `public/index.php` (Yii 3), `web/index.php` (Yii 2 Basic), or `frontend/web/index.php` + `backend/web/index.php` (Yii 2 Advanced).
- **Multi-app**: Yii2 Advanced is served as two independent apps, each with its own PHP worker and hostname (`frontend.serve`, `backend.serve`).
- **Auto-Config**: Creates `runtime/`, `console/runtime/`, `frontend/runtime/`, `backend/runtime/`, and the `web/assets` directories.

CleanServe passes each app's `index.php` as the PHP built-in server router, so Yii2 pretty URLs (`/site/index`) work without extra configuration. Requests that do not match a static file are routed through the app's front controller.

### Multiple apps and domains

`cleanserve init` records discovered apps in `cleanserve.json`:

```json
{
  "server": { "domain_suffix": "serve", "port": 8080 },
  "apps": {
    "frontend": { "root": "frontend/web", "framework": "yii" },
    "backend":  { "root": "backend/web",  "framework": "yii" }
  }
}
```

Run `cleanserve setupdomain` once to route `*.serve` to loopback, then `cleanserve up`. A single app keeps the `localhost` hostname.

The same mechanism is intended for future adapters: Laravel and Symfony expose one app (`public/`), while frameworks with several entry points can expose several apps.

### CodeIgniter
- **Detection**: Found when `index.php` and `system/` directory are present.
- **Entry Point**: `index.php`.
- **Auto-Config**: Automatically creates `writable/cache/` and `writable/logs/` directories.

### WordPress
- **Detection**: Found when `wp-config.php` or `wp-load.php` are present.
- **Entry Point**: `index.php`.
- **Auto-Config**: Optimizes for performance and larger file handling.
  - **Memory Limit**: `256MB`
  - **Upload Limit**: `64MB`
  - **Max Execution Time**: `300s`

### Drupal
- **Detection**: Found when `core/` and `autoload.php` are present.
- **Entry Point**: `index.php`.

## Manual Setup and Vanilla PHP

If no supported framework is detected, CleanServe falls back to its **Vanilla** configuration.

- **Entry Point**: `index.php`.
- **Auto-Config**: Creates a basic `storage/` directory for general project needs.

## Automated Setup Details

For all frameworks, CleanServe performs several key operations during initialization:

1. **Version Detection**: Reads the `composer.json` file to identify specific framework and package versions for better compatibility.
2. **Directory Management**: Missing storage, log, or cache directories are automatically created.
3. **Permissions**: On Unix-based systems (macOS, Linux), CleanServe sets directory permissions to `755` to ensure the server can write required files without configuration overhead.
