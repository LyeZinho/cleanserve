# FastCGI Protocol Implementation

CleanServe includes a robust, native implementation of the FastCGI protocol (version 1), located in `crates/cleanserve-core/src/fastcgi/mod.rs`. This allows CleanServe to communicate efficiently with PHP-FPM and other FastCGI-compliant workers.

## Record Types

The implementation supports several key FastCGI record types:

*   `BEGIN_REQUEST`: Initiates a new request.
*   `ABORT_REQUEST`: Terminates an ongoing request.
*   `END_REQUEST`: Sent by the worker when the request is complete.
*   `PARAMS`: Transmits request-specific parameters (environment variables).
*   `STDIN`: Sends the request body (e.g., POST data).
*   `STDOUT`: Receives the response body from the worker.
*   `STDERR`: Captures errors or logs from the worker.
*   `DATA`: For applications that process additional data.
*   `GET_VALUES`, `GET_VALUES_RESULT`, `UNKNOWN_TYPE`: Used for protocol-level queries and error reporting.

## Binary Protocol Structure

Each FastCGI record follows a standard binary format:

1.  **Header (8 bytes)**: Contains the protocol version, record type, request ID, content length, and padding length.
2.  **Content**: The actual payload of the record.
3.  **Padding**: Extra bytes to ensure 8-byte alignment.

### Name-Value Encoding

FastCGI parameters are encoded as name-value pairs. Lengths are represented using a variable-length scheme:

*   **Length < 128**: Encoded in 1 byte.
*   **Length >= 128**: Encoded in 4 bytes, with the most significant bit set to 1 (31-bit length).

## Roles and Communication

CleanServe primarily operates in the `RESPONDER` role, where it forwards requests to and receives responses from PHP workers. Other roles like `AUTHORIZER` and `FILTER` are supported by the protocol implementation but are not currently used in the core flow.

### Connection Management

To optimize communication, CleanServe maintains a connection pool for FastCGI workers, reducing the overhead of establishing new TCP connections for every request.

## CGI Variable Mapping

HTTP headers and request metadata are mapped to FastCGI parameters following the CGI standard:

*   **Header Transformation**: Headers are prefixed with `HTTP_`, converted to uppercase, and dashes are replaced with underscores (e.g., `User-Agent` becomes `HTTP_USER_AGENT`).
*   **Standard Variables**:
    *   `REQUEST_METHOD`: The HTTP method (e.g., GET, POST).
    *   `SCRIPT_FILENAME`: The absolute path to the PHP script being executed.
    *   `REQUEST_URI`: The original request URI.
    *   `QUERY_STRING`: The URL query parameters.
    *   `SERVER_PROTOCOL`: The protocol version (e.g., HTTP/1.1).
    *   `GATEWAY_INTERFACE`: Fixed as `CGI/1.1`.
    *   `SERVER_SOFTWARE`: Set to `CleanServe/0.1`.

## Response Parsing

The FastCGI client parses responses from the worker into two main parts:

1.  **Headers**: Separated from the body by an empty line. The `Status` header is extracted to set the final HTTP response code.
2.  **Body**: The remaining output, which is streamed back to the client via the proxy.

## Further Reading

*   [Architecture Overview](overview.md)
*   [Proxy Server Details](proxy-server.md)
*   [Worker Pool Management](worker-pool.md)
