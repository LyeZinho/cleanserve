//! Magic Error Overlay
//!
//! Beautiful error pages with VS Code / PHPStorm deep links for development.
//! Dev-only feature for enhanced debugging experience.

use serde::{Deserialize, Serialize};

/// PHP Error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhpError {
    /// Error type (Fatal, Warning, Notice, etc.)
    pub error_type: String,
    /// Error message
    pub message: String,
    /// File where error occurred
    pub file: String,
    /// Line number
    pub line: u32,
    /// Stack trace
    pub stack_trace: Vec<StackFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub file: String,
    pub line: u32,
    pub function: String,
    /// Whether this frame is in application code
    pub is_app_code: bool,
}

/// Parse PHP stderr output into structured error
pub fn parse_php_error(stderr: &str) -> Option<PhpError> {
    // Parse PHP error format: "PHP Fatal error: ..."
    for line in stderr.lines() {
        if line.starts_with("PHP ")
            || line.starts_with("Fatal error:")
            || line.starts_with("Parse error:")
        {
            return Some(parse_error_line(line));
        }
    }
    None
}

fn parse_error_line(line: &str) -> PhpError {
    let line = line.trim_start_matches("PHP ").trim();

    let error_types = [
        "Fatal error",
        "Parse error",
        "Warning",
        "Notice",
        "Deprecated",
        "Catchable fatal error",
    ];

    let (error_type, after_type) = error_types
        .iter()
        .find_map(|t| {
            if line.starts_with(t) {
                Some((*t, &line[t.len()..]))
            } else {
                None
            }
        })
        .unwrap_or(("Error", &line["Error".len()..]));

    let after_type = after_type.trim_start_matches(":").trim();

    let (file, line_num, message) = if let Some(pos) = after_type.rfind(" on line ") {
        let location_part = &after_type[..pos];
        let line_num_str = &after_type[pos + 9..];
        let line_num: u32 = line_num_str.parse().unwrap_or(0);

        if let Some(in_pos) = location_part.rfind(" in ") {
            let file = location_part[in_pos + 4..].to_string();
            let msg = location_part[..in_pos].to_string();
            (file, line_num, msg)
        } else {
            (location_part.to_string(), line_num, after_type.to_string())
        }
    } else {
        (String::new(), 0, after_type.to_string())
    };

    PhpError {
        error_type: normalize_error_type(error_type),
        message,
        file,
        line: line_num,
        stack_trace: Vec::new(),
    }
}

fn normalize_error_type(t: &str) -> String {
    match t.to_lowercase().as_str() {
        "fatal error" => "Fatal".to_string(),
        "parse error" => "Parse".to_string(),
        "warning" => "Warning".to_string(),
        "notice" => "Notice".to_string(),
        "deprecated" => "Deprecated".to_string(),
        "catchable fatal error" => "Catchable".to_string(),
        _ => t.to_string(),
    }
}

/// Generate VS Code deep link
/// Format: vscode://file/{absolute_path}:{line}:{column}
pub fn vscode_link(file: &str, line: u32, column: u32) -> String {
    let path = file.replace('\\', "/");
    format!("vscode://file/{}:{}:{}", path, line, column)
}

/// Generate JetBrains IDE deep link (PHPStorm, WebStorm, etc.)
/// Uses the modern jetbrains:// URL scheme via JetBrains Toolbox
///
/// Format: jetbrains://{tool}/navigate/reference?project={name}&path={file}:{line}:{col}
pub fn jetbrains_link(file: &str, line: u32, column: u32, project_name: &str) -> String {
    let path = file.replace('\\', "/");
    format!(
        "jetbrains://php-storm/navigate/reference?project={}&path={}:{}:{}",
        urlencoding_encode(project_name),
        urlencoding_encode(&path),
        line,
        column
    )
}

/// Generate PHPStorm-specific deep link (legacy format for older IDEs)
/// Falls back to legacy phpstorm:// format for IDEs without Toolbox
pub fn phpstorm_link_legacy(file: &str, line: u32) -> String {
    let path = file.replace('\\', "/");
    format!(
        "phpstorm://open?file={}&line={}",
        urlencoding_encode(&path),
        line
    )
}

/// Auto-detect the best IDE link based on user's setup
/// Prefers JetBrains Toolbox format, falls back to legacy
pub fn ide_link(
    file: &str,
    line: u32,
    column: u32,
    project_name: &str,
    prefer_jetbrains: bool,
) -> String {
    if prefer_jetbrains {
        jetbrains_link(file, line, column, project_name)
    } else {
        vscode_link(file, line, column)
    }
}

fn urlencoding_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

