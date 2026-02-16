use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Flex, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};

use crate::model::Log;
use crate::storage::LogManager;

use super::action::Action;
use super::error::AppError;
use super::screens::log_create::{LogCreateState, draw_log_create};
use super::screens::log_select::{LogSelectState, draw_log_select};

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
        matches!(self.screen, Screen::LogCreate)
    }

    /// Applies an [`Action`] returned by a screen handler.
    fn apply_action(&mut self, action: Action) {
        match action {
            Action::None => {}
            Action::Quit => self.should_quit = true,
            Action::Navigate(screen) => self.navigate(screen),
            Action::SelectLog(log) => {
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
                self.current_log = Some(log);
                self.screen = Screen::QsoEntry;
            }
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
            operator: "W1AW".into(),
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
                operator: "W1AW".into(),
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
        fn q_on_non_log_select_screens_navigates_back() {
            let non_log_select = [Screen::QsoEntry, Screen::QsoList, Screen::Export];
            for screen in non_log_select {
                let (_dir, mut app) = make_app();
                app.screen = screen;
                app.handle_key(press(KeyCode::Char('q')));
                assert_eq!(
                    app.screen(),
                    Screen::LogSelect,
                    "q on {screen:?} should navigate to LogSelect"
                );
                assert!(!app.should_quit());
            }
        }

        #[test]
        fn esc_on_non_log_select_screens_navigates_back() {
            let non_log_select = [Screen::QsoEntry, Screen::QsoList, Screen::Export];
            for screen in non_log_select {
                let (_dir, mut app) = make_app();
                app.screen = screen;
                app.handle_key(press(KeyCode::Esc));
                assert_eq!(
                    app.screen(),
                    Screen::LogSelect,
                    "Esc on {screen:?} should navigate to LogSelect"
                );
                assert!(!app.should_quit());
            }
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
}
