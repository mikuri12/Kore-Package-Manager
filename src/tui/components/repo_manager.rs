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
use crate::tui::ui::centered_rect;

pub struct RepoManager {}

impl RepoManager {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for RepoManager {
    fn draw(&mut self, f: &mut Frame, area: Rect, app: &mut App, _config: &Config) {
        if app.route == Route::RepoCategorySelect {
            let items = vec![
                ListItem::new("󰒋 Official Repository"),
                ListItem::new("󰃇 Community Repositories"),
                ListItem::new("󰈔 My Custom Repositories"),
            ];

            let list = List::new(items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(" 󰒋 Select Repository Type ")
                    .border_style(Style::default().fg(Color::Cyan)))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol(">> ");

            let area = centered_rect(50, 40, f.area());
            f.render_stateful_widget(list, area, &mut app.repo_category_state);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(2), Constraint::Length(3)])
            .split(area);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[0]);

        let title = match app.viewing_repo_type {
            crate::repo::RepoType::Official => " 󰒋 Official Repository ",
            crate::repo::RepoType::Community => " 󰃇 Community Repositories ",
            crate::repo::RepoType::User => " 󰈔 My Custom Repositories ",
        };

        let items: Vec<ListItem> = app
            .filtered_repos
            .iter()
            .map(|r| ListItem::new(format!("󰏫 {}", r.repo.name)))
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

        let selected_repo = if let Some(idx) = app.list_state.selected() {
            app.filtered_repos.get(idx)
        } else {
            None
        };

        let preview_text = if let Some(r) = selected_repo {
            let formats_str = r.repo.formats.iter()
                .map(|f| match f.as_str() {
                    "appimage" => "AppImage",
                    "tarball" => "Tarball",
                    _ => f.as_str(),
                })
                .collect::<Vec<_>>()
                .join(", ");

            let mut info = format!(
                "--- REPOSITORY INFO ---\n\nName: {}\nType: {}\nURL: {}\nCategory: {}\nRequires Root: {}\nFormats: {}\n\n",
                r.repo.name,
                match r.repo_type {
                    crate::repo::RepoType::Official => "Official",
                    crate::repo::RepoType::Community => "Community",
                    crate::repo::RepoType::User => "Custom",
                },
                r.repo.url,
                r.repo.category,
                if r.repo.requires_root { "Yes" } else { "No" },
                formats_str,
            );

            if let Some(desc) = &r.repo.description {
                info.push_str(&format!("Description:\n{}\n\n", desc));
            }

            info.push_str("Press Enter for options\n");
            if app.viewing_repo_type == crate::repo::RepoType::User {
                info.push_str("Press 'A' to add new custom repository");
            }
            info
        } else {
            match app.viewing_repo_type {
                crate::repo::RepoType::Official => "No selection\n\nOfficial repositories are managed by the application.".to_string(),
                crate::repo::RepoType::Community => "No selection\n\nCommunity repositories are managed by the community.".to_string(),
                crate::repo::RepoType::User => "No selection\n\nPress 'A' to add new custom repository".to_string(),
            }
        };

        let preview = Paragraph::new(preview_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Details ")
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

    fn handle_key_event(&mut self, key: KeyEvent, app: &mut App, config: &Config) -> Result<Option<AppAction>> {
        if app.route == Route::RepoCategorySelect {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    return Ok(Some(AppAction::ChangeRoute(Route::MainMenu)));
                }
                KeyCode::Down | KeyCode::Char('j') => { app.next(); }
                KeyCode::Up | KeyCode::Char('k') => { app.previous(); }
                KeyCode::Enter => {
                    let s = app.repo_category_state.selected().unwrap_or(0);
                    app.viewing_repo_type = match s {
                        0 => crate::repo::RepoType::Official,
                        1 => crate::repo::RepoType::Community,
                        _ => crate::repo::RepoType::User,
                    };
                    app.load_repos(config);
                    return Ok(Some(AppAction::ChangeRoute(Route::ManageRepos)));
                }
                _ => {}
            }
            return Ok(None);
        }

        match key.code {
            KeyCode::Esc => {
                app.input.clear();
                return Ok(Some(AppAction::ChangeRoute(Route::RepoCategorySelect)));
            }
            KeyCode::Down => { app.next(); }
            KeyCode::Up => { app.previous(); }
            KeyCode::Backspace => {
                app.input.pop();
                app.filter_repos();
            }
            KeyCode::Char('A') | KeyCode::Char('a') if app.input.is_empty() => {
                if app.viewing_repo_type != crate::repo::RepoType::User {
                    app.open_popup_info("Only Custom Repositories can be modified. Go to My Custom Repositories to add your own.");
                    return Ok(Some(AppAction::ShowPopup(PopupType::Information)));
                } else {
                    app.open_popup_input(PopupType::RepoNameInput, "");
                    return Ok(Some(AppAction::ShowPopup(PopupType::RepoNameInput)));
                }
            }
            KeyCode::Char(c) => {
                app.input.push(c);
                app.filter_repos();
            }
            KeyCode::Enter => {
                if let Some(idx) = app.list_state.selected() {
                    if let Some(repo) = app.filtered_repos.get(idx) {
                        if repo.repo_type == crate::repo::RepoType::User {
                            app.open_popup_list(PopupType::RepoActionSelect, vec![
                                "󰏫 Install Application".to_string(),
                                "󰆴 Remove Custom Repo".to_string(),
                                "󰈆 Cancel".to_string()
                            ]);
                        } else {
                            app.open_popup_list(PopupType::RepoActionSelect, vec![
                                "󰏫 Install Application".to_string(),
                                "󰈆 Cancel".to_string()
                            ]);
                        }
                        return Ok(Some(AppAction::ShowPopup(PopupType::RepoActionSelect)));
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }
}
