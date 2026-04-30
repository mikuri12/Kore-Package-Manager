#![allow(clippy::manual_range_contains)]

pub mod state;
pub mod ui;
pub mod handlers;
pub mod components;

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

pub async fn main_menu(config: &Config) -> anyhow::Result<()> {
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

        if let Some(rx) = &mut app.install_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    crate::core::install::InstallMessage::Progress(status, prog) => {
                        app.install_status = status.clone();
                        app.logs.push(status);
                        app.install_progress = prog;
                        if prog >= 100.0 || prog < 0.0 {
                            app.install_done = true;
                        }
                    }
                    crate::core::install::InstallMessage::SelectAsset(names, reply_tx) => {
                        app.popup_type = state::PopupType::InstallAssetSelect;
                        app.popup_items = names;
                        app.popup_state.select(Some(0));
                        app.pending_install_reply = Some(reply_tx);
                    }
                    crate::core::install::InstallMessage::SelectBinary(names, reply_tx) => {
                        app.popup_type = state::PopupType::InstallBinarySelect;
                        app.popup_items = names;
                        app.popup_state.select(Some(0));
                        app.pending_install_reply = Some(reply_tx);
                    }
                    crate::core::install::InstallMessage::SelectDesktop(names, reply_tx) => {
                        app.popup_type = state::PopupType::InstallDesktopSelect;
                        app.popup_items = names;
                        app.popup_state.select(Some(0));
                        app.pending_install_reply = Some(reply_tx);
                    }
                }
            }
        }
        
        if let Some(rx) = &mut app.preview_rx {
            while let Ok((name, text)) = rx.try_recv() {
                app.cached_preview = Some((name, text));
            }
        }
        
        if app.install_done {
            app.install_done = false;
            app.install_rx = None;
            
            if let Some(app_to_update) = app.update_queue.pop() {
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                app.install_rx = Some(rx);
                let all_repos = crate::repo::get_all_repos(config);
                if let Some(repo_source) = all_repos.iter().find(|r| {
                    let name_cmp = r.repo.name.to_lowercase().replace(' ', "-");
                    let pkg_cmp = r.repo.package_name.to_lowercase().replace(' ', "-");
                    let app_cmp = app_to_update.to_lowercase();
                    name_cmp == app_cmp || (!r.repo.package_name.is_empty() && pkg_cmp == app_cmp)
                }) {
                    let config_clone = config.clone();
                    let source = repo_source.repo.name.clone();
                    let app_name_opt = Some(app_to_update.clone());
                    tokio::spawn(async move {
                        let _ = crate::core::install_app(&config_clone, &source, app_name_opt.as_deref(), None, None, false, Some(tx), true).await;
                    });
                } else {
                    app.install_done = true; // immediately trigger next tick if failed
                }
            } else {
                app.popup_type = state::PopupType::Information;
                app.popup_info = format!("Installation Finished!\n\n{}", app.install_status);
                app.load_apps(config);
            }
        }

        let should_quit = handlers::handle_key_events(&mut terminal, &mut app, config).await?;
        if should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    Ok(())
}