/// Inject error overlay into HTML response
pub fn inject_error_overlay(
    html: &str,
    error: &PhpError,
    is_dev: bool,
    project_name: &str,
) -> String {
    if !is_dev {
        return html.to_string();
    }

    let overlay_html = generate_overlay_html(error, project_name);

    if let Some(pos) = html.find("</body>") {
        format!("{}{}{}", &html[..pos], overlay_html, &html[pos..])
    } else {
        format!("{}{}", html, overlay_html)
    }
}

fn generate_overlay_html(error: &PhpError, project_name: &str) -> String {
    let error_class = match error.error_type.to_lowercase().as_str() {
        "fatal" | "catchable" | "parse" => "cleanserve-error--fatal",
        "warning" => "cleanserve-error--warning",
        "deprecated" => "cleanserve-error--deprecated",
        _ => "cleanserve-error--notice",
    };

    let vscode_href = vscode_link(&error.file, error.line, 1);
    let jetbrains_href = jetbrains_link(&error.file, error.line, 1, project_name);
    let phpstorm_legacy_href = phpstorm_link_legacy(&error.file, error.line);

    format!(
        r#"
<div id="cleanserve-error-overlay" class="{}">
    <div class="cleanserve-error__badge">
        <span class="cleanserve-error__type">{}</span>
        <span class="cleanserve-error__close" onclick="this.parentElement.parentElement.remove()">×</span>
    </div>
    <div class="cleanserve-error__message">{}</div>
    <div class="cleanserve-error__location">
        <span class="cleanserve-error__file">{}</span>
        <span class="cleanserve-error__line">:{}</span>
    </div>
{}
    <div class="cleanserve-error__actions">
        <a href="{}" class="cleanserve-error__action">Open in VS Code</a>
        <a href="{}" class="cleanserve-error__action">Open in PHPStorm</a>
        <a href="{}" class="cleanserve-error__action cleanserve-error__action--legacy">PHPStorm (Legacy)</a>
    </div>
</div>
<style>
#cleanserve-error-overlay {{
    position: fixed;
    bottom: 20px;
    right: 20px;
    max-width: 550px;
    background: #1a1a2e;
    border: 2px solid #e74c3c;
    border-radius: 8px;
    padding: 16px;
    font-family: 'SF Mono', Monaco, Consolas, monospace;
    font-size: 13px;
    color: #fff;
    z-index: 999999;
    box-shadow: 0 4px 20px rgba(0,0,0,0.3);
    animation: cleanserve-error-slide 0.3s ease-out;
}}
@keyframes cleanserve-error-slide {{
    from {{ transform: translateX(100%); opacity: 0; }}
    to {{ transform: translateX(0); opacity: 1; }}
}}
.cleanserve-error__badge {{
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
}}
.cleanserve-error__type {{
    background: #e74c3c;
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: bold;
    font-size: 11px;
    text-transform: uppercase;
}}
.cleanserve-error--warning .cleanserve-error__type,
.cleanserve-error--deprecated .cleanserve-error__type {{
    background: #f39c12;
}}
.cleanserve-error--notice .cleanserve-error__type {{
    background: #3498db;
}}
.cleanserve-error__close {{
    cursor: pointer;
    font-size: 20px;
    opacity: 0.7;
}}
.cleanserve-error__close:hover {{ opacity: 1; }}
.cleanserve-error__message {{
    margin-bottom: 12px;
    line-height: 1.5;
    color: #f5f5f5;
    white-space: pre-wrap;
    word-break: break-word;
}}
.cleanserve-error__location {{
    font-size: 12px;
    color: #888;
    margin-bottom: 12px;
}}
.cleanserve-error__file {{
    color: #9b59b6;
}}
.cleanserve-error__line {{
    color: #e74c3c;
}}
.cleanserve-error__trace {{
    font-size: 11px;
    color: #888;
    margin-bottom: 12px;
    max-height: 120px;
    overflow-y: auto;
    border-top: 1px solid #333;
    padding-top: 8px;
}}
.cleanserve-error__trace-item {{
    margin-bottom: 4px;
}}
.cleanserve-error__trace-file {{
    color: #9b59b6;
}}
.cleanserve-error__trace-app {{
    color: #3498db;
}}
.cleanserve-error__trace-vendor {{
    color: #666;
}}
.cleanserve-error__actions {{
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
}}
.cleanserve-error__action {{
    background: #2d2d44;
    color: #fff;
    padding: 6px 12px;
    border-radius: 4px;
    text-decoration: none;
    font-size: 11px;
    transition: background 0.2s;
}}
.cleanserve-error__action:hover {{
    background: #3d3d5c;
}}
.cleanserve-error__action--legacy {{
    background: transparent;
    border: 1px solid #444;
}}
</style>
"#,
        error_class,
        error.error_type,
        escape_html(&error.message),
        escape_html(&error.file),
        error.line,
        generate_stack_trace_html(&error.stack_trace),
        vscode_href,
        jetbrains_href,
        phpstorm_legacy_href
    )
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn generate_stack_trace_html(trace: &[StackFrame]) -> String {
    if trace.is_empty() {
        return String::new();
    }

    let items: Vec<String> = trace.iter().map(|frame| {
        let class = if frame.is_app_code {
            "cleanserve-error__trace-app"
        } else {
            "cleanserve-error__trace-vendor"
        };
        let vscode = vscode_link(&frame.file, frame.line, 1);
        format!(
            r#"<div class="cleanserve-error__trace-item"><a href="{}" class="{}">{}:{}</a> in {}</div>"#,
            vscode,
            class,
            escape_html(&frame.file),
            frame.line,
            escape_html(&frame.function)
        )
    }).collect();

    format!(
        r#"<div class="cleanserve-error__trace"><div class="cleanserve-error__trace-title">Stack Trace (click to open):</div>{}</div>"#,
        items.join("")
    )
}

