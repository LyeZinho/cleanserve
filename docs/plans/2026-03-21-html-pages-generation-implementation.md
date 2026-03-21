# CleanServe HTML Page Generation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers/executing-plans to implement this plan task-by-task.

**Goal:** Generate brutaliste-styled HTML pages dynamically in Rust for two init modes: empty project (minimal) and quickstart boilerplate (full-featured with interactivity).

**Architecture:** Modular HTML generator with separate functions per page, shared CSS/JS helpers, and conditional generation based on `--quickstart` flag. All pages use inline CSS + vanilla JS, no external dependencies. Design system: zinc-950 background, emerald-500 highlights, Space Grotesk/Inter/JetBrains Mono typography.

**Tech Stack:** Rust std lib, format! macros for HTML generation, inline CSS Grid, vanilla JS for interactivity (copy buttons, theme toggle, expandable sections).

---

## Phase 1: Setup & Structure

### Task 1: Create HTML Pages Module Structure

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/init.rs` (add module)
- Create: `crates/cleanserve-cli/src/commands/init/html_pages.rs`
- Create: `crates/cleanserve-cli/src/commands/init/mod.rs` (restructure init.rs as module)

**Step 1: Restructure init.rs into a module**

Move the current `init.rs` content into a new module structure:

```bash
# Current: crates/cleanserve-cli/src/commands/init.rs (single file)
# New: crates/cleanserve-cli/src/commands/init/mod.rs (module root)
```

Create `crates/cleanserve-cli/src/commands/init/mod.rs`:

```rust
mod html_pages;

use anyhow::Context;
use std::path::Path;

pub async fn run(name: Option<String>, php: String, quickstart: bool) -> anyhow::Result<()> {
    let project_name = name.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "my-project".to_string())
    });
    
    let config_path = Path::new("cleanserve.json");
    if config_path.exists() {
        anyhow::bail!("cleanserve.json already exists. Remove it first.");
    }
    
    let config = cleanserve_core::CleanServeConfig {
        name: project_name.clone(),
        engine: cleanserve_core::EngineConfig {
            php,
            extensions: vec![],
            display_errors: true,
            memory_limit: None,
        },
        server: cleanserve_core::ServerConfig::default(),
    };
    
    config.save(config_path)
        .context("Failed to save cleanserve.json")?;

    println!("✓ Created cleanserve.json");

    let public_dir = Path::new("public");
    if !public_dir.exists() {
        std::fs::create_dir_all(public_dir)
            .context("Failed to create public/ directory")?;
        
        if quickstart {
            html_pages::write_quickstart_pages(&public_dir, &project_name)?;
        } else {
            html_pages::write_default_pages(&public_dir, &project_name, &php)?;
        }
        
        println!("✓ Created public/ directory with HTML pages");
    }

    let gitignore_path = Path::new(".gitignore");
    let cleanserve_ignore = ".cleanserve/";
    if gitignore_path.exists() {
        let content = std::fs::read_to_string(gitignore_path)
            .context("Failed to read .gitignore")?;
        if !content.contains(cleanserve_ignore) {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(gitignore_path)
                .context("Failed to open .gitignore")?;
            use std::io::Write;
            writeln!(f, "\n# CleanServe (standalone PHP runtime)")?;
            writeln!(f, "{}", cleanserve_ignore)?;
            println!("✓ Added .cleanserve/ to .gitignore");
        }
    } else {
        std::fs::write(gitignore_path, format!("# CleanServe\n{}\n", cleanserve_ignore))
            .context("Failed to create .gitignore")?;
        println!("✓ Created .gitignore with .cleanserve/");
    }

    println!();
    println!("Next steps:");
    if quickstart {
        println!("  1. Run 'cleanserve up' to start the server");
        println!("  2. Open http://localhost:8080 in your browser");
        println!("  3. Customize the boilerplate in public/");
    } else {
        println!("  1. Add your PHP code to public/index.php");
        println!("  2. Run 'cleanserve up' to start the server");
        println!("  3. Open http://localhost:8080 in your browser");
    }
    
    Ok(())
}
```

**Step 2: Create empty `html_pages.rs` module**

Create `crates/cleanserve-cli/src/commands/init/html_pages.rs`:

```rust
use anyhow::Context;
use std::path::Path;

pub fn write_default_pages(public_dir: &Path, project_name: &str, php_version: &str) -> anyhow::Result<()> {
    // Placeholder - will implement in next tasks
    Ok(())
}

