# Dynamic Worker Pool

The Dynamic Worker Pool, implemented in `crates/cleanserve-core/src/worker_pool.rs`, manages the lifecycle of PHP processes. It provides an efficient mechanism for executing PHP scripts by maintaining a pool of ready-to-use workers that can scale based on demand.

## Worker Configuration

The pool is governed by several configuration parameters to balance resource usage and responsiveness:

*   `min_workers`: 2 (The minimum number of workers kept alive at all times)
*   `max_workers`: 10 (The maximum number of concurrent workers allowed)
*   `idle_timeout`: 60 seconds (How long a worker can remain idle before being terminated)
*   `max_requests_per_worker`: 500 (The number of requests a worker can handle before being recycled)
*   `startup_timeout`: 5000ms (The maximum time allowed for a worker to become ready)

## Worker Lifecycle and States

Each worker in the pool can be in one of four states:

1.  **Starting**: The PHP process has been launched but is not yet ready to accept connections.
2.  **Idle**: The worker is ready and waiting for a request.
3.  **Busy**: The worker is currently processing a request.
4.  **Stopped**: The process has been terminated.

### Worker Process Command

Each worker is an instance of the PHP built-in server:

```bash
php -S 127.0.0.1:{port + 9000 + worker_id} -t {project_root}
```

## Scaling Logic

The worker pool employs an auto-scaling strategy to optimize performance:

*   **Warm-up**: At startup, the pool immediately spawns `min_workers`.
*   **Scale Up**: When all available workers are busy and a new request arrives, a new worker is spawned (up to `max_workers`).
*   **Scale Down**: If a worker has been in the `Idle` state for longer than `idle_timeout`, and the current count is above `min_workers`, it is terminated.
*   **Recycling**: After a worker processes `max_requests_per_worker`, it is automatically stopped and replaced to prevent memory leaks in the PHP process.

## Process Management

CleanServe handles process termination gracefully and across different platforms:

*   **Windows**: Uses `taskkill` to ensure all child processes are terminated.
*   **Unix-like**: Uses the `kill` signal.

## Pool Statistics

The pool tracks several metrics to provide insights into its performance:

*   **Total Workers**: The current number of spawned processes.
*   **Available**: Workers in the `Idle` state.
*   **Busy**: Workers currently handling requests.
*   **Total Requests**: A cumulative count of requests handled by the pool since it was started.

## Further Reading

*   [Architecture Overview](overview.md)
*   [Proxy Server Details](proxy-server.md)
*   [FastCGI Protocol Implementation](fastcgi.md)
