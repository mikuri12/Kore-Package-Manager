use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

#[allow(dead_code)]
pub enum AppAction {
    Quit,
    RefreshList,
    OpenPopup,
    ClosePopup,
    // Add more actions as needed to communicate with App state
}

#[allow(dead_code)]
pub trait Component {
    /// Draw the component in a specific area of the screen
    fn draw(&mut self, f: &mut Frame, area: Rect);
    
    /// Handle a local keyboard event. Returns an optional action to notify the parent `App`
    fn handle_key_event(&mut self, _key: KeyEvent) -> Result<Option<AppAction>> {
        Ok(None)
    }
}
