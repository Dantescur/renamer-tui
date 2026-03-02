use std::path::PathBuf;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_input::{Input, backend::crossterm::EventHandler as InputHandler};

use crate::{
    event::{AppEvent, Event, EventHandler},
    scanner::{FileEntry, scan_folder},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Focus {
    PathBar,
    FileList,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    ConfirmDialog,
    Done,
}

#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub mode: AppMode,
    pub focus: Focus,
    pub path_input: Input,
    pub entries: Vec<FileEntry>,
    pub selected: usize,
    pub log: Vec<String>,
    pub picker_open: bool,
    pub events: EventHandler,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            mode: AppMode::Normal,
            focus: Focus::PathBar,
            path_input: Input::default(),
            entries: vec![],
            selected: 0,
            log: vec![],
            picker_open: false,
            events: EventHandler::new(),
        }
    }

    pub fn current_path(&self) -> PathBuf {
        PathBuf::from(self.path_input.value())
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| crate::ui::render(&self, frame))?;
            match self.events.next().await? {
                Event::Tick => {}
                Event::Crossterm(event) => {
                    // Collapsed first if statement
                    if self.focus == Focus::PathBar
                        && self.mode == AppMode::Normal
                        && let ratatui::crossterm::event::Event::Key(key) = &event
                    {
                        self.path_input
                            .handle_event(&ratatui::crossterm::event::Event::Key(*key));
                    }

                    // Collapsed second if statement
                    if let ratatui::crossterm::event::Event::Key(key_event) = event
                        && key_event.kind == ratatui::crossterm::event::KeyEventKind::Press
                    {
                        self.handle_key(key_event);
                    }
                }
                Event::App(app_event) => self.handle_app_event(app_event).await,
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.events.send(AppEvent::Quit);
            return;
        }

        match self.mode {
            AppMode::Normal => match self.focus {
                Focus::PathBar => match key.code {
                    KeyCode::Enter => self.events.send(AppEvent::Scan),
                    KeyCode::Tab => self.events.send(AppEvent::ToggleFocus),
                    KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.events.send(AppEvent::OpenPicker);
                    }
                    _ => {}
                },
                Focus::FileList => match key.code {
                    KeyCode::Down | KeyCode::Char('j') => self.events.send(AppEvent::SelectNext),
                    KeyCode::Up | KeyCode::Char('k') => self.events.send(AppEvent::SelectPrev),
                    KeyCode::Tab => self.events.send(AppEvent::ToggleFocus),
                    KeyCode::Char(' ') => {
                        if !self.entries.is_empty() && self.selected < self.entries.len() {
                            self.entries[self.selected].skipped =
                                !self.entries[self.selected].skipped;
                            let status = if self.entries[self.selected].skipped {
                                "⏭️  Skipped"
                            } else {
                                "✓ Included"
                            };

                            self.log.push(format!(
                                "{}:{}",
                                status, self.entries[self.selected].original
                            ));
                        }
                    }
                    KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.events.send(AppEvent::OpenPicker);
                    }
                    KeyCode::Enter | KeyCode::Char('r') => {
                        let renameable = self
                            .entries
                            .iter()
                            .filter(|e| e.new_name.is_some() && !e.already_done && !e.skipped)
                            .count();
                        if renameable > 0 {
                            self.mode = AppMode::ConfirmDialog;
                        } else {
                            self.log.push(
                                "Nothing to rename — all files are skipped or already done.".into(),
                            );
                        }
                    }
                    KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                    _ => {}
                },
            },
            AppMode::ConfirmDialog => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    self.events.send(AppEvent::ConfirmRename);
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.mode = AppMode::Normal;
                }
                _ => {}
            },
            AppMode::Done => match key.code {
                KeyCode::Enter | KeyCode::Char('q') | KeyCode::Esc => {
                    self.mode = AppMode::Normal;
                    self.focus = Focus::PathBar;
                }
                _ => {}
            },
        }
    }

    async fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Quit => self.running = false,

            AppEvent::SetPath(path) => {
                self.path_input = Input::new(path.to_string_lossy().to_string());
                self.picker_open = false;
                self.do_scan();
                if !self.entries.is_empty() {
                    self.focus = Focus::FileList;
                }
            }

            AppEvent::CancelPicker => {
                self.picker_open = false;
            }

            AppEvent::Scan => {
                self.do_scan();
                if !self.entries.is_empty() {
                    self.focus = Focus::FileList;
                }
            }

            AppEvent::SelectNext => {
                if !self.entries.is_empty() {
                    self.selected = (self.selected + 1).min(self.entries.len() - 1);
                }
            }

            AppEvent::SelectPrev => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }

            AppEvent::ToggleFocus => {
                self.focus = match self.focus {
                    Focus::PathBar => Focus::FileList,
                    Focus::FileList => Focus::PathBar,
                };
            }

            AppEvent::OpenPicker => {
                if self.picker_open {
                    return;
                }
                self.picker_open = true;
                let sender = self.events.sender();
                tokio::spawn(async move {
                    let picked = rfd::AsyncFileDialog::new()
                        .set_title("Select folder")
                        .pick_folder()
                        .await;
                    match picked {
                        Some(folder) => {
                            let _ = sender
                                .send(Event::App(AppEvent::SetPath(folder.path().to_path_buf())));
                        }
                        None => {
                            let _ = sender.send(Event::App(AppEvent::CancelPicker));
                        }
                    }
                });
            }

            AppEvent::ConfirmRename => {
                self.do_rename();
                self.mode = AppMode::Done;
            }
        }
    }

    fn do_scan(&mut self) {
        self.entries.clear();
        self.selected = 0;
        self.log.clear();

        let path = self.current_path();

        if !path.exists() || !path.is_dir() {
            self.log.push(format!(
                "❌  '{}' is not a valid directory.",
                path.display()
            ));
            return;
        }

        self.entries = scan_folder(&path);

        if self.entries.is_empty() {
            self.log
                .push("No media files found in the selected folder.".to_string());
        } else {
            let renameable = self
                .entries
                .iter()
                .filter(|e| e.new_name.is_some() && !e.already_done && !e.skipped)
                .count();

            let skipped = self.entries.iter().filter(|e| e.skipped).count();
            let already_done = self.entries.iter().filter(|e| e.already_done).count();
            let total = self.entries.len();

            self.log.push(format!(
                "Found {} file(s) — {} to rename, {} already done, {} skipped.",
                total, renameable, already_done, skipped,
            ));
        }
    }

    fn do_rename(&mut self) {
        let path = self.current_path();
        let mut renamed = 0usize;
        let mut skipped = 0usize;
        let mut errors = 0usize;

        for entry in &mut self.entries {
            if entry.already_done {
                skipped += 1;
                continue;
            }

            if entry.skipped {
                self.log
                    .push(format!("⏭️  Skipped (manual): {}", entry.original));
                skipped += 1;
                continue;
            }

            let Some(new_name) = entry.new_name.clone() else {
                self.log
                    .push(format!("⚠️  Skipped (no number found): {}", entry.original));
                skipped += 1;
                continue;
            };

            let src = path.join(&entry.original);
            let dst = path.join(&new_name);

            if dst.exists() && dst != src {
                self.log.push(format!(
                    "⚠️  Skipped (target exists): {} -> {}",
                    entry.original, new_name
                ));
                skipped += 1;
                continue;
            }

            match std::fs::rename(&src, &dst) {
                Ok(_) => {
                    self.log
                        .push(format!("✅  {} -> {}", entry.original, new_name));
                    entry.already_done = true;
                    renamed += 1;
                }
                Err(e) => {
                    self.log
                        .push(format!("❌  {} -> {} : {}", entry.original, new_name, e));
                    errors += 1;
                }
            }
        }

        self.log.push(String::new());
        self.log.push(format!(
            "Done: {} renamed, {} skipped, {} error(s).",
            renamed, skipped, errors
        ));
    }
}
