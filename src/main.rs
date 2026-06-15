mod app;
mod tui;
mod scanner;

use crate::app::{App, CurrentScreen};
use ratatui::layout::Rect;
use std::{env};
use std::path::{Path};

use ratatui::{
    Frame, 
    crossterm::{
        self, event::KeyEventKind
    }, 
    layout::{
        Constraint, Direction, Layout
    }
};
use ratatui::widgets::{ListItem, List, Block, Borders, Paragraph};
use crossterm::event::{self, Event, KeyCode};

fn main() -> Result<(), Box<dyn std::error::Error>> {

    color_eyre::install()?;

    let mut terminal = tui::init()?;
    let mut app = App::new();

    let current_dir = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    app.venvs = scanner::find_venvs(current_dir);

    update_app_metadata(&mut app);

    while !app.should_quit {

        while let Ok((path, bytes)) = app.size_rx.try_recv() {
            if !app.venvs.is_empty() && app.venvs[app.selected_index] == path {
                app.current_size = Some(format_bytes(bytes));
            }
        }

        terminal.draw(|f| draw_ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.current_screen{
                    CurrentScreen::Main => match key.code {
                        KeyCode::Char('q') => app.should_quit = true,
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.next();
                            update_app_metadata(&mut app);
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.previous();
                            update_app_metadata(&mut app);
                        }
                        KeyCode::Char('d') => {
                            if !app.venvs.is_empty() {
                                app.current_screen = CurrentScreen::ConfirmDelete;
                            }
                        }
                        KeyCode::Enter | KeyCode::Char('a') => {
                            if !app.venvs.is_empty() {
                                let target_path = app.venvs[app.selected_index].clone();
                                if let Err(e) = activate_subshell(&target_path, &mut terminal) {
                                    eprintln!("Error activating subshell: {}", e);
                                }
                            }
                        }
                        _ => {}
                    },
                    CurrentScreen::ConfirmDelete => match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            let path_to_remove = app.venvs.remove(app.selected_index);
                            let _ = std::fs::remove_dir_all(&path_to_remove);

                            if app.selected_index >= app.venvs.len() && !app.venvs.is_empty() {
                                app.selected_index = app.venvs.len() - 1;
                            }
                            app.current_screen = CurrentScreen::Main;
                            update_app_metadata(&mut app);
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            app.current_screen = CurrentScreen::Main;
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}

    tui::restore(&mut terminal)?;
    Ok(())
}

fn update_app_metadata(app: &mut App) {
    if !app.venvs.is_empty() {
        let selected_path = app.venvs[app.selected_index].clone();
        app.current_metadata = scanner::parse_venv_metadata(&selected_path);
        app.current_size = Some("Calculating size...".to_string());

        let tx = app.size_tx.clone();
        std::thread::spawn(move || {
            let total_size = walkdir::WalkDir::new(&selected_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
                .sum();
            let _ = tx.send((selected_path, total_size));
        });
    } else {
        app.current_metadata = None;
        app.current_size = None;
    }
}

fn activate_subshell(venv_path: &Path, terminal: &mut tui::TuiTerminal) -> Result<(), Box<dyn std::error::Error>> {
    tui::restore(terminal)?;

    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let old_path = env::var("PATH").unwrap_or_default();
    let bin_path = venv_path.join("bin");
    let new_path = format!("{}:{}", bin_path.display(), old_path);

    println!("\x1b[1;32[+] Entering virtual environment subshell\x1b[0m: {}", venv_path.display());
    println!("Type \x1b[1;33m'exit'\x1b[0m or hit Ctrl+D to return directly back to your TUI panel.\n");

    std::process::Command::new(&shell)
        .env("PATH", new_path)
        .env("VIRTUAL_ENV", venv_path)
        .status()?;

    *terminal = tui::init()?;
    Ok(())
}

fn draw_ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
            ])
        .split(f.area());
    
    let items: Vec<ListItem> = app.venvs
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let prefix = if i == app.selected_index { ">>" } else { "  " };
            ListItem::new(format!("{} {}", prefix, path.display()))
        })
    .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Virtual Environments"));
    f.render_widget(list, chunks[0]);

    let details_content = if app.venvs.is_empty() {
        "No virtual environments found.".to_string()
    } else {
        let base_path = format!("Path: {}", app.venvs[app.selected_index].display());
        let size_info = format!("Disk footprint: {}", app.current_size.as_deref().unwrap_or("Pending..."));
        
        let config_details = match &app.current_metadata {
            Some(meta) => format!(
                "Python Version: {}\nExecutable: {}\nInclude System Site Packages: {}",
                if meta.version.is_empty() {"Unknown"} else {&meta.version},
                if meta.executable.is_empty() {"Unknown"} else {&meta.executable},
                if meta.include_system_packages.is_empty() {"Unknown"} else {&meta.include_system_packages},
            ),
            None => "Failed to read pyvenv.cfg for this environment target".to_string(),
        };
        format !("{}\n{}\n{}", base_path, size_info, config_details)
    };

    let details = Paragraph::new(details_content)
        .block(Block::default().borders(Borders::ALL).title("Environment details"));
    f.render_widget(details, chunks[1]);

    if let CurrentScreen::ConfirmDelete = app.current_screen {
        let popup_area = centered_rect(60, 20, f.area());
        f.render_widget(ratatui::widgets::Clear, popup_area);

        let confirm_message = Paragraph::new("\n Are you sure you want to delete this virtual environment? This action cannot be undone.\n\n Press 'y' to confirm or 'n' to cancel.")
            .block(Block::default().borders(Borders::ALL).title(" DANGER ZONE "));
        f.render_widget(confirm_message, popup_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
