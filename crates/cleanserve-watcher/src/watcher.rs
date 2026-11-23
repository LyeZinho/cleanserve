use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::info;

pub enum FileEvent {
    PhpChanged(Vec<PathBuf>),
    StyleChanged(Vec<PathBuf>),
}

/// Guard that keeps the file watcher alive. Drop this to stop watching.
pub type WatcherGuard = Debouncer<RecommendedWatcher>;

const IGNORED_DIRS: &[&str] = &[
    "vendor", "node_modules", ".git", ".cleanserve", "storage", "var", "runtime", "cache", "tmp",
    "logs",
];

fn is_ignored(path: &std::path::Path, root: &std::path::Path) -> bool {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components().any(|c| {
        c.as_os_str()
            .to_str()
            .is_some_and(|name| IGNORED_DIRS.contains(&name))
    })
}

pub struct FileWatcher {
    root: PathBuf,
}

impl FileWatcher {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Start watching for file changes.
    ///
    /// Returns both the event receiver AND the debouncer guard.
    /// **The caller MUST hold onto the WatcherGuard** — dropping it stops the watcher.
    pub fn watch(&self) -> anyhow::Result<(mpsc::Receiver<FileEvent>, WatcherGuard)> {
        let (tx, rx) = mpsc::channel(100);
        let root = self.root.clone();
        let filter_root = root.clone();

        let mut debouncer = new_debouncer(
            Duration::from_millis(300),
            move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
                if let Ok(events) = res {
                    let mut php_events = Vec::new();
                    let mut style_events = Vec::new();

                    for event in events {
                        if event.kind == DebouncedEventKind::Any {
                            let path = event.path.clone();
                            if is_ignored(&path, &filter_root) {
                                continue;
                            }
                            if let Some(ext) = path.extension() {
                                let ext_str = ext.to_string_lossy().to_lowercase();
                                if matches!(
                                    ext_str.as_str(),
                                    "php" | "phtml" | "twig" | "html" | "htm" | "env" | "json"
                                        | "yaml" | "yml" | "xml" | "ini" | "neon" | "latte"
                                ) {
                                    php_events.push(path);
                                } else if ext_str == "css" || ext_str == "js" {
                                    style_events.push(path);
                                }
                            }
                        }
                    }

                    if !php_events.is_empty() {
                        let _ = tx.blocking_send(FileEvent::PhpChanged(php_events));
                    }
                    if !style_events.is_empty() {
                        let _ = tx.blocking_send(FileEvent::StyleChanged(style_events));
                    }
                }
            },
        )?;

        debouncer.watcher().watch(&root, RecursiveMode::Recursive)?;
        info!("👀 Watching {} for changes", root.display());

        Ok((rx, debouncer))
    }
}

#[cfg(test)]
mod tests {
    use super::is_ignored;
    use std::path::Path;

    #[test]
    fn ignores_vendor_and_storage_but_not_src() {
        let root = Path::new("/p");
        assert!(is_ignored(Path::new("/p/vendor/x/a.php"), root));
        assert!(is_ignored(Path::new("/p/storage/logs/a.php"), root));
        assert!(is_ignored(Path::new("/p/.git/x.php"), root));
        assert!(!is_ignored(Path::new("/p/src/Controller/a.php"), root));
        assert!(!is_ignored(Path::new("/p/public/style.css"), root));
    }
}
