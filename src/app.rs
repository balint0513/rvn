use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use ratatui::widgets::ListState;

const STATUS_TTL: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, Default)]
pub struct VenvMetadata {
    pub version: String,
    pub executable: String,
    pub include_system_packages: String,
    pub packages: Vec<String>, //Stores parsed packages as strings
}

pub enum CurrentScreen {
    Main,
    ConfirmDelete,
}

#[derive(Debug, Clone, Copy)]
pub enum MsgLevel {
    Info,
    Error,
}

pub struct StatusMessage {
    pub text: String,
    pub level: MsgLevel,
    expires: Instant,
}

impl StatusMessage {
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires
    }
}

pub struct App {
    pub venvs: Vec<PathBuf>,
    pub selected_index: usize,
    pub current_metadata: Option<VenvMetadata>,
    pub current_size: Option<String>,
    pub should_quit: bool,
    pub current_screen: CurrentScreen,
    pub list_state: ListState,
    pub status: Option<StatusMessage>,
    pub size_rx: Receiver<(PathBuf, u64)>,
    size_tx: Sender<PathBuf>,
}

impl App {
    pub fn new() -> Self {
        let (req_tx, req_rx) = channel::<PathBuf>();
        let (res_tx, res_rx) = channel::<(PathBuf, u64)>();

        // Single background worker: at most one directory walk at a time.
        // Bursts of requests are drained down to the newest one ("latest wins").
        std::thread::spawn(move || {
            while let Ok(first) = req_rx.recv() {
                let mut target = first;
                while let Ok(newer) = req_rx.try_recv() {
                    target = newer;
                }

                let total_size: u64 = walkdir::WalkDir::new(&target)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
                    .sum();

                if res_tx.send((target, total_size)).is_err() {
                    break;
                }
            }
        });

        Self {
            venvs: Vec::new(),
            selected_index: 0,
            current_metadata: None,
            current_size: None,
            should_quit: false,
            current_screen: CurrentScreen::Main,
            list_state: ListState::default(),
            status: None,
            size_tx: req_tx,
            size_rx: res_rx,
        }
    }

    pub fn set_status(&mut self, text: impl Into<String>, level: MsgLevel) {
        self.status = Some(StatusMessage {
            text: text.into(),
            level,
            expires: Instant::now() + STATUS_TTL,
        });
    }

    pub fn clear_expired_status(&mut self) {
        if self.status.as_ref().is_some_and(StatusMessage::is_expired) {
            self.status = None;
        }
    }

    pub fn request_size(&self, path: &Path) {
        let _ = self.size_tx.send(path.to_path_buf());
    }

    pub fn next(&mut self) {
        if !self.venvs.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.venvs.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.venvs.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.venvs.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }
}
