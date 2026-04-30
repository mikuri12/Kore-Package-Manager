use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem},
    Frame,
};

use crate::config::Config;
use crate::tui::state::{App, Route, PopupType};
use super::{AppAction, Component};
use crate::tui::ui::centered_rect;

pub struct MainMenu {}

impl MainMenu {
    pub fn new() -> Self {
        Self {}
    }

    fn next(app: &mut App) {
        let i = match app.list_state.selected() {
            Some(i) => (i + 1) % 6,
            None => 0,
        };
        app.list_state.select(Some(i));
    }

    fn previous(app: &mut App) {
        let i = match app.list_state.selected() {
            Some(i) => if i == 0 { 5 } else { i - 1 },
            None => 0,
        };
        app.list_state.select(Some(i));
    }
}

impl Component for MainMenu {
    fn draw(&mut self, f: &mut Frame, _area: Rect, app: &mut App, _config: &Config) {
        let items = vec![
            ListItem::new("󰉍 Install New Package"),
            ListItem::new("󰏗 Manage Installed"),
            ListItem::new("󰆴 Uninstall Application"),
            ListItem::new("󰚰 Update Applications"),
            ListItem::new("󰒋 Repositories"),
            ListItem::new("󰈆 Exit"),
        ];
        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" 󰀼 KORE PACKAGE MANAGER ")
                .border_style(Style::default().fg(Color::Cyan)))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

        let area = centered_rect(50, 40, f.area());
        f.render_stateful_widget(list, area, &mut app.list_state);

        let help_rect = ratatui::layout::Rect::new(0, f.area().height.saturating_sub(2), f.area().width, 1);
        let help_text = ratatui::widgets::Paragraph::new("Press (?) for help")
            .style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray).add_modifier(ratatui::style::Modifier::ITALIC))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(help_text, help_rect);
    }

    fn handle_key_event(&mut self, key: KeyEvent, app: &mut App, config: &Config) -> Result<Option<AppAction>> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { return Ok(Some(AppAction::Quit)); }
            KeyCode::Down | KeyCode::Char('j') => { Self::next(app); }
            KeyCode::Up | KeyCode::Char('k') => { Self::previous(app); }
            KeyCode::Char('?') => {
                return Ok(Some(AppAction::ShowPopup(PopupType::Help)));
            }
            KeyCode::Enter => {
                let s = app.list_state.selected().unwrap_or(0);
                match s {
                    0 => {
                        app.load_dir();
                        return Ok(Some(AppAction::ChangeRoute(Route::FileBrowser)));
                    }
                    1 => {
                        app.load_apps(config);
                        return Ok(Some(AppAction::ChangeRoute(Route::ManageApps)));
                    }
                    2 => {
                        app.load_apps(config);
                        return Ok(Some(AppAction::ChangeRoute(Route::RemoveApps)));
                    }
                    3 => {
                        app.load_updatable_apps(config);
                        return Ok(Some(AppAction::ChangeRoute(Route::UpdateApps)));
                    }
                    4 => {
                        app.repo_category_state.select(Some(0));
                        return Ok(Some(AppAction::ChangeRoute(Route::RepoCategorySelect)));
                    }
                    _ => { return Ok(Some(AppAction::Quit)); }
                }
            }
            _ => {}
        }
        Ok(None)
    }
}
