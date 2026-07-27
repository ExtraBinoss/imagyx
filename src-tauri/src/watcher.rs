use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        mpsc::{Receiver, RecvTimeoutError, Sender, channel},
    },
    time::{Duration, Instant},
};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::AppHandle;

use crate::{AppError, indexer, models::FollowedFolder, state::AppState};

const DEBOUNCE_DELAY: Duration = Duration::from_millis(700);
const POLL_INTERVAL: Duration = Duration::from_millis(200);

enum WatchMessage {
    Event(notify::Result<Event>),
    Add(FollowedFolder),
    Remove(FollowedFolder),
}

#[derive(Debug, Clone)]
pub struct FolderWatcher {
    sender: Sender<WatchMessage>,
}

impl FolderWatcher {
    pub fn start(
        app: AppHandle,
        state: Arc<AppState>,
        initial_folders: Vec<FollowedFolder>,
    ) -> Result<Self, AppError> {
        let (sender, receiver) = channel();
        let callback_sender = sender.clone();
        let watcher = notify::recommended_watcher(move |event| {
            let _ = callback_sender.send(WatchMessage::Event(event));
        })
        .map_err(|error| AppError::Watcher(error.to_string()))?;

        std::thread::Builder::new()
            .name("imagyx-folder-watcher".into())
            .spawn(move || run_loop(watcher, receiver, app, state, HashMap::new()))?;

        let folder_watcher = Self { sender };
        for folder in initial_folders {
            folder_watcher.watch(folder).map_err(AppError::Watcher)?;
        }
        Ok(folder_watcher)
    }

    pub fn watch(&self, folder: FollowedFolder) -> Result<(), String> {
        self.sender
            .send(WatchMessage::Add(folder))
            .map_err(|error| error.to_string())
    }

    pub fn unwatch(&self, folder: FollowedFolder) -> Result<(), String> {
        self.sender
            .send(WatchMessage::Remove(folder))
            .map_err(|error| error.to_string())
    }
}

fn run_loop(
    mut watcher: RecommendedWatcher,
    receiver: Receiver<WatchMessage>,
    app: AppHandle,
    state: Arc<AppState>,
    mut watched: HashMap<String, FollowedFolder>,
) {
    let mut pending: HashMap<String, Instant> = HashMap::new();

    loop {
        match receiver.recv_timeout(POLL_INTERVAL) {
            Ok(WatchMessage::Event(Ok(event))) => {
                if !matches!(event.kind, EventKind::Access(_)) {
                    queue_affected_folders(&event, &watched, &state.paths.root, &mut pending);
                }
            }
            Ok(WatchMessage::Event(Err(error))) => {
                eprintln!("Imagyx watcher error: {error}");
            }
            Ok(WatchMessage::Add(folder)) => {
                let path = PathBuf::from(&folder.path);
                match watcher.watch(&path, RecursiveMode::Recursive) {
                    Ok(()) => {
                        watched.insert(folder.id.clone(), folder);
                    }
                    Err(error) => eprintln!("Imagyx could not watch {}: {error}", path.display()),
                }
            }
            Ok(WatchMessage::Remove(folder)) => {
                let path = PathBuf::from(&folder.path);
                let _ = watcher.unwatch(&path);
                watched.remove(&folder.id);
                pending.remove(&folder.id);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        let now = Instant::now();
        let due: Vec<String> = pending
            .iter()
            .filter_map(|(folder_id, deadline)| (now >= *deadline).then_some(folder_id.clone()))
            .collect();

        for folder_id in due {
            pending.remove(&folder_id);
            let Some(folder) = watched.get(&folder_id).cloned() else {
                continue;
            };
            let state = Arc::clone(&state);
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(error) = indexer::index_folder(&state, &app, &folder) {
                    eprintln!("Imagyx watcher indexing failed: {error}");
                }
            });
        }
    }
}

fn queue_affected_folders(
    event: &Event,
    watched: &HashMap<String, FollowedFolder>,
    app_root: &Path,
    pending: &mut HashMap<String, Instant>,
) {
    for path in &event.paths {
        if path.starts_with(app_root) {
            continue;
        }
        for folder in watched.values() {
            if path.starts_with(&folder.path) {
                pending.insert(folder.id.clone(), Instant::now() + DEBOUNCE_DELAY);
            }
        }
    }
}
