use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{backend::Backend, Terminal};
use crate::config::Config;
use crate::core::{remove_app, update_desktop_file};
use super::state::{App, Route, PopupType};
use super::ui::centered_rect;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::style::{Color, Style};

pub fn handle_key_events<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    config: &Config,
) -> anyhow::Result<bool> {
    if crossterm::event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = crossterm::event::read()? {
            if key.kind != KeyEventKind::Release {
                
                if app.popup_type != PopupType::None {
                    match app.popup_type {
                        PopupType::ActionSelect => match key.code {
                            KeyCode::Esc => { app.popup_type = PopupType::None; }
                            KeyCode::Up => { app.previous(); }
                            KeyCode::Down => { app.next(); }
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
                            KeyCode::Esc => { app.popup_type = PopupType::None; }
                            KeyCode::Backspace => { app.popup_input.pop(); }
                            KeyCode::Char(c) => { app.popup_input.push(c); }
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
                            KeyCode::Esc => { app.popup_type = PopupType::None; }
                            KeyCode::Up => { app.previous(); }
                            KeyCode::Down => { app.next(); }
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
                            KeyCode::Esc => { app.popup_type = PopupType::None; }
                            KeyCode::Up => { app.previous(); }
                            KeyCode::Down => { app.next(); }
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
                            KeyCode::Esc => { app.popup_type = PopupType::None; }
                            KeyCode::Up => { app.previous(); }
                            KeyCode::Down => { app.next(); }
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
                            KeyCode::Esc => { app.popup_type = PopupType::None; }
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
                            KeyCode::Esc => { app.popup_type = PopupType::None; }
                            KeyCode::Up => { app.previous(); }
                            KeyCode::Down => { app.next(); }
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
                            KeyCode::Esc => { app.popup_type = PopupType::None; }
                            KeyCode::Backspace => { app.popup_input.pop(); }
                            KeyCode::Char(c) => { app.popup_input.push(c); }
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
                            KeyCode::Esc => { app.popup_type = PopupType::None; }
                            KeyCode::Up => { app.previous(); }
                            KeyCode::Down => { app.next(); }
                            KeyCode::Enter => {
                                app.pending_use_root = app.popup_state.selected().unwrap_or(0) == 1;
                                app.open_popup_list(PopupType::InstallCategorySelect, crate::core::get_all_categories(config));
                            }
                            _ => {}
                        },
                        PopupType::InstallCategorySelect => match key.code {
                            KeyCode::Esc => { app.popup_type = PopupType::None; }
                            KeyCode::Up => { app.previous(); }
                            KeyCode::Down => { app.next(); }
                            KeyCode::Enter => {
                                if let Some(cidx) = app.popup_state.selected() {
                                    app.pending_category = app.popup_items[cidx].clone();
                                    
                                    app.open_popup_info("Extracting tarball... Please wait.");
                                    let _ = terminal.draw(|f| { 
                                        let area = centered_rect(50, 40, f.area());
                                        f.render_widget(Clear, area);
                                        let p = Paragraph::new(format!("{}\n\n[ Waiting ]", app.popup_info))
                                            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(" Information ").border_style(Style::default().fg(Color::Yellow)))
                                            .wrap(ratatui::widgets::Wrap { trim: true });
                                        f.render_widget(p, area);
                                    });

                                    if let Ok(Some((target, _raw, executables))) = crate::core::extract_and_scan(config, &app.pending_tarball, true) {
                                        app.pending_target = target;
                                        app.pending_executables = executables;
                                        
                                        if app.pending_executables.is_empty() {
                                            app.open_popup_info("No executable binary found in tarball.");
                                        } else if app.pending_executables.len() == 1 {
                                            app.pending_selected_exec = app.pending_executables[0].clone();
                                            let _ = crate::core::finalize_installation(config, &app.pending_target, &app.pending_selected_exec, &app.pending_app_name, app.pending_use_root, &app.pending_category, true);
                                            app.open_popup_info(&format!("Installation completed for: {}", app.pending_app_name));
                                        } else {
                                            let choices: Vec<String> = app.pending_executables.iter().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).collect();
                                            app.open_popup_list(PopupType::InstallBinarySelect, choices);
                                        }
                                    } else {
                                        app.open_popup_info("Extraction failed or tarball invalid.");
                                    }
                                } else {
                                    app.popup_type = PopupType::None;
                                }
                            }
                            _ => {}
                        },
                        PopupType::InstallBinarySelect => match key.code {
                            KeyCode::Esc => { app.popup_type = PopupType::None; }
                            KeyCode::Up => { app.previous(); }
                            KeyCode::Down => { app.next(); }
                            KeyCode::Enter => {
                                if let Some(bidx) = app.popup_state.selected() {
                                    app.pending_selected_exec = app.pending_executables[bidx].clone();
                                    let _ = crate::core::finalize_installation(config, &app.pending_target, &app.pending_selected_exec, &app.pending_app_name, app.pending_use_root, &app.pending_category, true);
                                    app.open_popup_info(&format!("Installation completed for: {}", app.pending_app_name));
                                } else {
                                    app.popup_type = PopupType::None;
                                }
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
                        _ => {}
                    }
                    return Ok(false);
                }

                match app.route {
                    Route::MainMenu => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => { return Ok(true); }
                        KeyCode::Down => { app.next(); }
                        KeyCode::Up => { app.previous(); }
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
                                } else if choice.contains(".tar.") {
                                    let tarball_path = app.current_dir.join(choice);
                                    app.pending_tarball = tarball_path;
                                    app.pending_raw_name = choice.replace(".tar.gz", "")
                                        .replace(".tar.xz", "")
                                        .replace(".tar.bz2", "");
                                        
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
                    }
                }
            }
        }
    }
    Ok(false)
}
