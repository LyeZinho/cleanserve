# Security Overview

CleanServe implements a multi-layer defense strategy. Every incoming request must pass through a series of security checks before it reaches the application code or serves a static file.

## Security Pipeline

Requests flow through the following components in order:

1.  **Slowloris Protection**: Monitors connection duration to prevent header-based denial of service.
2.  **Rate Limiting**: Controls request frequency per IP address.
3.  **Request Validation**: Enforces limits on payload size and header length.
4.  **Path Traversal Defense**: Validates and normalizes requested paths to prevent root escape.
5.  **Static Blacklist**: Blocks access to sensitive configuration files and prevents execution of uploaded scripts.
6.  **Application**: The request is finally passed to the PHP handler or static file server.

## Security Headers

CleanServe automatically injects security headers into every response to harden the client-side environment.

| Header | Value | Description |
| :--- | :--- | :--- |
| `Strict-Transport-Security` | `max-age=31536000; includeSubDomains; preload` | Enforces HTTPS for one year. |
| `X-Content-Type-Options` | `nosniff` | Prevents MIME type sniffing. |
| `X-Frame-Options` | `SAMEORIGIN` | Protects against clickjacking. |
| `X-XSS-Protection` | `1; mode=block` | Enables browser XSS filtering. |
| `Referrer-Policy` | `strict-origin-when-cross-origin` | Limits referrer information shared across origins. |
| `Permissions-Policy` | `geolocation=(), microphone=(), camera=()` | Disables access to sensitive browser APIs. |
| `X-Permitted-Cross-Domain-Policies` | `none` | Restricts Adobe Flash and PDF cross-domain policies. |

The `X-Powered-By` header is deliberately omitted to avoid leaking server implementation details.

## Content Security Policy (CSP)

CleanServe provides two distinct CSP profiles:

*   **Development**: Allows `unsafe-inline` for scripts and styles. Supports `ws:` and `wss:` protocols to facilitate Hot Module Replacement (HMR).
*   **Production**: Implements a strict policy that forbids inline scripts and styles.

## Error Response Format

All security-related blocks return a JSON response with a consistent structure and the appropriate HTTP status code.

```json
{
  "error": "security_code",
  "message": "Detailed description of the security violation."
}
```

Detailed documentation for each layer is available in the following sections:

*   [Rate Limiting](rate-limiting.md)
*   [Path Traversal](path-traversal.md)
*   [Static Blacklist](static-blacklist.md)
*   [Slowloris Protection](slowloris-protection.md)
*   [Request Validation](request-validation.md)
*   [SSL/TLS](ssl-tls.md)
