//! Path Traversal Protection Module
//!
//! Prevents directory traversal attacks (e.g., `../../../etc/passwd`)
//! by normalizing and validating request paths before serving.

use std::path::{Component, PathBuf};

/// Path traversal attack detector and normalizer
pub struct PathTraversal;

impl PathTraversal {
    /// Normalize a path and detect traversal attempts
    ///
    /// Returns normalized path if safe, or None if traversal detected
    pub fn normalize_and_validate(path: &str) -> Option<String> {
        if path.is_empty() {
            return None;
        }

        let path_lower = path.to_lowercase();
        if Self::contains_traversal_patterns(&path_lower) {
            return None;
        }

        let normalized = Self::normalize_path(path);
        if normalized.starts_with("..") || normalized.contains("/..") {
            return None;
        }

        Some(normalized)
    }

    /// Check if path contains obvious traversal patterns
    fn contains_traversal_patterns(path: &str) -> bool {
        path.contains("..")
            || path.contains("\\..")
            || path.contains("%2e%2e")
            || path.contains("%252e%252e")
            || path.contains("..\\")
            || path.contains("..")
    }

    /// Normalize a path by resolving components
    fn normalize_path(path: &str) -> String {
        let path_buf = PathBuf::from(path);
        let mut components = path_buf.components().peekable();
        let mut ret = PathBuf::new();

        while let Some(component) = components.next() {
            match component {
                Component::Normal(c) => {
                    ret.push(c);
                }
                Component::CurDir => {
                    continue;
                }
                Component::ParentDir => {
                    ret.pop();
                }
                _ => {
                    ret.push(component);
                }
            }
        }

        // Normalize to forward slashes for consistency across platforms
        ret.to_string_lossy().to_string().replace("\\", "/")
    }

    /// Check if path escapes beyond the root
    pub fn escapes_root(path: &str) -> bool {
        // Check raw path for parent directory attempts
        let path_lower = path.to_lowercase();
        if path_lower.starts_with("..") || path_lower.contains("/..") || path_lower.contains("\\..")
        {
            return true;
        }

        let normalized = Self::normalize_path(path);
        // If normalization removes all components and we're left with just slashes or empty,
        // but original had traversal, that's an escape attempt
        normalized.starts_with("..") || normalized.contains("/..")
    }

    /// Resolve a path relative to a root directory
    ///
    /// Returns Some(resolved_path) if safe, None if traversal detected
    pub fn resolve_safe(root: &str, request_path: &str) -> Option<String> {
        if request_path.is_empty() {
            return Some(root.to_string());
        }

        let normalized = Self::normalize_and_validate(request_path)?;

        if Self::escapes_root(&normalized) {
            return None;
        }

        let mut full_path = PathBuf::from(root);
        full_path.push(&normalized);

        // Canonicalize root to absolute path
        let root_path = PathBuf::from(root).canonicalize().ok()?;

        // Normalize full path without requiring it to exist
        let normalized_full = full_path.to_string_lossy().to_string().replace("\\", "/");

        // Final check: ensure normalized path doesn't start with ../
        if normalized_full.contains("..") {
            return None;
        }

        Some(normalized_full)
    }

    /// Check if path contains null bytes (common attack vector)
    pub fn has_null_bytes(path: &str) -> bool {
        path.contains('\0')
    }

    /// Validate request path for common traversal patterns
    pub fn is_valid_request_path(path: &str) -> bool {
        if path.is_empty() {
            return false;
        }

        if Self::has_null_bytes(path) {
            return false;
        }

        if Self::contains_traversal_patterns(&path.to_lowercase()) {
            return false;
        }

        if Self::escapes_root(path) {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocks_parent_directory_traversal() {
        assert!(PathTraversal::normalize_and_validate("../etc/passwd").is_none());
        assert!(PathTraversal::normalize_and_validate("../../etc/passwd").is_none());
        assert!(PathTraversal::normalize_and_validate("path/../../../etc/passwd").is_none());
    }

    #[test]
    fn test_blocks_backslash_traversal() {
        assert!(PathTraversal::normalize_and_validate("..\\etc\\passwd").is_none());
        assert!(PathTraversal::normalize_and_validate("path\\..\\..\\passwd").is_none());
    }

    #[test]
    fn test_blocks_url_encoded_traversal() {
        assert!(PathTraversal::normalize_and_validate("%2e%2e/etc/passwd").is_none());
        assert!(PathTraversal::normalize_and_validate("%252e%252e/passwd").is_none());
    }

    #[test]
    fn test_allows_normal_paths() {
        assert_eq!(
            PathTraversal::normalize_and_validate("/index.php"),
            Some("/index.php".to_string())
        );
        assert_eq!(
            PathTraversal::normalize_and_validate("/upload/file.pdf"),
            Some("/upload/file.pdf".to_string())
        );
        assert_eq!(
            PathTraversal::normalize_and_validate("css/style.css"),
            Some("css/style.css".to_string())
        );
    }

    #[test]
    fn test_normalizes_current_directory() {
        assert_eq!(
            PathTraversal::normalize_and_validate("./index.php"),
            Some("index.php".to_string())
        );
        assert_eq!(
            PathTraversal::normalize_and_validate("/./upload/file.pdf"),
            Some("/upload/file.pdf".to_string())
        );
    }

    #[test]
    fn test_blocks_null_bytes() {
        assert!(!PathTraversal::is_valid_request_path("/index.php\0.txt"));
        assert!(!PathTraversal::is_valid_request_path("/upload\0/file.pdf"));
    }

    #[test]
    fn test_escapes_root_detection() {
        assert!(PathTraversal::escapes_root("../etc/passwd"));
        assert!(PathTraversal::escapes_root("../../root"));
        assert!(!PathTraversal::escapes_root("/index.php"));
        assert!(!PathTraversal::escapes_root("upload/file.pdf"));
    }

    #[test]
    fn test_case_insensitive_detection() {
        assert!(PathTraversal::normalize_and_validate("..%2E%2E/passwd").is_none());
    }

    #[test]
    fn test_empty_path() {
        assert!(PathTraversal::normalize_and_validate("").is_none());
    }

    #[test]
    fn test_valid_request_path_validation() {
        assert!(PathTraversal::is_valid_request_path("/index.php"));
        assert!(PathTraversal::is_valid_request_path("/upload/file.pdf"));
        assert!(!PathTraversal::is_valid_request_path("../passwd"));
        assert!(!PathTraversal::is_valid_request_path(""));
    }

    #[test]
    fn test_resolve_safe_within_root() {
        // Use current directory as root since it exists
        let root = ".";
        // These should return Some (valid paths within root)
        assert!(PathTraversal::resolve_safe(root, "/index.php").is_some());
        assert!(PathTraversal::resolve_safe(root, "/upload/file.pdf").is_some());
    }

    #[test]
    fn test_resolve_safe_blocks_escape() {
        let root = ".";
        // Paths attempting to escape should return None
        assert!(PathTraversal::resolve_safe(root, "/../etc/passwd").is_none());
    }
}
