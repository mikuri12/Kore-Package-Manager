use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::config::Config;
use crate::utils::find_executables;
use super::state::{App, Route, PopupType};

pub fn calculate_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

pub fn format_size(size: u64) -> String {
    let mb = size as f64 / 1_048_576.0;
    if mb > 1024.0 {
        format!("{:.2} GB", mb / 1024.0)
    } else {
        format!("{:.2} MB", mb)
    }
}

pub fn generate_preview(config: &Config, app_name: &str) -> String {
    let target = config.install_dir.join(app_name);
    let size = calculate_size(&target);
    let size_str = format_size(size);

    let associated_bin = config.bin_dir.join(app_name);
    let bin_str = if associated_bin.exists() {
        if let Ok(target_link) = fs::read_link(&associated_bin) {
            format!("Symlink to: {}", target_link.display())
        } else {
            "Referenced binary (unknown)".to_string()
        }
    } else {
        let execs = find_executables(&target, 3);
        if execs.is_empty() {
            "No bin tracker".to_string()
        } else {
            format!("Potential binary: {}", execs[0].file_name().unwrap_or_default().to_string_lossy())
        }
    };

    let mut preview = format!("--- DETAILS ---\n");
    preview.push_str(&format!("Size: {}\n", size_str));
    preview.push_str(&format!("Binary: {}\n", bin_str));
    preview.push_str("\n--- CONTENT ---\n");

    if let Ok(entries) = fs::read_dir(&target) {
        let mut count = 0;
        let mut files = Vec::new();
        for entry in entries.flatten().take(15) {
            let name = entry.file_name().to_string_lossy().to_string();
            let suffix = if entry.path().is_dir() { "/" } else { "" };
            files.push(format!("{}{}", name, suffix));
            count += 1;
        }

        files.sort();
        for f in files {
            preview.push_str(&format!("{}\n", f));
        }

        if count == 15 {
            preview.push_str("... (more files)\n");
        }
    }

    preview
}

pub fn generate_tar_preview(file_path: &Path) -> String {
    let mut preview = format!("--- TARBALL DETAILS ---\n");
    if let Ok(metadata) = fs::metadata(file_path) {
        preview.push_str(&format!("Size: {}\n", format_size(metadata.len())));
    }
    preview.push_str("\n--- LIMITED PREVIEW ---\n");
    
    use std::process::{Command, Stdio};
    use std::io::{BufReader, BufRead};
    
    if let Ok(mut child) = Command::new("tar")
        .args(["-tf", file_path.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().take(15) {
                if let Ok(l) = line {
                    preview.push_str(&format!("{}\n", l));
                }
            }
        }
        let _ = child.kill(); 
        let _ = child.wait();
    }
    
    preview
}

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

