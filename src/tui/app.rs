use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Flex, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};

use crate::model::Log;
use crate::storage::LogManager;

use super::error::AppError;

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
}

impl App {
    /// Creates a new `App` starting on the [`Screen::LogSelect`] screen.
    pub fn new(manager: LogManager) -> Self {
        Self {
            screen: Screen::LogSelect,
            manager,
            current_log: None,
            should_quit: false,
        }
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

    /// Renders the current screen as a placeholder.
    #[cfg_attr(coverage_nightly, coverage(off))]
    #[mutants::skip]
    fn draw(&self, frame: &mut Frame) {
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

    /// Handles a key event: global keys first, then screen-specific.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Char('?') => {
                if self.screen != Screen::Help {
                    self.screen = Screen::Help;
                }
            }
            KeyCode::Char('q') | KeyCode::Esc => match self.screen {
                Screen::LogSelect => self.should_quit = true,
                _ => self.screen = Screen::LogSelect,
            },
            _ => {}
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
        (dir, App::new(manager))
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

    #[test]
    fn new_starts_on_log_select() {
        let (_dir, app) = make_app();
        assert_eq!(app.screen(), Screen::LogSelect);
        assert!(!app.should_quit());
        assert!(app.current_log().is_none());
    }

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
        let non_log_select = [
            Screen::LogCreate,
            Screen::QsoEntry,
            Screen::QsoList,
            Screen::Export,
        ];
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
        let non_log_select = [
            Screen::LogCreate,
            Screen::QsoEntry,
            Screen::QsoList,
            Screen::Export,
        ];
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
