use anyhow::Context;
use std::path::Path;

/// Generate shared brutaliste CSS for all pages
/// Color palette: zinc-950 (#09090b) bg, emerald-500 (#10b981) highlight
fn generate_css() -> &'static str {
    r#"<style>
/* === Color Palette === */
:root {
  --bg-dark: #09090b;
  --bg-darker: #18181b;
  --bg-card: rgba(24, 24, 27, 0.5);
  --text-primary: #f4f4f5;
  --text-secondary: #a1a1a6;
  --accent: #10b981;
  --accent-dark: #059669;
  --border: #27272a;
  --shadow-brutal: 8px 8px 0px 0px rgba(0, 0, 0, 0.5);
}

/* === Reset & Base === */
* { margin: 0; padding: 0; box-sizing: border-box; }

html, body {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  background: linear-gradient(135deg, var(--bg-dark) 0%, #0f0f12 100%);
  color: var(--text-primary);
  line-height: 1.6;
  min-height: 100vh;
  background-attachment: fixed;
}

code, pre {
  font-family: 'JetBrains Mono', 'Courier New', monospace;
  font-size: 0.9em;
}

a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }

/* === Typography === */
h1 { font-family: 'Space Grotesk', sans-serif; font-size: 3rem; font-weight: 700; letter-spacing: -0.02em; margin-bottom: 1rem; }
h2 { font-family: 'Space Grotesk', sans-serif; font-size: 2rem; font-weight: 700; margin-top: 2rem; margin-bottom: 0.5rem; }
h3 { font-family: 'Space Grotesk', sans-serif; font-size: 1.5rem; font-weight: 600; margin-top: 1.5rem; margin-bottom: 0.5rem; }
p { margin-bottom: 1rem; color: var(--text-secondary); }

/* === Layout === */
body > header { padding: 2rem 1rem; border-bottom: 2px solid var(--border); }
body > main { padding: 2rem 1rem; max-width: 1200px; margin: 0 auto; }
body > footer { padding: 2rem 1rem; border-top: 2px solid var(--border); text-align: center; }

/* === Brutal Components === */
.brutal-btn {
  background: var(--accent);
  color: var(--bg-dark);
  border: none;
  padding: 0.75rem 1.5rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.1s;
  font-family: 'Space Grotesk', sans-serif;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  font-size: 0.9rem;
  box-shadow: var(--shadow-brutal);
}

.brutal-btn:hover {
  transform: translate(-4px, -4px);
  box-shadow: 12px 12px 0px 0px rgba(16, 185, 129, 0.3);
}

.brutal-btn:active {
  transform: translate(2px, 2px);
  box-shadow: 4px 4px 0px 0px rgba(0, 0, 0, 0.5);
}

.glass-brutal {
  backdrop-filter: blur(10px);
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 0;
  padding: 1.5rem;
  box-shadow: var(--shadow-brutal);
}

.brutal-card {
  background: var(--bg-card);
  border: 2px solid var(--border);
  padding: 1.5rem;
  box-shadow: var(--shadow-brutal);
  transition: all 0.2s;
}

.brutal-card:hover {
  border-color: var(--accent);
  transform: translate(-2px, -2px);
}

.brutal-stat {
  text-align: center;
  padding: 1rem;
  background: var(--bg-darker);
  border: 1px solid var(--border);
  border-radius: 0;
}

.brutal-stat-value { font-size: 2rem; font-weight: 700; color: var(--accent); font-family: 'Space Grotesk', sans-serif; }
.brutal-stat-label { font-size: 0.875rem; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.1em; margin-top: 0.5rem; }

.brutal-code {
  background: var(--bg-darker);
  border-left: 4px solid var(--accent);
  padding: 1rem;
  overflow-x: auto;
  font-size: 0.875rem;
}

/* === Grid & Layout === */
.grid { display: grid; gap: 1.5rem; }
.grid-2 { grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); }
.grid-3 { grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); }

/* === Utility === */
.text-center { text-align: center; }
.mt-2 { margin-top: 1rem; }
.mt-4 { margin-top: 2rem; }
.mb-2 { margin-bottom: 1rem; }
.mb-4 { margin-bottom: 2rem; }
.p-2 { padding: 1rem; }
.p-4 { padding: 2rem; }

