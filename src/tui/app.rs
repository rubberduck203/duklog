use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Flex, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};

use crate::model::Log;
use crate::storage::{self, LogManager};

use super::action::Action;
use super::error::AppError;
use super::screens::export::{ExportState, draw_export};
use super::screens::log_create::{LogCreateState, draw_log_create};
use super::screens::log_select::{LogSelectState, draw_log_select};
use super::screens::qso_entry::{QsoEntryState, draw_qso_entry};
use super::screens::qso_list::{QsoListState, draw_qso_list};

/// All screens the app can navigate between.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Screen {
    /// List and select existing logs.
    LogSelect,
    /// Create a new log session.
    LogCreate,
    /// Enter a new QSO into the active log.
    QsoEntry,
    /// View QSOs in the active log.
    QsoList,
    /// Export the active log to ADIF.
    Export,
    /// Show keybinding help.
    Help,
}

impl Screen {
    /// Human-readable label for placeholder rendering.
    fn label(self) -> &'static str {
        match self {
            Self::LogSelect => "Log Select",
            Self::LogCreate => "Log Create",
            Self::QsoEntry => "QSO Entry",
            Self::QsoList => "QSO List",
            Self::Export => "Export",
            Self::Help => "Help",
        }
    }
}

/// Top-level application state.
pub struct App {
    screen: Screen,
    manager: LogManager,
    current_log: Option<Log>,
    should_quit: bool,
    log_select: LogSelectState,
    log_create: LogCreateState,
    qso_entry: QsoEntryState,
    qso_list: QsoListState,
    export: ExportState,
}

impl App {
    /// Creates a new `App` starting on the [`Screen::LogSelect`] screen.
    ///
    /// Loads the initial log list from storage.
    pub fn new(manager: LogManager) -> Result<Self, AppError> {
        let mut log_select = LogSelectState::new();
        log_select.load(&manager)?;

        Ok(Self {
            screen: Screen::LogSelect,
            manager,
            current_log: None,
            should_quit: false,
            log_select,
            log_create: LogCreateState::new(),
            qso_entry: QsoEntryState::new(),
            qso_list: QsoListState::new(),
            export: ExportState::new(),
        })
    }

