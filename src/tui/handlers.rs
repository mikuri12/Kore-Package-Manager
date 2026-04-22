use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{backend::Backend, Terminal};
use tm::config::Config;
use tm::core::{remove_app, update_desktop_file};
use super::state::{App, Route, PopupType};

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
                                    app.open_popup_list(PopupType::CategorySelect, tm::core::get_all_categories(config));
                                } else if selected_action == 2 {
                                    if let Some(idx) = app.list_state.selected() {
                                        let selected_app = app.filtered[idx].clone();
                                        let target = config.install_dir.join(&selected_app);
                                        let executables = tm::utils::find_executables(&target, 3);
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
                                            tm::core::update_desktop_file(config, &selected_app, &final_exec, "Exec", true);
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
                                        tm::core::update_exec_modifiers(config, &selected_app, Some(ridx == 1), None, true);
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
                                    tm::core::update_exec_modifiers(config, &selected_app, None, Some(app.popup_input.trim().to_string()), true);
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
                                app.open_popup_list(PopupType::InstallCategorySelect, tm::core::get_all_categories(config));
                            }
                            _ => {}
                        },
                        PopupType::InstallCategorySelect => match key.code {
                            KeyCode::Enter => {
                                if let Some(cidx) = app.popup_state.selected() {
                                    app.pending_category = app.popup_items[cidx].clone();
                                    
                                    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                                    app.install_rx = Some(rx);
                                    app.install_status = "Starting local installation...".to_string();
                                    app.install_progress = 0.0;
                                    app.install_done = false;
                                    app.popup_type = PopupType::InstallProgress;
                                    
                                    let config_clone = (*config).clone();
                                    let source = app.pending_tarball.to_string_lossy().to_string();
                                    let app_name = app.pending_app_name.clone();
                                    let use_root = if app.pending_use_root { "yes".to_string() } else { "no".to_string() };
                                    let category = app.pending_category.clone();
                                    
                                    tokio::spawn(async move {
                                        let _ = tm::core::install_app(
                                            &config_clone,
                                            &source,
                                            Some(&app_name),
                                            Some(&use_root),
                                            Some(&category),
                                            false,
                                            Some(tx)
                                        ).await;
                                    });
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
                                            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                                            app.install_rx = Some(rx);
                                            app.install_status = "Starting download...".to_string();
                                            app.install_progress = 0.0;
                                            app.install_done = false;
                                            app.popup_type = PopupType::InstallProgress;
                                            
                                            let config_clone = (*config).clone();
                                            let repo_name = repo.repo.name.clone();
                                            
                                            tokio::spawn(async move {
                                                let _ = tm::core::install_app(
                                                    &config_clone,
                                                    &repo_name,
                                                    Some(&repo_name),
                                                    None,
                                                    None,
                                                    false,
                                                    Some(tx)
                                                ).await;
                                            });
                                        }
                                    }
                                } else if action.contains("Remove Custom Repo") {
                                    if let Some(idx) = app.list_state.selected() {
                                        if let Some(repo) = app.filtered_repos.get(idx) {
                                            let _ = tm::repo::remove_user_repo(config, &repo.repo.name);
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
                                match tm::repo::add_user_repo(
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

                match app.route {
                    Route::MainMenu => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => { return Ok(true); }
                        KeyCode::Down | KeyCode::Char('j') => { app.next(); }
                        KeyCode::Up | KeyCode::Char('k') => { app.previous(); }
                        KeyCode::Char('?') => {
                            app.popup_type = PopupType::Help;
                            app.help_scroll = 0;
                        }
                        KeyCode::Enter => {
                            let s = app.list_state.selected().unwrap_or(0);
                            match s {
                                0 => {
                                    app.route = Route::FileBrowser;
                                    app.load_dir();
                                }
                                1 => {
                                    app.route = Route::ManageApps;
                                    app.load_apps(config);
                                }
                                2 => {
                                    app.route = Route::RemoveApps;
                                    app.load_apps(config);
                                }
                                3 => {
                                    app.route = Route::RepoCategorySelect;
                                    app.repo_category_state.select(Some(0));
                                }
                                _ => { return Ok(true); }
                            }
                        }
                        _ => {}
                    },
                    Route::ManageApps => match key.code {
                        KeyCode::Esc => {
                            app.route = Route::MainMenu;
                            app.list_state.select(Some(1));
                            app.input.clear();
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
                                app.open_popup_list(PopupType::ActionSelect, vec![
                                    "󰏫 Modify Name".to_string(), 
                                    "󰟝 Change Category".to_string(), 
                                    "󰒍 Change Binary".to_string(), 
                                    "󰌋 Toggle Root".to_string(), 
                                    "󰀩 Change Icon".to_string(), 
                                    "󰏫 Set Env Variables".to_string(),
                                    "󰈆 Return".to_string()
                                ]);
                            }
                        }
                        _ => {}
                    },
                    Route::RemoveApps => match key.code {
                        KeyCode::Esc => {
                            app.route = Route::MainMenu;
                            app.list_state.select(Some(2));
                            app.input.clear();
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
                                app.open_popup_list(PopupType::ConfirmUninstall, vec!["Yes, uninstall".to_string(), "No, cancel".to_string()]);
                            }
                        }
                        _ => {}
                    },
                    Route::FileBrowser => match key.code {
                        KeyCode::Esc => {
                            app.route = Route::MainMenu;
                            app.list_state.select(Some(0));
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
                                    // continue equivalent
                                } else if choice.ends_with('/') {
                                    app.current_dir = app.current_dir.join(&choice[..choice.len() - 1]);
                                    app.load_dir();
                                } else if choice.contains(".tar.") || choice.ends_with(".zip") {
                                    let tarball_path = app.current_dir.join(choice);
                                    app.pending_tarball = tarball_path;
                                    app.pending_raw_name = choice.replace(".tar.gz", "")
                                        .replace(".tar.xz", "")
                                        .replace(".tar.bz2", "")
                                        .replace(".zip", "");
                                        
                                    app.open_popup_input(PopupType::InstallNameInput, "");
                                }
                            }
                        }
                        _ => {}
                    },
                    Route::IconBrowser => match key.code {
                        KeyCode::Esc => {
                            app.route = Route::ManageApps;
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
                                    // continue equivalent
                                } else if choice.ends_with('/') {
                                    app.current_dir = app.current_dir.join(&choice[..choice.len() - 1]);
                                    app.load_dir();
                                } else if choice.ends_with(".png") || choice.ends_with(".svg") || choice.ends_with(".ico") {
                                    let icon_path = app.current_dir.join(choice).to_string_lossy().to_string();
                                    update_desktop_file(config, &app.pending_icon_target, &icon_path, "Icon", true);
                                    app.open_popup_info("Icon successfully updated.");
                                    app.route = Route::ManageApps;
                                    app.load_apps(config);
                                }
                            }
                        }
                        _ => {}
                    },
                    Route::RepoCategorySelect => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.route = Route::MainMenu;
                            app.list_state.select(Some(3));
                        }
                        KeyCode::Down | KeyCode::Char('j') => { app.next(); }
                        KeyCode::Up | KeyCode::Char('k') => { app.previous(); }
                        KeyCode::Enter => {
                            let s = app.repo_category_state.selected().unwrap_or(0);
                            app.viewing_repo_type = match s {
                                0 => tm::repo::RepoType::Official,
                                1 => tm::repo::RepoType::Community,
                                _ => tm::repo::RepoType::User,
                            };
                            app.route = Route::ManageRepos;
                            app.load_repos(config);
                        }
                        _ => {}
                    },
                    Route::ManageRepos => match key.code {
                        KeyCode::Esc => {
                            app.route = Route::RepoCategorySelect;
                            app.input.clear();
                        }
                        KeyCode::Down => { app.next(); }
                        KeyCode::Up => { app.previous(); }
                        KeyCode::Backspace => {
                            app.input.pop();
                            app.filter_repos();
                        }
                        KeyCode::Char('A') | KeyCode::Char('a') if app.input.is_empty() => {
                            if app.viewing_repo_type != tm::repo::RepoType::User {
                                app.open_popup_info("Only Custom Repositories can be modified. Go to My Custom Repositories to add your own.");
                            } else {
                                app.open_popup_input(PopupType::RepoNameInput, "");
                            }
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                            app.filter_repos();
                        }
                        KeyCode::Enter => {
                            if let Some(idx) = app.list_state.selected() {
                                if let Some(repo) = app.filtered_repos.get(idx) {
                                    if repo.repo_type == tm::repo::RepoType::User {
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
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(false)
}
