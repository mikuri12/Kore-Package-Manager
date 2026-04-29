#![allow(clippy::collapsible_match)]

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
use crate::utils::fs::generate_preview;

pub struct AppManager {

}

impl AppManager {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for AppManager {
    fn draw(&mut self, f: &mut Frame, area: Rect, app: &mut App, config: &Config) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(2), Constraint::Length(3)])
            .split(area);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[0]);

        let title = if app.route == Route::ManageApps {
            " 󰏗 Manage Installed "
        } else {
            " 󰆴 Uninstall App "
        };

        let items: Vec<ListItem> = app
            .filtered
            .iter()
            .map(|i| ListItem::new(i.as_str()))
            .collect();

        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title)
                .border_style(Style::default().fg(Color::Cyan)))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, main_chunks[0], &mut app.list_state);

        let selected_item = if let Some(idx) = app.list_state.selected() {
            app.filtered.get(idx).cloned()
        } else {
            None
        };

        let preview_text = if let Some(ref name) = selected_item {
            if let Some((cached_name, cached_text)) = &app.cached_preview {
                if cached_name == name {
                    cached_text.clone()
                } else {
                    app.cached_preview = Some((name.clone(), "Loading preview...".to_string()));
                    let name_clone = name.clone();
                    let tx = app.preview_tx.clone();
                    let config_clone = config.clone();
                    tokio::task::spawn_blocking(move || {
                        let text = generate_preview(&config_clone, &name_clone);
                        let _ = tx.send((name_clone, text));
                    });
                    "Loading preview...".to_string()
                }
            } else {
                app.cached_preview = Some((name.clone(), "Loading preview...".to_string()));
                let name_clone = name.clone();
                let tx = app.preview_tx.clone();
                let config_clone = config.clone();
                tokio::task::spawn_blocking(move || {
                    let text = generate_preview(&config_clone, &name_clone);
                    let _ = tx.send((name_clone, text));
                });
                "Loading preview...".to_string()
            }
        } else {
            "No selection".to_string()
        };

        let preview = Paragraph::new(preview_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Preview ")
                .border_style(Style::default().fg(Color::Green)))
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(preview, main_chunks[1]);

        let input_p = Paragraph::new(format!("> {}", app.input))
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(if app.popup_type != PopupType::None { " Hold " } else { " Search (Type to filter) " })
                .border_style(Style::default().fg(Color::White)));

        f.render_widget(input_p, chunks[1]);
    }

    fn handle_key_event(&mut self, key: KeyEvent, app: &mut App, _config: &Config) -> Result<Option<AppAction>> {
        match key.code {
            KeyCode::Esc => {
                app.input.clear();
                let _target_idx = if app.route == Route::ManageApps { 1 } else { 2 };
                return Ok(Some(AppAction::ChangeRoute(Route::MainMenu)));
            }
            KeyCode::Down => { app.next(); }
            KeyCode::Up => { app.previous(); }
            KeyCode::Backspace => {
                app.input.pop();
                app.filter_apps();
            }
            KeyCode::Char(c) => {
                app.input.push(c);
                app.filter_apps();
            }
            KeyCode::Enter => {
                if !app.filtered.is_empty() {
                    if app.route == Route::ManageApps {
                        app.open_popup_list(PopupType::ActionSelect, vec![
                            "󰏫 Modify Name".to_string(), 
                            "󰟝 Change Category".to_string(), 
                            "󰒍 Change Binary".to_string(), 
                            "󰌋 Toggle Root".to_string(), 
                            "󰀩 Change Icon".to_string(), 
                            "󰏫 Set Env Variables".to_string(),
                            "󰈆 Return".to_string()
                        ]);
                        return Ok(Some(AppAction::ShowPopup(PopupType::ActionSelect)));
                    } else {
                        app.open_popup_list(PopupType::ConfirmUninstall, vec![
                            "Yes, uninstall".to_string(), 
                            "No, cancel".to_string()
                        ]);
                        return Ok(Some(AppAction::ShowPopup(PopupType::ConfirmUninstall)));
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }
}
