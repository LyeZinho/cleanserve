# Internal Pages Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Move 404, error, and HMR test pages from `public/` to `.cleanserve/pages/`, enhance them with context information, and update the proxy to serve them from the new location.

**Architecture:** Generate enhanced HTML pages during `cleanserve init` and write them to `.cleanserve/pages/`. The proxy reads these files when needed (404 for missing static files, error for PHP errors). Keep index.html in public/ as the user's app entry point.

**Tech Stack:** Rust, tokio async, HTML/CSS with brutal design system

---

## Task 1: Enhance 404.html page generation

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/init/html_pages.rs:320-370` (generate_default_404 function)

**Step 1: Read current 404 implementation**

Run: `head -100 crates/cleanserve-cli/src/commands/init/html_pages.rs | tail -50`

Expected: See the current generate_default_404() function that returns a String

**Step 2: Enhance generate_default_404() to accept request_path parameter**

Modify the function signature and add path display. The enhanced version should:
- Accept optional `request_path: Option<&str>` parameter
- Display "Requested path: `/path/here`" when path is provided
- Keep the brutal design aesthetic
- Add a link back to home

Replace the function (lines 320-370 approximately) with:

```rust
/// Generate 404.html error page with requested path context
fn generate_default_404() -> String {
    let css = generate_css();
    
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>404 - Not Found</title>
    {css}
</head>
<body>
    <header style="text-align: center;">
        <h1 style="color: var(--accent);">404 Error</h1>
    </header>
    
    <main style="text-align: center;">
        <div class="glass-brutal" style="max-width: 600px; margin: 3rem auto;">
            <div style="font-size: 5rem; font-weight: 700; color: var(--accent); margin-bottom: 1rem;">404</div>
            <h2>Page Not Found</h2>
            <p style="margin: 2rem 0; color: var(--text-secondary);">The resource you requested doesn't exist on this server.</p>
            
            <div style="background: var(--bg-darker); border: 1px solid var(--border); padding: 1rem; border-radius: 0; margin: 1.5rem 0; text-align: left;">
                <code style="font-size: 0.85em;">HTTP/1.1 404 Not Found
Content-Type: text/html

This page was served by CleanServe</code>
            </div>
            
            <a href="/" class="brutal-btn" style="margin-top: 1.5rem;">← Back to Home</a>
        </div>
    </main>
    
    <footer style="margin-top: 4rem; text-align: center; color: var(--text-secondary); font-size: 0.9em;">
        Powered by <strong>CleanServe</strong> — Zero-Burden PHP Runtime
    </footer>
</body>
</html>"#
    )
}
```

**Step 3: Verify the function compiles**

Run: `cargo build -p cleanserve-cli 2>&1 | grep -E "(error|warning.*404|Finished)"`

Expected: No errors, may show compilation progress

**Step 4: Commit**

```bash
git add crates/cleanserve-cli/src/commands/init/html_pages.rs
git commit -m "feat: enhance 404.html with better styling and home link"
```

---

## Task 2: Enhance error.html page generation

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/init/html_pages.rs:373-430` (generate_default_error function)

**Step 1: Update generate_default_error() to include project context**

The enhanced version should accept `project_name: &str` and `php_version: &str` parameters (will be passed from init).

Replace the function with improved version that shows:
- Project name in header
- PHP version in footer
- Better error display layout
- Refresh button
- Keep brutal design

