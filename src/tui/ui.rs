use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use crate::config::Config;
use super::state::{App, Route};


pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

pub fn centered_rect_fixed_height(percent_x: u16, fixed_height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(fixed_height)) / 2),
            Constraint::Length(fixed_height),
            Constraint::Min(0),
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

use crate::tui::components::{Component, main_menu::MainMenu, app_manager::AppManager, file_browser::FileBrowser, repo_manager::RepoManager, popups::Popups};

pub fn draw(f: &mut Frame, app: &mut App, config: &Config) {
    let current_route = app.route;

    match current_route {
        Route::MainMenu => {
            let mut c = MainMenu::new();
            c.draw(f, f.area(), app, config);
        }
        Route::ManageApps | Route::RemoveApps | Route::UpdateApps => {
            let mut c = AppManager::new();
            c.draw(f, f.area(), app, config);
        }
        Route::FileBrowser | Route::IconBrowser => {
            let mut c = FileBrowser::new();
            c.draw(f, f.area(), app, config);
        }
        Route::RepoCategorySelect | Route::ManageRepos => {
            let mut c = RepoManager::new();
            c.draw(f, f.area(), app, config);
        }
        Route::Installer => {
            if let Some(mut c) = app.installer.take() {
                c.draw(f, f.area(), app, config);
                app.installer = Some(c);
            }
        }
    }

    let mut popups = Popups::new();
    popups.draw(f, f.area(), app, config);
}

