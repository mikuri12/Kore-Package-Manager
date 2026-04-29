#![allow(clippy::single_match, clippy::manual_clamp)]

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph},
    Frame,
};
use std::path::PathBuf;

use crate::config::Config;
use crate::core::install::tokenize_desktop_exec;
use crate::tui::state::{App, Route};
use super::{AppAction, Component};
use crate::tui::ui::centered_rect;

#[derive(Clone, PartialEq)]
pub enum InstallStep {
    Init,
    Resolving,
    SelectingTarball,
    Downloading,
    Extracting,
    SelectingBinary,
    Finalizing,
    Finished,
    Error(String),
}

pub struct InstallerState {
    pub step: InstallStep,
    pub source: String,
    pub app_name: String,
    pub display_name: String,
    pub use_root: bool,
    pub use_terminal: bool,
    pub category: String,
    pub progress: f64,
    pub logs: Vec<String>,
    
    pub resolved_url: Option<String>,
    pub tarball_path: Option<PathBuf>,
    pub target_folder: Option<PathBuf>,
    pub raw_name_folder: Option<String>,
    
    pub available_assets: Vec<(String, String)>,
    
    pub executables: Vec<PathBuf>,
    pub desktop_files: Vec<PathBuf>,
    pub combined_files: Vec<(String, PathBuf, bool)>,
    
    pub selected_exec: Option<PathBuf>,
    pub selected_desktop: Option<PathBuf>,

    pub list_state: ratatui::widgets::ListState,
    
    pub tx: tokio::sync::mpsc::UnboundedSender<InstallerEvent>,
    pub rx: Option<tokio::sync::mpsc::UnboundedReceiver<InstallerEvent>>,
}

pub enum InstallerEvent {
    Log(String),
    Progress(f64),
    SelectAsset(Vec<(String, String)>),
    Resolved(String),
    SetTerminal(bool),
    Downloaded(PathBuf),
    Extracted(PathBuf, String, Vec<PathBuf>, Vec<PathBuf>),
    Metadata(String, String),
    Finalized,
    Error(String),
}

impl InstallerState {
    pub fn new(source: String, app_name: String, use_root: bool, category: String) -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            step: InstallStep::Init,
            source,
            app_name: app_name.clone(),
            display_name: app_name,
            use_root,
            use_terminal: false,
            category,
            progress: 0.0,
            logs: vec![],
            resolved_url: None,
            tarball_path: None,
            target_folder: None,
            raw_name_folder: None,
            available_assets: vec![],
            executables: vec![],
            desktop_files: vec![],
            combined_files: vec![],
            selected_exec: None,
            selected_desktop: None,
            list_state: ratatui::widgets::ListState::default(),
            tx,
            rx: Some(rx),
        }
    }
}

pub struct Installer {
    pub state: InstallerState,
}

impl Installer {
    pub fn new(source: String, app_name: String, use_root: bool, category: String) -> Self {
        Self {
            state: InstallerState::new(source, app_name, use_root, category),
        }
    }

    fn log(&mut self, msg: String) {
        self.state.logs.push(msg);
        if self.state.logs.len() > 20 {
            self.state.logs.remove(0);
        }
    }
    