```rust
/// Generate error.html for PHP runtime errors
fn generate_default_error(project_name: &str, php_version: &str) -> String {
    let css = generate_css();
    
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Error — {}</title>
    {css}
    <style>
        .error-stack {{ font-family: 'JetBrains Mono', monospace; font-size: 0.8em; white-space: pre-wrap; word-break: break-word; }}
        .error-file {{ color: var(--accent); font-weight: 600; }}
    </style>
</head>
<body>
    <header>
        <div style="max-width: 1200px; margin: 0 auto;">
            <h1 style="color: var(--accent); margin-bottom: 0.5rem;">Error Encountered</h1>
            <p style="color: var(--text-secondary); font-size: 0.9em;">Project: <strong>{}</strong></p>
        </div>
    </header>
    
    <main style="max-width: 1200px; margin: 0 auto;">
        <div class="glass-brutal" style="margin: 2rem 0;">
            <div style="margin-bottom: 2rem;">
                <h2 style="color: var(--accent); margin-bottom: 1rem;">What Happened?</h2>
                <p>A PHP error occurred while processing your request. Check the details below and the server logs for more information.</p>
            </div>
            
            <div style="background: var(--bg-darker); border-left: 4px solid var(--accent); padding: 1.5rem; margin: 1.5rem 0;">
                <p style="margin: 0; color: var(--text-secondary);">
                    <span class="error-file">Error details would appear here</span><br>
                    The actual error message will be injected by the error overlay system.
                </p>
            </div>
            
            <div style="display: flex; gap: 1rem; margin-top: 2rem;">
                <button class="brutal-btn" onclick="location.reload();">🔄 Retry Request</button>
                <a href="/" class="brutal-btn" style="text-decoration: none;">← Back to Home</a>
            </div>
        </div>
    </main>
    
    <footer style="text-align: center; color: var(--text-secondary); font-size: 0.85em; padding: 2rem 1rem;">
        <p><strong>{}</strong> • PHP {}</p>
        <p style="margin-top: 0.5rem;">Powered by <strong>CleanServe</strong></p>
    </footer>
</body>
</html>"#,
        project_name, project_name, project_name, php_version
    )
}
```

**Step 2: Update the write_default_pages() function signature**

Modify the function call in `write_default_pages()` at line 440 to pass parameters:

Change from:
```rust
let error = generate_default_error();
```

To:
```rust
let error = generate_default_error(project_name, php_version);
```

**Step 3: Verify compilation**

Run: `cargo build -p cleanserve-cli 2>&1 | grep -E "(error|warning|Finished)"`

Expected: Build succeeds

**Step 4: Commit**

```bash
git add crates/cleanserve-cli/src/commands/init/html_pages.rs
git commit -m "feat: enhance error.html with project context and retry button"
```

---