/* === Dark Theme Texture === */
body::before {
  content: '';
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-image: 
    repeating-linear-gradient(
      90deg,
      transparent,
      transparent 2px,
      rgba(255, 255, 255, 0.03) 2px,
      rgba(255, 255, 255, 0.03) 4px
    );
  pointer-events: none;
  z-index: 1;
}

body > * { position: relative; z-index: 2; }

/* === Responsive === */
@media (max-width: 768px) {
  h1 { font-size: 2rem; }
  h2 { font-size: 1.5rem; }
  body > main { padding: 1rem; }
  .grid-2, .grid-3 { grid-template-columns: 1fr; }
}
</style>"#
}

/// Generate shared vanilla JS for interactivity (copy buttons, theme toggle, expandable sections, smooth scroll)
fn generate_js() -> &'static str {
    r##"<script>
document.addEventListener('DOMContentLoaded', function() {
  // Copy to clipboard functionality
  document.querySelectorAll('[data-copy]').forEach(btn => {
    btn.addEventListener('click', function() {
      const text = this.getAttribute('data-copy');
      navigator.clipboard.writeText(text).then(() => {
        const original = this.textContent;
        this.textContent = '✓ Copied!';
        setTimeout(() => { this.textContent = original; }, 2000);
      });
    });
  });

  // Expandable sections (accordion)
  document.querySelectorAll('[data-expand]').forEach(header => {
    header.addEventListener('click', function() {
      const content = this.nextElementSibling;
      const isOpen = content.style.display !== 'none';
      content.style.display = isOpen ? 'none' : 'block';
      this.classList.toggle('expanded');
    });
  });

  // Theme toggle (dark/light, if needed)
  const themeToggle = document.getElementById('theme-toggle');
  if (themeToggle) {
    const currentTheme = localStorage.getItem('theme') || 'dark';
    document.documentElement.setAttribute('data-theme', currentTheme);
    
    themeToggle.addEventListener('click', function() {
      const theme = document.documentElement.getAttribute('data-theme');
      const newTheme = theme === 'dark' ? 'light' : 'dark';
      document.documentElement.setAttribute('data-theme', newTheme);
      localStorage.setItem('theme', newTheme);
    });
  }

  // Smooth scroll for anchor links
  document.querySelectorAll('a[href^="#"]').forEach(a => {
    a.addEventListener('click', function(e) {
      const href = this.getAttribute('href');
      const target = document.querySelector(href);
      if (target) {
        e.preventDefault();
        target.scrollIntoView({ behavior: 'smooth' });
      }
    });
  });
});
</script>"##
}