    /// Main event loop: draw → read event → dispatch → check quit.
    #[cfg_attr(coverage_nightly, coverage(off))]
    #[mutants::skip]
    pub fn run<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<(), AppError> {
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;
            if let Event::Key(key) = event::read()? {
                self.handle_key(key);
            }
        }
        Ok(())
    }

    /// Renders the current screen.
    #[cfg_attr(coverage_nightly, coverage(off))]
    #[mutants::skip]
    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        match self.screen {
            Screen::LogSelect => draw_log_select(&self.log_select, frame, area),
            Screen::LogCreate => draw_log_create(&self.log_create, frame, area),
            Screen::QsoEntry => {
                draw_qso_entry(&self.qso_entry, self.current_log.as_ref(), frame, area);
            }
            Screen::QsoList => {
                draw_qso_list(&self.qso_list, self.current_log.as_ref(), frame, area);
            }
            Screen::Export => {
                draw_export(&self.export, self.current_log.as_ref(), frame, area);
            }
            _ => self.draw_placeholder(frame),
        }
    }

    /// Renders a placeholder for screens not yet implemented.
    #[cfg_attr(coverage_nightly, coverage(off))]
    #[mutants::skip]
    fn draw_placeholder(&self, frame: &mut Frame) {
        let area = frame.area();

        let block = Block::default()
            .title(" duklog ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let label = self.screen.label();
        let lines = vec![
            Line::from(""),
            Line::from(format!("[ {label} ]")),
            Line::from("Press ? for help, q to quit"),
        ];
        let paragraph = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(block);

        let [centered] = Layout::vertical([Constraint::Min(0)])
            .flex(Flex::Center)
            .areas(area);
        frame.render_widget(paragraph, centered);
    }

    /// Handles a key event: global keys first, then screen-specific delegation.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        // Global `?` for help — except on form-based screens where it's a valid char.
        if key.code == KeyCode::Char('?') && !self.is_form_screen() {
            if self.screen != Screen::Help {
                self.screen = Screen::Help;
            }
            return;
        }

        let action = match self.screen {
            Screen::LogSelect => self.log_select.handle_key(key),
            Screen::LogCreate => self.log_create.handle_key(key),
            Screen::QsoEntry => self.qso_entry.handle_key(key),
            Screen::QsoList => {
                let count = self.current_log.as_ref().map_or(0, |l| l.qsos.len());
                self.qso_list.handle_key(key, count)
            }
            Screen::Export => self.export.handle_key(key),
            _ => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => Action::Navigate(Screen::LogSelect),
                _ => Action::None,
            },
        };

        self.apply_action(action);
    }

    /// Returns `true` if the current screen uses a text form where keys should
    /// be forwarded rather than intercepted globally.
    fn is_form_screen(&self) -> bool {
        matches!(self.screen, Screen::LogCreate | Screen::QsoEntry)
    }

    /// Applies an [`Action`] returned by a screen handler.
    fn apply_action(&mut self, action: Action) {
        match action {
            Action::None => {}
            Action::Quit => self.should_quit = true,
            Action::Navigate(screen) => self.navigate(screen),
            Action::SelectLog(log) => {
                self.qso_entry.set_log_context(&log);
                self.current_log = Some(log);
                self.screen = Screen::QsoEntry;
            }
            Action::CreateLog(log) => {
                if let Err(e) = self.manager.save_log(&log) {
                    self.log_select
                        .set_error(format!("Failed to save log: {e}"));
                    self.screen = Screen::LogSelect;
                    return;
                }
                self.qso_entry.set_log_context(&log);
                self.current_log = Some(log);
                self.screen = Screen::QsoEntry;
            }
            Action::ExportLog => match self.current_log {
                Some(ref log) => {
                    let path = Path::new(self.export.path());
                    match storage::export_adif(log, path) {
                        Ok(()) => self.export.set_success(),
                        Err(e) => self.export.set_error(e.to_string()),
                    }
                }
                None => {
                    self.export.set_error("No active log selected".into());
                }
            },
            Action::EditQso(index) => match self.current_log {
                Some(ref log) => match log.qsos.get(index) {
                    Some(qso) => {
                        self.qso_entry.start_editing(index, qso);
                        self.screen = Screen::QsoEntry;
                    }
                    None => {
                        self.qso_entry
                            .set_error(format!("QSO index {index} out of bounds"));
                        self.screen = Screen::QsoEntry;
                    }
                },
                None => {
                    self.qso_entry.set_error("No active log selected".into());
                    self.screen = Screen::QsoEntry;
                }
            },
            Action::UpdateQso(index, qso) => match self.current_log {
                Some(ref mut log) => {
                    if log.replace_qso(index, qso).is_none() {
                        self.qso_entry
                            .set_error(format!("QSO index {index} out of bounds"));
                        self.qso_entry.clear_editing();
                        return;
                    }
                    if let Err(e) = self.manager.save_log(log) {
                        self.qso_entry.set_error(format!("Failed to save log: {e}"));
                        self.qso_entry.clear_editing();
                        return;
                    }
                    self.qso_entry.clear_editing();
                    self.screen = Screen::QsoList;
                }
                None => {
                    self.qso_entry.set_error("No active log selected".into());
                }
            },
            Action::DeleteLog(log_id) => {
                if let Err(e) = self.manager.delete_log(&log_id) {
                    self.log_select
                        .set_error(format!("Failed to delete log: {e}"));
                    return;
                }
                if self
                    .current_log
                    .as_ref()
                    .is_some_and(|l| l.log_id == log_id)
                {
                    self.current_log = None;
                }
                if let Err(e) = self.log_select.load(&self.manager) {
                    self.log_select
                        .set_error(format!("Failed to load logs: {e}"));
                }
            }
            Action::AddQso(qso) => match self.current_log {
                Some(ref mut log) => {
                    let duplicate_warning = (!log.find_duplicates(&qso).is_empty()).then(|| {
                        format!(
                            "Warning: duplicate contact — {} {} {} already logged today",
                            qso.their_call, qso.band, qso.mode
                        )
                    });
                    if let Err(e) = self.manager.append_qso(&log.log_id, &qso) {
                        self.qso_entry.set_error(format!("Failed to save QSO: {e}"));
                        return;
                    }
                    log.add_qso(qso.clone());
                    self.qso_entry.add_recent_qso(qso);
                    self.qso_entry.clear_fast_fields();
                    if let Some(msg) = duplicate_warning {
                        self.qso_entry.set_error(msg);
                    }
                }
                None => {
                    self.qso_entry.set_error("No active log selected".into());
                }
            },
        }
    }

    /// Handles screen navigation with side effects (resetting forms, reloading logs).
    fn navigate(&mut self, screen: Screen) {
        match screen {
            Screen::LogSelect => {
                if let Err(e) = self.log_select.load(&self.manager) {
                    self.log_select
                        .set_error(format!("Failed to load logs: {e}"));
                }
                self.screen = Screen::LogSelect;
            }
            Screen::LogCreate => {
                self.log_create.reset();
                self.screen = Screen::LogCreate;
            }
            Screen::QsoEntry => {
                if let Some(ref log) = self.current_log {
                    self.qso_entry.set_log_context(log);
                }
                self.screen = Screen::QsoEntry;
            }
            Screen::QsoList => {
                self.qso_list.reset();
                self.screen = Screen::QsoList;
            }
            Screen::Export => {
                self.export.prepare(self.current_log.as_ref());
                self.screen = Screen::Export;
            }
            other => self.screen = other,
        }
    }

    /// Returns the current screen.
    pub fn screen(&self) -> Screen {
        self.screen
    }

    /// Returns `true` if the app should quit.
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Returns a reference to the [`LogManager`].
    pub fn manager(&self) -> &LogManager {
        &self.manager
    }

    /// Returns a reference to the current [`Log`], if any.
    pub fn current_log(&self) -> Option<&Log> {
        self.current_log.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    use super::*;
    use crate::storage::LogManager;

    fn make_app() -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let manager = LogManager::with_path(dir.path()).unwrap();
        (dir, App::new(manager).unwrap())
    }

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn release(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        }
    }

    fn shift_press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn save_test_log(manager: &LogManager, id: &str) -> Log {
        let log = Log {
            station_callsign: "W1AW".into(),
            operator: Some("W1AW".into()),
            park_ref: Some("K-0001".into()),
            grid_square: "FN31".into(),
            qsos: vec![],
            created_at: chrono::Utc::now(),
            log_id: id.into(),
        };
        manager.save_log(&log).unwrap();
        log
    }

    fn type_string(app: &mut App, s: &str) {
        for ch in s.chars() {
            app.handle_key(press(KeyCode::Char(ch)));
        }
    }

    fn fill_create_form(app: &mut App) {
        type_string(app, "W1AW");
        app.handle_key(press(KeyCode::Tab));
        type_string(app, "W1AW");
        app.handle_key(press(KeyCode::Tab));
        app.handle_key(press(KeyCode::Tab));
        type_string(app, "FN31");
    }

    mod construction {
        use super::*;

        #[test]
        fn starts_on_log_select() {
            let (_dir, app) = make_app();
            assert_eq!(app.screen(), Screen::LogSelect);
            assert!(!app.should_quit());
            assert!(app.current_log().is_none());
        }

        #[test]
        fn screen_labels_match_expected() {
            let expected = [
                (Screen::LogSelect, "Log Select"),
                (Screen::LogCreate, "Log Create"),
                (Screen::QsoEntry, "QSO Entry"),
                (Screen::QsoList, "QSO List"),
                (Screen::Export, "Export"),
                (Screen::Help, "Help"),
            ];
            for (screen, label) in expected {
                assert_eq!(screen.label(), label, "{screen:?} label mismatch");
            }
        }
    }

    mod accessors {
        use super::*;

        #[test]
        fn current_log_returns_set_log() {
            let (_dir, mut app) = make_app();
            assert!(app.current_log().is_none());

            let log = Log {
                station_callsign: "W1AW".into(),
                operator: Some("W1AW".into()),
                park_ref: None,
                grid_square: "FN31pr".into(),
                qsos: vec![],
                created_at: chrono::Utc::now(),
                log_id: "test".into(),
            };
            app.current_log = Some(log.clone());
            assert_eq!(app.current_log().unwrap().log_id, "test");
        }

        #[test]
        fn manager_accessor_returns_manager() {
            let (_dir, app) = make_app();
            let _manager = app.manager();
        }
    }

    mod global_keys {
        use super::*;

        #[test]
        fn q_on_log_select_quits() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('q')));
            assert!(app.should_quit());
        }

        #[test]
        fn esc_on_log_select_quits() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Esc));
            assert!(app.should_quit());
        }

        #[test]
        fn question_mark_navigates_to_help() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('?')));
            assert_eq!(app.screen(), Screen::Help);
            assert!(!app.should_quit());
        }

        #[test]
        fn q_on_help_navigates_to_log_select() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('?')));
            assert_eq!(app.screen(), Screen::Help);

            app.handle_key(press(KeyCode::Char('q')));
            assert_eq!(app.screen(), Screen::LogSelect);
            assert!(!app.should_quit());
        }

        #[test]
        fn esc_on_help_navigates_to_log_select() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('?')));
            app.handle_key(press(KeyCode::Esc));
            assert_eq!(app.screen(), Screen::LogSelect);
            assert!(!app.should_quit());
        }

        #[test]
        fn question_mark_on_help_stays_on_help() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('?')));
            app.handle_key(press(KeyCode::Char('?')));
            assert_eq!(app.screen(), Screen::Help);
        }

        #[test]
        fn release_events_are_ignored() {
            let (_dir, mut app) = make_app();
            app.handle_key(release(KeyCode::Char('q')));
            assert!(!app.should_quit());
            assert_eq!(app.screen(), Screen::LogSelect);
        }

        #[test]
        fn unhandled_key_is_ignored() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('x')));
            assert_eq!(app.screen(), Screen::LogSelect);
            assert!(!app.should_quit());
        }

        #[test]
        fn q_on_qso_list_navigates_to_qso_entry() {
            let (_dir, mut app) = make_app();
            app.screen = Screen::QsoList;
            app.handle_key(press(KeyCode::Char('q')));
            assert_eq!(app.screen(), Screen::QsoEntry);
            assert!(!app.should_quit());
        }

        #[test]
        fn esc_on_qso_list_navigates_to_qso_entry() {
            let (_dir, mut app) = make_app();
            app.screen = Screen::QsoList;
            app.handle_key(press(KeyCode::Esc));
            assert_eq!(app.screen(), Screen::QsoEntry);
            assert!(!app.should_quit());
        }

        #[test]
        fn question_mark_on_qso_list_navigates_to_help() {
            let (_dir, mut app) = make_app();
            app.screen = Screen::QsoList;
            app.handle_key(press(KeyCode::Char('?')));
            assert_eq!(app.screen(), Screen::Help);
        }

        #[test]
        fn navigate_to_qso_list_resets_state() {
            let (_dir, mut app) = make_app();
            app.qso_list.set_selected(5);
            app.navigate(Screen::QsoList);
            assert_eq!(app.screen(), Screen::QsoList);
            assert_eq!(app.qso_list.selected(), 0);
        }
    }

    mod log_create_integration {
        use super::*;

        #[test]
        fn n_on_log_select_navigates_to_log_create() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('n')));
            assert_eq!(app.screen(), Screen::LogCreate);
        }

        #[test]
        fn esc_on_log_create_returns_to_log_select() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('n')));
            assert_eq!(app.screen(), Screen::LogCreate);
            app.handle_key(press(KeyCode::Esc));
            assert_eq!(app.screen(), Screen::LogSelect);
        }

        #[test]
        fn q_on_log_create_types_q_not_quit() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('n')));
            assert_eq!(app.screen(), Screen::LogCreate);
            app.handle_key(press(KeyCode::Char('q')));
            assert_eq!(app.screen(), Screen::LogCreate);
            assert!(!app.should_quit());
        }

        #[test]
        fn question_mark_on_log_create_types_not_help() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('n')));
            app.handle_key(press(KeyCode::Char('?')));
            assert_eq!(app.screen(), Screen::LogCreate);
        }

        #[test]
        fn form_reset_on_navigate_to_log_create() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('n')));
            app.handle_key(press(KeyCode::Char('X')));
            app.handle_key(press(KeyCode::Esc));
            app.handle_key(press(KeyCode::Char('n')));
            assert_eq!(app.log_create.form().value(0), "");
        }

        #[test]
        fn valid_create_log_saves_and_navigates() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('n')));
            fill_create_form(&mut app);

            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::QsoEntry);
            assert!(app.current_log().is_some());
            assert_eq!(app.current_log().unwrap().station_callsign, "W1AW");
        }

        #[test]
        fn invalid_create_log_stays_on_form() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('n')));
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::LogCreate);
            assert!(app.current_log().is_none());
        }

        #[test]
        fn tab_cycles_form_fields() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('n')));
            assert_eq!(app.log_create.form().focus(), 0);
            app.handle_key(press(KeyCode::Tab));
            assert_eq!(app.log_create.form().focus(), 1);
            app.handle_key(shift_press(KeyCode::BackTab));
            assert_eq!(app.log_create.form().focus(), 0);
        }

        #[test]
        fn create_log_persists_to_storage() {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            let mut app = App::new(manager).unwrap();

            app.handle_key(press(KeyCode::Char('n')));
            fill_create_form(&mut app);
            app.handle_key(press(KeyCode::Enter));

            let logs = app.manager().list_logs().unwrap();
            assert_eq!(logs.len(), 1);
            assert_eq!(logs[0].station_callsign, "W1AW");
        }
    }

    mod log_select_integration {
        use super::*;

        #[test]
        fn select_log_navigates_to_qso_entry() {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            save_test_log(&manager, "test-log");
            let mut app = App::new(manager).unwrap();

            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::QsoEntry);
            assert_eq!(app.current_log().unwrap().log_id, "test-log");
        }

        #[test]
        fn enter_on_empty_log_list_is_noop() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::LogSelect);
        }

        #[test]
        fn log_list_reloads_on_return_to_select() {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            let mut app = App::new(manager).unwrap();
            assert!(app.log_select.logs().is_empty());

            save_test_log(app.manager(), "new-log");

            app.handle_key(press(KeyCode::Char('n')));
            app.handle_key(press(KeyCode::Esc));
            assert_eq!(app.screen(), Screen::LogSelect);
            assert_eq!(app.log_select.logs().len(), 1);
        }
    }

    mod delete_log_integration {
        use super::*;

        fn make_app_with_logs(ids: &[&str]) -> (tempfile::TempDir, App) {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            for id in ids {
                save_test_log(&manager, id);
            }
            let app = App::new(manager).unwrap();
            (dir, app)
        }

        #[test]
        fn d_on_empty_list_is_noop() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('d')));
            assert!(app.log_select.logs().is_empty());
            assert!(!app.should_quit());
        }

        #[test]
        fn d_sets_pending_confirmation() {
            let (_dir, mut app) = make_app_with_logs(&["log1"]);
            app.handle_key(press(KeyCode::Char('d')));
            assert!(app.log_select.pending_delete_label().is_some());
            assert_eq!(app.screen(), Screen::LogSelect);
        }

        #[test]
        fn d_then_y_deletes_log_and_reloads_list() {
            let (dir, mut app) = make_app_with_logs(&["log1"]);
            assert_eq!(app.log_select.logs().len(), 1);

            app.handle_key(press(KeyCode::Char('d')));
            app.handle_key(press(KeyCode::Char('y')));

            assert!(app.log_select.logs().is_empty());
            assert!(!dir.path().join("log1.jsonl").exists());
        }

        #[test]
        fn d_then_n_preserves_list() {
            let (_dir, mut app) = make_app_with_logs(&["log1"]);
            app.handle_key(press(KeyCode::Char('d')));
            app.handle_key(press(KeyCode::Char('n')));
            assert_eq!(app.log_select.logs().len(), 1);
            assert!(app.log_select.pending_delete_label().is_none());
        }

        #[test]
        fn d_then_esc_preserves_list() {
            let (_dir, mut app) = make_app_with_logs(&["log1"]);
            app.handle_key(press(KeyCode::Char('d')));
            app.handle_key(press(KeyCode::Esc));
            assert_eq!(app.log_select.logs().len(), 1);
            assert!(app.log_select.pending_delete_label().is_none());
        }

        #[test]
        fn deleting_one_of_two_logs_leaves_one() {
            let (_dir, mut app) = make_app_with_logs(&["log1", "log2"]);
            assert_eq!(app.log_select.logs().len(), 2);

            app.handle_key(press(KeyCode::Char('d')));
            app.handle_key(press(KeyCode::Char('y')));

            assert_eq!(app.log_select.logs().len(), 1);
        }

        #[test]
        fn deleting_only_log_leaves_empty_list_with_no_selection() {
            let (_dir, mut app) = make_app_with_logs(&["log1"]);
            app.handle_key(press(KeyCode::Char('d')));
            app.handle_key(press(KeyCode::Char('y')));

            assert!(app.log_select.logs().is_empty());
            assert_eq!(app.log_select.selected(), None);
        }

        #[test]
        fn deleting_current_log_clears_current_log() {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            save_test_log(&manager, "log1");
            let mut app = App::new(manager).unwrap();

            // Open the log (sets current_log)
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::QsoEntry);
            assert!(app.current_log().is_some());

            // Return to log select, then delete it
            app.handle_key(press(KeyCode::Esc));
            assert_eq!(app.screen(), Screen::LogSelect);
            app.handle_key(press(KeyCode::Char('d')));
            app.handle_key(press(KeyCode::Char('y')));

            assert!(app.current_log().is_none());
        }

        #[test]
        fn deleting_different_log_preserves_current_log() {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            save_test_log(&manager, "log1");
            save_test_log(&manager, "log2");
            let mut app = App::new(manager).unwrap();

            // log list is sorted newest-first; select the highlighted one
            app.handle_key(press(KeyCode::Enter));
            assert!(app.current_log().is_some());
            let open_id = app.current_log().unwrap().log_id.clone();

            // Return to log select; the other log is now highlighted at position 0
            app.handle_key(press(KeyCode::Esc));
            // Delete whichever is highlighted (not necessarily the open one)
            app.handle_key(press(KeyCode::Char('d')));
            app.handle_key(press(KeyCode::Char('y')));

            // If the deleted log was not the open one, current_log should remain
            let remaining_ids: Vec<_> = app.log_select.logs().iter().map(|l| &l.log_id).collect();
            if remaining_ids.contains(&&open_id) {
                assert!(
                    app.current_log().is_some(),
                    "current_log should be preserved when a different log is deleted"
                );
            } else {
                assert!(
                    app.current_log().is_none(),
                    "current_log should be cleared when it was deleted"
                );
            }
        }

        #[test]
        fn delete_storage_error_preserves_error_message() {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            save_test_log(&manager, "log1");
            let mut app = App::new(manager).unwrap();

            // Remove the log file to cause a delete error, but keep the dir
            std::fs::remove_file(dir.path().join("log1.jsonl")).unwrap();

            app.apply_action(Action::DeleteLog("log1".into()));

            // Error should be set and not overwritten by a reload
            assert!(app.log_select.error().is_some(), "should show delete error");
        }
    }

    mod qso_entry_integration {
        use super::*;

        fn make_app_with_log() -> (tempfile::TempDir, App) {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            save_test_log(&manager, "test-log");
            let mut app = App::new(manager).unwrap();
            // Select the log to navigate to QsoEntry
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::QsoEntry);
            (dir, app)
        }

        fn submit_qso(app: &mut App) {
            type_string(app, "KD9XYZ");
            app.handle_key(press(KeyCode::Enter));
        }

        #[test]
        fn submit_qso_persists_to_storage() {
            let (_dir, mut app) = make_app_with_log();
            submit_qso(&mut app);

            // Should still be on QsoEntry
            assert_eq!(app.screen(), Screen::QsoEntry);
            // QSO should be in the current log
            assert_eq!(app.current_log().unwrap().qsos.len(), 1);
            assert_eq!(app.current_log().unwrap().qsos[0].their_call, "KD9XYZ");
            // QSO should be persisted
            let loaded = app.manager().load_log("test-log").unwrap();
            assert_eq!(loaded.qsos.len(), 1);
        }

        #[test]
        fn submit_qso_clears_form() {
            let (_dir, mut app) = make_app_with_log();
            submit_qso(&mut app);

            // Callsign should be cleared
            assert_eq!(app.qso_entry.form().value(0), "");
            // RST should be repopulated
            assert_eq!(app.qso_entry.form().value(1), "59");
        }

        #[test]
        fn submit_qso_adds_to_recent() {
            let (_dir, mut app) = make_app_with_log();
            submit_qso(&mut app);

            assert_eq!(app.qso_entry.recent_qsos().len(), 1);
            assert_eq!(app.qso_entry.recent_qsos()[0].their_call, "KD9XYZ");
        }

        #[test]
        fn esc_from_qso_entry_returns_to_log_select() {
            let (_dir, mut app) = make_app_with_log();
            app.handle_key(press(KeyCode::Esc));
            assert_eq!(app.screen(), Screen::LogSelect);
        }

        #[test]
        fn question_mark_on_qso_entry_types_char() {
            let (_dir, mut app) = make_app_with_log();
            app.handle_key(press(KeyCode::Char('?')));
            // Should NOT navigate to Help, should stay on QsoEntry
            assert_eq!(app.screen(), Screen::QsoEntry);
        }

        #[test]
        fn create_log_then_submit_qso() {
            let (_dir, mut app) = make_app();
            app.handle_key(press(KeyCode::Char('n')));
            fill_create_form(&mut app);
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::QsoEntry);

            // Submit a QSO
            type_string(&mut app, "N0CALL");
            app.handle_key(press(KeyCode::Enter));

            assert_eq!(app.current_log().unwrap().qsos.len(), 1);
            let log_id = app.current_log().unwrap().log_id.clone();
            let loaded = app.manager().load_log(&log_id).unwrap();
            assert_eq!(loaded.qsos.len(), 1);
        }

        #[test]
        fn select_log_populates_recent_qsos() {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            let log = save_test_log(&manager, "test-log");

            // Add a QSO to the log file
            let qso = crate::model::Qso::new(
                "W3ABC".to_string(),
                "59".to_string(),
                "59".to_string(),
                crate::model::Band::M20,
                crate::model::Mode::Ssb,
                chrono::Utc::now(),
                String::new(),
                None,
            )
            .unwrap();
            manager.append_qso(&log.log_id, &qso).unwrap();

            // Reload the log so it has the QSO
            let mut app = App::new(LogManager::with_path(dir.path()).unwrap()).unwrap();

            // Need to load the log with QSOs — the log_select loads them
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::QsoEntry);
            assert_eq!(app.qso_entry.recent_qsos().len(), 1);
        }

        #[test]
        fn multiple_qsos_persist_and_accumulate() {
            let (_dir, mut app) = make_app_with_log();

            type_string(&mut app, "W1AW");
            app.handle_key(press(KeyCode::Enter));
            type_string(&mut app, "N0CALL");
            app.handle_key(press(KeyCode::Enter));

            assert_eq!(app.current_log().unwrap().qsos.len(), 2);
            assert_eq!(app.qso_entry.recent_qsos().len(), 2);
            let loaded = app.manager().load_log("test-log").unwrap();
            assert_eq!(loaded.qsos.len(), 2);
        }

        #[test]
        fn navigate_to_qso_entry_sets_log_context() {
            let (_dir, mut app) = make_app_with_log();
            // Submit a QSO
            submit_qso(&mut app);
            // Navigate away and back
            app.handle_key(press(KeyCode::Esc));
            assert_eq!(app.screen(), Screen::LogSelect);
            // Select the same log again
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::QsoEntry);
            // Recent QSOs should be repopulated from log
            assert_eq!(app.qso_entry.recent_qsos().len(), 1);
        }

        #[test]
        fn navigate_action_to_qso_entry_sets_context() {
            let (_dir, mut app) = make_app_with_log();
            // Go back to LogSelect
            app.handle_key(press(KeyCode::Esc));
            // Manually trigger Navigate(QsoEntry) through apply_action
            app.apply_action(Action::Navigate(Screen::QsoEntry));
            assert_eq!(app.screen(), Screen::QsoEntry);
        }

        #[test]
        fn add_qso_without_active_log_shows_error() {
            let (_dir, mut app) = make_app();
            // Force into QsoEntry without selecting a log
            app.screen = Screen::QsoEntry;
            assert!(app.current_log().is_none());

            type_string(&mut app, "KD9XYZ");
            app.handle_key(press(KeyCode::Enter));

            assert!(app.qso_entry.error().is_some());
            assert!(app.qso_entry.error().unwrap().contains("No active log"),);
        }

        fn alt_press(code: KeyCode) -> KeyEvent {
            KeyEvent {
                code,
                modifiers: KeyModifiers::ALT,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            }
        }

        #[test]
        fn duplicate_qso_shows_warning_but_still_logged() {
            let (_dir, mut app) = make_app_with_log();
            // First QSO — no duplicate
            submit_qso(&mut app);
            assert_eq!(app.current_log().unwrap().qsos.len(), 1);
            assert_eq!(app.qso_entry.error(), None);

            // Second QSO with same call+band+mode — duplicate warning shown
            submit_qso(&mut app);
            assert_eq!(app.current_log().unwrap().qsos.len(), 2);
            let err = app
                .qso_entry
                .error()
                .expect("should show duplicate warning");
            assert!(err.contains("Warning"), "should say Warning, got: {err}");
            assert!(
                err.contains("duplicate"),
                "should say duplicate, got: {err}"
            );
            assert!(
                err.contains("KD9XYZ"),
                "should name the callsign, got: {err}"
            );
            assert!(err.contains("20M"), "should name the band, got: {err}");
            assert!(err.contains("SSB"), "should name the mode, got: {err}");
            assert!(err.contains("today"), "should say today, got: {err}");
        }

        #[test]
        fn no_warning_for_different_band() {
            let (_dir, mut app) = make_app_with_log();
            submit_qso(&mut app);

            // Same call, different band — not a duplicate
            app.handle_key(alt_press(KeyCode::Char('b'))); // cycle band
            submit_qso(&mut app);
            assert_eq!(app.qso_entry.error(), None);
        }

        #[test]
        fn no_warning_for_different_mode() {
            let (_dir, mut app) = make_app_with_log();
            submit_qso(&mut app);

            // Same call, different mode — not a duplicate
            app.handle_key(alt_press(KeyCode::Char('m'))); // cycle mode
            submit_qso(&mut app);
            assert_eq!(app.qso_entry.error(), None);
        }

        #[test]
        fn warning_cleared_when_next_qso_is_not_duplicate() {
            let (_dir, mut app) = make_app_with_log();
            // Log duplicate to get warning
            submit_qso(&mut app);
            submit_qso(&mut app);
            assert!(app.qso_entry.error().is_some());

            // Submit non-duplicate (different callsign via a fresh submit_qso would
            // still be KD9XYZ — type a different call manually)
            type_string(&mut app, "W3ABC");
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.current_log().unwrap().qsos.len(), 3);
            assert_eq!(app.qso_entry.error(), None);
        }

        #[test]
        fn storage_error_on_append_shows_error() {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            save_test_log(&manager, "test-log");
            let mut app = App::new(manager).unwrap();
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::QsoEntry);

            // Delete the log file to cause an append error
            std::fs::remove_file(dir.path().join("test-log.jsonl")).unwrap();

            type_string(&mut app, "KD9XYZ");
            app.handle_key(press(KeyCode::Enter));

            // Should show error, QSO not added
            assert!(app.qso_entry.error().is_some());
            assert_eq!(app.current_log().unwrap().qsos.len(), 0);
        }
    }

    mod export_integration {
        use super::*;
        use crate::tui::screens::export::ExportStatus;

        fn make_app_with_log() -> (tempfile::TempDir, App) {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            save_test_log(&manager, "test-log");
            let mut app = App::new(manager).unwrap();
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::QsoEntry);
            (dir, app)
        }

        fn alt_press(code: KeyCode) -> KeyEvent {
            KeyEvent {
                code,
                modifiers: KeyModifiers::ALT,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            }
        }

        #[test]
        fn alt_x_navigates_to_export() {
            let (_dir, mut app) = make_app_with_log();
            app.handle_key(alt_press(KeyCode::Char('x')));
            assert_eq!(app.screen(), Screen::Export);
        }

        #[test]
        fn export_screen_shows_path_and_count() {
            let (_dir, mut app) = make_app_with_log();
            // Add a QSO first
            type_string(&mut app, "KD9XYZ");
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.current_log().unwrap().qsos.len(), 1);

            app.handle_key(alt_press(KeyCode::Char('x')));
            assert_eq!(app.screen(), Screen::Export);
            assert_eq!(app.export.qso_count(), 1);
            assert!(!app.export.path().is_empty());
        }

        #[test]
        fn export_log_writes_file() {
            let (_dir, mut app) = make_app_with_log();
            type_string(&mut app, "KD9XYZ");
            app.handle_key(press(KeyCode::Enter));

            // Navigate to export to populate the default path
            app.handle_key(alt_press(KeyCode::Char('x')));
            assert_eq!(app.screen(), Screen::Export);

            // Override path to a temp directory so we don't pollute ~/
            let export_dir = tempfile::tempdir().unwrap();
            let export_path = export_dir.path().join("test.adif");
            app.export.set_path(export_path.display().to_string());

            app.apply_action(Action::ExportLog);
            assert_eq!(app.export.status(), &ExportStatus::Success);

            let content = std::fs::read_to_string(&export_path).unwrap();
            assert!(content.contains("<eoh>"));
            assert!(content.contains("KD9XYZ"));
        }

        #[test]
        fn export_to_invalid_path_sets_error() {
            let (_dir, mut app) = make_app_with_log();
            type_string(&mut app, "KD9XYZ");
            app.handle_key(press(KeyCode::Enter));

            app.handle_key(alt_press(KeyCode::Char('x')));
            // Point to a nonexistent directory to trigger an I/O error
            app.export.set_path("/nonexistent/dir/test.adif".into());

            app.apply_action(Action::ExportLog);
            match app.export.status() {
                ExportStatus::Error(msg) => {
                    assert!(!msg.is_empty(), "error message should not be empty");
                }
                other => panic!("expected Error, got {other:?}"),
            }
        }

        #[test]
        fn esc_on_export_returns_to_qso_entry() {
            let (_dir, mut app) = make_app_with_log();
            app.handle_key(alt_press(KeyCode::Char('x')));
            assert_eq!(app.screen(), Screen::Export);
            app.handle_key(press(KeyCode::Esc));
            assert_eq!(app.screen(), Screen::QsoEntry);
        }

        #[test]
        fn q_on_export_returns_to_qso_entry() {
            let (_dir, mut app) = make_app_with_log();
            app.handle_key(alt_press(KeyCode::Char('x')));
            assert_eq!(app.screen(), Screen::Export);
            app.handle_key(press(KeyCode::Char('q')));
            assert_eq!(app.screen(), Screen::QsoEntry);
        }

        #[test]
        fn any_key_after_success_returns_to_qso_entry() {
            let (_dir, mut app) = make_app_with_log();
            app.handle_key(alt_press(KeyCode::Char('x')));
            app.export.set_success();
            app.handle_key(press(KeyCode::Char('a')));
            assert_eq!(app.screen(), Screen::QsoEntry);
        }

        #[test]
        fn export_without_current_log_shows_error() {
            let (_dir, mut app) = make_app();
            app.screen = Screen::Export;
            app.apply_action(Action::ExportLog);
            match app.export.status() {
                ExportStatus::Error(msg) => {
                    assert!(msg.contains("No active log"));
                }
                other => panic!("expected Error, got {other:?}"),
            }
        }

        #[test]
        fn navigate_to_export_prepares_state() {
            let (_dir, mut app) = make_app_with_log();
            app.navigate(Screen::Export);
            assert_eq!(app.screen(), Screen::Export);
            assert!(!app.export.path().is_empty());
        }

        #[test]
        fn question_mark_on_export_navigates_to_help() {
            let (_dir, mut app) = make_app_with_log();
            app.handle_key(alt_press(KeyCode::Char('x')));
            assert_eq!(app.screen(), Screen::Export);
            app.handle_key(press(KeyCode::Char('?')));
            assert_eq!(app.screen(), Screen::Help);
        }
    }

    mod qso_list_integration {
        use super::*;

        fn alt_press(code: KeyCode) -> KeyEvent {
            KeyEvent {
                code,
                modifiers: KeyModifiers::ALT,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            }
        }

        fn make_app_with_log() -> (tempfile::TempDir, App) {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            save_test_log(&manager, "test-log");
            let mut app = App::new(manager).unwrap();
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::QsoEntry);
            (dir, app)
        }

        #[test]
        fn alt_e_navigates_to_qso_list() {
            let (_dir, mut app) = make_app_with_log();
            app.handle_key(alt_press(KeyCode::Char('e')));
            assert_eq!(app.screen(), Screen::QsoList);
        }

        #[test]
        fn q_on_qso_list_returns_to_qso_entry() {
            let (_dir, mut app) = make_app_with_log();
            app.handle_key(alt_press(KeyCode::Char('e')));
            assert_eq!(app.screen(), Screen::QsoList);
            app.handle_key(press(KeyCode::Char('q')));
            assert_eq!(app.screen(), Screen::QsoEntry);
        }

        #[test]
        fn esc_on_qso_list_returns_to_qso_entry() {
            let (_dir, mut app) = make_app_with_log();
            app.handle_key(alt_press(KeyCode::Char('e')));
            assert_eq!(app.screen(), Screen::QsoList);
            app.handle_key(press(KeyCode::Esc));
            assert_eq!(app.screen(), Screen::QsoEntry);
        }

        #[test]
        fn navigate_to_qso_list_resets_selected() {
            let (_dir, mut app) = make_app_with_log();
            app.qso_list.set_selected(5);
            app.handle_key(alt_press(KeyCode::Char('e')));
            assert_eq!(app.screen(), Screen::QsoList);
            assert_eq!(app.qso_list.selected(), 0);
        }

        #[test]
        fn arrow_keys_navigate_qso_list() {
            let (_dir, mut app) = make_app_with_log();
            // Add QSOs
            type_string(&mut app, "KD9XYZ");
            app.handle_key(press(KeyCode::Enter));
            type_string(&mut app, "W3ABC");
            app.handle_key(press(KeyCode::Enter));

            app.handle_key(alt_press(KeyCode::Char('e')));
            assert_eq!(app.screen(), Screen::QsoList);
            assert_eq!(app.qso_list.selected(), 0);

            app.handle_key(press(KeyCode::Down));
            assert_eq!(app.qso_list.selected(), 1);

            app.handle_key(press(KeyCode::Up));
            assert_eq!(app.qso_list.selected(), 0);
        }

        #[test]
        fn enter_on_qso_list_opens_edit_mode() {
            let (_dir, mut app) = make_app_with_log();
            type_string(&mut app, "KD9XYZ");
            app.handle_key(press(KeyCode::Enter));

            app.handle_key(alt_press(KeyCode::Char('e')));
            assert_eq!(app.screen(), Screen::QsoList);

            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::QsoEntry);
            assert!(app.qso_entry.is_editing());
            assert_eq!(app.qso_entry.form().value(0), "KD9XYZ");
        }

        #[test]
        fn edit_save_returns_to_list_with_selection() {
            let (_dir, mut app) = make_app_with_log();
            // Add two QSOs
            type_string(&mut app, "KD9XYZ");
            app.handle_key(press(KeyCode::Enter));
            type_string(&mut app, "W3ABC");
            app.handle_key(press(KeyCode::Enter));

            // Go to list, select second QSO, edit it
            app.handle_key(alt_press(KeyCode::Char('e')));
            app.handle_key(press(KeyCode::Down));
            assert_eq!(app.qso_list.selected(), 1);
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::QsoEntry);
            assert!(app.qso_entry.is_editing());

            // Submit the edit (just submit as-is)
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::QsoList);
            assert!(!app.qso_entry.is_editing());
            // Selection should be preserved
            assert_eq!(app.qso_list.selected(), 1);
        }

        #[test]
        fn cancel_edit_returns_to_list() {
            let (_dir, mut app) = make_app_with_log();
            type_string(&mut app, "KD9XYZ");
            app.handle_key(press(KeyCode::Enter));

            app.handle_key(alt_press(KeyCode::Char('e')));
            app.handle_key(press(KeyCode::Enter));
            assert_eq!(app.screen(), Screen::QsoEntry);
            assert!(app.qso_entry.is_editing());

            app.handle_key(press(KeyCode::Esc));
            assert_eq!(app.screen(), Screen::QsoList);
            assert!(!app.qso_entry.is_editing());
        }

        #[test]
        fn edit_persists_to_storage() {
            let (_dir, mut app) = make_app_with_log();
            type_string(&mut app, "KD9XYZ");
            app.handle_key(press(KeyCode::Enter));

            // Open edit, change callsign
            app.handle_key(alt_press(KeyCode::Char('e')));
            app.handle_key(press(KeyCode::Enter));

            // Clear callsign and type a new one
            for _ in 0..6 {
                app.handle_key(press(KeyCode::Backspace));
            }
            type_string(&mut app, "N0CALL");
            app.handle_key(press(KeyCode::Enter));

            // Verify in-memory
            assert_eq!(app.current_log().unwrap().qsos[0].their_call, "N0CALL");
            // Verify on disk
            let loaded = app.manager().load_log("test-log").unwrap();
            assert_eq!(loaded.qsos[0].their_call, "N0CALL");
        }

        #[test]
        fn edit_qso_without_active_log_shows_error() {
            let (_dir, mut app) = make_app();
            app.screen = Screen::QsoList;
            app.apply_action(Action::EditQso(0));
            assert_eq!(app.screen(), Screen::QsoEntry);
            assert!(app.qso_entry.error().is_some());
            assert!(app.qso_entry.error().unwrap().contains("No active log"));
        }

        #[test]
        fn edit_qso_out_of_bounds_shows_error() {
            let (_dir, mut app) = make_app_with_log();
            app.apply_action(Action::EditQso(99));
            assert_eq!(app.screen(), Screen::QsoEntry);
            assert!(app.qso_entry.error().is_some());
            assert!(app.qso_entry.error().unwrap().contains("out of bounds"));
        }

        #[test]
        fn update_qso_storage_error_shows_error() {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            save_test_log(&manager, "test-log");
            let mut app = App::new(manager).unwrap();
            app.handle_key(press(KeyCode::Enter));

            // Add a QSO
            type_string(&mut app, "KD9XYZ");
            app.handle_key(press(KeyCode::Enter));

            // Delete the storage dir to cause save error
            std::fs::remove_file(dir.path().join("test-log.jsonl")).unwrap();
            std::fs::remove_dir_all(dir.path()).unwrap();

            let qso = app.current_log().unwrap().qsos[0].clone();
            app.apply_action(Action::UpdateQso(0, qso));
            assert!(app.qso_entry.error().is_some());
            assert!(!app.qso_entry.is_editing());
        }

        #[test]
        fn update_qso_out_of_bounds_shows_error() {
            let (_dir, mut app) = make_app_with_log();
            type_string(&mut app, "KD9XYZ");
            app.handle_key(press(KeyCode::Enter));

            let qso = app.current_log().unwrap().qsos[0].clone();
            app.apply_action(Action::UpdateQso(99, qso));
            assert!(app.qso_entry.error().is_some());
            assert!(app.qso_entry.error().unwrap().contains("out of bounds"));
            assert!(!app.qso_entry.is_editing());
        }

        #[test]
        fn enter_on_empty_qso_list_does_nothing() {
            let (_dir, mut app) = make_app_with_log();
            app.handle_key(alt_press(KeyCode::Char('e')));
            assert_eq!(app.screen(), Screen::QsoList);
            app.handle_key(press(KeyCode::Enter));
            // Should stay on QsoList since no QSOs
            assert_eq!(app.screen(), Screen::QsoList);
        }
    }
}
