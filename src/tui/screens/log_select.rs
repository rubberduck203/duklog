//! Log selection screen — lists existing logs for the user to choose from.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use crate::model::Log;
use crate::storage::{LogManager, StorageError};
use crate::tui::action::Action;
use crate::tui::app::Screen;

/// State for the log selection screen.
#[derive(Debug, Clone)]
pub struct LogSelectState {
    /// Cached list of logs from storage.
    logs: Vec<Log>,
    /// Index of the currently highlighted log, or `None` if the list is empty.
    selected: Option<usize>,
    /// Error message from the last failed operation.
    error: Option<String>,
}

impl Default for LogSelectState {
    fn default() -> Self {
        Self::new()
    }
}

impl LogSelectState {
    /// Creates an empty state. Call [`load`](Self::load) to populate from storage.
    pub fn new() -> Self {
        Self {
            logs: Vec::new(),
            selected: None,
            error: None,
        }
    }

    /// Loads the log list from the given manager, updating selection state.
    pub fn load(&mut self, manager: &LogManager) -> Result<(), StorageError> {
        self.logs = manager.list_logs()?;
        self.selected = if self.logs.is_empty() { None } else { Some(0) };
        self.error = None;
        Ok(())
    }

    /// Handles a key event, returning an [`Action`] for the app to apply.
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up => {
                self.select_prev();
                Action::None
            }
            KeyCode::Down => {
                self.select_next();
                Action::None
            }
            KeyCode::Enter => self.select_current(),
            KeyCode::Char('n') => Action::Navigate(Screen::LogCreate),
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            _ => Action::None,
        }
    }

    /// Returns the cached log list.
    pub fn logs(&self) -> &[Log] {
        &self.logs
    }

    /// Returns the selected index.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Returns the current error message, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Sets an error message to display on this screen.
    pub fn set_error(&mut self, msg: String) {
        self.error = Some(msg);
    }

    /// Returns an action to open the currently selected log.
    fn select_current(&self) -> Action {
        match self.selected {
            Some(i) => self
                .logs
                .get(i)
                .map_or(Action::None, |log| Action::SelectLog(log.clone())),
            None => Action::None,
        }
    }

    /// Moves the selection up by one (no wrap).
    fn select_prev(&mut self) {
        self.selected = match self.selected {
            Some(i) if i > 0 => Some(i - 1),
            other => other,
        };
    }

    /// Moves the selection down by one (no wrap).
    fn select_next(&mut self) {
        self.selected = match self.selected {
            Some(i) if i + 1 < self.logs.len() => Some(i + 1),
            other => other,
        };
    }
}