/// Generate index.html for default (empty) project
fn generate_default_index(project_name: &str, php_version: &str) -> String {
    let css = generate_css();
    let js = generate_js();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="description" content="CleanServe: Zero-Config PHP Development Server">
  <title>{project_name} - CleanServe</title>
  {css}
</head>
<body>
  <header>
    <nav style="display: flex; justify-content: space-between; align-items: center;">
      <h1 style="margin: 0; font-size: 1.5rem;">🌀 {project_name}</h1>
      <div style="font-size: 0.875rem; color: var(--text-secondary);">PHP {php_version}</div>
    </nav>
  </header>

  <main>
    <section class="p-4">
      <div class="glass-brutal">
        <h2>Welcome to CleanServe</h2>
        <p style="color: var(--text-secondary); margin: 1rem 0;">
          Your zero-configuration PHP development server is running. 
          <strong>Start building!</strong>
        </p>

        <div class="brutal-code mt-4">
<pre>📂 public/
   ├── index.php      ← Your app entry point
   └── assets/        ← CSS, JS, images</pre>
        </div>

        <div class="mt-4">
          <h3>Quick Start</h3>
          <ol style="margin-left: 1.5rem; color: var(--text-secondary);">
            <li>Edit <code style="background: var(--bg-darker); padding: 0.25rem 0.5rem;">public/index.php</code></li>
            <li>Save the file (hot reload enabled)</li>
            <li>Refresh your browser</li>
          </ol>
        </div>

        <div class="mt-4">
          <button class="brutal-btn" data-copy="cleanserve use 8.5">
            Switch PHP Version →
          </button>
        </div>
      </div>
    </section>

    <section class="p-4 mt-4">
      <div class="grid grid-2">
        <div class="brutal-card">
          <h3 style="color: var(--accent);">🔄 Hot Reload</h3>
          <p>Changes to .php files trigger instant reload. CSS updates without page refresh.</p>
        </div>
        <div class="brutal-card">
          <h3 style="color: var(--accent);">🔐 Auto HTTPS</h3>
          <p>Self-signed certificates generated automatically. Secure by default.</p>
        </div>
        <div class="brutal-card">
          <h3 style="color: var(--accent);">📦 Portable PHP</h3>
          <p>Ultra-lightweight PHP binaries (~4-5MB). Switch versions instantly.</p>
        </div>
        <div class="brutal-card">
          <h3 style="color: var(--accent);">🎼 Composer Native</h3>
          <p>Run Composer commands with your project's PHP version automatically.</p>
        </div>
      </div>
    </section>

    <section class="p-4 mt-4">
      <div class="glass-brutal">
        <h3>Next Steps</h3>
        <ul style="margin-left: 1.5rem; color: var(--text-secondary); line-height: 2;">
          <li><a href="https://github.com/LyeZinho/cleanserve">📖 Documentation</a></li>
          <li><a href="https://github.com/LyeZinho/cleanserve/issues">🐛 Report Issues</a></li>
          <li><a href="https://github.com/LyeZinho/cleanserve">⭐ Star on GitHub</a></li>
        </ul>
      </div>
    </section>
  </main>

  <footer>
    <p style="font-size: 0.875rem; color: var(--text-secondary);">
      ⚡ Powered by <strong>CleanServe</strong> • Zero Config, Maximum Speed
    </p>
  </footer>

  {js}
</body>
</html>"#
    )
}

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

pub fn write_default_pages(
    cleanserve_pages_dir: &Path,
    project_name: &str,
    php_version: &str,
) -> anyhow::Result<()> {
    let index = generate_default_index(project_name, php_version);
    std::fs::write(cleanserve_pages_dir.join("index.html"), index)
        .context("Failed to write .cleanserve/pages/index.html")?;

    let not_found = generate_default_404();
    std::fs::write(cleanserve_pages_dir.join("404.html"), not_found)
        .context("Failed to write .cleanserve/pages/404.html")?;

    let error = generate_default_error(project_name, php_version);
    std::fs::write(cleanserve_pages_dir.join("error.html"), error)
        .context("Failed to write .cleanserve/pages/error.html")?;

    let hmr_test = generate_hmr_test_page();
    std::fs::write(cleanserve_pages_dir.join("hmr-test.html"), hmr_test)
        .context("Failed to write .cleanserve/pages/hmr-test.html")?;

    Ok(())
}