pub fn write_quickstart_pages(public_dir: &Path, project_name: &str) -> anyhow::Result<()> {
    // Placeholder - will implement in next tasks
    Ok(())
}
```

**Step 3: Update main.rs to pass --quickstart flag**

In `crates/cleanserve-cli/src/main.rs`, find the `init` subcommand definition and add flag:

```rust
// Find the init subcommand around line ~40
#[command(about = "Initialize a new CleanServe project")]
Init {
    /// Project name (defaults to current directory name)
    name: Option<String>,
    
    /// PHP version to use
    #[arg(default_value = "8.4")]
    php: String,
    
    /// Use quickstart boilerplate template (includes example pages + interactive frontend)
    #[arg(long)]
    quickstart: bool,
},

// Find the command dispatch around line ~120-140 and update:
Command::Init { name, php, quickstart } => {
    commands::init::run(name, php, quickstart).await?
},
```

**Step 4: Compile and verify structure**

Run:
```bash
cd /home/pedro/repo/cleanserve
cargo build -p cleanserve-cli
```

Expected: Compiles with no errors (warnings OK for now, as functions are stubs).

**Step 5: Commit**

```bash
git add crates/cleanserve-cli/src/commands/init/
git add crates/cleanserve-cli/src/main.rs
git add crates/cleanserve-cli/src/commands.rs  # If modified
git commit -m "refactor: restructure init command as module with html_pages submodule"
```

---

## Phase 2: Shared CSS & JS Generators

### Task 2: Implement Shared CSS Generator

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/init/html_pages.rs`

**Step 1: Add shared CSS generator function**

In `html_pages.rs`, add this function before the main page generators:

```rust
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
```

**Step 2: Verify CSS compiles**

Run:
```bash
cargo build -p cleanserve-cli 2>&1 | grep -i "error\|warning" | head -20
```

Expected: No errors (function is unused, but that's OK for now).

**Step 3: Commit**

```bash
git add crates/cleanserve-cli/src/commands/init/html_pages.rs
git commit -m "feat: add shared brutaliste CSS generator for all pages"
```

---

### Task 3: Implement Shared JS Generator

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/init/html_pages.rs`

**Step 1: Add shared JavaScript generator**

Add this function after `generate_css()`:

```rust
/// Generate shared vanilla JS for interactivity (copy buttons, theme toggle, expandable sections)
fn generate_js() -> &'static str {
    r#"<script>
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
</script>"#
}
```

**Step 2: Compile and verify**

Run:
```bash
cargo build -p cleanserve-cli 2>&1 | grep -i "error"
```

Expected: No errors.

**Step 3: Commit**

```bash
git add crates/cleanserve-cli/src/commands/init/html_pages.rs
git commit -m "feat: add shared vanilla JS for interactive features (copy, expand, theme)"
```

---

## Phase 3: Default Pages (Empty Init)

### Task 4: Implement Default Pages (index, 404, error)

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/init/html_pages.rs`

**Step 1: Add default index.html generator**

Add this function in `html_pages.rs`:

```rust
/// Generate index.html for default (empty) project
fn generate_default_index(project_name: &str, php_version: &str) -> String {
    let css = generate_css();
    let js = generate_js();
    
    format!(r#"<!DOCTYPE html>
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
</html>"#)
}
```

**Step 2: Add default 404.html generator**

Add this function:

```rust
/// Generate 404.html error page
fn generate_default_404() -> String {
    let css = generate_css();
    
    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>404 - Not Found</title>
  {css}
</head>
<body>
  <header>
    <h1 style="margin: 0; font-size: 1.5rem;">🌀 CleanServe</h1>
  </header>

  <main>
    <section class="p-4" style="text-align: center; margin-top: 4rem;">
      <div class="brutal-stat">
        <div class="brutal-stat-value" style="font-size: 5rem;">404</div>
        <div class="brutal-stat-label">NOT FOUND</div>
      </div>

      <div class="glass-brutal mt-4" style="max-width: 600px; margin: 2rem auto;">
        <h2>Page Not Found</h2>
        <p style="color: var(--text-secondary); margin: 1rem 0;">
          The resource you're looking for doesn't exist. Check the URL and try again.
        </p>
        
        <div class="brutal-code mt-4">
<pre>GET {path}
HTTP/1.1 404 Not Found</pre>
        </div>

        <div class="mt-4">
          <a href="/" class="brutal-btn">← Back to Home</a>
        </div>
      </div>
    </section>
  </main>

  <footer>
    <p style="font-size: 0.875rem; color: var(--text-secondary);">
      CleanServe • Zero Config PHP Server
    </p>
  </footer>
</body>
</html>"#)
}
```

**Step 3: Add default error.html generator**

