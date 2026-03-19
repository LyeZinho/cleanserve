//! Environment Injection Module
//!
//! Maps HTTP headers and request data to PHP $_SERVER format
//! for full compatibility with Laravel, Symfony, and other frameworks.

use std::collections::HashMap;

/// HTTP Header to FastCGI/SERVER variable mapping
pub fn build_php_environment(
    method: &str,
    uri: &str,
    http_headers: &HashMap<String, String>,
    remote_addr: &str,
    server_name: &str,
    server_port: u16,
    script_filename: &str,
    is_https: bool,
) -> HashMap<String, String> {
    let mut env = HashMap::new();

    // === Core Request Variables ===
    env.insert("REQUEST_METHOD".to_string(), method.to_string());
    env.insert("REQUEST_URI".to_string(), uri.to_string());
    env.insert("QUERY_STRING".to_string(), extract_query_string(uri));
    env.insert("SCRIPT_FILENAME".to_string(), script_filename.to_string());
    env.insert("SCRIPT_NAME".to_string(), script_filename.to_string());
    env.insert("PATH_INFO".to_string(), extract_path_info(uri));
    env.insert("PATH_TRANSLATED".to_string(), script_filename.to_string());

    // === Server Protocol & Interface ===
    env.insert("SERVER_PROTOCOL".to_string(), "HTTP/1.1".to_string());
    env.insert("GATEWAY_INTERFACE".to_string(), "CGI/1.1".to_string());
    env.insert("SERVER_SOFTWARE".to_string(), "CleanServe/0.1".to_string());

    // === Server Identity ===
    env.insert("SERVER_NAME".to_string(), server_name.to_string());
    env.insert("SERVER_PORT".to_string(), server_port.to_string());

    // === Remote Client ===
    env.insert("REMOTE_ADDR".to_string(), remote_addr.to_string());
    env.insert("REMOTE_PORT".to_string(), "0".to_string()); // Not available in CGI

    // === HTTPS Detection ===
    if is_https {
        env.insert("HTTPS".to_string(), "on".to_string());
        env.insert("HTTP_X_FORWARDED_PROTO".to_string(), "https".to_string());
    }

    // === HTTP Headers ===
    // Standard HTTP headers (without HTTP_ prefix for common ones)
    for (key, value) in http_headers {
        let key_upper = key.to_uppercase().replace('-', "_");

        // Special headers that don't get HTTP_ prefix
        match key_upper.as_str() {
            "CONTENT_TYPE" | "CONTENT_LENGTH" => {
                env.insert(key_upper, value.clone());
            }
            "HOST" => {
                env.insert("HTTP_HOST".to_string(), value.clone());
                // Also extract server name from Host header
                if let Some(port_pos) = value.find(':') {
                    env.insert("SERVER_NAME".to_string(), value[..port_pos].to_string());
                }
            }
            _ => {
                // All other headers get HTTP_ prefix
                env.insert(format!("HTTP_{}", key_upper), value.clone());
            }
        }
    }

    // === Framework Detection Variables ===
    // These help Laravel, Symfony, etc. identify the environment
    env.insert("APP_ENV".to_string(), "local".to_string());
    env.insert("APP_DEBUG".to_string(), "true".to_string());

    // === PHP Specific ===
    env.insert("PHP_SELF".to_string(), script_filename.to_string());
    env.insert(
        "DOCUMENT_ROOT".to_string(),
        extract_document_root(script_filename),
    );

    // === CleanServe Custom Variables ===
    env.insert("CLEANSERVE_VERSION".to_string(), "0.1.0".to_string());
    env.insert("CLEANSERVE_MODE".to_string(), "development".to_string());

    env
}

/// Extract query string from URI
fn extract_query_string(uri: &str) -> String {
    if let Some(pos) = uri.find('?') {
        uri[pos + 1..].to_string()
    } else {
        String::new()
    }
}

/// Extract PATH_INFO from URI
fn extract_path_info(uri: &str) -> String {
    // PATH_INFO is the part after the script but before the query string
    if let Some(pos) = uri.find('?') {
        uri[..pos].to_string()
    } else {
        uri.to_string()
    }
}

/// Extract document root from script filename
fn extract_document_root(script_filename: &str) -> String {
    // Remove filename to get directory, then back up to root
    let path = std::path::Path::new(script_filename);
    if let Some(parent) = path.parent() {
        parent.to_string_lossy().to_string()
    } else {
        String::new()
    }
}

/// Extract just the filename from a path
pub fn extract_filename(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Detect if request is for a static file
pub fn is_static_file(path: &str) -> bool {
    let static_extensions = [
        ".css", ".js", ".jpg", ".jpeg", ".png", ".gif", ".ico", ".svg", ".woff", ".woff2", ".ttf",
        ".eot", ".map", ".html", ".htm", ".txt", ".xml", ".json", ".yaml", ".yml", ".md", ".webp",
        ".avif", ".webm", ".mp4", ".mp3",
    ];

    let path_lower = path.to_lowercase();
    static_extensions
        .iter()
        .any(|ext| path_lower.ends_with(ext))
}

/// Resolve request path to filesystem path
pub fn resolve_request_path(root: &str, uri: &str) -> std::path::PathBuf {
    let root_path = std::path::Path::new(root);

    // Normalize URI: remove query string, decode URL encoding
    let clean_uri = if let Some(pos) = uri.find('?') {
        &uri[..pos]
    } else {
        uri
    };

    // Remove leading slash
    let clean_uri = clean_uri.trim_start_matches('/');

    // Handle index.php fallback
    let relative_path = if clean_uri.is_empty() {
        "index.php"
    } else {
        clean_uri
    };

    root_path.join(relative_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_query_string() {
        assert_eq!(extract_query_string("/path?foo=bar&baz=1"), "foo=bar&baz=1");
        assert_eq!(extract_query_string("/path"), "");
        assert_eq!(extract_query_string("/path?"), "");
    }

    #[test]
    fn test_is_static_file() {
        assert!(is_static_file("/style.css"));
        assert!(is_static_file("/script.js"));
        assert!(!is_static_file("/index.php"));
        assert!(!is_static_file("/api/users"));
    }
}
