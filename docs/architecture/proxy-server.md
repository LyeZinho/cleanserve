# Proxy Server

The Proxy Server, implemented in `crates/cleanserve-proxy/src/server.rs`, is the primary entry point for all incoming HTTP traffic. It binds to `127.0.0.1:{port}` and manages the lifecycle of each request using the `hyper` library.

## Request Pipeline

Each incoming request must pass through a multi-stage pipeline of security checks and validation before it is served or routed to a PHP worker.

1.  **Slowloris Check**: Detects and mitigates slow-headers attacks by monitoring connection speed and header completeness.
2.  **Rate Limiting**: Limits the number of requests per IP address. This step is skipped for `localhost` connections to ensure a smooth development experience.
3.  **Request Validation**:
    *   Verifies `Content-Length` and `Content-Type` headers for consistency.
    *   Enforces maximum header size limits.
4.  **Path Traversal Check**: Prevents directory traversal attacks by normalizing and validating the request path against the project root.
5.  **Static Blacklist Check**: Blocks access to sensitive files and directories (e.g., `.git`, `vendor`, `.env`).
6.  **Dangerous Upload Check**: Inspects incoming request bodies for patterns indicative of malicious file uploads.
7.  **Routing**: Determines if the request targets a static asset (served directly) or requires PHP processing.

## Hot Module Replacement (HMR)

The Proxy Server includes an integrated HMR system to provide a seamless development experience. It consists of two components: a WebSocket server and an HTML injector.

### HTML Injection

For responses with a `Content-Type` of `text/html`, the proxy automatically injects a small JavaScript snippet before the closing `</body>` tag. This script establishes a connection to the HMR WebSocket server.

### WebSocket Server

The HMR server listens on a port derived from the main server port (`port + 1`). It uses `tokio-tungstenite` and a broadcast channel with a capacity of 100 to send events to connected clients.

There are two primary HMR event types:

*   `PhpReload`: Triggers a full page reload in the browser when a PHP file or configuration change is detected.
*   `StyleReload(path)`: Injects updated CSS into the page without a full reload, providing instant feedback for style changes.

## Error Handling

When the proxy encounters an error (e.g., a failed security check or a routing error), it returns a standard JSON response:

```json
{
  "code": "ERROR_CODE",
  "message": "Human-readable error description"
}
```

This format ensures that even error responses are predictable and can be handled gracefully by client-side tools or the CleanServe error overlay.

## Further Reading

*   [Architecture Overview](overview.md)
*   [Worker Pool Management](worker-pool.md)
*   [FastCGI Protocol Implementation](fastcgi.md)
