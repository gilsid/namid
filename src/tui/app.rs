//! Application state machine & event loop for the TUI wizard.

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;

use crate::cli::Cli;
use crate::config::RenameConfig;
use crate::core::rename::{FinalStats, RenameOp, RenameStatus};
use crate::core::{discover, rename};

use super::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Folder = 0,
    Rules = 1,
    Preview = 2,
    Execute = 3,
}

impl Step {
    pub fn next(self) -> Option<Self> {
        match self {
            Step::Folder => Some(Step::Rules),
            Step::Rules => Some(Step::Preview),
            Step::Preview => Some(Step::Execute),
            Step::Execute => None,
        }
    }

    pub fn prev(self) -> Option<Self> {
        match self {
            Step::Folder => None,
            Step::Rules => Some(Step::Folder),
            Step::Preview => Some(Step::Rules),
            Step::Execute => Some(Step::Preview),
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(Step::Folder),
            1 => Some(Step::Rules),
            2 => Some(Step::Preview),
            3 => Some(Step::Execute),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExecuteEvent {
    Progress {
        done: usize,
        total: usize,
        current_file: String,
    },
    Finished(FinalStats),
}

pub struct ExecuteProgress {
    pub done: usize,
    pub total: usize,
    pub current_file: String,
    pub rx: mpsc::Receiver<ExecuteEvent>,
}

pub enum AppState {
    Wizard { step: Step },
    Executing { progress: ExecuteProgress },
    Done { stats: FinalStats },
    ConfirmQuit { return_to: QuitReturn },
}

/// What to restore if the user cancels out of the quit confirmation.
#[derive(Debug, Clone)]
pub enum QuitReturn {
    Wizard(Step),
    Done(FinalStats),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderFocus {
    PathBar,
    List,
}

#[derive(Debug, Clone)]
pub struct RulesField {
    pub label: &'static str,
    pub value: String,
    pub placeholder: &'static str,
    /// Cursor position in **characters**, not bytes — see `cursor_byte()`.
    pub cursor: usize,
}

impl RulesField {
    fn char_len(&self) -> usize {
        self.value.chars().count()
    }

    /// Byte offset in `value` corresponding to the current char-based cursor.
    fn cursor_byte(&self) -> usize {
        self.value
            .char_indices()
            .nth(self.cursor)
            .map(|(b, _)| b)
            .unwrap_or(self.value.len())
    }
}

pub struct RulesState {
    pub fields: Vec<RulesField>,
    pub focus: usize, // 0-3 = field (Title/Prefix/Suffix/Extensions), 4 = Simulate/Execute toggle, 5 = collision mode toggle
    pub collision_auto_num: bool,
    pub show_error: bool,
}

impl RulesState {
    pub fn from_config(config: &RenameConfig) -> Self {
        Self {
            fields: vec![
                RulesField {
                    label: "Series Title *",
                    value: config.title.clone(),
                    placeholder: "Required",
                    cursor: config.title.chars().count(),
                },
                RulesField {
                    label: "Strip Prefix",
                    value: config.prefix.clone(),
                    placeholder: "Text to strip from start",
                    cursor: config.prefix.chars().count(),
                },
                RulesField {
                    label: "Strip Suffix",
                    value: config.suffix.clone(),
                    placeholder: "Text to strip before extension",
                    cursor: config.suffix.chars().count(),
                },
                RulesField {
                    label: "Extensions",
                    value: config.extensions.join("|"),
                    placeholder: "mp4|mkv|webm|avi (leave empty to match every file)",
                    cursor: config.extensions.join("|").chars().count(),
                },
            ],
            focus: 0,
            collision_auto_num: false,
            show_error: false,
        }
    }

    pub fn to_config(&self) -> (String, String, String, Vec<String>) {
        let title = if !self.fields.is_empty() {
            self.fields[0].value.clone()
        } else {
            String::new()
        };
        let prefix = if self.fields.len() > 1 {
            self.fields[1].value.clone()
        } else {
            String::new()
        };
        let suffix = if self.fields.len() > 2 {
            self.fields[2].value.clone()
        } else {
            String::new()
        };
        let exts = if self.fields.len() > 3 {
            crate::config::parse_extensions(&self.fields[3].value)
        } else {
            vec![] // empty → all files
        };
        (title, prefix, suffix, exts)
    }
}

pub struct App {
    pub state: AppState,
    pub config: RenameConfig,
    pub ops: Vec<RenameOp>,
    pub stats: Option<FinalStats>,
    pub errors: Vec<String>,

    // Step 1 — Folder
    pub dir_buffer: String,
    pub dir_entries: Vec<String>,
    pub selected_entry: usize,
    pub folder_confirmed: bool,
    pub folder_focus: FolderFocus,
    pub path_cursor: usize,

    // Step 2 — Rules
    pub rules: RulesState,

    // Step 3 — Preview
    pub preview_filter: usize, // 0=All, 1=ToRename, 2=Skipped, 3=Error
    pub preview_scroll: usize,

    // Global
    pub visited: [bool; 4],
    pub cheatsheet_open: bool,
}

impl App {
    pub fn new(config: RenameConfig) -> Self {
        let dir_buf = config.dir.to_string_lossy().to_string();
        let path_cursor = dir_buf.chars().count();
        let rules = RulesState::from_config(&config);
        let mut app = Self {
            state: AppState::Wizard { step: Step::Folder },
            config,
            ops: Vec::new(),
            stats: None,
            errors: Vec::new(),
            dir_buffer: dir_buf,
            dir_entries: Vec::new(),
            selected_entry: 0,
            folder_confirmed: false,
            folder_focus: FolderFocus::List,
            path_cursor,
            rules,
            preview_filter: 0,
            preview_scroll: 0,
            visited: [true, false, false, false],
            cheatsheet_open: false,
        };
        app.refresh_dir_entries();
        app
    }

    pub fn current_step(&self) -> Step {
        match &self.state {
            AppState::Wizard { step } => *step,
            AppState::Executing { .. } => Step::Execute,
            AppState::Done { .. } => Step::Execute,
            AppState::ConfirmQuit { return_to } => match return_to {
                QuitReturn::Wizard(step) => *step,
                QuitReturn::Done(_) => Step::Execute,
            },
        }
    }

    pub fn is_wizard(&self) -> bool {
        matches!(self.state, AppState::Wizard { .. })
    }

    /// Number of *characters* in `dir_buffer` — see `path_cursor_byte()`.
    pub fn dir_buffer_char_len(&self) -> usize {
        self.dir_buffer.chars().count()
    }

    /// Byte offset in `dir_buffer` corresponding to the current char-based
    /// `path_cursor`.
    pub fn path_cursor_byte(&self) -> usize {
        self.dir_buffer
            .char_indices()
            .nth(self.path_cursor)
            .map(|(b, _)| b)
            .unwrap_or(self.dir_buffer.len())
    }

    pub fn refresh_dir_entries(&mut self) {
        self.dir_entries.clear();
        let raw = self.dir_buffer.trim();
        if raw.is_empty() {
            return;
        }
        let dir = crate::config::expand_tilde(&PathBuf::from(raw));
        if !dir.exists() || !dir.is_dir() {
            return;
        }
        let mut entries: Vec<String> = match std::fs::read_dir(&dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if e.path().is_dir() {
                        format!("{name}/")
                    } else {
                        name
                    }
                })
                .collect(),
            Err(_) => return,
        };
        entries.sort();
        entries.insert(0, "..".into());
        self.dir_entries = entries;
        self.selected_entry = self
            .selected_entry
            .min(self.dir_entries.len().saturating_sub(1));
    }

    pub fn navigate_into_selected(&mut self) {
        if self.selected_entry >= self.dir_entries.len() {
            return;
        }
        let entry = &self.dir_entries[self.selected_entry];
        if entry == ".." {
            // Go to parent
            let current = crate::config::expand_tilde(&PathBuf::from(self.dir_buffer.trim()));
            if let Some(parent) = current.parent() {
                self.dir_buffer = parent.to_string_lossy().to_string();
            }
        } else if entry.ends_with('/') {
            let name = entry.trim_end_matches('/');
            let base = crate::config::expand_tilde(&PathBuf::from(self.dir_buffer.trim()));
            let new_path = base.join(name);
            self.dir_buffer = new_path.to_string_lossy().to_string();
        }
        self.flush_dir_buffer();
        self.refresh_dir_entries();
        self.selected_entry = 0;
    }

    pub fn select_current_dir(&mut self) {
        self.flush_dir_buffer();
        if self.config.dir.exists() && self.config.dir.is_dir() {
            self.folder_confirmed = true;
            self.visited[1] = true;
            // Pre-fill rules title from dir name if empty
            if self.rules.fields[0].value.is_empty() {
                if let Some(dirname) = self.config.dir.file_name().and_then(|n| n.to_str()) {
                    self.rules.fields[0].value = dirname.to_string();
                    self.rules.fields[0].cursor = dirname.chars().count();
                }
            }
            self.state = AppState::Wizard { step: Step::Rules };
        }
    }

    pub fn flush_dir_buffer(&mut self) {
        let s = self.dir_buffer.trim();
        let expanded = crate::config::expand_tilde(&PathBuf::from(s));
        if expanded != self.config.dir {
            self.folder_confirmed = false;
        }
        self.config.dir = expanded;
    }

    pub fn apply_rules(&mut self) {
        let (title, prefix, suffix, exts) = self.rules.to_config();
        self.config.title = title;
        self.config.prefix = prefix;
        self.config.suffix = suffix;
        self.config.extensions = exts;
        self.config.collision_auto_num = self.rules.collision_auto_num;
    }

    pub fn refresh_plan(&mut self) {
        self.apply_rules();
        self.errors.clear();
        let dir = self.config.dir.clone();
        if !dir.exists() || self.config.title.is_empty() {
            self.ops.clear();
            return;
        }
        match discover::discover_files(&dir, &self.config.extensions) {
            Ok(files) => match rename::generate_plan(&files, &self.config) {
                Ok(ops) => self.ops = ops,
                Err(e) => {
                    self.ops.clear();
                    self.errors.push(format!("Plan: {e}"));
                }
            },
            Err(e) => {
                self.ops.clear();
                self.errors.push(format!("Discover: {e}"));
            }
        }
        self.preview_scroll = 0;
    }

    pub fn pending_count(&self) -> usize {
        self.ops
            .iter()
            .filter(|o| matches!(o.status, RenameStatus::Pending))
            .count()
    }

    pub fn start_execute(&mut self) {
        self.apply_rules();

        // Spawn rename work on a thread, send progress via channel
        let mut ops_clone = self.ops.clone();
        let dry_run = self.config.dry_run;
        let (tx, rx) = mpsc::channel();
        let total = ops_clone.len();

        thread::spawn(move || {
            let start = std::time::Instant::now();
            let mut done = 0;
            for op in ops_clone.iter_mut() {
                if !matches!(op.status, RenameStatus::Pending) {
                    continue;
                }
                let fname = op
                    .from
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                if !dry_run {
                    match std::fs::rename(&op.from, &op.to) {
                        Ok(()) => op.status = RenameStatus::Success,
                        Err(e) => op.status = RenameStatus::Error(e.to_string()),
                    }
                } else {
                    op.status = RenameStatus::Success;
                }
                done += 1;
                let _ = tx.send(ExecuteEvent::Progress {
                    done,
                    total,
                    current_file: fname,
                });
            }

            let renamed = ops_clone
                .iter()
                .filter(|o| matches!(o.status, RenameStatus::Success))
                .count();
            let skipped = ops_clone
                .iter()
                .filter(|o| {
                    matches!(
                        o.status,
                        RenameStatus::SkipExists
                            | RenameStatus::SkipCollision
                            | RenameStatus::SkipNoChange
                            | RenameStatus::SkipEmptyName
                    )
                })
                .count();
            let errs = ops_clone
                .iter()
                .filter(|o| matches!(o.status, RenameStatus::Error(_)))
                .count();
            let _ = tx.send(ExecuteEvent::Finished(FinalStats {
                renamed,
                skipped,
                errors: errs,
                duration: start.elapsed(),
            }));
        });

        let progress = ExecuteProgress {
            done: 0,
            total,
            current_file: String::new(),
            rx,
        };
        self.state = AppState::Executing { progress };
    }

    pub fn poll_execute(&mut self) {
        if let AppState::Executing { ref mut progress } = self.state {
            while let Ok(event) = progress.rx.try_recv() {
                match event {
                    ExecuteEvent::Progress {
                        done,
                        total,
                        current_file,
                    } => {
                        progress.done = done;
                        progress.total = total;
                        progress.current_file = current_file;
                    }
                    ExecuteEvent::Finished(stats) => {
                        // Replace Executing state with Done
                        // We need to take ownership of progress to avoid borrow issues
                        // Use a temporary state swap
                        let old_state = std::mem::replace(
                            &mut self.state,
                            AppState::ConfirmQuit {
                                return_to: QuitReturn::Wizard(Step::Execute),
                            },
                        );
                        if let AppState::Executing { .. } = old_state {
                            self.state = AppState::Done { stats };
                            self.visited[3] = true;
                        } else {
                            self.state = old_state;
                        }
                        return;
                    }
                }
            }
        }
    }

    pub fn reset_keep_folder_rules(&mut self) {
        self.apply_rules();
        self.refresh_plan();
        self.state = AppState::Wizard {
            step: Step::Preview,
        };
        self.preview_scroll = 0;
        self.preview_filter = 0;
    }

    pub fn reset_new(&mut self) {
        *self = App::new(RenameConfig::default());
    }

    pub fn filtered_ops(&self) -> Vec<RenameOp> {
        self.ops
            .iter()
            .filter(|op| match self.preview_filter {
                0 => true,
                1 => matches!(op.status, RenameStatus::Pending | RenameStatus::Success),
                2 => matches!(
                    op.status,
                    RenameStatus::SkipExists
                        | RenameStatus::SkipCollision
                        | RenameStatus::SkipNoChange
                        | RenameStatus::SkipEmptyName
                ),
                3 => matches!(op.status, RenameStatus::Error(_)),
                _ => true,
            })
            .cloned()
            .collect()
    }
}

pub fn run_tui(args: &Cli) -> anyhow::Result<()> {
    let config: RenameConfig = if args.title.is_some() || args.dir.is_some() {
        RenameConfig::from(args)
    } else {
        RenameConfig::default()
    };

    let mut app = App::new(config);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let _orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        _orig_hook(panic_info);
    }));

    let restore = || {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    };
    ctrlc::set_handler(move || {
        restore();
        std::process::exit(130);
    })?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    let _ = std::panic::take_hook();

    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> anyhow::Result<()> {
    loop {
        // Poll execute progress each frame
        if matches!(app.state, AppState::Executing { .. }) {
            app.poll_execute();
        }

        terminal.draw(|frame| ui::render(frame, app))?;

        if !event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if !handle_key(app, key.code, key.modifiers) {
                break;
            }
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode, mods: event::KeyModifiers) -> bool {
    use KeyCode::*;

    // Cheatsheet overlay swallows all input except what closes it again.
    if app.cheatsheet_open {
        if matches!(code, Char('?') | Esc | Char('q' | 'Q')) {
            app.cheatsheet_open = false;
        }
        return true;
    }

    let in_text_input = (app.current_step() == Step::Rules && app.rules.focus < 4)
        || (app.current_step() == Step::Folder && app.folder_focus == FolderFocus::PathBar);
    if !in_text_input {
        if let Char('?') = code {
            app.cheatsheet_open = true;
            return true;
        }
        // Consolidated jump/quit for all steps
        match code {
            Char('1') => {
                if app.visited[0] {
                    jump_to(app, 0);
                    return true;
                }
            }
            Char('2') => {
                if app.visited[1] {
                    jump_to(app, 1);
                    return true;
                }
            }
            Char('3') => {
                if app.visited[2] {
                    jump_to(app, 2);
                    return true;
                }
            }
            Char('4') => {
                if app.visited[3] {
                    jump_to(app, 3);
                    return true;
                }
            }
            Char('q' | 'Q') => {
                if matches!(app.state, AppState::Wizard { .. }) {
                    app.state = AppState::ConfirmQuit {
                        return_to: QuitReturn::Wizard(app.current_step()),
                    };
                    return true;
                }
            }
            _ => {}
        }
    }

    match (&app.state, code) {
        (AppState::ConfirmQuit { return_to }, _) => {
            let return_to = return_to.clone();
            match code {
                Esc | Char('n' | 'N') => {
                    app.state = match return_to {
                        QuitReturn::Wizard(step) => AppState::Wizard { step },
                        QuitReturn::Done(stats) => AppState::Done { stats },
                    };
                    true
                }
                Enter | Char('y' | 'Y') => false,
                _ => true,
            }
        }
        (AppState::Done { stats }, _) => match code {
            Char('r' | 'R') => {
                app.reset_keep_folder_rules();
                true
            }
            Char('n' | 'N') => {
                app.reset_new();
                true
            }
            Char('q' | 'Q') => {
                let stats = stats.clone();
                app.state = AppState::ConfirmQuit {
                    return_to: QuitReturn::Done(stats),
                };
                true
            }
            _ => true,
        },
        (AppState::Executing { .. }, _) => true, // ignore input

        (AppState::Wizard { step }, key) => {
            if key == Char('c') && mods.contains(event::KeyModifiers::CONTROL) {
                app.state = AppState::ConfirmQuit {
                    return_to: QuitReturn::Wizard(*step),
                };
                return true;
            }
            match step {
                Step::Folder => handle_folder(app, key),
                Step::Rules => handle_rules(app, key, mods),
                Step::Preview => handle_preview(app, key),
                Step::Execute => handle_execute(app, key),
            }
        }
    }
}

fn handle_folder(app: &mut App, code: KeyCode) -> bool {
    match app.folder_focus {
        FolderFocus::PathBar => handle_folder_pathbar(app, code),
        FolderFocus::List => handle_folder_list(app, code),
    }
}

/// Free-text editing of the path buffer. Every printable character — including
/// `/` and `~` — is inserted literally at the cursor; they are only
/// interpreted (root / home expansion) when the buffer is *committed*
/// (Enter/Down/Tab), via `flush_dir_buffer`. This fixes the earlier bug where
/// typing was silently dropped whenever the buffer already resolved to a
/// valid directory, making it impossible to type a full path like
/// e.g. `~/files/`.
fn handle_folder_pathbar(app: &mut App, code: KeyCode) -> bool {
    use KeyCode::*;
    match code {
        Left => {
            app.path_cursor = app.path_cursor.saturating_sub(1);
            true
        }
        Right => {
            app.path_cursor = (app.path_cursor + 1).min(app.dir_buffer_char_len());
            true
        }
        Home => {
            app.path_cursor = 0;
            true
        }
        End => {
            app.path_cursor = app.dir_buffer_char_len();
            true
        }
        Backspace => {
            if app.path_cursor > 0 {
                let byte_idx = app
                    .dir_buffer
                    .char_indices()
                    .nth(app.path_cursor - 1)
                    .map(|(b, _)| b)
                    .unwrap_or(0);
                app.dir_buffer.remove(byte_idx);
                app.path_cursor -= 1;
                app.flush_dir_buffer();
                app.refresh_dir_entries();
            }
            true
        }
        Delete => {
            if app.path_cursor < app.dir_buffer_char_len() {
                let byte_idx = app.path_cursor_byte();
                app.dir_buffer.remove(byte_idx);
                app.flush_dir_buffer();
                app.refresh_dir_entries();
            }
            true
        }
        Enter | Down => {
            // Commit the typed path and drop back into browsing.
            app.flush_dir_buffer();
            app.refresh_dir_entries();
            app.selected_entry = 0;
            app.folder_focus = FolderFocus::List;
            true
        }
        Esc => {
            app.folder_focus = FolderFocus::List;
            true
        }
        Up => true, // nothing above the path bar
        Tab => {
            tab_forward(app);
            true
        }
        Char(c) => {
            let byte_idx = app.path_cursor_byte();
            app.dir_buffer.insert(byte_idx, c);
            app.path_cursor += 1;
            app.flush_dir_buffer();
            app.refresh_dir_entries();
            true
        }
        _ => true,
    }
}

/// Browsing the directory listing. `h/j/k/l` work as vim-style aliases for
/// Left/Down/Up/Right here (list-only — never in a text field, so they can't
/// clobber typing). Any *other* printable character switches focus to the
/// path bar and starts typing with that character, so the moment the user
/// wants to type an arbitrary path they just start typing.
fn handle_folder_list(app: &mut App, code: KeyCode) -> bool {
    use KeyCode::*;
    match code {
        Up | Char('k') => {
            if app.selected_entry == 0 {
                // Top of the list → hop up into the path bar to edit it directly.
                app.folder_focus = FolderFocus::PathBar;
                app.path_cursor = app.dir_buffer_char_len();
            } else {
                app.selected_entry -= 1;
            }
            true
        }
        Down | Char('j') => {
            let max = app.dir_entries.len().saturating_sub(1);
            app.selected_entry = app.selected_entry.min(max).saturating_add(1).min(max);
            true
        }
        Enter | Right | Char('l') | Char(' ') => {
            if app.dir_entries.is_empty() {
                app.select_current_dir();
            } else if app.selected_entry < app.dir_entries.len() {
                let entry = &app.dir_entries[app.selected_entry];
                if entry.ends_with('/') || entry == ".." {
                    app.navigate_into_selected();
                } else {
                    app.select_current_dir();
                }
            }
            true
        }
        Left | Char('h') | Backspace => {
            let current = crate::config::expand_tilde(&PathBuf::from(app.dir_buffer.trim()));
            if let Some(parent) = current.parent() {
                app.dir_buffer = parent.to_string_lossy().to_string();
                app.flush_dir_buffer();
                app.refresh_dir_entries();
                app.selected_entry = 0;
            }
            true
        }
        Tab => {
            tab_forward(app);
            true
        }
        BackTab => true, // Folder is the first step — nothing to go back to
        Char(c) => {
            // Start typing a full path directly from browse mode.
            app.folder_focus = FolderFocus::PathBar;
            app.dir_buffer.push(c);
            app.path_cursor = app.dir_buffer_char_len();
            app.flush_dir_buffer();
            app.refresh_dir_entries();
            true
        }
        _ => true,
    }
}

fn handle_rules(app: &mut App, code: KeyCode, _mods: event::KeyModifiers) -> bool {
    use KeyCode::*;
    let focus = app.rules.focus;

    match code {
        Up => {
            app.rules.focus = focus.saturating_sub(1);
            true
        }
        Down => {
            app.rules.focus = (focus + 1).min(5);
            true
        }
        Left if focus < 4 => {
            let f = &mut app.rules.fields[focus];
            f.cursor = f.cursor.saturating_sub(1);
            true
        }
        Right if focus < 4 => {
            let f = &mut app.rules.fields[focus];
            f.cursor = f.cursor.saturating_add(1).min(f.char_len());
            true
        }
        Backspace if focus < 4 => {
            let f = &mut app.rules.fields[focus];
            if f.cursor > 0 {
                let byte_idx = f
                    .value
                    .char_indices()
                    .nth(f.cursor - 1)
                    .map(|(b, _)| b)
                    .unwrap_or(0);
                f.value.remove(byte_idx);
                f.cursor -= 1;
            }
            app.refresh_plan();
            true
        }
        Delete if focus < 4 => {
            let f = &mut app.rules.fields[focus];
            if f.cursor < f.char_len() {
                let byte_idx = f.cursor_byte();
                f.value.remove(byte_idx);
            }
            app.refresh_plan();
            true
        }
        Home if focus < 4 => {
            app.rules.fields[focus].cursor = 0;
            true
        }
        End if focus < 4 => {
            let f = &mut app.rules.fields[focus];
            f.cursor = f.char_len();
            true
        }
        Enter if focus < 4 => {
            // Newline doesn't make sense — move focus down, same as Tab would.
            app.rules.focus = (focus + 1).min(5);
            true
        }
        Char(c) if focus < 4 => {
            let f = &mut app.rules.fields[focus];
            let byte_idx = f.cursor_byte();
            f.value.insert(byte_idx, c);
            f.cursor += 1;
            app.refresh_plan();
            true
        }
        Char(' ') if focus == 4 => {
            app.config.dry_run = !app.config.dry_run;
            true
        }
        Char(' ') if focus == 5 => {
            app.rules.collision_auto_num = !app.rules.collision_auto_num;
            true
        }
        Tab => {
            app.apply_rules();
            app.refresh_plan();
            if app.rules.fields[0].value.is_empty() {
                app.rules.show_error = true;
            } else {
                app.visited[2] = true;
                app.state = AppState::Wizard {
                    step: Step::Preview,
                };
            }
            true
        }
        BackTab => {
            app.apply_rules();
            app.state = AppState::Wizard { step: Step::Folder };
            app.visited[1] = true;
            true
        }
        _ => true,
    }
}

fn handle_preview(app: &mut App, code: KeyCode) -> bool {
    use KeyCode::*;
    match code {
        Up | Char('k') => {
            app.preview_scroll = app.preview_scroll.saturating_sub(1);
            true
        }
        Down | Char('j') => {
            app.preview_scroll = app.preview_scroll.saturating_add(1);
            true
        }
        Tab => {
            app.state = AppState::Wizard {
                step: Step::Execute,
            };
            true
        }
        BackTab => {
            app.apply_rules();
            app.state = AppState::Wizard { step: Step::Rules };
            app.visited[2] = true;
            true
        }
        Char('f' | 'F') => {
            // Cycle filter tabs — moved off Tab, which now always means
            // "advance a step" for consistency with every other step.
            app.preview_filter = (app.preview_filter + 1) % 4;
            app.preview_scroll = 0;
            true
        }
        _ => true,
    }
}

fn handle_execute(app: &mut App, code: KeyCode) -> bool {
    use KeyCode::*;
    match code {
        Enter | Tab => {
            app.start_execute();
            true
        }
        BackTab => {
            app.state = AppState::Wizard {
                step: Step::Preview,
            };
            true
        }
        _ => true,
    }
}

fn tab_forward(app: &mut App) {
    let step = app.current_step();
    if step == Step::Folder && !app.folder_confirmed {
        app.select_current_dir();
        return;
    }
    if let Some(next) = step.next() {
        let idx = next.index();
        app.visited[idx] = true;
        app.state = AppState::Wizard { step: next };
    }
}

fn jump_to(app: &mut App, idx: usize) {
    if idx <= 3 && app.visited[idx] {
        let step = Step::from_index(idx).unwrap();
        // Re-apply rules when jumping back to adjust
        app.state = AppState::Wizard { step };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: multi-byte UTF-8 must not panic field cursor.
    #[test]
    fn field_cursor_survives_multibyte_utf8() {
        let mut f = RulesField {
            label: "Series Title *",
            value: String::new(),
            placeholder: "",
            cursor: 0,
        };

        // Type "é" then "x" (mirrors the Char(c) arm in handle_rules).
        for c in ['é', 'x', '—', 'y'] {
            let byte_idx = f.cursor_byte();
            f.value.insert(byte_idx, c);
            f.cursor += 1;
        }
        assert_eq!(f.value, "éx—y");
        assert_eq!(f.cursor, 4); // 4 characters, not byte length

        // Backspace all the way (mirrors the Backspace arm) — must not panic.
        while f.cursor > 0 {
            let byte_idx = f
                .value
                .char_indices()
                .nth(f.cursor - 1)
                .map(|(b, _)| b)
                .unwrap_or(0);
            f.value.remove(byte_idx);
            f.cursor -= 1;
        }
        assert_eq!(f.value, "");
        assert_eq!(f.cursor, 0);
    }

    #[test]
    fn field_char_len_counts_characters_not_bytes() {
        let f = RulesField {
            label: "x",
            value: "呪術廻戦".to_string(),
            placeholder: "",
            cursor: 0,
        };
        assert_eq!(f.char_len(), 4); // 4 characters
        assert_eq!(f.value.len(), 12); // 12 bytes (3 bytes/char)
    }
}