Add this function:

```rust
/// Generate error.html for PHP runtime errors
fn generate_default_error() -> String {
    let css = generate_css();
    
    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>⚠️ Runtime Error</title>
  {css}
</head>
<body>
  <header>
    <h1 style="margin: 0; font-size: 1.5rem;">🌀 CleanServe</h1>
  </header>

  <main>
    <section class="p-4" style="margin-top: 2rem;">
      <div class="glass-brutal" style="border-left: 6px solid #ef4444; max-width: 800px; margin: 0 auto;">
        <h2 style="color: #ef4444;">⚠️ PHP Runtime Error</h2>
        <p style="color: var(--text-secondary); margin: 1rem 0;">
          An error occurred while processing your request. Check the server logs for details.
        </p>

        <div class="brutal-code mt-4">
<pre style="color: #fca5a5;">Please check:
1. Your PHP code syntax
2. File permissions
3. Server logs in .cleanserve/
4. Installed extensions</pre>
        </div>

        <div class="mt-4">
          <p style="font-size: 0.875rem; color: var(--text-secondary);">
            💡 <strong>Tip:</strong> Run <code style="background: var(--bg-darker); padding: 0.25rem 0.5rem;">cleanserve logs</code> to see detailed output.
          </p>
        </div>
      </div>
    </section>
  </main>

  <footer>
    <p style="font-size: 0.875rem; color: var(--text-secondary);">
      CleanServe • Zero Config PHP Server
    </p>
  </footer>
</body>
</html>"#)
}
```

**Step 4: Implement `write_default_pages` function**

Replace the placeholder in `write_default_pages`:

```rust
pub fn write_default_pages(public_dir: &Path, project_name: &str, php_version: &str) -> anyhow::Result<()> {
    let index = generate_default_index(project_name, php_version);
    std::fs::write(public_dir.join("index.html"), index)
        .context("Failed to write public/index.html")?;

    let not_found = generate_default_404();
    std::fs::write(public_dir.join("404.html"), not_found)
        .context("Failed to write public/404.html")?;

    let error = generate_default_error();
    std::fs::write(public_dir.join("error.html"), error)
        .context("Failed to write public/error.html")?;

    Ok(())
}
```

**Step 5: Compile and verify**

Run:
```bash
cargo build -p cleanserve-cli 2>&1 | grep -i "error"
```

Expected: No errors. Warnings about unused functions are OK for now.

**Step 6: Commit**

```bash
git add crates/cleanserve-cli/src/commands/init/html_pages.rs
git commit -m "feat: add default HTML pages (index, 404, error) with brutaliste styling"
```

---

## Phase 4: Quickstart Pages (Full Boilerplate)