/// Generate quickstart index.html with hero, features, quick start, API routes
fn generate_quickstart_index(project_name: &str) -> String {
    let css = generate_css();
    let js = generate_js();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CleanServe - Modern PHP Development</title>
    {css}
</head>
<body>
    <header>
        <nav style="display: flex; justify-content: space-between; align-items: center; max-width: 1200px; margin: 0 auto;">
            <a href="/" style="font-family: 'Space Grotesk', sans-serif; font-weight: 700; font-size: 1.5rem;">CleanServe</a>
            <div style="display: flex; gap: 2rem;">
                <a href="/#features">Features</a>
                <a href="/#quick-start">Quick Start</a>
                <a href="/docs.html">API</a>
                <a href="/about.html">About</a>
            </div>
        </nav>
    </header>

    <main>
        <!-- Hero Section -->
        <section style="background: linear-gradient(135deg, var(--accent) 0%, #059669 100%); color: var(--bg-dark); padding: 4rem 2rem; text-align: center; margin-bottom: 4rem; box-shadow: var(--shadow-brutal);">
            <h1 style="color: var(--bg-dark); margin-bottom: 1rem;">Modern PHP Development</h1>
            <p style="color: var(--bg-dark); font-size: 1.25rem; margin-bottom: 2rem;">Built with CleanServe • Zero Config • Maximum Speed</p>
            <button class="brutal-btn" data-copy="cleanserve use 8.5" style="background: var(--bg-dark); color: var(--accent);">Copy PHP Command</button>
        </section>

        <!-- Features Section -->
        <section id="features">
            <h2 class="text-center" style="margin-bottom: 2rem;">Features</h2>
            <div class="grid grid-3 mb-4">
                <div class="brutal-card">
                    <h3>🔄 Hot Reload</h3>
                    <p>CSS updates without refresh, PHP restarts on save</p>
                </div>
                <div class="brutal-card">
                    <h3>🎼 Composer Ready</h3>
                    <p>Native Composer support with version-specific PHP</p>
                </div>
                <div class="brutal-card">
                    <h3>🗂 Modern Structure</h3>
                    <p>Organized public/, app/, config/ directories</p>
                </div>
                <div class="brutal-card">
                    <h3>🔐 Auto HTTPS</h3>
                    <p>SSL certificates generated automatically</p>
                </div>
                <div class="brutal-card">
                    <h3>📦 Portable</h3>
                    <p>Isolated PHP binaries, no system pollution</p>
                </div>
                <div class="brutal-card">
                    <h3>📝 Example Code</h3>
                    <p>Ready-to-use API examples and patterns</p>
                </div>
            </div>
        </section>

        <!-- Quick Start Section -->
        <section id="quick-start">
            <h2 class="text-center" style="margin-bottom: 2rem;">Quick Start</h2>
            <div style="max-width: 600px; margin: 0 auto;">
                <div style="margin-bottom: 1.5rem;">
                    <button class="brutal-btn" data-expand style="width: 100%; text-align: left; padding: 1rem;">▶ Step 1: Install Dependencies</button>
                    <div style="display: none; background: var(--bg-card); border: 1px solid var(--border); padding: 1rem; margin-top: 0.5rem;">
                        <pre class="brutal-code">composer install</pre>
                    </div>
                </div>
                <div style="margin-bottom: 1.5rem;">
                    <button class="brutal-btn" data-expand style="width: 100%; text-align: left; padding: 1rem;">▶ Step 2: Start the Server</button>
                    <div style="display: none; background: var(--bg-card); border: 1px solid var(--border); padding: 1rem; margin-top: 0.5rem;">
                        <pre class="brutal-code">cleanserve up</pre>
                    </div>
                </div>
                <div style="margin-bottom: 1.5rem;">
                    <button class="brutal-btn" data-expand style="width: 100%; text-align: left; padding: 1rem;">▶ Step 3: View Example API</button>
                    <div style="display: none; background: var(--bg-card); border: 1px solid var(--border); padding: 1rem; margin-top: 0.5rem;">
                        <pre class="brutal-code">curl https://localhost:8080/api/example</pre>
                    </div>
                </div>
            </div>
        </section>

        <!-- API Routes Section -->
        <section style="margin-top: 4rem;">
            <h2 class="text-center" style="margin-bottom: 2rem;">API Routes</h2>
            <div class="grid grid-2">
                <div class="brutal-card">
                    <h3>GET /api/example</h3>
                    <p>Example API endpoint returning JSON</p>
                    <button class="brutal-btn" data-copy="curl -s https://localhost:8080/api/example" style="width: 100%; margin-top: 1rem;">Copy cURL</button>
                </div>
                <div class="brutal-card">
                    <h3>GET /docs</h3>
                    <p>API documentation and examples</p>
                    <button class="brutal-btn" data-copy="open https://localhost:8080/docs.html" style="width: 100%; margin-top: 1rem;">View Docs</button>
                </div>
            </div>
        </section>

        <!-- CTA Section -->
        <section style="text-align: center; margin-top: 4rem; padding: 2rem; background: var(--bg-card); border: 2px solid var(--border); box-shadow: var(--shadow-brutal);">
            <h2>Ready to Build?</h2>
            <p style="margin-bottom: 1.5rem;">Learn more about CleanServe and best practices</p>
            <a href="/about.html" class="brutal-btn">View Documentation</a>
        </section>
    </main>

    <footer>
        <p>&copy; 2026 {} • Built with CleanServe</p>
    </footer>

    {js}
</body>
</html>"#,
        project_name
    )
}

