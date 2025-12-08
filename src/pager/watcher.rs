use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

/// File watcher that monitors a file for changes
pub struct FileWatcher {
    /// The watcher instance (kept alive to maintain the watch)
    _watcher: RecommendedWatcher,
    /// Receiver for file change events
    receiver: Receiver<Result<Event, notify::Error>>,
}

impl FileWatcher {
    /// Create a new file watcher for the given path
    pub fn new(path: &PathBuf) -> Result<Self, notify::Error> {
        let (tx, rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        )?;

        // Watch the file (not recursively since it's a single file)
        watcher.watch(path, RecursiveMode::NonRecursive)?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
        })
    }

    /// Check if the file has changed (non-blocking)
    /// Returns true if the file was modified
    pub fn check_changed(&self) -> bool {
        // Drain all pending events and check for modifications
        let mut changed = false;
        while let Ok(result) = self.receiver.try_recv() {
            if let Ok(event) = result {
                // Check if this is a modify event
                if matches!(
                    event.kind,
                    notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                ) {
                    changed = true;
                }
            }
        }
        changed
    }
}
