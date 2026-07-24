use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

/// File watcher that monitors a file for changes
pub struct FileWatcher {
    /// The watcher instance (kept alive to maintain the watch)
    _watcher: RecommendedWatcher,
    /// Receiver for file change events
    receiver: Receiver<Result<Event, notify::Error>>,
    path: PathBuf,
}

impl FileWatcher {
    /// Create a new file watcher for the given path
    pub fn new(path: &Path) -> Result<Self, notify::Error> {
        let (tx, rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        )?;

        let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        watcher.watch(parent, RecursiveMode::NonRecursive)?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
            path: path.to_path_buf(),
        })
    }

    /// Check if the file has changed (non-blocking)
    /// Returns true if the file was modified
    pub fn check_changed(&self) -> bool {
        // Drain all pending events and check for modifications
        let mut changed = false;
        while let Ok(result) = self.receiver.try_recv() {
            if let Ok(event) = result {
                if event.paths.iter().any(|path| {
                    path == &self.path
                        || (path.file_name().is_some() && path.file_name() == self.path.file_name())
                }) && matches!(
                    event.kind,
                    notify::EventKind::Modify(_)
                        | notify::EventKind::Create(_)
                        | notify::EventKind::Remove(_)
                ) {
                    changed = true;
                }
            }
        }
        changed
    }
}