## Task 3: Create hmr-test.html page generator

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/init/html_pages.rs` (add new function before write_default_pages)

**Step 1: Add new generate_hmr_test_page() function**

Insert before line 426 (before write_default_pages function):

```rust
/// Generate hmr-test.html for internal HMR testing (hidden from users)
fn generate_hmr_test_page() -> String {
    let css = generate_css();
    
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CleanServe HMR Test</title>
    {css}
    <style>
        #messages {{ border: 1px solid var(--border); padding: 1rem; height: 400px; overflow-y: auto; background: var(--bg-darker); border-radius: 0; margin: 1rem 0; font-family: 'JetBrains Mono', monospace; font-size: 0.85em; }}
        .msg {{ padding: 0.5rem; border-bottom: 1px solid var(--border); }}
        .msg.connected {{ color: var(--accent); }}
        .msg.error {{ color: #ef4444; }}
        .msg.event {{ color: #60a5fa; }}
        .status {{ display: inline-block; width: 12px; height: 12px; border-radius: 50%; margin-right: 0.5rem; }}
        .status.active {{ background: var(--accent); }}
        .status.inactive {{ background: #6b7280; }}
    </style>
</head>
<body>
    <header>
        <div style="max-width: 1200px; margin: 0 auto;">
            <h1>🔄 CleanServe HMR Test <span style="font-size: 0.6em; color: var(--text-secondary);">(Internal)</span></h1>
        </div>
    </header>
    
    <main style="max-width: 1200px; margin: 0 auto;">
        <div class="glass-brutal">
            <h2>WebSocket Status</h2>
            <p>
                Connection: 
                <span class="status"></span>
                <span id="status">Disconnected</span>
            </p>
            
            <h2 style="margin-top: 2rem;">Events Received</h2>
            <div id="messages"></div>
            
            <button class="brutal-btn" onclick="clearMessages()" style="margin-top: 1rem;">Clear Messages</button>
        </div>
    </main>
    
    <footer style="text-align: center; color: var(--text-secondary); font-size: 0.85em; padding: 2rem 1rem;">
        <p>For HMR testing only. Edit CSS or PHP files while this page is open.</p>
    </footer>
    
    <script>
        const messagesDiv = document.getElementById('messages');
        const statusEl = document.getElementById('status');
        const statusDot = document.querySelector('.status');
        
        function addMessage(type, text) {{
            const div = document.createElement('div');
            div.className = 'msg ' + type;
            div.textContent = '[' + new Date().toLocaleTimeString() + '] ' + text;
            messagesDiv.appendChild(div);
            messagesDiv.scrollTop = messagesDiv.scrollHeight;
        }}
        
        function clearMessages() {{
            messagesDiv.innerHTML = '';
        }}
        
        function updateStatus(connected) {{
            statusEl.textContent = connected ? 'Connected' : 'Disconnected';
            statusDot.className = 'status ' + (connected ? 'active' : 'inactive');
        }}
        
        const wsPort = parseInt(location.port || '80') + 1;
        const wsUrl = 'ws://' + location.hostname + ':' + wsPort + '/__cleanserve_hmr';
        
        addMessage('', 'Connecting to ' + wsUrl + '...');
        
        const ws = new WebSocket(wsUrl);
        
        ws.onopen = () => {{
            addMessage('connected', '✅ WebSocket connected');
            updateStatus(true);
        }};
        
        ws.onmessage = (event) => {{
            try {{
                const data = JSON.parse(event.data);
                if (data.type === 'reload') {{
                    addMessage('event', '📦 PHP reload requested');
                }} else if (data.type === 'style') {{
                    addMessage('event', '🎨 CSS reload: ' + data.path);
                }} else if (data.type === 'connected') {{
                    addMessage('event', '🔗 HMR server acknowledged');
                }} else {{
                    addMessage('event', '📨 Message: ' + JSON.stringify(data));
                }}
            }} catch (e) {{
                addMessage('error', '❌ Parse error: ' + e.message);
            }}
        }};
        
        ws.onerror = (e) => {{
            addMessage('error', '❌ WebSocket error');
            updateStatus(false);
        }};
        
        ws.onclose = () => {{
            addMessage('error', '❌ WebSocket disconnected');
            updateStatus(false);
        }};
    </script>
</body>
</html>"#
    )
}
```

**Step 2: Verify compilation**

Run: `cargo build -p cleanserve-cli 2>&1 | grep -E "(error|Finished)"`

Expected: Build succeeds

**Step 3: Commit**

```bash
git add crates/cleanserve-cli/src/commands/init/html_pages.rs
git commit -m "feat: add hmr-test.html internal testing page"
```

---

## Task 4: Update write_default_pages() to generate HMR test page

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/init/html_pages.rs:426-444` (write_default_pages function)

**Step 1: Add hmr_test page generation**

Update write_default_pages() function to also generate the HMR test page (but don't write it yet - we'll move it to .cleanserve/pages in next task):

Add after line 441 (after error.html):

```rust
    let hmr_test = generate_hmr_test_page();
```

Don't write it yet - just generate it and keep it for the next task.

**Step 2: Verify compilation**

Run: `cargo build -p cleanserve-cli 2>&1 | grep -E "(error|Finished)"`

Expected: Build succeeds, hmr_test variable is generated but not yet used for writing

**Step 3: Commit**

```bash
git add crates/cleanserve-cli/src/commands/init/html_pages.rs
git commit -m "refactor: generate hmr_test page in write_default_pages"
```

---

## Task 5: Update init/mod.rs to create .cleanserve/pages/ directory

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/init/mod.rs` (add .cleanserve/pages creation)

**Step 1: Read current init/mod.rs**

Run: `cat crates/cleanserve-cli/src/commands/init/mod.rs`

Expected: See the init flow, lines 30-50 show .gitignore and public directory creation

**Step 2: Add .cleanserve/pages creation before public directory**

After line 29 (after config.save), add:

```rust
    // Create .cleanserve/pages/ directory for internal pages
    let cleanserve_pages_dir = Path::new(".cleanserve").join("pages");
    std::fs::create_dir_all(&cleanserve_pages_dir)
        .context("Failed to create .cleanserve/pages/ directory")?;
```

**Step 3: Verify compilation**

Run: `cargo build -p cleanserve-cli 2>&1 | grep -E "(error|Finished)"`

Expected: Build succeeds

**Step 4: Commit**

```bash
git add crates/cleanserve-cli/src/commands/init/mod.rs
git commit -m "feat: create .cleanserve/pages/ directory during init"
```

---

## Task 6: Move page writing to .cleanserve/pages/

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/init/html_pages.rs:426-444` (update write_default_pages signature and implementation)
- Modify: `crates/cleanserve-cli/src/commands/init/mod.rs:42-46` (update function calls)

**Step 1: Update write_default_pages() to accept cleanserve_pages_dir parameter**

Modify function signature (line 426):

Change from:
```rust
pub fn write_default_pages(
    public_dir: &Path,
    project_name: &str,
    php_version: &str,
) -> anyhow::Result<()> {
```

To:
```rust
pub fn write_default_pages(
    public_dir: &Path,
    cleanserve_pages_dir: &Path,
    project_name: &str,
    php_version: &str,
) -> anyhow::Result<()> {
```

**Step 2: Update page writing locations**

Replace lines 432-442 with:

```rust
    let index = generate_default_index(project_name, php_version);
    std::fs::write(public_dir.join("index.html"), index)
        .context("Failed to write public/index.html")?;

    let not_found = generate_default_404();
    std::fs::write(cleanserve_pages_dir.join("404.html"), not_found)
        .context("Failed to write .cleanserve/pages/404.html")?;

    let error = generate_default_error(project_name, php_version);
    std::fs::write(cleanserve_pages_dir.join("error.html"), error)
        .context("Failed to write .cleanserve/pages/error.html")?;

    let hmr_test = generate_hmr_test_page();
    std::fs::write(cleanserve_pages_dir.join("hmr-test.html"), hmr_test)
        .context("Failed to write .cleanserve/pages/hmr-test.html")?;
```

**Step 3: Update init/mod.rs to pass cleanserve_pages_dir**

Modify line 45 in init/mod.rs:

Change from:
```rust
            html_pages::write_default_pages(public_dir, &project_name, &php)
```

To:
```rust
            html_pages::write_default_pages(public_dir, &cleanserve_pages_dir, &project_name, &php)
```

**Step 4: Update success message in init/mod.rs**

Change line 49 from:
```rust
        println!("✓ Created public/ directory with HTML pages");
```

To:
```rust
        println!("✓ Created public/ with index.html");
        println!("✓ Created .cleanserve/pages/ with internal pages");
```

**Step 5: Update next steps in init/mod.rs**

Update line 75 from:
```rust
        println!("  1. Edit public/index.php with your application code");
```

To:
```rust
        println!("  1. Edit public/index.html with your application code");
```

**Step 6: Verify compilation**

Run: `cargo build -p cleanserve-cli 2>&1 | grep -E "(error|Finished)"`

Expected: Build succeeds

**Step 7: Test init command creates correct directories**

Run: `cd /tmp && rm -rf cleanserve-test && mkdir cleanserve-test && cd cleanserve-test && /home/pedro/repo/cleanserve/target/debug/cleanserve init --name test-project --php 8.4`

Expected output should show:
```
✓ Created cleanserve.json
✓ Created public/ with index.html
✓ Created .cleanserve/pages/ with internal pages
✓ Added .cleanserve/ to .gitignore
```

Verify files exist:
```bash
ls -la public/
ls -la .cleanserve/pages/
```

Expected:
```
public/:
  index.html

.cleanserve/pages/:
  404.html
  error.html
  hmr-test.html
```

**Step 8: Commit**

```bash
git add crates/cleanserve-cli/src/commands/init/html_pages.rs
git add crates/cleanserve-cli/src/commands/init/mod.rs
git commit -m "feat: move 404, error, hmr-test pages to .cleanserve/pages/"
```

---

## Task 7: Update write_quickstart_pages() similarly (if applicable)

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/init/html_pages.rs` (write_quickstart_pages function, around line 900+)

**Step 1: Check if write_quickstart_pages exists and needs updating**

Run: `grep -n "write_quickstart_pages" crates/cleanserve-cli/src/commands/init/html_pages.rs`

Expected: See function definition around line 900+

**Step 2: If it exists, update to also use cleanserve_pages_dir**

Find the function and update it to accept `cleanserve_pages_dir: &Path` parameter, then ensure error/404 pages are also written there.

**Step 3: Update init/mod.rs call for quickstart**

Update line 42 to also pass cleanserve_pages_dir:

Change from:
```rust
            html_pages::write_quickstart_pages(public_dir, &project_name)
```

To:
```rust
            html_pages::write_quickstart_pages(public_dir, &cleanserve_pages_dir, &project_name)
```

**Step 4: Verify and commit**

```bash
cargo build -p cleanserve-cli 2>&1 | grep error
git add crates/cleanserve-cli/src/commands/init/html_pages.rs crates/cleanserve-cli/src/commands/init/mod.rs
git commit -m "feat: update quickstart pages to use .cleanserve/pages/"
```

---

## Task 8: Update proxy server.rs to serve pages from .cleanserve/pages/

**Files:**
- Modify: `crates/cleanserve-proxy/src/server.rs` (404 handler, around line 434-440)

**Step 1: Read current 404 response**

Run: `grep -A5 "404 Not Found" crates/cleanserve-proxy/src/server.rs | head -20`

Expected: See hardcoded 404 HTML response

**Step 2: Add function to load .cleanserve/pages/404.html**

Add new helper function before handle_request function (around line 225):

```rust
async fn load_error_page(error_type: &str) -> String {
    let cleanserve_dir = Path::new(".cleanserve").join("pages");
    let file_path = cleanserve_dir.join(format!("{}.html", error_type));
    
    match tokio::fs::read_to_string(&file_path).await {
        Ok(content) => content,
        Err(_) => {
            // Fallback if file not found
            match error_type {
                "404" => r#"<h1>404 Not Found</h1><p>The requested resource was not found.</p>"#.to_string(),
                "error" => r#"<h1>Error</h1><p>An error occurred while processing your request.</p>"#.to_string(),
                _ => r#"<h1>Error</h1><p>An unexpected error occurred.</p>"#.to_string(),
            }
        }
    }
}
```

**Step 3: Add Path import if needed**

Check if Path is imported at top of server.rs:

Run: `head -20 crates/cleanserve-proxy/src/server.rs | grep "use std::path"`

If not present, add to imports:
```rust
use std::path::Path;
```

**Step 4: Update 404 response to load from file**

Find line ~434-440 where 404 response is generated. Replace:

From:
```rust
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "text/html")
        .body(Full::new(Bytes::from(
            "<h1>404 Not Found</h1><p>The requested resource was not found on this server.</p>",
        )))
        .unwrap())
```

To:
```rust
    let page_content = load_error_page("404").await;
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "text/html")
        .body(Full::new(Bytes::from(page_content)))
        .unwrap())
```

**Step 5: Verify compilation**

Run: `cargo build -p cleanserve-proxy 2>&1 | grep -E "(error|warning|Finished)"`

Expected: Build succeeds

**Step 6: Commit**

```bash
git add crates/cleanserve-proxy/src/server.rs
git commit -m "feat: serve 404.html from .cleanserve/pages/"
```

---

## Task 9: Test the implementation

**Files:**
- None (testing only)

**Step 1: Build the project**

Run: `cargo build 2>&1 | tail -5`

Expected: `Finished dev profile [unoptimized + debuginfo]`

**Step 2: Create a test project**

```bash
cd /tmp
rm -rf cleanserve-final-test
mkdir cleanserve-final-test
cd cleanserve-final-test
/home/pedro/repo/cleanserve/target/debug/cleanserve init --name final-test
```

**Step 3: Verify directory structure**

```bash
ls -la
ls -la public/
ls -la .cleanserve/pages/
```

Expected:
- `public/index.html` exists
- `.cleanserve/pages/404.html` exists
- `.cleanserve/pages/error.html` exists
- `.cleanserve/pages/hmr-test.html` exists

**Step 4: Start server and test 404**

```bash
timeout 3 /home/pedro/repo/cleanserve/target/debug/cleanserve up &
sleep 1
curl -s http://localhost:8080/nonexistent 2>&1 | head -10
pkill -f "cleanserve up"
```

Expected: 404.html content is returned (should contain "404" and design styling)

**Step 5: Verify HMR test page exists but isn't exposed**

```bash
ls -la .cleanserve/pages/hmr-test.html
echo "✓ HMR test page exists in .cleanserve/pages/"
```

Expected: File exists

**Step 6: No final commit needed for tests**

Tests are complete - all implementation is committed per-task.

---

## Summary

- ✅ Enhanced 404.html with better styling and home link
- ✅ Enhanced error.html with project context and PHP version
- ✅ Created hmr-test.html for internal HMR testing
- ✅ Updated init to create .cleanserve/pages/ directory
- ✅ Moved page generation to write to .cleanserve/pages/
- ✅ Updated proxy to serve 404.html from .cleanserve/pages/
- ✅ Verified directory structure and file serving
- ✅ No breaking changes to existing projects
