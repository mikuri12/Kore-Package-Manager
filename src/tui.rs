use crate::config::Config;
use crate::core::{remove_app, update_desktop_file};
use crate::utils::find_executables;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use directories::BaseDirs;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(PartialEq)]
enum Route {
    MainMenu,
    ManageApps,
    RemoveApps,
    FileBrowser,
}

#[derive(PartialEq)]
enum PopupType {
    None,
    ActionSelect,
    NameInput,
    CategorySelect,
    ConfirmUninstall,
    InstallNameInput,
    InstallRootSelect,
    InstallCategorySelect,
    InstallBinarySelect,
    Information,
}

struct App {
    route: Route,
    // Estado de Administración/Eliminación
    apps: Vec<String>,
    filtered: Vec<String>,
    list_state: ListState,
    input: String,

    // Estado del Explorador de Archivos
    current_dir: PathBuf,
    fb_items: Vec<String>,
    fb_filtered: Vec<String>,
    fb_state: ListState,
    fb_input: String,

    // Caché compartido
    cached_preview: Option<(String, String)>,

    // Estado de Pop-ups superpuestos
    popup_type: PopupType,
    popup_state: ListState,
    popup_items: Vec<String>,
    popup_input: String,
    popup_info: String,

    // Variables de trabajo asíncrono para flujos de instalación
    pending_tarball: PathBuf,
    pending_raw_name: String,
    pending_app_name: String,
    pending_use_root: bool,
    pending_category: String,
    pending_target: PathBuf,
    pending_executables: Vec<PathBuf>,
    pending_selected_exec: PathBuf,
}

impl App {
    fn new(start_dir: &Path) -> Self {
        App {
            route: Route::MainMenu,
            apps: Vec::new(),
            filtered: Vec::new(),
            list_state: ListState::default(),
            input: String::new(),
            current_dir: start_dir.to_path_buf(),
            fb_items: Vec::new(),
            fb_filtered: Vec::new(),
            fb_state: ListState::default(),
            fb_input: String::new(),
            cached_preview: None,
            popup_type: PopupType::None,
            popup_state: ListState::default(),
            popup_items: Vec::new(),
            popup_input: String::new(),
            popup_info: String::new(),
            pending_tarball: PathBuf::new(),
            pending_raw_name: String::new(),
            pending_app_name: String::new(),
            pending_use_root: false,
            pending_category: String::new(),
            pending_target: PathBuf::new(),
            pending_executables: Vec::new(),
            pending_selected_exec: PathBuf::new(),
        }
    }

    fn open_popup_list(&mut self, p_type: PopupType, items: Vec<String>) {
        self.popup_type = p_type;
        self.popup_items = items;
        self.popup_state.select(if self.popup_items.is_empty() { None } else { Some(0) });
    }

    fn open_popup_input(&mut self, p_type: PopupType, current_val: &str) {
        self.popup_type = p_type;
        self.popup_input = current_val.to_string();
    }

    fn open_popup_info(&mut self, text: &str) {
        self.popup_type = PopupType::Information;
        self.popup_info = text.to_string();
    }

    fn load_apps(&mut self, config: &Config) {
        self.apps.clear();
        if let Ok(entries) = fs::read_dir(&config.install_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    self.apps.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
        self.apps.sort();
        self.input.clear();
        self.filter_apps();
    }

    fn filter_apps(&mut self) {
        if self.input.is_empty() {
            self.filtered = self.apps.clone();
        } else {
            let lower = self.input.to_lowercase();
            self.filtered = self
                .apps
                .iter()
                .filter(|i| i.to_lowercase().contains(&lower))
                .cloned()
                .collect();
        }
        self.list_state.select(if self.filtered.is_empty() { None } else { Some(0) });
    }

    fn load_dir(&mut self) {
        self.fb_items.clear();
        if let Some(parent) = self.current_dir.parent() {
            if parent.to_string_lossy() != "" {
                self.fb_items.push("../".to_string());
            }
        } else {
             self.fb_items.push("../".to_string());
        }
        self.fb_items.push("./".to_string());

        if let Ok(paths) = fs::read_dir(&self.current_dir) {
            for entry in paths.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if entry.path().is_dir() {
                    self.fb_items.push(format!("{}/", name));
                } else if name.contains(".tar.") {
                    self.fb_items.push(name);
                }
            }
        }
        if self.fb_items.len() > 2 {
            let slice = &mut self.fb_items[2..];
            slice.sort();
        }
        
        self.fb_input.clear();
        self.filter_fb();
    }

    fn filter_fb(&mut self) {
        if self.fb_input.is_empty() {
            self.fb_filtered = self.fb_items.clone();
        } else {
            let lower = self.fb_input.to_lowercase();
            let mut filtered = Vec::new();
            if self.fb_items.len() >= 2 {
                filtered.push(self.fb_items[0].clone());
                filtered.push(self.fb_items[1].clone());
            }
            if self.fb_items.len() > 2 {
                filtered.extend(
                    self.fb_items.iter().skip(2).filter(|i| i.to_lowercase().contains(&lower)).cloned()
                );
            }
            self.fb_filtered = filtered;
        }
        self.fb_state.select(if self.fb_filtered.is_empty() { None } else { Some(0) });
    }

