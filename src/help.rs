use crate::app::CurrentScreen;
pub struct Keybind {
    pub keys: &'static str,
    pub desc: &'static str,
}

pub fn hints(screen: &CurrentScreen) -> &'static [Keybind] {
    match screen {
        CurrentScreen::Main => &[
            Keybind {
                keys: "↑/↓ or j/k",
                desc: "Navigate",
            },
            Keybind {
                keys: "a/Enter",
                desc: "Activate",
            },
            Keybind {
                keys: "d",
                desc: "Delete",
            },
            Keybind {
                keys: "q",
                desc: "Quit",
            },
        ],
        CurrentScreen::ConfirmDelete => &[
            Keybind {
                keys: "y",
                desc: "Confirm",
            },
            Keybind {
                keys: "n/Esc",
                desc: "Cancel",
            },
        ],
    }
}
