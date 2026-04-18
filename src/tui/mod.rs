pub mod state;
pub mod ui;
pub mod handlers;

use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use directories::BaseDirs;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;

use crate::config::Config;
use state::App;

pub fn main_menu(config: &Config) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let home = match BaseDirs::new() {
        Some(bd) => bd.home_dir().to_path_buf(),
        None => PathBuf::from("/"),
    };

    let mut app = App::new(&home);
    app.list_state.select(Some(0));

    loop {
        terminal.draw(|f| {
            ui::draw(f, &mut app, config);
        })?;

        let should_quit = handlers::handle_key_events(&mut terminal, &mut app, config)?;
        if should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    Ok(())
}