    fn next(&mut self) {
        if self.popup_type == PopupType::ActionSelect || self.popup_type == PopupType::CategorySelect
          || self.popup_type == PopupType::ConfirmUninstall || self.popup_type == PopupType::InstallRootSelect 
          || self.popup_type == PopupType::InstallCategorySelect || self.popup_type == PopupType::InstallBinarySelect {
            let i = match self.popup_state.selected() {
                Some(i) => (i + 1) % self.popup_items.len(),
                None => 0,
            };
            self.popup_state.select(Some(i));
        } else if self.route == Route::MainMenu {
            let len = 4;
            let i = match self.list_state.selected() {
                Some(i) => (i + 1) % len,
                None => 0,
            };
            self.list_state.select(Some(i));
        } else if self.route == Route::FileBrowser {
            let i = match self.fb_state.selected() {
                Some(i) => {
                    if i >= self.fb_filtered.len().saturating_sub(1) { 0 } else { i + 1 }
                }
                None => 0,
            };
            self.fb_state.select(Some(i));
        } else {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i >= self.filtered.len().saturating_sub(1) { 0 } else { i + 1 }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }

    fn previous(&mut self) {
        if self.popup_type == PopupType::ActionSelect || self.popup_type == PopupType::CategorySelect
          || self.popup_type == PopupType::ConfirmUninstall || self.popup_type == PopupType::InstallRootSelect 
          || self.popup_type == PopupType::InstallCategorySelect || self.popup_type == PopupType::InstallBinarySelect {
            let i = match self.popup_state.selected() {
                Some(i) => {
                    if i == 0 { self.popup_items.len().saturating_sub(1) } else { i - 1 }
                }
                None => 0,
            };
            self.popup_state.select(Some(i));
        } else if self.route == Route::MainMenu {
            let len = 4;
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 { len - 1 } else { i - 1 }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        } else if self.route == Route::FileBrowser {
            let i = match self.fb_state.selected() {
                Some(i) => {
                    if i == 0 { self.fb_filtered.len().saturating_sub(1) } else { i - 1 }
                }
                None => 0,
            };
            self.fb_state.select(Some(i));
        } else {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 { self.filtered.len().saturating_sub(1) } else { i - 1 }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }
}

fn calculate_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

fn format_size(size: u64) -> String {
    let mb = size as f64 / 1_048_576.0;
    if mb > 1024.0 {
        format!("{:.2} GB", mb / 1024.0)
    } else {
        format!("{:.2} MB", mb)
    }
}

fn generate_preview(config: &Config, app_name: &str) -> String {
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

fn generate_tar_preview(file_path: &Path) -> String {
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
        // Asesinar el proceso al instante para no leer el Gigabyte restante.
        let _ = child.kill(); 
        let _ = child.wait();
    }
    
    preview
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

pub fn main_menu(config: &Config) -> io::Result<()> {
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
                Route::FileBrowser => {
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

                    let list = List::new(items)
                        .block(Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .title(format!(" 󰉋 {} ", app.current_dir.display()))
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

            // Renderizado superior de Pop-Ups y Overlay
            if app.popup_type != PopupType::None {
                let area = centered_rect(50, 40, f.area());
                f.render_widget(Clear, area);

                match app.popup_type {
                    PopupType::ActionSelect | PopupType::CategorySelect | PopupType::ConfirmUninstall | PopupType::InstallRootSelect | PopupType::InstallCategorySelect | PopupType::InstallBinarySelect => {
                        let popup_title = match app.popup_type {
                            PopupType::ActionSelect => " Action ",
                            PopupType::CategorySelect | PopupType::InstallCategorySelect => " Select Category ",
                            PopupType::ConfirmUninstall => " Are you sure? ",
                            PopupType::InstallRootSelect => " Needs Root? ",
                            PopupType::InstallBinarySelect => " Select Main Binary ",
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
                    PopupType::NameInput | PopupType::InstallNameInput => {
                        let title = if app.popup_type == PopupType::NameInput { " New Name " } else { " Target Name " };
                        let p = Paragraph::new(format!("{}█", app.popup_input))
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
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
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
                                        app.open_popup_list(PopupType::CategorySelect, vec!["Utility".to_string(), "Network".to_string(), "Game".to_string(), "Development".to_string(), "Graphics".to_string(), "AudioVideo".to_string(), "System".to_string(), "Office".to_string()]);
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
                                            app.open_popup_info(&format!("App {} succesfully uninstalled.", selected_app));
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
                                    app.open_popup_list(PopupType::InstallCategorySelect, vec!["Utility".to_string(), "Network".to_string(), "Game".to_string(), "Development".to_string(), "Graphics".to_string(), "AudioVideo".to_string(), "System".to_string(), "Office".to_string()]);
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
                                        
                                        // Detiene momentáneamente la TUI para EJECUTAR LA EXTRACCION
                                        app.open_popup_info("Extracting tarball... Please wait.");
                                        terminal.draw(|f| { 
                                            // Forzamos un cuadro estático de carga para evitar que la UI se perciba congelada
                                            let area = centered_rect(50, 40, f.area());
                                            f.render_widget(Clear, area);
                                            let p = Paragraph::new(format!("{}\n\n[ Waiting ]", app.popup_info))
                                                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(" Information ").border_style(Style::default().fg(Color::Yellow)))
                                                .wrap(ratatui::widgets::Wrap { trim: true });
                                            f.render_widget(p, area);
                                        })?;

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
                                    // Tras completar la lectura del mensaje final, regresamos al menú inicial
                                    if app.route == Route::FileBrowser {
                                        app.route = Route::MainMenu;
                                        app.list_state.select(Some(0));
                                    }
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                        continue;
                    }

                    // ENRUTAMIENTO NORMAL VISUAL
                    match app.route {
                        Route::MainMenu => match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => { break; }
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
                                    _ => { break; }
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
                                    app.open_popup_list(PopupType::ActionSelect, vec!["󰏫 Modify Name".to_string(), "󰟝 Change Category".to_string(), "󰈆 Return".to_string()]);
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
                                    app.open_popup_list(PopupType::ConfirmUninstall, vec!["Yes, eliminate".to_string(), "No, cancel".to_string()]);
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
                                        continue;
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
                        }
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    Ok(())
}
