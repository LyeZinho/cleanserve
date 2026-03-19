# Configuration Reference

CleanServe uses a `cleanserve.json` file in the project root for most settings. Command line options and environment variables provide additional control.

## Example Project Configuration

```json
{
  "name": "my-project",
  "engine": {
    "php": "8.4",
    "extensions": ["gd", "pdo_mysql"],
    "display_errors": true,
    "memory_limit": "256M"
  },
  "server": {
    "root": "public/",
    "port": 8080,
    "hot_reload": true
  }
}
```

## cleanserve.json Schema

| Field | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `name` | string | (required) | Project name. |
| `engine.php` | string | `"8.4"` | PHP version to use. |
| `engine.extensions` | string[] | `[]` | PHP extensions to enable. |
| `engine.display_errors` | boolean | `true` | Show PHP errors in browser. |
| `engine.memory_limit` | string\|null | `null` | PHP memory limit (e.g., "256M"). |
| `server.root` | string | `"public/"` | Document root directory. |
| `server.port` | number | `8080` | HTTP server port. |
| `server.hot_reload` | boolean | `true` | Enable Hot Module Replacement. |

## Internal Defaults

These settings are not user-configurable yet and represent the engine's hardcoded defaults.

### Worker Pool

| Setting | Value |
| :--- | :--- |
| Min workers | 2 |
| Max workers | 10 |
| Idle timeout | 60 seconds |
| Max requests per worker | 500 |
| Startup timeout | 5000ms |

### Rate Limiter

| Setting | Value |
| :--- | :--- |
| Max requests | 1000 |
| Window | 60 seconds |
| Localhost exempt | Yes |

### Request Validator

| Setting | Value |
| :--- | :--- |
| Max content length | 10MB (10,000,000 bytes) |
| Max header size | 50KB (50,000 bytes) |

### Slowloris Protection

| Setting | Value |
| :--- | :--- |
| Header timeout | 30,000ms |
| Cleanup grace period | 60,000ms |

### Static Cache

| Setting | Value |
| :--- | :--- |
| Max entries | 1000 |
| Max size | 100MB |
| Compression | Gzip + Brotli (prefers Brotli) |
| Min file size for compression | 256 bytes |

### SSL/TLS

| Setting | Value |
| :--- | :--- |
| Certificate validity | 365 days |
| Storage | ~/.cleanserve/certs/ |
| Production TLS version | 1.3 only |

### Security Headers

CleanServe automatically injects these headers into every response.

| Header | Value |
| :--- | :--- |
| Strict-Transport-Security | max-age=31536000; includeSubDomains; preload |
| X-Content-Type-Options | nosniff |
| X-Frame-Options | SAMEORIGIN |
| X-XSS-Protection | 1; mode=block |
| Referrer-Policy | strict-origin-when-cross-origin |
| Permissions-Policy | geolocation=(), microphone=(), camera=() |
| X-Permitted-Cross-Domain-Policies | none |

### HMR (WebSocket)

| Setting | Value |
| :--- | :--- |
| Port | server.port + 1 |
| Broadcast capacity | 100 |
| Reconnect timeout | 3000ms |
| Debounce interval | 100ms |

## CLI Options Reference

| Command | Option | Type | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `init` | `--name, -n` | string | directory name | Project name. |
| `init` | `--php, -p` | string | "8.4" | PHP version. |
| `up` | `--port, -p` | number | 8080 | Server port. |
| `use` | `VERSION` | string | (required) | PHP version. |
| `update` | `--version, -v` | string | (optional) | PHP version to download. |
| `composer` | `ARGS...` | strings | | Composer arguments. |

## File and Directory Paths

| Path | Purpose |
| :--- | :--- |
| `~/.cleanserve/` | CleanServe home directory. |
| `~/.cleanserve/bin/` | PHP binary storage. |
| `~/.cleanserve/bin/php-{version}/` | Specific PHP version. |
| `~/.cleanserve/certs/` | SSL certificates. |
| `~/.cleanserve/certs/{domain}.key` | Private key. |
| `~/.cleanserve/certs/{domain}.crt` | Certificate. |
| `./cleanserve.json` | Project configuration. |

## Environment Variables

| Variable | Purpose |
| :--- | :--- |
| `CLEANSERVE_HOME` | Override home directory (Docker default: /cleanserve). |
| `RUST_LOG` | Logging filter (default: cleanserve=info). |

## Error Responses

CleanServe returns JSON error responses for system-level failures.

### Response Format

```json
{
  "error": "error_code",
  "message": "Human readable message"
}
```

### Error Codes

| Code | Description |
| :--- | :--- |
| `rate_limit_exceeded` | Client sent too many requests in a short period. |
| `request_timeout` | Server waited too long for a request header or body. |
| `payload_too_large` | Request content exceeds the 10MB limit. |
| `bad_request` | The request format is invalid. |
| `header_too_large` | Request headers exceed the 50KB limit. |
| `forbidden` | Access to the requested resource is denied. |
