# Slowloris Protection

Slowloris is a type of denial-of-service attack that attempts to exhaust the server's connection pool by sending HTTP headers extremely slowly. CleanServe protects against this by monitoring the time taken to complete each request.

## Connection Tracking

The server tracks every active connection to ensure it is progressing at a reasonable rate.

*   **Mechanism**: A `HashMap` of `SocketAddr` to the timestamp of the first byte received.
*   **Timeout**: The default header timeout is 30,000 milliseconds (30 seconds).

## Request Life Cycle

1.  **`register_connection()`**: This method is called as soon as a new connection is established, recording the start time.
2.  **`is_connection_valid()`**: Periodically checks if the elapsed time since connection start has exceeded the 30-second timeout.
3.  **`mark_request_complete()`**: Resets the timer for the connection after a successful request, allowing it to be reused for subsequent requests (keep-alive).
4.  **`cleanup_expired()`**: Automatically removes stale entries from the tracking map after the timeout period plus a 60-second grace period.

## Error Response

If a connection is found to be timed out during the header phase, the server terminates the connection and returns a `408 Request Timeout` status code with a JSON body.

```json
{
  "error": "request_timeout",
  "message": "The server timed out while waiting for the request headers."
}
```

## Related Documentation

*   [Rate Limiting](rate-limiting.md)
*   [Security Overview](overview.md)