/// Renders the log selection screen.
#[mutants::skip]
pub fn draw_log_select(state: &LogSelectState, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Select Log ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    if state.logs().is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from("No logs found."),
            Line::from("Press 'n' to create a new log."),
        ];
        let paragraph = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let header = Row::new(vec!["Park", "Callsign", "Date", "QSOs"])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    let rows: Vec<Row> = state
        .logs()
        .iter()
        .enumerate()
        .map(|(i, log)| {
            let style = if state.selected() == Some(i) {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default()
            };
            Row::new(vec![
                log.park_ref.as_deref().unwrap_or("-").to_string(),
                log.station_callsign.clone(),
                log.created_at.format("%Y-%m-%d").to_string(),
                log.qsos.len().to_string(),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(6),
    ];

    let table = Table::new(rows, widths).header(header);

    let [table_area, footer_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);

    frame.render_widget(table, table_area);

    let footer =
        Paragraph::new("n: new  Enter: open  q: quit").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);

    if let Some(err) = state.error() {
        let err_line = Paragraph::new(err)
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        frame.render_widget(err_line, footer_area);
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    use super::*;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn make_log(id: &str, callsign: &str, park: Option<&str>) -> Log {
        Log {
            station_callsign: callsign.into(),
            operator: Some(callsign.into()),
            park_ref: park.map(Into::into),
            grid_square: "FN31".into(),
            qsos: vec![],
            created_at: Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap(),
            log_id: id.into(),
        }
    }

    fn make_populated_state() -> LogSelectState {
        LogSelectState {
            logs: vec![
                make_log("log1", "W1AW", Some("K-0001")),
                make_log("log2", "N0CALL", None),
                make_log("log3", "KD9XYZ", Some("K-1234")),
            ],
            selected: Some(0),
            error: None,
        }
    }

    mod construction {
        use super::*;

        #[test]
        fn new_starts_empty() {
            let state = LogSelectState::new();
            assert!(state.logs().is_empty());
            assert_eq!(state.selected(), None);
            assert_eq!(state.error(), None);
        }
    }

    mod load {
        use super::*;

        #[test]
        fn populates_from_manager() {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            let log = make_log("test", "W1AW", Some("K-0001"));
            manager.save_log(&log).unwrap();

            let mut state = LogSelectState::new();
            state.load(&manager).unwrap();
            assert_eq!(state.logs().len(), 1);
            assert_eq!(state.selected(), Some(0));
        }

        #[test]
        fn empty_directory() {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();

            let mut state = LogSelectState::new();
            state.load(&manager).unwrap();
            assert!(state.logs().is_empty());
            assert_eq!(state.selected(), None);
        }

        #[test]
        fn clears_error() {
            let dir = tempfile::tempdir().unwrap();
            let manager = LogManager::with_path(dir.path()).unwrap();
            let mut state = LogSelectState::new();
            state.set_error("old error".into());
            state.load(&manager).unwrap();
            assert_eq!(state.error(), None);
        }
    }

    mod navigation {
        use super::*;

        #[test]
        fn down_moves_selection() {
            let mut state = make_populated_state();
            let action = state.handle_key(press(KeyCode::Down));
            assert_eq!(action, Action::None);
            assert_eq!(state.selected(), Some(1));
        }

        #[test]
        fn up_moves_selection() {
            let mut state = make_populated_state();
            state.selected = Some(2);
            let action = state.handle_key(press(KeyCode::Up));
            assert_eq!(action, Action::None);
            assert_eq!(state.selected(), Some(1));
        }

        #[test]
        fn down_at_bottom_is_noop() {
            let mut state = make_populated_state();
            state.selected = Some(2);
            let action = state.handle_key(press(KeyCode::Down));
            assert_eq!(action, Action::None);
            assert_eq!(state.selected(), Some(2));
        }

        #[test]
        fn up_at_top_is_noop() {
            let mut state = make_populated_state();
            let action = state.handle_key(press(KeyCode::Up));
            assert_eq!(action, Action::None);
            assert_eq!(state.selected(), Some(0));
        }

        #[test]
        fn empty_list_is_noop() {
            let mut state = LogSelectState::new();
            assert_eq!(state.handle_key(press(KeyCode::Up)), Action::None);
            assert_eq!(state.handle_key(press(KeyCode::Down)), Action::None);
            assert_eq!(state.handle_key(press(KeyCode::Enter)), Action::None);
        }
    }

    mod selection {
        use super::*;

        #[test]
        fn enter_selects_current_log() {
            let mut state = make_populated_state();
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::SelectLog(log) => assert_eq!(log.log_id, "log1"),
                other => panic!("expected SelectLog, got {other:?}"),
            }
        }

        #[test]
        fn n_navigates_to_log_create() {
            let mut state = make_populated_state();
            let action = state.handle_key(press(KeyCode::Char('n')));
            assert_eq!(action, Action::Navigate(Screen::LogCreate));
        }
    }

    mod quit {
        use super::*;

        #[test]
        fn q_quits() {
            let mut state = make_populated_state();
            assert_eq!(state.handle_key(press(KeyCode::Char('q'))), Action::Quit);
        }

        #[test]
        fn esc_quits() {
            let mut state = make_populated_state();
            assert_eq!(state.handle_key(press(KeyCode::Esc)), Action::Quit);
        }

        #[test]
        fn unhandled_key_returns_none() {
            let mut state = make_populated_state();
            assert_eq!(state.handle_key(press(KeyCode::Char('x'))), Action::None);
        }
    }

    mod rendering {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        use super::*;

        fn buffer_to_string(buf: &ratatui::buffer::Buffer) -> String {
            let mut s = String::new();
            for y in 0..buf.area.height {
                for x in 0..buf.area.width {
                    s.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
                }
                s.push('\n');
            }
            s
        }

        fn render_log_select(state: &LogSelectState, width: u16, height: u16) -> String {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| {
                    draw_log_select(state, frame, frame.area());
                })
                .unwrap();
            buffer_to_string(terminal.backend().buffer())
        }

        #[test]
        fn renders_empty_state() {
            let state = LogSelectState::new();
            let output = render_log_select(&state, 60, 10);
            assert!(
                output.contains("No logs found"),
                "should show empty message"
            );
            assert!(output.contains("Select Log"), "should show title");
        }

        #[test]
        fn renders_log_table() {
            let state = make_populated_state();
            let output = render_log_select(&state, 60, 12);
            assert!(output.contains("K-0001"), "should show park ref");
            assert!(output.contains("W1AW"), "should show callsign");
            assert!(output.contains("N0CALL"), "should show second callsign");
            assert!(output.contains("Park"), "should show table header");
            assert!(output.contains("Callsign"), "should show table header");
        }

        #[test]
        fn renders_footer() {
            let state = make_populated_state();
            let output = render_log_select(&state, 60, 12);
            assert!(output.contains("n: new"), "should show footer keybindings");
        }

        #[test]
        fn renders_error_message() {
            let mut state = make_populated_state();
            state.set_error("disk full".into());
            let output = render_log_select(&state, 60, 12);
            assert!(output.contains("disk full"), "should show error message");
        }
    }

    mod error {
        use super::*;

        #[test]
        fn returns_set_value() {
            let mut state = LogSelectState::new();
            assert_eq!(state.error(), None);
            state.set_error("storage failed".into());
            assert_eq!(state.error(), Some("storage failed"));
        }
    }
}
