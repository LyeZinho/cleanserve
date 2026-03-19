# Path Traversal

The PathTraversal layer prevents attackers from accessing files outside the application's root directory. It performs strict validation and normalization of all requested paths.

## Validation Logic

CleanServe blocks several patterns commonly used in directory traversal attacks:

*   **Traversal Sequences**: Detects and blocks `..` and `\..`.
*   **Encoded Sequences**: Detects and blocks URL-encoded (`%2e%2e`) and double-encoded (`%252e%252e`) sequences.
*   **Null Bytes**: Scans for the presence of null bytes (`\0`) in paths.
*   **Path Components**: Validates that no component of the path attempts to navigate above the root.

## Path Normalization

Every path is resolved to its canonical form before any file system operations occur. This process involves resolving all `.` (current directory) and `..` (parent directory) components to ensure the path is consistent.

## Safe Path Resolution

1.  The requested relative path is resolved against the configured application root.
2.  The resulting absolute path is verified to ensure it remains within the root directory's boundaries.
3.  Any attempt to escape the root directory is detected and blocked.

## Error Response

If a path is found to be invalid or an attempt to escape the root is detected, the server returns a `400 Bad Request` status code with a JSON body.

```json
{
  "error": "invalid_path",
  "message": "The requested path is invalid or attempts to escape the root directory."
}
```

## Related Documentation

*   [Static Blacklist](static-blacklist.md)
*   [Security Overview](overview.md)
