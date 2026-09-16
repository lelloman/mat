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

        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        watcher.watch(parent, RecursiveMode::NonRecursive)?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
            path: path.to_path_buf(),
        })
    }

    /// Drain coalesced events while surfacing backend failures.
    pub fn check_changed(&self) -> Result<bool, notify::Error> {
        drain_events(&self.receiver, &self.path)
    }
}

fn drain_events(
    receiver: &Receiver<Result<Event, notify::Error>>,
    path: &Path,
) -> Result<bool, notify::Error> {
    let mut changed = false;
    loop {
        match receiver.try_recv() {
            Ok(Ok(event)) => {
                if event.paths.iter().any(|event_path| {
                    event_path == path
                        || (event_path.file_name().is_some()
                            && event_path.file_name() == path.file_name())
                }) && matches!(
                    event.kind,
                    notify::EventKind::Modify(_)
                        | notify::EventKind::Create(_)
                        | notify::EventKind::Remove(_)
                ) {
                    changed = true;
                }
            }
            Ok(Err(error)) => return Err(error),
            Err(mpsc::TryRecvError::Empty) => return Ok(changed),
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(notify::Error::generic("File watcher disconnected"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watcher_errors_and_disconnection_are_reported() {
        let (tx, rx) = mpsc::channel();
        tx.send(Err(notify::Error::generic("backend failed")))
            .unwrap();
        assert!(drain_events(&rx, Path::new("test"))
            .unwrap_err()
            .to_string()
            .contains("backend failed"));
        drop(tx);
        assert!(drain_events(&rx, Path::new("test"))
            .unwrap_err()
            .to_string()
            .contains("disconnected"));
    }

    #[test]
    fn watcher_events_coalesce_and_ignore_unrelated_files() {
        let (tx, rx) = mpsc::channel();
        for name in ["test", "other", "test"] {
            tx.send(Ok(Event::new(notify::EventKind::Create(
                notify::event::CreateKind::File,
            ))
            .add_path(name.into())))
                .unwrap();
        }
        assert!(drain_events(&rx, Path::new("test")).unwrap());
        assert!(!drain_events(&rx, Path::new("test")).unwrap());
        tx.send(Ok(Event::new(notify::EventKind::Remove(
            notify::event::RemoveKind::File,
        ))
        .add_path("other".into())))
            .unwrap();
        assert!(!drain_events(&rx, Path::new("test")).unwrap());
    }
}