    fn spawn_resolver(&self, config: Config, source: String) {
        let tx = self.state.tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(InstallerEvent::Log("Resolving source...".into()));
            match crate::core::install::resolve_source(&config, &source).await {
                Ok(Some(resolved)) => {
                    let _ = tx.send(InstallerEvent::SetTerminal(resolved.repo_terminal.unwrap_or(false)));
                    if let (Some(pkg), Some(name)) = (resolved.repo_package_name, resolved.repo_name) {
                        let _ = tx.send(InstallerEvent::Metadata(pkg, name));
                    }
                    if resolved.is_git {
                        let _ = tx.send(InstallerEvent::Log("Fetching GitHub/GitLab releases...".into()));
                        match crate::core::download::get_latest_release_assets(&resolved.url).await {
                            Ok(assets) => {
                                if assets.is_empty() {
                                    let _ = tx.send(InstallerEvent::Error("No suitable tarball assets found in the latest release.".into()));
                                } else {
                                    let mut tuples = Vec::new();
                                    for a in assets {
                                        tuples.push((a.name.clone(), a.browser_download_url.clone()));
                                    }
                                    let _ = tx.send(InstallerEvent::SelectAsset(tuples));
                                }
                            }
                            Err(e) => { let _ = tx.send(InstallerEvent::Error(e.to_string())); }
                        }
                    } else {
                        let _ = tx.send(InstallerEvent::Log("Resolving dynamic URL...".into()));
                        match crate::core::dynamic_links::resolve_dynamic_url(&resolved.url).await {
                            Ok(u) => { let _ = tx.send(InstallerEvent::Resolved(u)); }
                            Err(e) => { let _ = tx.send(InstallerEvent::Error(e.to_string())); }
                        }
                    }
                }
                Ok(None) => {
                    let path = PathBuf::from(&source);
                    if path.exists() {
                        let _ = tx.send(InstallerEvent::Downloaded(path));
                    } else {
                        let _ = tx.send(InstallerEvent::Error("Source is not a valid file or repository.".into()));
                    }
                }
                Err(e) => { let _ = tx.send(InstallerEvent::Error(e.to_string())); }
            }
        });
    }

    fn spawn_downloader(&self, url: String) {
        let tx = self.state.tx.clone();
        tokio::spawn(async move {
            let tmp_dir = std::env::temp_dir().join("tm_downloads");
            if let Err(e) = std::fs::create_dir_all(&tmp_dir) {
                let _ = tx.send(InstallerEvent::Error(format!("Failed to create temp dir: {}", e)));
                return;
            }
            
            let (dl_tx, mut dl_rx) = tokio::sync::mpsc::unbounded_channel();
            let dl_tx_opt = Some(dl_tx);
            let tx_clone = tx.clone();
            
            tokio::spawn(async move {
                while let Some(msg) = dl_rx.recv().await {
                    match msg {
                        crate::core::install::InstallMessage::Progress(status, p) => {
                            let _ = tx_clone.send(InstallerEvent::Log(status));
                            let _ = tx_clone.send(InstallerEvent::Progress(p));
                        }
                        _ => {}
                    }
                }
            });

            match crate::core::download::download_file(&url, &tmp_dir, dl_tx_opt).await {
                Ok(path) => { let _ = tx.send(InstallerEvent::Downloaded(path)); }
                Err(e) => { let _ = tx.send(InstallerEvent::Error(e.to_string())); }
            }
        });
    }

    fn spawn_extractor(&self, config: Config, tarball: PathBuf) {
        let tx = self.state.tx.clone();
        let app_name = self.state.app_name.clone();
        tokio::task::spawn_blocking(move || {
            match crate::core::install::extract_and_scan(&config, &tarball, Some(&app_name), true) {
                Ok(Some((target, raw_name, execs, desks))) => {
                    let _ = tx.send(InstallerEvent::Extracted(target, raw_name, execs, desks));
                }
                Ok(None) => { let _ = tx.send(InstallerEvent::Error("Failed to extract or archive is invalid.".into())); }
                Err(e) => { let _ = tx.send(InstallerEvent::Error(e.to_string())); }
            }
        });
    }

    fn spawn_finalizer(&self, config: Config) {
        let tx = self.state.tx.clone();
        let target = self.state.target_folder.clone().unwrap();
        let exec_path = self.state.selected_exec.clone().unwrap();
        let app_name = self.state.app_name.clone();
        let display_name = self.state.display_name.clone();
        let use_root = self.state.use_root;
        let use_terminal = if use_root { false } else { self.state.use_terminal };
        let category = self.state.category.clone();
        let desk = self.state.selected_desktop.clone();

        tokio::task::spawn_blocking(move || {
            match crate::core::install::finalize_installation(&config, &target, &exec_path, &app_name, &display_name, use_root, use_terminal, &category, desk, true) {
                Ok(_) => { let _ = tx.send(InstallerEvent::Finalized); }
                Err(e) => { let _ = tx.send(InstallerEvent::Error(e.to_string())); }
            }
        });
    }

    pub fn poll_events(&mut self, config: &Config) {
        let mut rx = match self.state.rx.take() {
            Some(r) => r,
            None => return,
        };
        
        while let Ok(event) = rx.try_recv() {
            match event {
                InstallerEvent::Log(msg) => self.log(msg),
                InstallerEvent::Progress(p) => self.state.progress = p,
                InstallerEvent::SelectAsset(assets) => {
                    self.state.available_assets = assets;
                    self.state.step = InstallStep::SelectingTarball;
                    self.state.list_state.select(Some(0));
                    self.state.progress = 0.0;
                    self.log("Waiting for user to select an asset...".into());
                }
                InstallerEvent::Resolved(url) => {
                    self.state.resolved_url = Some(url.clone());
                    self.state.step = InstallStep::Downloading;
                    self.state.progress = 0.0;
                    self.spawn_downloader(url);
                }
                InstallerEvent::SetTerminal(term) => {
                    self.state.use_terminal = term;
                }
                InstallerEvent::Downloaded(path) => {
                    self.state.tarball_path = Some(path.clone());
                    self.state.step = InstallStep::Extracting;
                    self.state.progress = 50.0;
                    self.log("Extracting archive...".into());
                    self.spawn_extractor(config.clone(), path);
                }
                InstallerEvent::Extracted(target, raw_name, execs, desks) => {
                    self.state.target_folder = Some(target);
                    self.state.raw_name_folder = Some(raw_name);
                    self.state.executables = execs.clone();
                    self.state.desktop_files = desks.clone();
                    
                    self.state.combined_files.clear();
                    for e in &execs {
                        self.state.combined_files.push((format!("[BIN] {}", e.file_name().unwrap_or_default().to_string_lossy()), e.clone(), true));
                    }
                    for d in &desks {
                        self.state.combined_files.push((format!("[DESK] {}", d.file_name().unwrap_or_default().to_string_lossy()), d.clone(), false));
                    }
                    
                    self.state.step = InstallStep::SelectingBinary;
                    self.state.list_state.select(Some(0));
                    self.state.progress = 75.0;
                }
                InstallerEvent::Metadata(app_name, display_name) => {
                    self.state.app_name = app_name;
                    self.state.display_name = display_name;
                }
                InstallerEvent::Finalized => {
                    self.state.step = InstallStep::Finished;
                    self.state.progress = 100.0;
                    self.log("Installation completed successfully!".into());
                }
                InstallerEvent::Error(e) => {
                    self.state.step = InstallStep::Error(e);
                    self.state.progress = -1.0;
                }
            }
        }
        self.state.rx = Some(rx);
    }
}

