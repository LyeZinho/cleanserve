use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::info;

pub enum FileEvent {
    PhpChanged(Vec<PathBuf>),
    StyleChanged(Vec<PathBuf>),
}

pub struct FileWatcher {
    root: PathBuf,
}

impl FileWatcher {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn watch(&self) -> anyhow::Result<mpsc::Receiver<FileEvent>> {
        let (tx, rx) = mpsc::channel(100);
        let root = self.root.clone();

        let mut debouncer = new_debouncer(
            Duration::from_millis(100),
            move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
                if let Ok(events) = res {
                    let mut php_events = Vec::new();
                    let mut style_events = Vec::new();

                    for event in events {
                        if event.kind == DebouncedEventKind::Any {
                            let path = event.path.clone();
                            if let Some(ext) = path.extension() {
                                let ext_str = ext.to_string_lossy().to_lowercase();
                                if ext_str == "php" {
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

        Ok(rx)
    }
}
