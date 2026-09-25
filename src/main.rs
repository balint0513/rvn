mod app;
mod help;
mod scanner;
mod tui;

use crate::app::{App, CurrentScreen, MsgLevel};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::env;
use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{
    Frame,
    crossterm::{self, event::KeyEventKind},
    layout::{Constraint, Direction, Layout},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;
    tui::install_panic_hook();

    let mut terminal = tui::init()?;
    let mut app = App::new();

    let current_dir = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    app.venvs = scanner::find_venvs(current_dir);

    update_app_metadata(&mut app);

    // Unrecoverable errors: break out of the loop, still restore the terminal, then report.
    let mut fatal: Option<Box<dyn std::error::Error>> = None;

    while !app.should_quit {
        app.clear_expired_status();

        while let Ok((path, bytes)) = app.size_rx.try_recv() {
            if !app.venvs.is_empty() && app.venvs[app.selected_index] == path {
                app.current_size = Some(format_bytes(bytes));
            }
        }

        if let Err(e) = terminal.draw(|f| draw_ui(f, &mut app)) {
            fatal = Some(e.into());
            break;
        }

        let event_ready = match event::poll(std::time::Duration::from_millis(16)) {
            Ok(ready) => ready,
            Err(e) => {
                fatal = Some(e.into());
                break;
            }
        };

        if event_ready {
            match event::read() {
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        app.should_quit = true;
                        continue;
                    }

                    match app.current_screen {
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
                            KeyCode::Enter | KeyCode::Char('a') if !app.venvs.is_empty() => {
                                let target_path = app.venvs[app.selected_index].clone();
                                match activate_subshell(&target_path, &mut terminal) {
                                    Ok(Some(msg)) => app.set_status(msg, MsgLevel::Error),
                                    Ok(None) => {}
                                    Err(e) => {
                                        fatal = Some(e);
                                        app.should_quit = true;
                                    }
                                }
                            }
                            _ => {}
                        },
                        CurrentScreen::ConfirmDelete => match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                let path_to_remove = app.venvs[app.selected_index].clone();

                                let outcome = match std::fs::symlink_metadata(&path_to_remove) {
                                    Ok(md) if md.file_type().is_symlink() => {
                                        Err("refusing to delete a symlink".to_string())
                                    }
                                    _ => std::fs::remove_dir_all(&path_to_remove)
                                        .map_err(|e| e.to_string()),
                                };

                                match outcome {
                                    Ok(()) => {
                                        app.venvs.remove(app.selected_index);
                                        if app.selected_index >= app.venvs.len() {
                                            app.selected_index = app.venvs.len().saturating_sub(1);
                                        }
                                        app.set_status(
                                            format!("Deleted {}", path_to_remove.display()),
                                            MsgLevel::Info,
                                        );
                                    }
                                    Err(msg) => {
                                        app.set_status(
                                            format!("Delete failed: {msg}"),
                                            MsgLevel::Error,
                                        );
                                    }
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
                Ok(_) => {}
                Err(e) => {
                    fatal = Some(e.into());
                    break;
                }
            }
        }
    }

    let restore_result = tui::restore(&mut terminal);
    if let Some(e) = fatal {
        return Err(e);
    }
    restore_result?;
    Ok(())
}

fn update_app_metadata(app: &mut App) {
    if !app.venvs.is_empty() {
        let selected_path = app.venvs[app.selected_index].clone();
        app.current_metadata = scanner::parse_venv_metadata(&selected_path);
        app.current_size = Some("Calculating size...".to_string());
        app.request_size(&selected_path);
    } else {
        app.current_metadata = None;
        app.current_size = None;
    }
}