/// Check if error overlay should be shown (dev mode only)
pub fn should_show_overlay(is_dev: bool) -> bool {
    is_dev
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vscode_link() {
        let link = vscode_link("/path/to/file.php", 42, 1);
        assert_eq!(link, "vscode://file//path/to/file.php:42:1");
    }

    #[test]
    fn test_vscode_link_windows() {
        let link = vscode_link(r"C:\projects\myapp\index.php", 15, 5);
        assert!(link.starts_with("vscode://file/"));
        assert!(link.contains("/projects/myapp/index.php:15:5"));
    }

    #[test]
    fn test_jetbrains_link() {
        let link = jetbrains_link("/path/to/file.php", 42, 1, "my-project");
        assert!(link.starts_with("jetbrains://php-storm/navigate/reference?"));
        assert!(link.contains("project=my-project"));
        assert!(link.contains("path="));
    }

    #[test]
    fn test_phpstorm_link_legacy() {
        let link = phpstorm_link_legacy("/path/to/file.php", 42);
        assert!(link.starts_with("phpstorm://open?file="));
    }

    #[test]
    fn test_parse_php_error() {
        let stderr = "PHP Fatal error: Allowed memory size of 134217728 bytes exhausted in /app/index.php on line 15";
        let error = parse_php_error(stderr).unwrap();
        assert_eq!(error.error_type, "Fatal");
        assert!(error.message.contains("memory"));
        assert!(error.file.contains("index.php"));
        assert_eq!(error.line, 15);
    }

    #[test]
    fn test_parse_php_warning() {
        let stderr = "PHP Warning: Division by zero in /app/calc.php on line 23";
        let error = parse_php_error(stderr).unwrap();
        assert_eq!(error.error_type, "Warning");
        assert!(error.message.contains("Division"));
        assert_eq!(error.line, 23);
    }

    #[test]
    fn test_parse_php_notice() {
        let stderr = "PHP Notice: Undefined variable: username in /app/user.php on line 10";
        let error = parse_php_error(stderr).unwrap();
        assert_eq!(error.error_type, "Notice");
        assert!(error.message.contains("Undefined"));
        assert_eq!(error.line, 10);
    }

    #[test]
    fn test_parse_php_parse_error() {
        let stderr =
            "PHP Parse error: syntax error, unexpected token '}' in /app/broken.php on line 5";
        let error = parse_php_error(stderr).unwrap();
        assert_eq!(error.error_type, "Parse");
        assert!(error.message.contains("syntax error"));
        assert_eq!(error.line, 5);
    }

    #[test]
    fn test_inject_overlay() {
        let html = r#"<html><body><h1>Hello</h1></body></html>"#;
        let error = PhpError {
            error_type: "Fatal".to_string(),
            message: "Test error".to_string(),
            file: "/app/test.php".to_string(),
            line: 10,
            stack_trace: vec![],
        };
        let result = inject_error_overlay(html, &error, true, "my-project");
        assert!(result.contains("cleanserve-error-overlay"));
        assert!(result.contains("vscode://file/"));
        assert!(result.contains("jetbrains://php-storm/"));
    }

    #[test]
    fn test_inject_overlay_no_dev_mode() {
        let html = r#"<html><body><h1>Hello</h1></body></html>"#;
        let error = PhpError {
            error_type: "Fatal".to_string(),
            message: "Test error".to_string(),
            file: "/app/test.php".to_string(),
            line: 10,
            stack_trace: vec![],
        };
        let result = inject_error_overlay(html, &error, false, "my-project");
        assert_eq!(result, html);
    }
}
