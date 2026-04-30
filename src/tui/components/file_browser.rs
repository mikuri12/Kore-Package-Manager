use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::config::Config;
use crate::tui::state::{App, Route, PopupType};
use super::{AppAction, Component};
use crate::utils::fs::generate_archive_preview;

pub struct FileBrowser {}

impl FileBrowser {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for FileBrowser {
    fn draw(&mut self, f: &mut Frame, area: Rect, app: &mut App, _config: &Config) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(2), Constraint::Length(3)])
            .split(area);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[0]);

        let items: Vec<ListItem> = app
            .fb_filtered
            .iter()
            .map(|i| ListItem::new(i.as_str()))
            .collect();

        let title = if app.route == Route::FileBrowser {
            format!(" 󰉋 {} ", app.current_dir.display())
        } else {
            format!(" 󰀩 Custom Icon: {} ", app.current_dir.display())
        };

        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title)
                .border_style(Style::default().fg(Color::Cyan)))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, main_chunks[0], &mut app.fb_state);

        let selected_item = if let Some(idx) = app.fb_state.selected() {
            app.fb_filtered.get(idx).cloned()
        } else {
            None
        };

        let preview_text = if let Some(ref name) = selected_item {
            if name.ends_with('/') || name == "../" || name == "./" {
                "Directory".to_string()
            } else if app.route == Route::IconBrowser {
                if name.ends_with(".png") || name.ends_with(".svg") || name.ends_with(".ico") {
                    "Valid Icon Image".to_string()
                } else {
                    "Unsupported File".to_string()
                }
            } else if name.contains(".tar.") || name.ends_with(".zip") || name.to_lowercase().ends_with(".appimage") {
                if let Some((cached_name, cached_text)) = &app.cached_preview {
                    if cached_name == name {
                        cached_text.clone()
                    } else {
                        app.cached_preview = Some((name.clone(), "Loading archive preview...".to_string()));
                        let name_clone = name.clone();
                        let target_path = app.current_dir.join(name);
                        let tx = app.preview_tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let text = generate_archive_preview(&target_path);
                            let _ = tx.send((name_clone, text));
                        });
                        "Loading archive preview...".to_string()
                    }
                } else {
                    app.cached_preview = Some((name.clone(), "Loading archive preview...".to_string()));
                    let name_clone = name.clone();
                    let target_path = app.current_dir.join(name);
                    let tx = app.preview_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let text = generate_archive_preview(&target_path);
                        let _ = tx.send((name_clone, text));
                    });
                    "Loading archive preview...".to_string()
                }
            } else {
                "File".to_string()
            }
        } else {
            "".to_string()
        };

        let preview = Paragraph::new(preview_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Preview ")
                .border_style(Style::default().fg(Color::Green)))
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(preview, main_chunks[1]);
        
        let input_fb = Paragraph::new(format!("> {}", app.fb_input))
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Search (Type to filter) ")
                .border_style(Style::default().fg(Color::White)));
        f.render_widget(input_fb, chunks[1]);
    }

    fn handle_key_event(&mut self, key: KeyEvent, app: &mut App, config: &Config) -> Result<Option<AppAction>> {
        match key.code {
            KeyCode::Esc => {
                if app.route == Route::FileBrowser {
                    return Ok(Some(AppAction::ChangeRoute(Route::MainMenu)));
                } else {
                    return Ok(Some(AppAction::ChangeRoute(Route::ManageApps)));
                }
            }
            KeyCode::Down => { app.next(); }
            KeyCode::Up => { app.previous(); }
            KeyCode::Backspace => {
                app.fb_input.pop();
                app.filter_fb();
            }
            KeyCode::Char(c) => {
                app.fb_input.push(c);
                app.filter_fb();
            }
            KeyCode::Enter => {
                if let Some(idx) = app.fb_state.selected() {
                    let choice = &app.fb_filtered[idx];
                    if choice == "../" {
                        if let Some(parent) = app.current_dir.parent() {
                            app.current_dir = parent.to_path_buf();
                            app.load_dir();
                        }
                    } else if choice == "./" {
                    } else if choice.ends_with('/') {
                        app.current_dir = app.current_dir.join(&choice[..choice.len() - 1]);
                        app.load_dir();
                    } else if app.route == Route::FileBrowser && (choice.contains(".tar.") || choice.ends_with(".zip") || choice.to_lowercase().ends_with(".appimage")) {
                        let tarball_path = app.current_dir.join(choice);
                        app.pending_tarball = tarball_path;
                        app.pending_raw_name = choice.replace(".tar.gz", "")
                            .replace(".tar.xz", "")
                            .replace(".tar.bz2", "")
                            .replace(".zip", "")
                            .replace(".AppImage", "")
                            .replace(".appimage", "");
                            
                        app.open_popup_input(PopupType::InstallNameInput, "");
                        return Ok(Some(AppAction::ShowPopup(PopupType::InstallNameInput)));
                    } else if app.route == Route::IconBrowser && (choice.ends_with(".png") || choice.ends_with(".svg") || choice.ends_with(".ico")) {
                        let icon_path = app.current_dir.join(choice).to_string_lossy().to_string();
                        crate::core::update_desktop_file(config, &app.pending_icon_target, &icon_path, "Icon", true);
                        app.open_popup_info("Icon successfully updated.");
                        app.load_apps(config);
                        return Ok(Some(AppAction::ChangeRoute(Route::ManageApps)));
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }
}
