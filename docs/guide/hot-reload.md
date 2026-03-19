# Hot Module Replacement (HMR)

CleanServe includes a built-in Hot Module Replacement (HMR) system to speed up development by automatically reflecting file changes in the browser.

## How it Works

CleanServe monitors your project directory for changes using the `notify` Rust crate. To prevent excessive updates, it uses a **100ms debounce**.

When a change is detected, it handles it based on the file type:

- **PHP Files (`.php`)**: CleanServe restarts the PHP worker pool and triggers a full page reload in the browser.
- **Static Assets (`.css`, `.js`)**: CleanServe sends a message via WebSocket to the HMR client. CSS changes are injected directly into the DOM (no page refresh required), while JavaScript changes trigger a page reload.

## WebSocket Server

By default, the HMR WebSocket server runs on the development port **+1**. If the development server is running on port **8080**, the WebSocket server will be on **8081**.

You can disable this feature in `cleanserve.json`:

```json
{
  "server": {
    "hot_reload": false
  }
}
```

## HMR Client Script

CleanServe automatically injects a small HMR client script into the HTML response just before the closing `</body>` tag. This script:
- Establishes a connection with the WebSocket server.
- Listens for update events.
- Handles CSS injection and page reloads.
- Automatically attempts to reconnect every **3 seconds** if the connection is lost.

## Broadcast Architecture

Internally, CleanServe uses a broadcast channel with a capacity of **100 messages**. This ensures that multiple connected clients receive update notifications simultaneously without missing events during rapid file changes.
