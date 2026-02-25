//! QSO list screen — scrollable table of all QSOs in the active log.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Row, Table};

use crate::model::Log;
use crate::tui::action::Action;
use crate::tui::app::Screen;
use crate::tui::widgets::{StatusBarContext, draw_status_bar};

/// State for the QSO list screen.
#[derive(Debug, Clone)]
pub struct QsoListState {
    /// Index of the currently highlighted row (0-based).
    selected: usize,
}

impl Default for QsoListState {
    fn default() -> Self {
        Self::new()
    }
}

impl QsoListState {
    /// Creates a new state with the cursor at the first row.
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    /// Handles a key event, returning an [`Action`] for the app to apply.
    pub fn handle_key(&mut self, key: KeyEvent, qso_count: usize) -> Action {
        match key.code {
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                Action::None
            }
            KeyCode::Down => {
                if qso_count > 0 {
                    self.selected = (self.selected + 1).min(qso_count - 1);
                }
                Action::None
            }
            KeyCode::Home => {
                self.selected = 0;
                Action::None
            }
            KeyCode::End => {
                self.selected = qso_count.saturating_sub(1);
                Action::None
            }
            KeyCode::Enter => {
                if qso_count > 0 {
                    Action::EditQso(self.selected)
                } else {
                    Action::None
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => Action::Navigate(Screen::QsoEntry),
            _ => Action::None,
        }
    }

    /// Returns the currently selected row index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Sets the selected row index.
    pub fn set_selected(&mut self, idx: usize) {
        self.selected = idx;
    }

    /// Resets the cursor to the first row.
    pub fn reset(&mut self) {
        self.selected = 0;
    }
}

/// Renders the QSO list screen.
#[mutants::skip]
pub fn draw_qso_list(state: &QsoListState, log: Option<&Log>, frame: &mut Frame, area: Rect) {
    let [status_area, title_area, table_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);

    let ctx = log
        .map(|l| StatusBarContext {
            callsign: l.header().station_callsign.clone(),
            park_ref: l.park_ref().map(|s| s.to_string()),
            qso_count: l.qso_count_today(),
            is_activated: l.is_activated(),
        })
        .unwrap_or_default();
    draw_status_bar(&ctx, frame, status_area);

    // Title
    let qso_count = log.map_or(0, |l| l.header().qsos.len());
    let title_text = if log.is_some() {
        format!("QSO List ({qso_count} QSOs)")
    } else {
        "QSO List (no log)".to_string()
    };
    let title = Paragraph::new(Line::from(title_text))
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(title, title_area);

    // Table or empty state
    if qso_count == 0 {
        let empty = Paragraph::new("No QSOs logged yet").alignment(Alignment::Center);
        frame.render_widget(empty, table_area);
    } else if let Some(log) = log {
        let qsos = &log.header().qsos;

        let header = Row::new(vec![
            "Time", "Date", "Call", "Band", "Mode", "RST S/R", "Park", "Comments",
        ])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(1);

        let rows: Vec<Row> = qsos
            .iter()
            .enumerate()
            .map(|(i, qso)| {
                let style = if i == state.selected() {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default()
                };
                Row::new(vec![
                    qso.timestamp.format("%H:%M").to_string(),
                    qso.timestamp.format("%Y-%m-%d").to_string(),
                    qso.their_call.clone(),
                    qso.band.to_string(),
                    qso.mode.to_string(),
                    format!("{}/{}", qso.rst_sent, qso.rst_rcvd),
                    qso.their_park.as_deref().unwrap_or("").to_string(),
                    qso.comments.clone(),
                ])
                .style(style)
            })
            .collect();

        let widths = [
            Constraint::Length(6),
            Constraint::Length(11),
            Constraint::Length(12),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(0),
        ];

        let table = Table::new(rows, widths).header(header);
        frame.render_widget(table, table_area);
    }

    // Footer
    let footer = Paragraph::new("↑↓: navigate  Home/End: jump  Enter: edit  q: back")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    use super::*;
    use crate::model::{Band, Mode, PotaLog, Qso};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn make_qso(call: &str) -> Qso {
        Qso::new(
            call.to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
            String::new(),
            None,
        )
        .unwrap()
    }

    fn make_log_with_qsos(n: usize) -> Log {
        let mut log = Log::Pota(
            PotaLog::new(
                "W1AW".to_string(),
                None,
                Some("K-0001".to_string()),
                "FN31".to_string(),
            )
            .unwrap(),
        );
        for i in 0..n {
            log.add_qso(make_qso(&format!("W{i}AW")));
        }
        log
    }

    mod construction {
        use super::*;

        #[test]
        fn new_starts_at_zero() {
            let state = QsoListState::new();
            assert_eq!(state.selected(), 0);
        }

        #[test]
        fn default_trait() {
            let state = QsoListState::default();
            assert_eq!(state.selected(), 0);
        }
    }

    mod navigation {
        use super::*;

        #[test]
        fn down_increments_selected() {
            let mut state = QsoListState::new();
            let action = state.handle_key(press(KeyCode::Down), 5);
            assert_eq!(action, Action::None);
            assert_eq!(state.selected(), 1);
        }

        #[test]
        fn up_decrements_selected() {
            let mut state = QsoListState::new();
            state.set_selected(3);
            let action = state.handle_key(press(KeyCode::Up), 5);
            assert_eq!(action, Action::None);
            assert_eq!(state.selected(), 2);
        }

        #[test]
        fn up_at_top_saturates() {
            let mut state = QsoListState::new();
            state.handle_key(press(KeyCode::Up), 5);
            assert_eq!(state.selected(), 0);
        }

        #[test]
        fn down_at_bottom_saturates() {
            let mut state = QsoListState::new();
            state.set_selected(4);
            state.handle_key(press(KeyCode::Down), 5);
            assert_eq!(state.selected(), 4);
        }

        #[test]
        fn down_with_empty_list_stays_at_zero() {
            let mut state = QsoListState::new();
            state.handle_key(press(KeyCode::Down), 0);
            assert_eq!(state.selected(), 0);
        }

        #[test]
        fn home_jumps_to_first() {
            let mut state = QsoListState::new();
            state.set_selected(4);
            let action = state.handle_key(press(KeyCode::Home), 5);
            assert_eq!(action, Action::None);
            assert_eq!(state.selected(), 0);
        }

        #[test]
        fn end_jumps_to_last() {
            let mut state = QsoListState::new();
            let action = state.handle_key(press(KeyCode::End), 5);
            assert_eq!(action, Action::None);
            assert_eq!(state.selected(), 4);
        }

        #[test]
        fn end_with_empty_list_stays_at_zero() {
            let mut state = QsoListState::new();
            state.handle_key(press(KeyCode::End), 0);
            assert_eq!(state.selected(), 0);
        }
    }

    mod edit_navigation {
        use super::*;

        #[test]
        fn enter_returns_edit_qso() {
            let mut state = QsoListState::new();
            let action = state.handle_key(press(KeyCode::Enter), 5);
            assert_eq!(action, Action::EditQso(0));
        }

        #[test]
        fn enter_returns_selected_index() {
            let mut state = QsoListState::new();
            state.set_selected(3);
            let action = state.handle_key(press(KeyCode::Enter), 5);
            assert_eq!(action, Action::EditQso(3));
        }

        #[test]
        fn enter_on_empty_list_returns_none() {
            let mut state = QsoListState::new();
            let action = state.handle_key(press(KeyCode::Enter), 0);
            assert_eq!(action, Action::None);
        }
    }

    mod back_navigation {
        use super::*;

        #[test]
        fn esc_navigates_to_qso_entry() {
            let mut state = QsoListState::new();
            let action = state.handle_key(press(KeyCode::Esc), 5);
            assert_eq!(action, Action::Navigate(Screen::QsoEntry));
        }

        #[test]
        fn q_navigates_to_qso_entry() {
            let mut state = QsoListState::new();
            let action = state.handle_key(press(KeyCode::Char('q')), 5);
            assert_eq!(action, Action::Navigate(Screen::QsoEntry));
        }
    }

    mod unhandled {
        use super::*;

        #[test]
        fn unhandled_key_returns_none() {
            let mut state = QsoListState::new();
            let action = state.handle_key(press(KeyCode::Char('x')), 5);
            assert_eq!(action, Action::None);
        }

        #[test]
        fn f1_returns_none() {
            let mut state = QsoListState::new();
            let action = state.handle_key(press(KeyCode::F(1)), 5);
            assert_eq!(action, Action::None);
        }
    }

    mod setters {
        use super::*;

        #[test]
        fn set_selected_updates_value() {
            let mut state = QsoListState::new();
            state.set_selected(42);
            assert_eq!(state.selected(), 42);
        }

        #[test]
        fn reset_returns_to_zero() {
            let mut state = QsoListState::new();
            state.set_selected(10);
            state.reset();
            assert_eq!(state.selected(), 0);
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

        fn render_qso_list(
            state: &QsoListState,
            log: Option<&Log>,
            width: u16,
            height: u16,
        ) -> String {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| {
                    draw_qso_list(state, log, frame, frame.area());
                })
                .unwrap();
            buffer_to_string(terminal.backend().buffer())
        }

        #[test]
        fn renders_title_with_count() {
            let state = QsoListState::new();
            let log = make_log_with_qsos(3);
            let output = render_qso_list(&state, Some(&log), 80, 20);
            assert!(
                output.contains("QSO List (3 QSOs)"),
                "should show title with count"
            );
        }

        #[test]
        fn renders_no_log_title() {
            let state = QsoListState::new();
            let output = render_qso_list(&state, None, 80, 20);
            assert!(
                output.contains("QSO List (no log)"),
                "should show no-log title"
            );
        }

        #[test]
        fn renders_empty_state() {
            let state = QsoListState::new();
            let log = make_log_with_qsos(0);
            let output = render_qso_list(&state, Some(&log), 80, 20);
            assert!(
                output.contains("No QSOs logged yet"),
                "should show empty message"
            );
        }

        #[test]
        fn renders_qso_data() {
            let state = QsoListState::new();
            let log = make_log_with_qsos(2);
            let output = render_qso_list(&state, Some(&log), 80, 20);
            assert!(output.contains("W0AW"), "should show first QSO call");
            assert!(output.contains("W1AW"), "should show second QSO call");
            assert!(output.contains("20M"), "should show band");
            assert!(output.contains("SSB"), "should show mode");
            assert!(output.contains("14:30"), "should show time");
            assert!(output.contains("2026-02-16"), "should show date");
        }

        #[test]
        fn renders_header_row() {
            let state = QsoListState::new();
            let log = make_log_with_qsos(1);
            let output = render_qso_list(&state, Some(&log), 80, 20);
            assert!(output.contains("Time"), "should show Time header");
            assert!(output.contains("Call"), "should show Call header");
            assert!(output.contains("Band"), "should show Band header");
            assert!(output.contains("Mode"), "should show Mode header");
        }

        #[test]
        fn renders_footer() {
            let state = QsoListState::new();
            let output = render_qso_list(&state, None, 80, 20);
            assert!(output.contains("navigate"), "should show navigation hint");
            assert!(output.contains("Enter: edit"), "should show edit hint");
            assert!(output.contains("q: back"), "should show back hint");
            assert!(output.contains("Home/End"), "should show jump hint");
        }

        #[test]
        fn selected_row_has_highlight() {
            let state = QsoListState::new();
            let log = make_log_with_qsos(3);
            let backend = TestBackend::new(80, 20);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| {
                    draw_qso_list(&state, Some(&log), frame, frame.area());
                })
                .unwrap();

            let buf = terminal.backend().buffer();
            // The first data row (row index 0) should be selected (yellow bg).
            // Status bar at y=0, title at y=1, header at y=2, separator margin at y=3,
            // first data row at y=4.
            let first_data_y = 4;
            let cell = &buf[(0, first_data_y)];
            assert_eq!(
                cell.bg,
                Color::Yellow,
                "selected row should have yellow background"
            );
            assert_eq!(
                cell.fg,
                Color::Black,
                "selected row should have black foreground"
            );
        }

        #[test]
        fn renders_park_and_comments() {
            let state = QsoListState::new();
            let mut log = make_log_with_qsos(0);
            let qso = Qso::new(
                "W3ABC".to_string(),
                "59".to_string(),
                "59".to_string(),
                Band::M20,
                Mode::Ssb,
                Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
                "nice signal".to_string(),
                Some("K-5678".to_string()),
            )
            .unwrap();
            log.add_qso(qso);
            let output = render_qso_list(&state, Some(&log), 100, 20);
            assert!(output.contains("K-5678"), "should show park reference");
            assert!(output.contains("nice signal"), "should show comments");
        }

        #[test]
        fn renders_zero_qsos_title() {
            let state = QsoListState::new();
            let log = make_log_with_qsos(0);
            let output = render_qso_list(&state, Some(&log), 80, 20);
            assert!(
                output.contains("QSO List (0 QSOs)"),
                "should show zero count"
            );
        }
    }
}