/// Generate quickstart about.html with documentation
fn generate_quickstart_about() -> String {
    let css = generate_css();
    let js = generate_js();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>📖 Documentation - CleanServe</title>
    {css}
</head>
<body>
    <header>
        <div style="max-width: 1200px; margin: 0 auto; display: flex; justify-content: space-between; align-items: center;">
            <h1 style="margin: 0;">📖 Documentation</h1>
            <a href="/" class="brutal-btn">← Back to Home</a>
        </div>
    </header>

    <main>
        <!-- Project Structure -->
        <section>
            <h2>Project Structure</h2>
            <pre class="brutal-code">my-project/
├── public/              # Web root
│   ├── index.html      # Landing page
│   └── api/            # API endpoints
├── app/                # Application code
├── config/             # Configuration files
├── vendor/             # Composer dependencies
├── composer.json       # Project manifest
└── cleanserve.json     # CleanServe config</pre>
        </section>

        <!-- CleanServe Commands -->
        <section>
            <h2>CleanServe Commands</h2>
            <div class="grid grid-2 mb-4">
                <div class="brutal-card">
                    <h3>Start Server</h3>
                    <pre class="brutal-code">cleanserve up</pre>
                    <p style="font-size: 0.9rem; margin-top: 1rem;">Starts the development server on https://localhost:8080</p>
                </div>
                <div class="brutal-card">
                    <h3>Switch PHP Version</h3>
                    <pre class="brutal-code">cleanserve use 8.5</pre>
                    <p style="font-size: 0.9rem; margin-top: 1rem;">Switch to PHP 8.5 instantly</p>
                </div>
                <div class="brutal-card">
                    <h3>List Versions</h3>
                    <pre class="brutal-code">cleanserve list</pre>
                    <p style="font-size: 0.9rem; margin-top: 1rem;">Show available PHP versions</p>
                </div>
                <div class="brutal-card">
                    <h3>View Logs</h3>
                    <pre class="brutal-code">cleanserve logs</pre>
                    <p style="font-size: 0.9rem; margin-top: 1rem;">Display server logs in real-time</p>
                </div>
            </div>
        </section>

        <!-- Getting Help -->
        <section style="padding: 2rem; background: var(--bg-card); border: 2px solid var(--border); margin-top: 2rem;">
            <h2>Getting Help</h2>
            <ul style="list-style: none;">
                <li style="margin-bottom: 1rem;"><a href="https://github.com/LyeZinho/cleanserve">📚 CleanServe GitHub Repository</a></li>
                <li style="margin-bottom: 1rem;"><a href="https://github.com/LyeZinho/cleanserve/issues">🐛 Report Issues</a></li>
                <li style="margin-bottom: 1rem;"><a href="https://github.com/LyeZinho/cleanserve/discussions">💬 Discussions</a></li>
            </ul>
        </section>
    </main>

    <footer>
        <p>&copy; 2026 CleanServe • Modern PHP Development</p>
    </footer>

    {js}
</body>
</html>"#
    )
}

