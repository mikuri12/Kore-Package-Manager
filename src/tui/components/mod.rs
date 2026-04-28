pub mod main_menu;
pub mod app_manager;
pub mod file_browser;
pub mod repo_manager;
pub mod popups;
pub mod installer;

use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::config::Config;
use crate::tui::state::{App, Route, PopupType};

#[allow(dead_code)]
pub enum AppAction {
    Quit,
    RefreshList,
    ChangeRoute(Route),
    ShowPopup(PopupType),
    ClosePopup,
}

#[allow(dead_code)]
pub trait Component {
    fn draw(&mut self, f: &mut Frame, area: Rect, app: &mut App, config: &Config);
    
    fn handle_key_event(&mut self, _key: KeyEvent, _app: &mut App, _config: &Config) -> Result<Option<AppAction>> {
        Ok(None)
    }
}