impl Component for Installer {
    fn draw(&mut self, f: &mut Frame, area: Rect, _app: &mut App, config: &Config) {
        if self.state.step == InstallStep::Init {
            self.state.step = InstallStep::Resolving;
            self.spawn_resolver(config.clone(), self.state.source.clone());
        }

        self.poll_events(config);

        let main_rect = centered_rect(80, 80, area);
        f.render_widget(Clear, main_rect);
        
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" 󰏫 Installer ")
            .border_style(Style::default().fg(Color::Cyan));
        
        f.render_widget(block.clone(), main_rect);

        let inner = main_rect.inner(ratatui::layout::Margin { vertical: 1, horizontal: 2 });
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(3),
                ratatui::layout::Constraint::Percentage(50),
                ratatui::layout::Constraint::Percentage(50),
            ])
            .split(inner);

        let valid_progress = self.state.progress.max(0.0).min(100.0);
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL))
            .gauge_style(Style::default().fg(if self.state.progress < 0.0 { Color::Red } else { Color::Green }).bg(Color::Black).add_modifier(Modifier::BOLD))
            .use_unicode(true)
            .ratio(valid_progress / 100.0)
            .label(format!("{:.1}%", valid_progress));
        f.render_widget(gauge, chunks[0]);

        match self.state.step {
            InstallStep::SelectingTarball => {
                let mut items = vec![];
                for a in &self.state.available_assets {
                    items.push(ListItem::new(a.0.clone()));
                }
                items.push(ListItem::new("󰈆  Abort"));

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(" Select Tarball / Asset "))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol("󰇙 ");

                f.render_stateful_widget(list, chunks[1], &mut self.state.list_state);
            }
            InstallStep::SelectingBinary => {
                let mut items = vec![];
                for f in &self.state.combined_files {
                    items.push(ListItem::new(f.0.clone()));
                }
                items.push(ListItem::new("󰈆  Skip / Abort"));

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(" Select Main Binary or .desktop "))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol("󰇙 ");

                f.render_stateful_widget(list, chunks[1], &mut self.state.list_state);
            }
            InstallStep::Error(ref e) => {
                let p = Paragraph::new(format!("Installation Error:\n{}", e))
                    .style(Style::default().fg(Color::Red))
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(p, chunks[1]);
            }
            InstallStep::Finished => {
                let p = Paragraph::new("Installation completed successfully!\n\nPress Esc or Enter to return.")
                    .style(Style::default().fg(Color::Green))
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(p, chunks[1]);
            }
            _ => {
                let p = Paragraph::new("Working... please wait.")
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(p, chunks[1]);
            }
        }

        let logs_text = self.state.logs.join("\n");
        let logs_p = Paragraph::new(logs_text)
            .block(Block::default().borders(Borders::ALL).title(" Action Log ").border_style(Style::default().fg(Color::DarkGray)));
        f.render_widget(logs_p, chunks[2]);
    }

    fn handle_key_event(&mut self, key: KeyEvent, _app: &mut App, config: &Config) -> Result<Option<AppAction>> {
        match self.state.step {
            InstallStep::SelectingTarball => {
                let total = self.state.available_assets.len() + 1;
                match key.code {
                    KeyCode::Esc => return Ok(Some(AppAction::ChangeRoute(Route::MainMenu))),
                    KeyCode::Up | KeyCode::Char('k') => {
                        let i = match self.state.list_state.selected() {
                            Some(i) => if i == 0 { total - 1 } else { i - 1 },
                            None => 0,
                        };
                        self.state.list_state.select(Some(i));
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let i = match self.state.list_state.selected() {
                            Some(i) => if i >= total - 1 { 0 } else { i + 1 },
                            None => 0,
                        };
                        self.state.list_state.select(Some(i));
                    }
                    KeyCode::Enter => {
                        if let Some(i) = self.state.list_state.selected() {
                            if i < self.state.available_assets.len() {
                                let url = self.state.available_assets[i].1.clone();
                                self.state.resolved_url = Some(url.clone());
                                self.state.step = InstallStep::Downloading;
                                self.state.progress = 0.0;
                                self.spawn_downloader(url);
                            } else {
                                return Ok(Some(AppAction::ChangeRoute(Route::MainMenu)));
                            }
                        }
                    }
                    _ => {}
                }
            }
            InstallStep::SelectingBinary => {
                let total = self.state.combined_files.len() + 1;
                match key.code {
                    KeyCode::Esc => return Ok(Some(AppAction::ChangeRoute(Route::MainMenu))),
                    KeyCode::Up | KeyCode::Char('k') => {
                        let i = match self.state.list_state.selected() {
                            Some(i) => if i == 0 { total - 1 } else { i - 1 },
                            None => 0,
                        };
                        self.state.list_state.select(Some(i));
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let i = match self.state.list_state.selected() {
                            Some(i) => if i >= total - 1 { 0 } else { i + 1 },
                            None => 0,
                        };
                        self.state.list_state.select(Some(i));
                    }
                    KeyCode::Enter => {
                        if let Some(i) = self.state.list_state.selected() {
                            if i < self.state.combined_files.len() {
                                let (_, path, is_bin) = &self.state.combined_files[i];
                                if *is_bin {
                                    self.state.selected_exec = Some(path.clone());
                                    self.state.selected_desktop = None;
                                } else {
                                    self.state.selected_desktop = Some(path.clone());
                                    
                                    let content = std::fs::read_to_string(path).unwrap_or_default();
                                    let mut exec_name = String::new();
                                    for line in content.lines() {
                                        if line.trim_start().starts_with("Exec=") {
                                            let exec_value = line.trim_start()["Exec=".len()..].trim();
                                            let tokens = tokenize_desktop_exec(exec_value);
                                            let candidate = tokens.into_iter().find(|token| {
                                                token != "env"
                                                    && token != "pkexec"
                                                    && !token.contains('=')
                                                    && !token.starts_with('%')
                                            });
                                            if let Some(name) = candidate {
                                                if let Some(n) = std::path::Path::new(&name).file_name() {
                                                    exec_name = n.to_string_lossy().to_string();
                                                } else {
                                                    exec_name = name;
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    
                                    let mut found_exec = None;
                                    for e in &self.state.executables {
                                        if e.file_name().unwrap_or_default().to_string_lossy() == exec_name {
                                            found_exec = Some(e.clone());
                                            break;
                                        }
                                    }
                                    
                                    if let Some(e) = found_exec {
                                        self.state.selected_exec = Some(e);
                                    } else {
                                        self.state.selected_exec = self.state.executables.first().cloned();
                                    }
                                    
                                    if self.state.selected_exec.is_none() {
                                        self.state.selected_exec = Some(path.clone());
                                    }
                                }
                                self.state.step = InstallStep::Finalizing;
                                self.spawn_finalizer(config.clone());
                            } else {
                                return Ok(Some(AppAction::ChangeRoute(Route::MainMenu)));
                            }
                        }
                    }
                    _ => {}
                }
            }
            InstallStep::Finished | InstallStep::Error(_) => {
                match key.code {
                    KeyCode::Enter | KeyCode::Esc => return Ok(Some(AppAction::ChangeRoute(Route::MainMenu))),
                    _ => {}
                }
            }
            _ => {
                if key.code == KeyCode::Esc {
                    return Ok(Some(AppAction::ChangeRoute(Route::MainMenu)));
                }
            }
        }
        Ok(None)
    }
}