/// Generate quickstart docs.html with API documentation
fn generate_quickstart_docs() -> String {
    let css = generate_css();
    let js = generate_js();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>📡 API Docs - CleanServe</title>
    {css}
</head>
<body>
    <header>
        <div style="max-width: 1200px; margin: 0 auto; display: flex; justify-content: space-between; align-items: center;">
            <h1 style="margin: 0;">📡 API Docs</h1>
            <a href="/" class="brutal-btn">← Back to Home</a>
        </div>
    </header>

    <main>
        <!-- Example Endpoint -->
        <section>
            <h2>Example API Endpoint</h2>
            <div class="brutal-card" style="margin-bottom: 1.5rem;">
                <h3>GET /api/example</h3>
                
                <h4 style="margin-top: 1.5rem;">Request</h4>
                <pre class="brutal-code">curl -s https://localhost:8080/api/example</pre>
                
                <h4 style="margin-top: 1.5rem;">Response</h4>
                <pre class="brutal-code">{{
  "status": "success",
  "message": "Hello from CleanServe!",
  "timestamp": "2026-03-21T12:00:00Z"
}}</pre>

                <h4 style="margin-top: 1.5rem;">Source Code (app/api/example.php)</h4>
                <pre class="brutal-code">&lt;?php
header('Content-Type: application/json');
echo json_encode([
  'status' => 'success',
  'message' => 'Hello from CleanServe!',
  'timestamp' => date('c')
]);</pre>
            </div>
        </section>

        <!-- Add Your Own Endpoints -->
        <section>
            <h2>Add Your Own Endpoints</h2>
            <div class="brutal-card">
                <h3>Creating a New Endpoint</h3>
                <p style="margin-top: 1rem;">Create a new PHP file in <code>app/api/users.php</code></p>
                
                <pre class="brutal-code">&lt;?php
header('Content-Type: application/json');

$users = [
  ['id' => 1, 'name' => 'Alice'],
  ['id' => 2, 'name' => 'Bob']
];

echo json_encode(['data' => $users]);</pre>

                <p style="margin-top: 1rem;">Access it at <code>https://localhost:8080/api/users</code></p>
                <p style="color: var(--accent); margin-top: 1rem;"><strong>💡 Tip:</strong> Changes are reflected instantly thanks to hot reload!</p>
            </div>
        </section>

        <!-- Common Patterns -->
        <section>
            <h2>Common Patterns</h2>
            
            <div style="margin-bottom: 1.5rem;">
                <button class="brutal-btn" data-expand style="width: 100%; text-align: left; padding: 1rem;">▶ JSON Response Pattern</button>
                <div style="display: none; background: var(--bg-card); border: 1px solid var(--border); padding: 1rem; margin-top: 0.5rem;">
                    <pre class="brutal-code">&lt;?php
header('Content-Type: application/json');
$data = ['key' => 'value'];
http_response_code(200);
echo json_encode($data);</pre>
                </div>
            </div>

            <div style="margin-bottom: 1.5rem;">
                <button class="brutal-btn" data-expand style="width: 100%; text-align: left; padding: 1rem;">▶ Error Handling Pattern</button>
                <div style="display: none; background: var(--bg-card); border: 1px solid var(--border); padding: 1rem; margin-top: 0.5rem;">
                    <pre class="brutal-code">&lt;?php
header('Content-Type: application/json');
try {{
  // Your code here
  echo json_encode(['status' => 'ok']);
}} catch (Exception $e) {{
  http_response_code(500);
  echo json_encode(['error' => $e->getMessage()]);
}}</pre>
                </div>
            </div>

            <div style="margin-bottom: 1.5rem;">
                <button class="brutal-btn" data-expand style="width: 100%; text-align: left; padding: 1rem;">▶ CORS Headers Pattern</button>
                <div style="display: none; background: var(--bg-card); border: 1px solid var(--border); padding: 1rem; margin-top: 0.5rem;">
                    <pre class="brutal-code">&lt;?php
header('Access-Control-Allow-Origin: *');
header('Access-Control-Allow-Methods: GET, POST, PUT, DELETE');
header('Access-Control-Allow-Headers: Content-Type');
header('Content-Type: application/json');
echo json_encode(['message' => 'CORS enabled']);</pre>
                </div>
            </div>
        </section>
    </main>

    <footer>
        <p>&copy; 2026 CleanServe • API Documentation</p>
    </footer>

    {js}
</body>
</html>"#
    )
}

