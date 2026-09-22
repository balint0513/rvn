use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

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

pub struct App {
    pub venvs: Vec<PathBuf>,
    pub selected_index: usize,
    pub current_metadata: Option<VenvMetadata>,
    pub current_size: Option<String>,
    pub should_quit: bool,
    pub current_screen: CurrentScreen,

    pub size_tx: Sender<(PathBuf, u64)>,
    pub size_rx: Receiver<(PathBuf, u64)>,
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            venvs: Vec::new(),
            selected_index: 0,
            current_metadata: None,
            current_size: None,
            should_quit: false,
            current_screen: CurrentScreen::Main,
            size_tx: tx,
            size_rx: rx,
        }
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