/// Spawns a subshell inside the venv with the terminal handed back to the user.
///
/// Returns `Ok(None)` on a normal shell exit, `Ok(Some(msg))` if the shell
/// could not be started but the TUI was successfully re-initialized (caller
/// shows `msg`), or `Err` if the terminal could not be brought back — in which
/// case the caller must stop the event loop and exit.
fn activate_subshell(
    venv_path: &Path,
    terminal: &mut tui::TuiTerminal,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    tui::restore(terminal)?;

    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let old_path = env::var("PATH").unwrap_or_default();
    let bin_path = venv_path.join("bin");
    let new_path = format!("{}:{}", bin_path.display(), old_path);

    println!(
        "\x1b[1;32[+] Entering virtual environment subshell\x1b[0m: {}",
        venv_path.display()
    );
    println!(
        "Type \x1b[1;33m'exit'\x1b[0m or hit Ctrl+D to return directly back to your TUI panel.\n"
    );

    let shell_result = std::process::Command::new(&shell)
        .env("PATH", new_path)
        .env("VIRTUAL_ENV", venv_path)
        .status();

    // No matter what the shell did, bring the TUI back before reporting anything.
    match tui::init() {
        Ok(t) => {
            *terminal = t;
            match shell_result {
                Ok(_) => Ok(None),
                Err(e) => Ok(Some(format!("Could not start subshell: {e}"))),
            }
        }
        Err(reinit_err) => {
            let detail = match shell_result {
                Ok(_) => "the shell exited".to_string(),
                Err(e) => format!("the shell could not be started: {e}"),
            };
            Err(format!("{detail}; also failed to restore the terminal: {reinit_err}").into())
        }
    }
}

fn draw_ui(f: &mut Frame, app: &mut App) {
    let [main_area, status_area, footer_area] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(f.area());

    let chunks = Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(main_area);

    let mut footer_spans: Vec<Span> = Vec::new();
    for (i, kb) in help::hints(&app.current_screen).iter().enumerate() {
        if i > 0 {
            footer_spans.push(Span::styled("  ·  ", Style::default().fg(Color::DarkGray)));
        }
        footer_spans.push(Span::styled(
            kb.keys,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        footer_spans.push(Span::raw(" "));
        footer_spans.push(Span::styled(kb.desc, Style::default().fg(Color::DarkGray)));
    }

    let items: Vec<ListItem> = app
        .venvs
        .iter()
        .map(|path| ListItem::new(path.display().to_string()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Virtual Environments"),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    app.list_state.select(if app.venvs.is_empty() {
        None
    } else {
        Some(app.selected_index)
    });
    f.render_stateful_widget(list, chunks[0], &mut app.list_state);

    let details_content = if app.venvs.is_empty() {
        "No virtual environments found.".to_string()
    } else {
        let base_path = format!("Path: {}", app.venvs[app.selected_index].display());
        let size_info = format!(
            "Disk footprint: {}",
            app.current_size.as_deref().unwrap_or("Pending...")
        );

        let config_details = match &app.current_metadata {
            Some(meta) => {
                let package_list = if meta.packages.is_empty() {
                    " (No installed libraries found)".to_string()
                } else {
                    meta.packages
                        .iter()
                        .take(20)
                        .map(|pkg| format!("  • {}", pkg))
                        .collect::<Vec<_>>()
                        .join("\n")
                };

                let overflow_note = if meta.packages.len() > 20 {
                    format!("\n .. and {} more", meta.packages.len() - 20)
                } else {
                    String::new()
                };

                format!(
                    "Python Version: {}\nExecutable: {}\nInclude System Site Packages: {}\n Installed packages: ({}):\n{}{}",
                    if meta.version.is_empty() {
                        "Unknown"
                    } else {
                        &meta.version
                    },
                    if meta.executable.is_empty() {
                        "Unknown"
                    } else {
                        &meta.executable
                    },
                    if meta.include_system_packages.is_empty() {
                        "Unknown"
                    } else {
                        &meta.include_system_packages
                    },
                    meta.packages.len(),
                    package_list,
                    overflow_note
                )
            }
            None => "Failed to read pyvenv.cfg for this environment target".to_string(),
        };
        format!("{}\n{}\n{}", base_path, size_info, config_details)
    };

    let details = Paragraph::new(details_content).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Environment details"),
    );
    f.render_widget(details, chunks[1]);

    let status_line = match &app.status {
        Some(msg) if !msg.is_expired() => {
            let (symbol, color) = match msg.level {
                MsgLevel::Error => ("✗", Color::Red),
                MsgLevel::Info => ("✓", Color::Green),
            };
            Line::from(Span::styled(
                format!(" {symbol} {}", msg.text),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ))
        }
        _ => Line::from(Span::styled(
            format!(
                " {} environment{} found",
                app.venvs.len(),
                if app.venvs.len() == 1 { "" } else { "s" }
            ),
            Style::default().fg(Color::DarkGray),
        )),
    };
    f.render_widget(Paragraph::new(status_line), status_area);

    f.render_widget(Paragraph::new(Line::from(footer_spans)), footer_area);

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
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
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