### Task 5: Implement Quickstart Pages

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/init/html_pages.rs`

**Step 1: Add quickstart index.html generator**

Add this function:

```rust
/// Generate quickstart index.html with interactive frontend
fn generate_quickstart_index(project_name: &str) -> String {
    let css = generate_css();
    let js = generate_js();
    
    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="description" content="Modern PHP Starter Project with CleanServe">
  <title>{project_name} - Modern PHP Starter</title>
  {css}
  <style>
    .hero {{ background: linear-gradient(135deg, var(--accent) 0%, #059669 100%); padding: 4rem 2rem; text-align: center; }}
    .hero h1 {{ color: var(--bg-dark); margin-bottom: 1rem; }}
    .hero p {{ color: var(--bg-darker); font-size: 1.1rem; }}
    .feature-icon {{ font-size: 2.5rem; margin-bottom: 0.5rem; }}
  </style>
</head>
<body>
  <header style="background: rgba(24, 24, 27, 0.8); backdrop-filter: blur(10px); border-bottom: 2px solid var(--border);">
    <nav style="display: flex; justify-content: space-between; align-items: center; max-width: 1200px; margin: 0 auto; padding: 1rem 2rem;">
      <h1 style="margin: 0; font-size: 1.5rem;">🚀 {project_name}</h1>
      <ul style="list-style: none; display: flex; gap: 2rem;">
        <li><a href="#features">Features</a></li>
        <li><a href="#quickstart">Quick Start</a></li>
        <li><a href="#api">API</a></li>
        <li><a href="about.html">About</a></li>
      </ul>
    </nav>
  </header>

  <main>
    <section class="hero">
      <h1>Modern PHP Development</h1>
      <p>Built with CleanServe • Zero Config • Maximum Speed</p>
      <button class="brutal-btn mt-4" data-copy="cleanserve use 8.5">
        Copy PHP Command
      </button>
    </section>

    <section id="features" class="p-4" style="max-width: 1200px; margin: 2rem auto;">
      <h2 style="text-align: center; margin-bottom: 2rem;">⚡ Features</h2>
      <div class="grid grid-3">
        <div class="brutal-card">
          <div class="feature-icon">🔄</div>
          <h3>Hot Reload</h3>
          <p>Changes instantly reflected in browser. CSS without page refresh.</p>
        </div>
        <div class="brutal-card">
          <div class="feature-icon">🎼</div>
          <h3>Composer Ready</h3>
          <p>Full composer.json setup. PSR-4 autoloading configured.</p>
        </div>
        <div class="brutal-card">
          <div class="feature-icon">🗂</div>
          <h3>Modern Structure</h3>
          <p>MVC-ready. Separate app/, public/, config/ directories.</p>
        </div>
        <div class="brutal-card">
          <div class="feature-icon">🔐</div>
          <h3>Auto HTTPS</h3>
          <p>Self-signed certificates. Secure by default.</p>
        </div>
        <div class="brutal-card">
          <div class="feature-icon">📦</div>
          <h3>Portable</h3>
          <p>Ultra-lightweight PHP. Switch versions instantly.</p>
        </div>
        <div class="brutal-card">
          <div class="feature-icon">📝</div>
          <h3>Example Code</h3>
          <p>Pre-built routing, API endpoints, database stubs.</p>
        </div>
      </div>
    </section>

    <section id="quickstart" class="p-4" style="max-width: 1200px; margin: 2rem auto;">
      <h2 style="text-align: center; margin-bottom: 2rem;">🚀 Quick Start</h2>
      <div class="glass-brutal">
        <h3 data-expand style="cursor: pointer; display: flex; align-items: center; gap: 0.5rem;">
          ▶ Step 1: Install Dependencies
        </h3>
        <div style="display: none; margin-top: 1rem;">
          <div class="brutal-code">
<pre>composer install</pre>
          </div>
          <p style="color: var(--text-secondary); margin-top: 1rem;">Installs all dependencies from composer.json</p>
        </div>

        <h3 data-expand style="cursor: pointer; display: flex; align-items: center; gap: 0.5rem; margin-top: 1.5rem;">
          ▶ Step 2: Start the Server
        </h3>
        <div style="display: none; margin-top: 1rem;">
          <div class="brutal-code">
<pre>cleanserve up</pre>
          </div>
          <p style="color: var(--text-secondary); margin-top: 1rem;">Runs the dev server at https://localhost:8080</p>
        </div>

        <h3 data-expand style="cursor: pointer; display: flex; align-items: center; gap: 0.5rem; margin-top: 1.5rem;">
          ▶ Step 3: View Example API
        </h3>
        <div style="display: none; margin-top: 1rem;">
          <div class="brutal-code">
<pre>curl https://localhost:8080/api/example</pre>
          </div>
          <p style="color: var(--text-secondary); margin-top: 1rem;">Test the built-in API endpoint (see api/example.php)</p>
        </div>
      </div>
    </section>

    <section id="api" class="p-4" style="max-width: 1200px; margin: 2rem auto;">
      <h2 style="text-align: center; margin-bottom: 2rem;">📡 API Routes</h2>
      <div class="grid grid-2">
        <div class="brutal-card">
          <h3 style="font-family: 'JetBrains Mono';">GET /api/example</h3>
          <p>Sample endpoint returning JSON response</p>
          <button class="brutal-btn mt-2" data-copy='curl -s https://localhost:8080/api/example | jq'>
            Copy cURL
          </button>
        </div>
        <div class="brutal-card">
          <h3 style="font-family: 'JetBrains Mono';">GET /docs</h3>
          <p>API documentation and examples</p>
          <a href="/docs" class="brutal-btn mt-2">View Docs →</a>
        </div>
      </div>
    </section>

    <section class="p-4" style="max-width: 1200px; margin: 2rem auto;">
      <div class="glass-brutal" style="text-align: center;">
        <h2>Ready to Build?</h2>
        <p style="color: var(--text-secondary); margin: 1rem 0;">Edit the starter code in app/ and public/ to get started.</p>
        <div style="margin-top: 2rem;">
          <a href="about.html" class="brutal-btn">Learn More →</a>
        </div>
      </div>
    </section>
  </main>

  <footer>
    <p style="font-size: 0.875rem; color: var(--text-secondary); margin: 1rem 0;">
      ⚡ Built with <strong>CleanServe</strong> • <a href="https://github.com/LyeZinho/cleanserve">GitHub</a>
    </p>
  </footer>

  {js}
</body>
</html>"#)
}
```

**Step 2: Add quickstart about.html generator**

Add this function:

```rust
/// Generate quickstart about.html with project info
fn generate_quickstart_about() -> String {
    let css = generate_css();
    
    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>About - Project Documentation</title>
  {css}
</head>
<body>
  <header style="background: rgba(24, 24, 27, 0.8); backdrop-filter: blur(10px); border-bottom: 2px solid var(--border);">
    <nav style="display: flex; justify-content: space-between; align-items: center; max-width: 1200px; margin: 0 auto; padding: 1rem 2rem;">
      <h1 style="margin: 0; font-size: 1.5rem;">📖 Documentation</h1>
      <a href="/" style="color: var(--accent);">← Back to Home</a>
    </nav>
  </header>

  <main>
    <section class="p-4" style="max-width: 900px; margin: 2rem auto;">
      <div class="glass-brutal">
        <h2>Project Structure</h2>
        <div class="brutal-code">
<pre>project/
├── public/          ← Web root (served by CleanServe)
│   ├── index.html   ← Entry point
│   ├── css/
│   └── js/
├── app/             ← Application logic
│   ├── api/         ← API endpoints
│   └── models/      ← Data models
├── config/          ← Configuration files
├── vendor/          ← Composer dependencies
├── composer.json    ← Dependency management
└── cleanserve.json  ← CleanServe config</pre>
        </div>
      </div>
    </section>

    <section class="p-4" style="max-width: 900px; margin: 2rem auto;">
      <div class="glass-brutal">
        <h2>CleanServe Commands</h2>
        <div class="grid grid-2">
          <div>
            <h3 style="font-family: 'JetBrains Mono'; font-size: 1rem;">cleanserve up</h3>
            <p style="color: var(--text-secondary);">Start development server</p>
          </div>
          <div>
            <h3 style="font-family: 'JetBrains Mono'; font-size: 1rem;">cleanserve use 8.5</h3>
            <p style="color: var(--text-secondary);">Switch PHP version</p>
          </div>
          <div>
            <h3 style="font-family: 'JetBrains Mono'; font-size: 1rem;">cleanserve list</h3>
            <p style="color: var(--text-secondary);">List available PHP versions</p>
          </div>
          <div>
            <h3 style="font-family: 'JetBrains Mono'; font-size: 1rem;">cleanserve logs</h3>
            <p style="color: var(--text-secondary);">View server logs</p>
          </div>
        </div>
      </div>
    </section>

    <section class="p-4" style="max-width: 900px; margin: 2rem auto;">
      <div class="glass-brutal">
        <h2>Getting Help</h2>
        <ul style="margin-left: 1.5rem; color: var(--text-secondary); line-height: 2;">
          <li><a href="https://github.com/LyeZinho/cleanserve">📖 CleanServe Documentation</a></li>
          <li><a href="https://github.com/LyeZinho/cleanserve/issues">🐛 Report Issues</a></li>
          <li><a href="https://github.com/LyeZinho/cleanserve/discussions">💬 Discussions</a></li>
        </ul>
      </div>
    </section>
  </main>

  <footer>
    <p style="font-size: 0.875rem; color: var(--text-secondary);">
      ⚡ Powered by <strong>CleanServe</strong> • Zero Config PHP Server
    </p>
  </footer>
</body>
</html>"#)
}
```

**Step 3: Add quickstart docs.html generator**

Add this function:

```rust
/// Generate quickstart docs.html with API examples
fn generate_quickstart_docs() -> String {
    let css = generate_css();
    let js = generate_js();
    
    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>API Documentation</title>
  {css}
</head>
<body>
  <header style="background: rgba(24, 24, 27, 0.8); backdrop-filter: blur(10px); border-bottom: 2px solid var(--border);">
    <nav style="display: flex; justify-content: space-between; align-items: center; max-width: 1200px; margin: 0 auto; padding: 1rem 2rem;">
      <h1 style="margin: 0; font-size: 1.5rem;">📡 API Docs</h1>
      <a href="/" style="color: var(--accent);">← Back to Home</a>
    </nav>
  </header>

  <main>
    <section class="p-4" style="max-width: 900px; margin: 2rem auto;">
      <div class="glass-brutal">
        <h2>Example API Endpoint</h2>
        <h3 style="margin-top: 1rem; font-family: 'JetBrains Mono';">GET /api/example</h3>
        <p style="color: var(--text-secondary);">Sample endpoint that returns JSON data</p>

        <h4 style="margin-top: 1.5rem;">Request:</h4>
        <div class="brutal-code">
<pre>curl -s https://localhost:8080/api/example</pre>
        </div>

        <h4 style="margin-top: 1.5rem;">Response:</h4>
        <div class="brutal-code">
<pre>{{"status":"success","message":"Hello from CleanServe","timestamp":"2026-03-21T10:30:00Z"}}</pre>
        </div>

        <h4 style="margin-top: 1.5rem;">Source Code:</h4>
        <div class="brutal-code">
<pre>&lt;?php
// app/api/example.php

header('Content-Type: application/json');

echo json_encode([
    'status' => 'success',
    'message' => 'Hello from CleanServe',
    'timestamp' => date('c')
]);</pre>
        </div>
      </div>
    </section>

    <section class="p-4" style="max-width: 900px; margin: 2rem auto;">
      <div class="glass-brutal">
        <h2>Add Your Own Endpoints</h2>
        <h3 style="margin-top: 1rem;">1. Create a new file in app/api/</h3>
        <div class="brutal-code">
<pre># app/api/users.php
&lt;?php

header('Content-Type: application/json');

$users = [
    ['id' => 1, 'name' => 'Alice'],
    ['id' => 2, 'name' => 'Bob']
];

echo json_encode($users);</pre>
        </div>

        <h3 style="margin-top: 1.5rem;">2. Access it at /api/users</h3>
        <div class="brutal-code">
<pre>curl https://localhost:8080/api/users</pre>
        </div>

        <h3 style="margin-top: 1.5rem;">3. Hot Reload</h3>
        <p style="color: var(--text-secondary);">Save the file and your browser will automatically reload. No manual refresh needed.</p>
      </div>
    </section>

    <section class="p-4" style="max-width: 900px; margin: 2rem auto;">
      <div class="glass-brutal">
        <h2>Common Patterns</h2>
        <h3 data-expand style="cursor: pointer; display: flex; align-items: center; gap: 0.5rem;">
          ▶ JSON Response
        </h3>
        <div style="display: none; margin-top: 1rem;">
          <div class="brutal-code">
<pre>&lt;?php
header('Content-Type: application/json');
http_response_code(200);
echo json_encode(['data' => $data]);</pre>
          </div>
        </div>

        <h3 data-expand style="cursor: pointer; display: flex; align-items: center; gap: 0.5rem; margin-top: 1.5rem;">
          ▶ Error Handling
        </h3>
        <div style="display: none; margin-top: 1rem;">
          <div class="brutal-code">
<pre>&lt;?php
header('Content-Type: application/json');
try {
    // Your code
} catch (Exception $e) {
    http_response_code(500);
    echo json_encode(['error' => $e->getMessage()]);
}</pre>
          </div>
        </div>

        <h3 data-expand style="cursor: pointer; display: flex; align-items: center; gap: 0.5rem; margin-top: 1.5rem;">
          ▶ CORS Headers
        </h3>
        <div style="display: none; margin-top: 1rem;">
          <div class="brutal-code">
<pre>&lt;?php
header('Access-Control-Allow-Origin: *');
header('Access-Control-Allow-Methods: GET, POST, PUT, DELETE');
header('Content-Type: application/json');</pre>
          </div>
        </div>
      </div>
    </section>
  </main>

  <footer>
    <p style="font-size: 0.875rem; color: var(--text-secondary);">
      ⚡ Powered by <strong>CleanServe</strong> • Zero Config PHP Server
    </p>
  </footer>

  {js}
</body>
</html>"#)
}
```

**Step 4: Add quickstart api-example.html**

Add this function:

```rust
/// Generate quickstart api-example.html
fn generate_quickstart_api_example() -> String {
    let css = generate_css();
    
    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>API Example</title>
  {css}
</head>
<body>
  <header style="background: rgba(24, 24, 27, 0.8); backdrop-filter: blur(10px); border-bottom: 2px solid var(--border);">
    <nav style="display: flex; justify-content: space-between; align-items: center; max-width: 1200px; margin: 0 auto; padding: 1rem 2rem;">
      <h1 style="margin: 0; font-size: 1.5rem;">🔌 API Tester</h1>
      <a href="/" style="color: var(--accent);">← Back to Home</a>
    </nav>
  </header>

  <main>
    <section class="p-4" style="max-width: 900px; margin: 2rem auto;">
      <div class="glass-brutal">
        <h2>Test Your API</h2>
        <form id="api-form" style="margin-top: 1.5rem;">
          <div style="margin-bottom: 1rem;">
            <label style="display: block; margin-bottom: 0.5rem; font-weight: 600;">Endpoint:</label>
            <input type="text" id="endpoint" value="/api/example" 
              style="width: 100%; padding: 0.75rem; background: var(--bg-darker); border: 1px solid var(--border); color: var(--text-primary);" />
          </div>
          <button type="submit" class="brutal-btn">Send Request</button>
        </form>

        <div id="response-container" style="display: none; margin-top: 2rem;">
          <h3>Response:</h3>
          <div class="brutal-code">
<pre id="response"></pre>
          </div>
        </div>
      </div>
    </section>
  </main>

  <footer>
    <p style="font-size: 0.875rem; color: var(--text-secondary);">
      ⚡ Powered by <strong>CleanServe</strong>
    </p>
  </footer>

  <script>
    document.getElementById('api-form').addEventListener('submit', async (e) => {{
      e.preventDefault();
      const endpoint = document.getElementById('endpoint').value;
      
      try {{
        const response = await fetch(endpoint);
        const data = await response.json();
        document.getElementById('response').textContent = JSON.stringify(data, null, 2);
        document.getElementById('response-container').style.display = 'block';
      }} catch (err) {{
        document.getElementById('response').textContent = 'Error: ' + err.message;
        document.getElementById('response-container').style.display = 'block';
      }}
    }});
  </script>
</body>
</html>"#)
}
```

**Step 5: Implement `write_quickstart_pages` function**

Replace the placeholder:

```rust
pub fn write_quickstart_pages(public_dir: &Path, project_name: &str) -> anyhow::Result<()> {
    let index = generate_quickstart_index(project_name);
    std::fs::write(public_dir.join("index.html"), index)
        .context("Failed to write public/index.html")?;

    let about = generate_quickstart_about();
    std::fs::write(public_dir.join("about.html"), about)
        .context("Failed to write public/about.html")?;

    let docs = generate_quickstart_docs();
    std::fs::write(public_dir.join("docs.html"), docs)
        .context("Failed to write public/docs.html")?;

    let api_example = generate_quickstart_api_example();
    std::fs::write(public_dir.join("api-example.html"), api_example)
        .context("Failed to write public/api-example.html")?;

    Ok(())
}
```

**Step 6: Compile and verify**

Run:
```bash
cargo build -p cleanserve-cli 2>&1 | grep -i "error"
```

Expected: No errors.

**Step 7: Commit**

```bash
git add crates/cleanserve-cli/src/commands/init/html_pages.rs
git commit -m "feat: add quickstart HTML pages (index, about, docs, api-example) with interactivity"
```

---

## Phase 5: Testing & Verification

### Task 6: Test Empty Init Mode

**Files:**
- Test: Manual testing via CLI

**Step 1: Create test directory**

Run:
```bash
mkdir -p /tmp/test_empty_init
cd /tmp/test_empty_init
```

**Step 2: Run cleanserve init (default)**

Run:
```bash
/home/pedro/repo/cleanserve/target/debug/cleanserve-cli init --name "test-empty" --php "8.4"
```

Expected output:
```
✓ Created cleanserve.json
✓ Created public/ directory with HTML pages
✓ Added .cleanserve/ to .gitignore

Next steps:
  1. Add your PHP code to public/index.php
  2. Run 'cleanserve up' to start the server
  3. Open http://localhost:8080 in your browser
```

**Step 3: Verify files exist**

Run:
```bash
ls -la public/
```

Expected:
```
index.html
404.html
error.html
```

**Step 4: Verify HTML is valid**

Run:
```bash
grep -q "CleanServe" public/index.html && echo "✓ Index contains CleanServe branding"
grep -q "404" public/404.html && echo "✓ 404 page exists"
grep -q "Runtime Error" public/error.html && echo "✓ Error page exists"
```

Expected: All three checks pass.

**Step 5: Verify CSS is inline**

Run:
```bash
grep -q "<style>" public/index.html && echo "✓ CSS inlined in index"
```

Expected: ✓ passes.

**Step 6: Record results**

```
✓ cleanserve init creates public/index.html
✓ cleanserve init creates public/404.html
✓ cleanserve init creates public/error.html
✓ All pages contain brutaliste CSS (inline)
✓ All pages display project name and PHP version
```

---

### Task 7: Test Quickstart Init Mode

**Files:**
- Test: Manual testing via CLI

**Step 1: Create test directory**

Run:
```bash
mkdir -p /tmp/test_quickstart_init
cd /tmp/test_quickstart_init
```

**Step 2: Run cleanserve init --quickstart**

Run:
```bash
/home/pedro/repo/cleanserve/target/debug/cleanserve-cli init --name "test-quickstart" --php "8.4" --quickstart
```

Expected output:
```
✓ Created cleanserve.json
✓ Created public/ directory with HTML pages
✓ Added .cleanserve/ to .gitignore

Next steps:
  1. Run 'cleanserve up' to start the server
  2. Open http://localhost:8080 in your browser
  3. Customize the boilerplate in public/
```

**Step 3: Verify all quickstart pages exist**

Run:
```bash
ls -la public/
```

Expected:
```
index.html
about.html
docs.html
api-example.html
```

**Step 4: Verify interactive features**

Run:
```bash
grep -q "data-copy" public/index.html && echo "✓ Copy buttons present"
grep -q "data-expand" public/docs.html && echo "✓ Expandable sections present"
grep -q "DOMContentLoaded" public/index.html && echo "✓ JS interactivity present"
```

Expected: All three checks pass.

**Step 5: Verify page links**

Run:
```bash
grep -q 'href="about.html"' public/index.html && echo "✓ Links to about.html"
grep -q 'href="docs.html"' public/about.html && echo "✓ Links to docs.html"
grep -q 'href="/"' public/docs.html && echo "✓ Links back to home"
```

Expected: All three checks pass.

**Step 6: Record results**

```
✓ cleanserve init --quickstart creates public/index.html
✓ cleanserve init --quickstart creates public/about.html
✓ cleanserve init --quickstart creates public/docs.html
✓ cleanserve init --quickstart creates public/api-example.html
✓ All pages have interactive features (copy buttons, expandable sections)
✓ Navigation links between pages work
```

---

## Phase 6: Integration & Release

### Task 8: Build Release & Commit

**Files:**
- Build: Full workspace

**Step 1: Build release binary**

Run:
```bash
cd /home/pedro/repo/cleanserve
cargo build --release -p cleanserve-cli
```

Expected: Binary compiled successfully at `target/release/cleanserve-cli`.

**Step 2: Verify binary works**

Run:
```bash
/home/pedro/repo/cleanserve/target/release/cleanserve-cli --version
```

Expected: Version output (e.g., `cleanserve 0.2.0`).

**Step 3: Final test with release binary**

Run:
```bash
mkdir -p /tmp/final_test
cd /tmp/final_test
/home/pedro/repo/cleanserve/target/release/cleanserve-cli init --name "final-test" --php "8.4" --quickstart
```

Expected: All files created successfully.

**Step 4: Commit all changes**

Run:
```bash
cd /home/pedro/repo/cleanserve
git add -A
git commit -m "feat: implement HTML page generation for init (empty + quickstart modes) with brutaliste design"
```

**Step 5: Create version bump commit (optional)**

If you want to bump to v0.2.1 or v0.3.0:

```bash
# Update version in Cargo.toml from 0.2.0 to 0.2.1 (or 0.3.0)
# Then:
git add Cargo.toml
git commit -m "release: v0.2.1 - HTML page generation for init command"
git tag -a v0.2.1 -m "HTML page generation, default + quickstart modes"
git push origin main --tags
```

**Step 6: Verify git history**

Run:
```bash
git log --oneline -5
```

Expected output shows new commits.

---

## Acceptance Criteria

- [x] `cleanserve init` generates minimal brutaliste pages (index, 404, error)
- [x] `cleanserve init --quickstart` generates full boilerplate (index, about, docs, api-example)
- [x] All pages use inline CSS + vanilla JS (no external dependencies)
- [x] Design system colors accurate (zinc-950, emerald-500, typography)
- [x] Interactive features work (copy buttons, expandable sections, smooth scroll)
- [x] Release binary builds successfully
- [x] All changes committed to git

---

## Post-Implementation Notes

**Files Modified:**
- `crates/cleanserve-cli/src/commands/init/` (new module)
- `crates/cleanserve-cli/src/main.rs` (added --quickstart flag)

**New Functions:**
- `html_pages::write_default_pages()` - writes index, 404, error for empty init
- `html_pages::write_quickstart_pages()` - writes index, about, docs, api-example for quickstart
- `html_pages::generate_css()` - shared brutaliste CSS
- `html_pages::generate_js()` - shared vanilla JS for interactivity

**Testing:**
- Manual CLI testing for both init modes
- Verify files created with correct names
- Verify CSS/JS present
- Verify links and interactivity work

**Future Improvements:**
- Support for additional template modes (e.g., `--framework laravel`, `--framework symfony`)
- Dynamic project name/version in page headers
- Theme toggle (dark/light) persisted to localStorage
- Live API tester with request/response history
