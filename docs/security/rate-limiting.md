# Rate Limiting

The RateLimiter protects the server from brute-force attacks and abuse by restricting the frequency of requests from individual IP addresses.

## Implementation

Rate limiting uses a sliding window algorithm. This ensures that the request limit is enforced consistently over time, rather than resetting at fixed intervals.

*   **Default Limit**: 1,000 requests per 60 seconds.
*   **Storage**: A thread-safe `Arc<RwLock<HashMap>>` stores a `Vec<Instant>` for each client IP.
*   **Cleanup**: Timestamps older than the current window are discarded on each check.

## IP Extraction

To correctly identify clients, the RateLimiter extracts the IP address using the following priority:

1.  The first IP listed in the `X-Forwarded-For` header.
2.  The value of the `X-Real-IP` header.
3.  The direct connection `remote_addr`.

## Localhost Exemption

The localhost address (`127.0.0.1`) is exempt from all rate limiting. This prevents development tools and scripts running on the same machine from being throttled.

## Error Response

When a client exceeds the request limit, the server returns a `429 Too Many Requests` status code with a JSON body.

```json
{
  "error": "rate_limit_exceeded",
  "message": "Too many requests. Please try again later."
}
```

The server tracks remaining requests per IP, which can be queried to provide feedback to the client.

## Related Documentation

*   [Security Overview](overview.md)
*   [Slowloris Protection](slowloris-protection.md)
