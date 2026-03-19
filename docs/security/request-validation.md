# Request Validation

The RequestValidator enforces protocol-level constraints on incoming HTTP requests to ensure they adhere to safety limits.

## Content-Length Enforcement

CleanServe limits the size of request payloads to prevent memory exhaustion.

*   **Max Payload Size**: 10MB (10,000,000 bytes).
*   **Validation**: Requests with a `Content-Length` header exceeding this limit are rejected.
*   **Invalid Values**: Non-numeric or otherwise malformed `Content-Length` values are also rejected.

**Error Response (Payload Too Large):**
`413 Payload Too Large`
```json
{
  "error": "payload_too_large",
  "message": "The request payload exceeds the maximum allowed size of 10MB."
}
```

## Content-Type Requirement

To prevent CSRF and other types of request confusion, CleanServe mandates the presence of the `Content-Type` header for requests that include a payload.

*   **Enforced Methods**: `POST`, `PUT`, `PATCH`.
*   **Requirement**: These requests must include a valid `Content-Type` header.

**Error Response (Missing Content-Type):**
`400 Bad Request`
```json
{
  "error": "missing_content_type",
  "message": "POST, PUT, and PATCH requests must include a Content-Type header."
}
```

## Header Size Validation

Total header size is limited to prevent attacks that use excessively large headers.

*   **Max Header Size**: 50KB (50,000 bytes).
*   **Calculation**: Includes all header names and values in the request.

**Error Response (Headers Too Large):**
`431 Request Header Fields Too Large`
```json
{
  "error": "headers_too_large",
  "message": "The total size of the request headers exceeds the 50KB limit."
}
```

## Related Documentation

*   [Slowloris Protection](slowloris-protection.md)
*   [Security Overview](overview.md)
