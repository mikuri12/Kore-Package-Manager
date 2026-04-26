use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{backend::Backend, Terminal};
use crate::config::Config;
use crate::core::{remove_app, update_desktop_file};
use super::state::{App, Route, PopupType};
use crate::tui::components::Component;

pub async fn handle_key_events<B: Backend>(
    _terminal: &mut Terminal<B>,
    app: &mut App,
    config: &Config,
) -> anyhow::Result<bool> {
    if crossterm::event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = crossterm::event::read()? {
            if key.kind != KeyEventKind::Release {
                if key.code == KeyCode::F(12) {
                    if app.popup_type == PopupType::Logs {
                        app.popup_type = PopupType::None;
                    } else {
                        app.popup_type = PopupType::Logs;
                    }
                    return Ok(false);
                }

                if app.popup_type != PopupType::None {
                    let is_list_popup = matches!(app.popup_type,
                        PopupType::ActionSelect | PopupType::CategorySelect | PopupType::ChangeBinarySelect |
                        PopupType::ChangeRootSelect | PopupType::ConfirmUninstall | PopupType::InstallRootSelect |
                        PopupType::InstallCategorySelect | PopupType::RepoActionSelect | PopupType::RepoRootInput
                    );

                    let is_input_popup = matches!(app.popup_type,
                        PopupType::NameInput | PopupType::EnvVarInput | PopupType::InstallNameInput |
                        PopupType::RepoNameInput | PopupType::RepoPackageNameInput | PopupType::RepoUrlInput |
                        PopupType::RepoCategoryInput
                    );

                    if is_list_popup {
                        if matches!(key.code, KeyCode::Esc) {
                            app.popup_type = PopupType::None;
                            return Ok(false);
                        } else if matches!(key.code, KeyCode::Up | KeyCode::Char('k')) {
                            app.previous();
                            return Ok(false);
                        } else if matches!(key.code, KeyCode::Down | KeyCode::Char('j')) {
                            app.next();
                            return Ok(false);
                        }
                    } else if is_input_popup {
                        if matches!(key.code, KeyCode::Esc) {
                            app.popup_type = PopupType::None;
                            return Ok(false);
                        }
                    }

                    match app.popup_type {
                        PopupType::ActionSelect => match key.code {
                            KeyCode::Enter => {
                                let selected_action = app.popup_state.selected().unwrap_or(0);
                                if selected_action == 0 {
                                    app.open_popup_input(PopupType::NameInput, "");
                                } else if selected_action == 1 {
                                    app.open_popup_list(PopupType::CategorySelect, crate::core::get_all_categories(config));
                                } else if selected_action == 2 {
                                    if let Some(idx) = app.list_state.selected() {
                                        let selected_app = app.filtered[idx].clone();
                                        let target = config.install_dir.join(&selected_app);
                                        let executables = crate::utils::find_executables(&target, 3);
                                        app.pending_target = target;
                                        app.pending_executables = executables.clone();
                                        if executables.is_empty() {
                                            app.open_popup_info("No executable binary found for this app.");
                                        } else {
                                            let choices: Vec<String> = executables.iter().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).collect();
                                            app.open_popup_list(PopupType::ChangeBinarySelect, choices);
                                        }
                                    }
                                } else if selected_action == 3 {
                                    app.open_popup_list(PopupType::ChangeRootSelect, vec!["No".to_string(), "Yes (pkexec)".to_string()]);
                                } else if selected_action == 4 {
                                    if let Some(idx) = app.list_state.selected() {
                                        app.pending_icon_target = app.filtered[idx].clone();
                                        app.route = Route::IconBrowser;
                                        app.popup_type = PopupType::None;
                                        if let Some(bd) = directories::BaseDirs::new() {
                                            app.current_dir = bd.home_dir().to_path_buf();
                                        } else {
                                            app.current_dir = std::path::PathBuf::from("/");
                                        }
                                        app.load_dir();
                                    }
                                } else if selected_action == 5 {
                                    app.open_popup_input(PopupType::EnvVarInput, "");
                                } else {
                                    app.popup_type = PopupType::None;
                                }
                            }
                            _ => {}
                        },
                        PopupType::NameInput => match key.code {
                            KeyCode::Backspace => { app.popup_input.pop(); }
                            KeyCode::Char(c) => { 
                                if !['@', '$', '/', '\\', '|', '*', '?', '<', '>', ':', '\"'].contains(&c) {
                                    app.popup_input.push(c); 
                                }
                            }
                            KeyCode::Enter => {
                                if !app.popup_input.trim().is_empty() {
                                    if let Some(idx) = app.list_state.selected() {
                                        let selected_app = app.filtered[idx].clone();
                                        update_desktop_file(config, &selected_app, app.popup_input.trim(), "Name", true);
                                    }
                                }
                                app.popup_type = PopupType::None;
                                app.load_apps(config);
                            }
                            _ => {}
                        },
                        PopupType::CategorySelect => match key.code {
                            KeyCode::Enter => {
                                if let Some(cidx) = app.popup_state.selected() {
                                    if let Some(idx) = app.list_state.selected() {
                                        let selected_app = app.filtered[idx].clone();
                                        let cat = app.popup_items[cidx].clone();
                                        update_desktop_file(config, &selected_app, &cat, "Categories", true);
                                    }
                                }
                                app.popup_type = PopupType::None;
                                app.load_apps(config);
                            }
                            _ => {}
                        },
                        PopupType::ChangeBinarySelect => match key.code {
                            KeyCode::Enter => {
                                if let Some(bidx) = app.popup_state.selected() {
                                    app.pending_selected_exec = app.pending_executables[bidx].clone();
                                    if let Some(idx) = app.list_state.selected() {
                                        let selected_app = app.filtered[idx].clone();
                                        let bin_dest = config.bin_dir.join(&selected_app);
                                        if bin_dest.exists() {
                                            let _ = std::fs::remove_file(&bin_dest);
                                        }
                                        if let Err(e) = std::os::unix::fs::symlink(&app.pending_selected_exec, &bin_dest) {
                                            app.open_popup_info(&format!("Error creating symlink: {}", e));
                                        } else {
                                            let final_exec = bin_dest.to_string_lossy().to_string();
                                            crate::core::update_desktop_file(config, &selected_app, &final_exec, "Exec", true);
                                            app.cached_preview = None; // Reset preview
                                            app.open_popup_info("Binary successfully updated.");
                                        }
                                    }
                                } else {
                                    app.popup_type = PopupType::None;
                                }
                            }
                            _ => {}
                        },
                        PopupType::ChangeRootSelect => match key.code {
                            KeyCode::Enter => {
                                if let Some(ridx) = app.popup_state.selected() {
                                    if let Some(idx) = app.list_state.selected() {
                                        let selected_app = app.filtered[idx].clone();
                                        crate::core::update_exec_modifiers(config, &selected_app, Some(ridx == 1), None, true);
                                        app.open_popup_info("Root requirement updated.");
                                    }
                                } else {
                                    app.popup_type = PopupType::None;
                                }
                            }
                            _ => {}
                        },
                        PopupType::EnvVarInput => match key.code {
                            KeyCode::Backspace => { app.popup_input.pop(); }
                            KeyCode::Char(c) => { app.popup_input.push(c); }
                            KeyCode::Enter => {
                                if let Some(idx) = app.list_state.selected() {
                                    let selected_app = app.filtered[idx].clone();
                                    crate::core::update_exec_modifiers(config, &selected_app, None, Some(app.popup_input.trim().to_string()), true);
                                    app.open_popup_info("Environment variables updated.");
                                } else {
                                    app.popup_type = PopupType::None;
                                }
                            }
                            _ => {}
                        },
                        PopupType::ConfirmUninstall => match key.code {
                            KeyCode::Enter => {
                                let cidx = app.popup_state.selected().unwrap_or(1);
                                if cidx == 0 {
                                    if let Some(idx) = app.list_state.selected() {
                                        let selected_app = app.filtered[idx].clone();
                                        let _ = remove_app(config, &selected_app, true, true);
                                        app.open_popup_info(&format!("App {} successfully uninstalled.", selected_app));
                                        app.load_apps(config);
                                    }
                                } else {
                                    app.popup_type = PopupType::None;
                                }
                            }
                            _ => {}
                        },
                        PopupType::InstallNameInput => match key.code {
                            KeyCode::Backspace => { app.popup_input.pop(); }
                            KeyCode::Char(c) => { 
                                if !['@', '$', '/', '\\', '|', '*', '?', '<', '>', ':', '\"'].contains(&c) {
                                    app.popup_input.push(c); 
                                }
                            }
                            KeyCode::Enter => {
                                app.pending_app_name = if app.popup_input.trim().is_empty() {
                                    app.pending_raw_name.clone()
                                } else {
                                    app.popup_input.trim().to_string()
                                };
                                app.open_popup_list(PopupType::InstallRootSelect, vec!["No".to_string(), "Yes (pkexec)".to_string()]);
                            }
                            _ => {}
                        },
                        PopupType::InstallRootSelect => match key.code {
                            KeyCode::Enter => {
                                app.pending_use_root = app.popup_state.selected().unwrap_or(0) == 1;
                                app.open_popup_list(PopupType::InstallCategorySelect, crate::core::get_all_categories(config));
                            }
                            _ => {}
                        },
                        PopupType::InstallCategorySelect => match key.code {
                            KeyCode::Enter => {
                                if let Some(cidx) = app.popup_state.selected() {
                                    app.pending_category = app.popup_items[cidx].clone();
                                    
                                    app.route = Route::Installer;
                                    app.installer = Some(crate::tui::components::installer::Installer::new(
                                        app.pending_tarball.to_string_lossy().to_string(),
                                        app.pending_app_name.clone(),
                                        app.pending_use_root,
                                        app.pending_category.clone(),
                                    ));
                                    app.popup_type = PopupType::None;
                                } else {
                                    app.popup_type = PopupType::None;
                                }
                            }
                            _ => {}
                        },
                        PopupType::InstallBinarySelect | PopupType::InstallAssetSelect | PopupType::InstallDesktopSelect => match key.code {
                            KeyCode::Esc => {
                                if let Some(tx) = app.pending_install_reply.take() {
                                    if app.popup_type == PopupType::InstallDesktopSelect {
                                        let _ = tx.send(app.popup_items.len().saturating_sub(1)); // Send 'Skip' on escape
                                    } else {
                                        let _ = tx.send(0); // Default to 0 on escape
                                    }
                                }
                                app.popup_type = PopupType::InstallProgress;
                            }
                            KeyCode::Up | KeyCode::Char('k') => { app.previous(); }
                            KeyCode::Down | KeyCode::Char('j') => { app.next(); }
                            KeyCode::Enter => {
                                if let Some(idx) = app.popup_state.selected() {
                                    if let Some(tx) = app.pending_install_reply.take() {
                                        let _ = tx.send(idx);
                                    }
                                } else {
                                    if let Some(tx) = app.pending_install_reply.take() {
                                        if app.popup_type == PopupType::InstallDesktopSelect {
                                            let _ = tx.send(app.popup_items.len().saturating_sub(1));
                                        } else {
                                            let _ = tx.send(0);
                                        }
                                    }
                                }
                                app.popup_type = PopupType::InstallProgress;
                            }
                            _ => {}
                        },
                        PopupType::Information => match key.code {
                            KeyCode::Esc | KeyCode::Enter => { 
                                app.popup_type = PopupType::None;
                                if app.route == Route::FileBrowser {
                                    app.route = Route::MainMenu;
                                    app.list_state.select(Some(0));
                                }
                            }
                            _ => {}
                        },
                        PopupType::Help => match key.code {
                            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => { 
                                app.popup_type = PopupType::None;
                                app.help_scroll = 0;
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.help_scroll = app.help_scroll.saturating_add(1);
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.help_scroll = app.help_scroll.saturating_sub(1);
                            }
                            _ => {}
                        },
                        PopupType::RepoActionSelect => match key.code {
                            KeyCode::Enter => {
                                let cidx = app.popup_state.selected().unwrap_or(0);
                                if app.popup_items.is_empty() { return Ok(false); }
                                let action = app.popup_items[cidx].clone();
                                
                                if action.contains("Install Application") {
                                    if let Some(idx) = app.list_state.selected() {
                                        if let Some(repo) = app.filtered_repos.get(idx) {
                                            let app_name = if !repo.repo.package_name.is_empty() {
                                                repo.repo.package_name.clone()
                                            } else {
                                                repo.repo.name.clone()
                                            };
                                            app.route = Route::Installer;
                                            app.installer = Some(crate::tui::components::installer::Installer::new(
                                                repo.repo.name.clone(),
                                                app_name,
                                                repo.repo.requires_root,
                                                repo.repo.category.clone(),
                                            ));
                                            app.popup_type = PopupType::None;
                                        }
                                    }
                                } else if action.contains("Remove Custom Repo") {
                                    if let Some(idx) = app.list_state.selected() {
                                        if let Some(repo) = app.filtered_repos.get(idx) {
                                            let _ = crate::repo::remove_user_repo(config, &repo.repo.name);
                                            app.open_popup_info("Repository successfully removed.");
                                            app.load_repos(config);
                                        }
                                    }
                                } else {
                                    app.popup_type = PopupType::None;
                                }
                            }
                            _ => {}
                        },
                        PopupType::RepoNameInput => match key.code {
                            KeyCode::Backspace => { app.popup_input.pop(); }
                            KeyCode::Char(c) => { 
                                if !['@', '$', '/', '\\', '|', '*', '?', '<', '>', ':', '\"'].contains(&c) {
                                    app.popup_input.push(c); 
                                }
                            }
                            KeyCode::Enter => {
                                if !app.popup_input.trim().is_empty() {
                                    app.pending_repo_name = app.popup_input.trim().to_string();
                                    app.open_popup_input(PopupType::RepoPackageNameInput, "");
                                }
                            }
                            _ => {}
                        },
                        PopupType::RepoPackageNameInput => match key.code {
                            KeyCode::Backspace => { app.popup_input.pop(); }
                            KeyCode::Char(c) => { 
                                if !['@', '$', '/', '\\', '|', '*', '?', '<', '>', ':', '\"', ' '].contains(&c) {
                                    app.popup_input.push(c); 
                                }
                            }
                            KeyCode::Enter => {
                                if !app.popup_input.trim().is_empty() {
                                    app.pending_repo_package_name = app.popup_input.trim().to_string();
                                    app.open_popup_input(PopupType::RepoUrlInput, "");
                                }
                            }
                            _ => {}
                        },
                        PopupType::RepoUrlInput => match key.code {
                            KeyCode::Backspace => { app.popup_input.pop(); }
                            KeyCode::Char(c) => { app.popup_input.push(c); }
                            KeyCode::Enter => {
                                if !app.popup_input.trim().is_empty() {
                                    app.pending_repo_url = app.popup_input.trim().to_string();
                                    app.open_popup_input(PopupType::RepoCategoryInput, "Utility");
                                }
                            }
                            _ => {}
                        },
                        PopupType::RepoCategoryInput => match key.code {
                            KeyCode::Backspace => { app.popup_input.pop(); }
                            KeyCode::Char(c) => { app.popup_input.push(c); }
                            KeyCode::Enter => {
                                if !app.popup_input.trim().is_empty() {
                                    app.pending_repo_category = app.popup_input.trim().to_string();
                                    app.open_popup_list(PopupType::RepoRootInput, vec!["No".to_string(), "Yes".to_string()]);
                                }
                            }
                            _ => {}
                        },
                        PopupType::RepoRootInput => match key.code {
                            KeyCode::Enter => {
                                app.pending_repo_root = app.popup_state.selected().unwrap_or(0) == 1;
                                match crate::repo::add_user_repo(
                                    config, 
                                    &app.pending_repo_name, 
                                    &app.pending_repo_package_name,
                                    &app.pending_repo_url, 
                                    &app.pending_repo_category, 
                                    app.pending_repo_root
                                ).await {
                                    Ok(_) => app.open_popup_info("Custom repository added successfully!"),
                                    Err(e) => app.open_popup_info(&format!("Failed: {}", e)),
                                }
                                app.load_repos(config);
                            }
                            _ => {}
                        },
                        PopupType::Logs => match key.code {
                            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::F(12) => {
                                app.popup_type = PopupType::None;
                            }
                            KeyCode::Char('?') => {
                                app.popup_type = PopupType::Help;
                                app.help_scroll = 0;
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.logs_scroll = app.logs_scroll.saturating_add(1);
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.logs_scroll = app.logs_scroll.saturating_sub(1);
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                    return Ok(false);
                }

                let action = match app.route {
                    Route::MainMenu => {
                        let mut c = crate::tui::components::main_menu::MainMenu::new();
                        c.handle_key_event(key, app, config)?
                    }
                    Route::ManageApps | Route::RemoveApps => {
                        let mut c = crate::tui::components::app_manager::AppManager::new();
                        c.handle_key_event(key, app, config)?
                    }
                    Route::FileBrowser | Route::IconBrowser => {
                        let mut c = crate::tui::components::file_browser::FileBrowser::new();
                        c.handle_key_event(key, app, config)?
                    }
                    Route::RepoCategorySelect | Route::ManageRepos => {
                        let mut c = crate::tui::components::repo_manager::RepoManager::new();
                        c.handle_key_event(key, app, config)?
                    }
                    Route::Installer => {
                        let mut action = None;
                        if let Some(mut c) = app.installer.take() {
                            action = c.handle_key_event(key, app, config)?;
                            app.installer = Some(c);
                        }
                        action
                    }
                };

                if let Some(act) = action {
                    match act {
                        crate::tui::components::AppAction::Quit => return Ok(true),
                        crate::tui::components::AppAction::ChangeRoute(route) => app.route = route,
                        crate::tui::components::AppAction::ShowPopup(popup) => app.popup_type = popup,
                        crate::tui::components::AppAction::ClosePopup => app.popup_type = PopupType::None,
                        _ => {}
                    }
                }

            }
        }
    }
    Ok(false)
}