/// Generate quickstart api-example.html interactive API tester
fn generate_quickstart_api_example() -> String {
    let css = generate_css();
    let js = generate_js();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>🔌 API Tester - CleanServe</title>
    {css}
    <style>
        .response-container {{
            background: var(--bg-card);
            border: 2px solid var(--border);
            border-radius: 0;
            padding: 1.5rem;
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.9rem;
            max-height: 400px;
            overflow-y: auto;
            white-space: pre-wrap;
            word-wrap: break-word;
            display: none;
            box-shadow: var(--shadow-brutal);
        }}
        .response-container.show {{
            display: block;
        }}
        .error {{
            color: #ef4444;
        }}
        .success {{
            color: var(--accent);
        }}
        .input-group {{
            display: flex;
            gap: 1rem;
            margin-bottom: 1.5rem;
        }}
        .input-group input {{
            flex: 1;
            padding: 0.75rem;
            background: var(--bg-darker);
            border: 1px solid var(--border);
            color: var(--text-primary);
            font-family: 'JetBrains Mono', monospace;
        }}
        .input-group input:focus {{
            outline: none;
            border-color: var(--accent);
        }}
    </style>
</head>
<body>
    <header>
        <div style="max-width: 1200px; margin: 0 auto; display: flex; justify-content: space-between; align-items: center;">
            <h1 style="margin: 0;">🔌 API Tester</h1>
            <a href="/" class="brutal-btn">← Back to Home</a>
        </div>
    </header>

    <main style="max-width: 600px; margin: 0 auto;">
        <div class="brutal-card">
            <h2 style="margin-top: 0;">Test API Endpoints</h2>
            
            <form id="api-form" style="margin-top: 1.5rem;">
                <div class="input-group">
                    <input type="text" id="endpoint" placeholder="/api/example" value="/api/example">
                    <button type="submit" class="brutal-btn" style="width: auto; padding: 0.75rem 1.5rem;">Send Request</button>
                </div>
            </form>

            <div id="response" class="response-container">
                <div id="response-content"></div>
            </div>

            <p style="margin-top: 1.5rem; font-size: 0.9rem; color: var(--text-secondary);">
                💡 <strong>Tip:</strong> Try <code>/api/example</code> first to test the default endpoint.
            </p>
        </div>
    </main>

    <footer>
        <p>&copy; 2026 CleanServe • API Tester</p>
    </footer>

    {js}
    <script>
        document.getElementById('api-form').addEventListener('submit', async (e) => {{
            e.preventDefault();
            const endpoint = document.getElementById('endpoint').value || '/api/example';
            const responseDiv = document.getElementById('response');
            const responseContent = document.getElementById('response-content');
            
            responseDiv.classList.add('show');
            responseContent.textContent = '⏳ Loading...';
            responseContent.className = '';
            
            try {{
                const response = await fetch(`https://localhost:8080${{endpoint}}`);
                const data = await response.text();
                
                if (response.ok) {{
                    try {{
                        const json = JSON.parse(data);
                        responseContent.textContent = JSON.stringify(json, null, 2);
                        responseContent.className = 'success';
                    }} catch {{
                        responseContent.textContent = data;
                        responseContent.className = 'success';
                    }}
                }} else {{
                    responseContent.textContent = `Error ${{response.status}}: ${{data}}`;
                    responseContent.className = 'error';
                }}
            }} catch (error) {{
                responseContent.textContent = `Connection Error: ${{error.message}}\n\nMake sure CleanServe is running on https://localhost:8080`;
                responseContent.className = 'error';
            }}
        }});
    </script>
</body>
</html>"#
    )
}

pub fn write_quickstart_pages(
    cleanserve_pages_dir: &Path,
    project_name: &str,
) -> anyhow::Result<()> {
    // Write index.html
    let index_html = cleanserve_pages_dir.join("index.html");
    std::fs::write(&index_html, generate_quickstart_index(project_name))
        .context("Failed to write .cleanserve/pages/index.html")?;

    // Write about.html
    let about_html = cleanserve_pages_dir.join("about.html");
    std::fs::write(&about_html, generate_quickstart_about())
        .context("Failed to write .cleanserve/pages/about.html")?;

    // Write docs.html
    let docs_html = cleanserve_pages_dir.join("docs.html");
    std::fs::write(&docs_html, generate_quickstart_docs())
        .context("Failed to write .cleanserve/pages/docs.html")?;

    // Write api-example.html (API tester)
    let api_tester_html = cleanserve_pages_dir.join("api-example.html");
    std::fs::write(&api_tester_html, generate_quickstart_api_example())
        .context("Failed to write .cleanserve/pages/api-example.html")?;

    Ok(())
}
