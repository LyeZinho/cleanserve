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

pub fn write_default_pages(public_dir: &Path) -> anyhow::Result<()> {
    let index_php = public_dir.join("index.php");
    std::fs::write(
        &index_php,
        r#"<?php
/**
 * CleanServe - Zero Config PHP Development Server
 * This is your application entry point.
 */

echo "Hello from CleanServe!\n";
phpinfo();
"#,
    )?;

    Ok(())
}

pub fn write_quickstart_pages(public_dir: &Path) -> anyhow::Result<()> {
    let index_php = public_dir.join("index.php");
    std::fs::write(
        &index_php,
        r#"<?php
/**
 * CleanServe - Zero Config PHP Development Server
 * This is your application entry point.
 */

echo "Hello from CleanServe!\n";
phpinfo();
"#,
    )?;

    Ok(())
}