pub fn draw(f: &mut Frame, app: &mut App, config: &Config) {
    let current_route = &app.route;

    match current_route {
        Route::MainMenu => {
            let items = vec![
                ListItem::new("󰉍 Install New Tarball"),
                ListItem::new("󰏗 Manage Installed"),
                ListItem::new("󰆴 Uninstall Application"),
                ListItem::new("󰈆 Exit"),
            ];
            let list = List::new(items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(" 󰀼 TARBALL MANAGER ")
                    .border_style(Style::default().fg(Color::Cyan)))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol(">> ");

            let area = centered_rect(50, 40, f.area());
            f.render_stateful_widget(list, area, &mut app.list_state);
        }
        Route::ManageApps | Route::RemoveApps => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(2), Constraint::Length(3)])
                .split(f.area());

            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(chunks[0]);

            let title = if *current_route == Route::ManageApps {
                " 󰏗 Manage Installed "
            } else {
                " 󰆴 Uninstall App "
            };

            let items: Vec<ListItem> = app
                .filtered
                .iter()
                .map(|i| ListItem::new(i.clone()))
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

            let selected_item = if let Some(idx) = app.list_state.selected() {
                app.filtered.get(idx).cloned()
            } else {
                None
            };

            let preview_text = if let Some(ref name) = selected_item {
                if let Some((cached_name, cached_text)) = &app.cached_preview {
                    if cached_name == name {
                        cached_text.clone()
                    } else {
                        let text = generate_preview(config, name);
                        app.cached_preview = Some((name.clone(), text.clone()));
                        text
                    }
                } else {
                    let text = generate_preview(config, name);
                    app.cached_preview = Some((name.clone(), text.clone()));
                    text
                }
            } else {
                "No selection".to_string()
            };

            let preview = Paragraph::new(preview_text)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(" Preview ")
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
        Route::FileBrowser | Route::IconBrowser => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(2), Constraint::Length(3)])
                .split(f.area());

            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[0]);

            let items: Vec<ListItem> = app
                .fb_filtered
                .iter()
                .map(|i| ListItem::new(i.clone()))
                .collect();

            let title = if *current_route == Route::FileBrowser {
                format!(" 󰉋 {} ", app.current_dir.display())
            } else {
                format!(" 󰀩 Custom Icon: {} ", app.current_dir.display())
            };

            let list = List::new(items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(title)
                    .border_style(Style::default().fg(Color::Cyan)))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol(">> ");

            f.render_stateful_widget(list, main_chunks[0], &mut app.fb_state);

            let selected_item = if let Some(idx) = app.fb_state.selected() {
                app.fb_filtered.get(idx).cloned()
            } else {
                None
            };

            let preview_text = if let Some(ref name) = selected_item {
                if name.ends_with('/') || name == "../" || name == "./" {
                    "Directory".to_string()
                } else if *current_route == Route::IconBrowser {
                    if name.ends_with(".png") || name.ends_with(".svg") || name.ends_with(".ico") {
                        "Valid Icon Image".to_string()
                    } else {
                        "Unsupported File".to_string()
                    }
                } else if name.contains(".tar.") {
                    if let Some((cached_name, cached_text)) = &app.cached_preview {
                        if cached_name == name {
                            cached_text.clone()
                        } else {
                            let text = generate_tar_preview(&app.current_dir.join(name));
                            app.cached_preview = Some((name.clone(), text.clone()));
                            text
                        }
                    } else {
                        let text = generate_tar_preview(&app.current_dir.join(name));
                        app.cached_preview = Some((name.clone(), text.clone()));
                        text
                    }
                } else {
                    "File".to_string()
                }
            } else {
                "".to_string()
            };

            let preview = Paragraph::new(preview_text)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(" Preview ")
                    .border_style(Style::default().fg(Color::Green)))
                .wrap(ratatui::widgets::Wrap { trim: true });

            f.render_widget(preview, main_chunks[1]);
            
            let input_fb = Paragraph::new(format!("> {}", app.fb_input))
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(" Search (Type to filter) ")
                    .border_style(Style::default().fg(Color::White)));
            f.render_widget(input_fb, chunks[1]);
        }
    }

    if app.popup_type != PopupType::None {
        let area = centered_rect(50, 40, f.area());
        f.render_widget(Clear, area);

        match app.popup_type {
            PopupType::ActionSelect | PopupType::CategorySelect | PopupType::ChangeBinarySelect | PopupType::ChangeRootSelect | PopupType::ConfirmUninstall | PopupType::InstallRootSelect | PopupType::InstallCategorySelect | PopupType::InstallBinarySelect => {
                let popup_title = match app.popup_type {
                    PopupType::ActionSelect => " Action ",
                    PopupType::CategorySelect | PopupType::InstallCategorySelect => " Select Category ",
                    PopupType::ConfirmUninstall => " Are you sure? ",
                    PopupType::InstallRootSelect | PopupType::ChangeRootSelect => " Needs Root? ",
                    PopupType::InstallBinarySelect | PopupType::ChangeBinarySelect => " Select Main Binary ",
                    _ => " Options ",
                };

                let p_items: Vec<ListItem> = app
                    .popup_items
                    .iter()
                    .map(|i| ListItem::new(i.clone()))
                    .collect();

                let p_list = List::new(p_items)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(popup_title)
                        .border_style(Style::default().fg(Color::Cyan)))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol("󰇙 ");

                f.render_stateful_widget(p_list, area, &mut app.popup_state);
            }
            PopupType::NameInput | PopupType::InstallNameInput | PopupType::EnvVarInput => {
                let title = if app.popup_type == PopupType::NameInput { " New Name " } else if app.popup_type == PopupType::InstallNameInput { " Target Name " } else { " Environment Variables " };
                let mut content = format!("{}█", app.popup_input);
                if app.popup_type == PopupType::NameInput || app.popup_type == PopupType::InstallNameInput {
                    content.push_str("\n\n(No special characters allowed)");
                }
                let p = Paragraph::new(content)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(title)
                        .border_style(Style::default().fg(Color::Cyan)))
                    .wrap(ratatui::widgets::Wrap { trim: true });
                f.render_widget(p, area);
            }
            PopupType::Information => {
                let p = Paragraph::new(format!("{}\n\n[ Press Enter ]", app.popup_info))
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(" Information ")
                        .border_style(Style::default().fg(Color::Yellow)))
                    .wrap(ratatui::widgets::Wrap { trim: true });
                f.render_widget(p, area);
            }
            _ => {}
        }
    }
}
