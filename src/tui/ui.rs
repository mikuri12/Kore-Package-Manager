use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use tm::config::Config;
use tm::utils::find_executables;
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

pub fn generate_archive_preview(file_path: &Path) -> String {
    let mut preview = format!("--- ARCHIVE DETAILS ---\n");
    if let Ok(metadata) = fs::metadata(file_path) {
        preview.push_str(&format!("Size: {}\n", format_size(metadata.len())));
    }
    preview.push_str("\n--- LIMITED PREVIEW ---\n");
    
    use std::process::{Command, Stdio};
    use std::io::{BufReader, BufRead};
    
    let is_zip = file_path.to_string_lossy().ends_with(".zip");
    
    let child_res = if is_zip {
        Command::new("unzip")
            .args(["-Z1", file_path.to_str().unwrap()])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
    } else {
        Command::new("tar")
            .args(["-tf", file_path.to_str().unwrap()])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
    };
    
    if let Ok(mut child) = child_res {
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

pub fn draw(f: &mut Frame, app: &mut App, config: &Config) {
    let current_route = &app.route;

    match current_route {
        Route::MainMenu => {
            let items = vec![
                ListItem::new("󰉍 Install New Tarball"),
                ListItem::new("󰏗 Manage Installed"),
                ListItem::new("󰆴 Uninstall Application"),
                ListItem::new("󰒋 Repositories"),
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

            let help_rect = ratatui::layout::Rect::new(0, f.area().height.saturating_sub(2), f.area().width, 1);
            let help_text = ratatui::widgets::Paragraph::new("Press (?) for help")
                .style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray).add_modifier(ratatui::style::Modifier::ITALIC))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(help_text, help_rect);
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
        Route::RepoCategorySelect => {
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
        }
        Route::ManageRepos => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(2), Constraint::Length(3)])
                .split(f.area());

            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(chunks[0]);

            let title = match app.viewing_repo_type {
                tm::repo::RepoType::Official => " 󰒋 Official Repository ",
                tm::repo::RepoType::Community => " 󰃇 Community Repositories ",
                tm::repo::RepoType::User => " 󰈔 My Custom Repositories ",
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
                let mut info = format!(
                    "--- REPOSITORY INFO ---\n\nName: {}\nType: {}\nURL: {}\nCategory: {}\nRequires Root: {}\n\n",
                    r.repo.name,
                    match r.repo_type {
                        tm::repo::RepoType::Official => "Official",
                        tm::repo::RepoType::Community => "Community",
                        tm::repo::RepoType::User => "Custom",
                    },
                    r.repo.url,
                    r.repo.category,
                    if r.repo.requires_root { "Yes" } else { "No" },
                );

                if let Some(desc) = &r.repo.description {
                    info.push_str(&format!("Description:\n{}\n\n", desc));
                }

                info.push_str("Press Enter for options\n");
                if app.viewing_repo_type == tm::repo::RepoType::User {
                    info.push_str("Press 'A' to add new custom repository");
                }
                info
            } else {
                match app.viewing_repo_type {
                    tm::repo::RepoType::Official => "No selection\n\nOfficial repositories are managed by the application.".to_string(),
                    tm::repo::RepoType::Community => "No selection\n\nCommunity repositories are managed by the community.".to_string(),
                    tm::repo::RepoType::User => "No selection\n\nPress 'A' to add new custom repository".to_string(),
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
                } else if name.contains(".tar.") || name.ends_with(".zip") {
                    if let Some((cached_name, cached_text)) = &app.cached_preview {
                        if cached_name == name {
                            cached_text.clone()
                        } else {
                            let text = generate_archive_preview(&app.current_dir.join(name));
                            app.cached_preview = Some((name.clone(), text.clone()));
                            text
                        }
                    } else {
                        let text = generate_archive_preview(&app.current_dir.join(name));
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
        let area = if app.popup_type == PopupType::Help {
            centered_rect(65, 60, f.area())
        } else {
            centered_rect(50, 40, f.area())
        };
        f.render_widget(Clear, area);

        match app.popup_type {
            PopupType::ActionSelect | PopupType::CategorySelect | PopupType::ChangeBinarySelect | PopupType::ChangeRootSelect | PopupType::ConfirmUninstall | PopupType::InstallRootSelect | PopupType::InstallCategorySelect | PopupType::InstallBinarySelect | PopupType::InstallAssetSelect | PopupType::InstallDesktopSelect | PopupType::RepoActionSelect | PopupType::RepoRootInput => {
                let popup_title = match app.popup_type {
                    PopupType::ActionSelect => " Action ",
                    PopupType::RepoActionSelect => " Repo Action ",
                    PopupType::CategorySelect | PopupType::InstallCategorySelect => " Select Category ",
                    PopupType::ConfirmUninstall => " Are you sure? ",
                    PopupType::InstallRootSelect | PopupType::ChangeRootSelect | PopupType::RepoRootInput => " Needs Root? ",
                    PopupType::InstallBinarySelect | PopupType::ChangeBinarySelect => " Select Main Binary ",
                    PopupType::InstallAssetSelect => " Select Tarball ",
                    PopupType::InstallDesktopSelect => " Select Desktop File ",
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
            PopupType::NameInput | PopupType::InstallNameInput | PopupType::EnvVarInput | PopupType::RepoNameInput | PopupType::RepoPackageNameInput | PopupType::RepoUrlInput | PopupType::RepoCategoryInput => {
                let title = match app.popup_type {
                    PopupType::NameInput => " New Name ",
                    PopupType::InstallNameInput => " Target Name ",
                    PopupType::RepoNameInput => " Repo Name ",
                    PopupType::RepoPackageNameInput => " Package Name ",
                    PopupType::RepoUrlInput => " Repo URL ",
                    PopupType::RepoCategoryInput => " Repo Category ",
                    _ => " Environment Variables "
                };
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
            PopupType::Help => {
                let version = env!("CARGO_PKG_VERSION");
                let bold_style = Style::default().add_modifier(Modifier::BOLD);
                let center = ratatui::layout::Alignment::Center;
                let dark_gray = Style::default().fg(Color::DarkGray);
                
                let lines = vec![
                    ratatui::text::Line::from("").alignment(center),
                    ratatui::text::Line::from("· Tarball Manager ·").alignment(center),
                    ratatui::text::Line::from("").alignment(center),
                    ratatui::text::Line::from("TUI Application to manage Linux tarballs,").alignment(center),
                    ratatui::text::Line::from("AppImages and binaries directly from the terminal.").alignment(center),
                    ratatui::text::Line::from("").alignment(center),
                    ratatui::text::Line::from(ratatui::text::Span::styled("─────────────────────────────────────────────────────────", dark_gray)).alignment(center),
                    ratatui::text::Line::from("").alignment(center),
                    ratatui::text::Line::from(ratatui::text::Span::styled(version, bold_style)).alignment(center),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(ratatui::text::Span::styled("DISCLAIMER", bold_style)),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from("  Tarball Manager is in an early alpha phase. It is highly susceptible"),
                    ratatui::text::Line::from("  to bugs and unexpected behavior. Please report any issues or feedback"),
                    ratatui::text::Line::from("  by opening an issue on the official GitHub repository."),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(ratatui::text::Span::styled("BASIC USAGE & KEYBINDS", bold_style)),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from("  • Navigate Lists: Up/Down Arrows or 'k'/'j'"),
                    ratatui::text::Line::from("  • Select/Confirm: Enter"),
                    ratatui::text::Line::from("  • Back/Cancel: Esc"),
                    ratatui::text::Line::from("  • Search/Filter: Type directly when a search bar is visible"),
                    ratatui::text::Line::from("  • Quit Application: 'q' or Esc (from Main Menu)"),
                    ratatui::text::Line::from("  • View this Help: '?'"),
                    ratatui::text::Line::from("  • Internal Logs: F12"),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(ratatui::text::Span::styled("Press Enter or Esc to close", dark_gray)).alignment(center),
                ];
                let help_area = centered_rect(65, 75, f.area());
                
                let max_scroll = lines.len().saturating_sub(help_area.height.saturating_sub(4) as usize) as u16;
                app.help_scroll = app.help_scroll.min(max_scroll);

                let p = Paragraph::new(lines)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .padding(ratatui::widgets::Padding::new(4, 4, 1, 1))
                        .title(ratatui::text::Line::from(ratatui::text::Span::styled(" Help ", Style::default().add_modifier(Modifier::ITALIC))).alignment(ratatui::layout::Alignment::Left))
                        .title(ratatui::text::Line::from(ratatui::text::Span::styled(" (?) ", Style::default().add_modifier(Modifier::ITALIC))).alignment(ratatui::layout::Alignment::Right))
                        .border_style(Style::default().fg(Color::DarkGray)))
                    .wrap(ratatui::widgets::Wrap { trim: true })
                    .scroll((app.help_scroll, 0));
                
                f.render_widget(Clear, help_area);
                f.render_widget(p, help_area);

                let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(max_scroll as usize).position(app.help_scroll as usize);
                let scrollbar = ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"));
                
                f.render_stateful_widget(
                    scrollbar,
                    help_area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 }),
                    &mut scrollbar_state,
                );
            }
            PopupType::Logs => {
                let area = centered_rect(80, 80, f.area());
                let logs_text: String = app.logs.iter()
                    .map(|l| format!("> {}\n", l))
                    .collect::<Vec<String>>()
                    .join("");
                
                let lines_count = app.logs.len();
                let max_scroll = lines_count.saturating_sub(area.height.saturating_sub(2) as usize) as u16;
                app.logs_scroll = app.logs_scroll.min(max_scroll);

                let p = Paragraph::new(logs_text)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(" 󰈔 Internal Logs ")
                        .border_style(Style::default().fg(Color::Yellow)))
                    .wrap(ratatui::widgets::Wrap { trim: true })
                    .scroll((app.logs_scroll, 0));
                
                f.render_widget(Clear, area);
                f.render_widget(p, area);

                let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(max_scroll as usize).position(app.logs_scroll as usize);
                let scrollbar = ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"));
                
                f.render_stateful_widget(
                    scrollbar,
                    area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 }),
                    &mut scrollbar_state,
                );
            }
            PopupType::InstallProgress => {
                let progress = app.install_progress;
                let valid_progress = progress.max(0.0).min(100.0);
                let label = format!("{} ({:.1}%)", app.install_status, progress);
                
                let gauge = ratatui::widgets::Gauge::default()
                    .block(Block::default().borders(Borders::ALL).title(" 󰏫 Installing... ").border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Cyan)))
                    .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black).add_modifier(Modifier::BOLD))
                    .use_unicode(true)
                    .ratio(valid_progress / 100.0)
                    .label(label);
                
                let p_area = centered_rect_fixed_height(70, 3, f.area());
                f.render_widget(Clear, p_area);
                f.render_widget(gauge, p_area);
            }
            _ => {}
        }
    }
}
