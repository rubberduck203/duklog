//! Export confirmation screen — review path and QSO count, then write ADIF.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::model::Log;
use crate::storage::default_export_path;
use crate::tui::action::Action;
use crate::tui::app::Screen;
use crate::tui::widgets::{StatusBarContext, draw_status_bar};

/// Current status of the export operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportStatus {
    /// Awaiting user confirmation.
    Ready,
    /// Export completed successfully.
    Success,
    /// Export failed with the given error message.
    Error(String),
}

/// State for the export confirmation screen.
#[derive(Debug, Clone)]
pub struct ExportState {
    path: String,
    status: ExportStatus,
    qso_count: usize,
}

impl Default for ExportState {
    fn default() -> Self {
        Self::new()
    }
}

impl ExportState {
    /// Creates a new export state with empty defaults.
    pub fn new() -> Self {
        Self {
            path: String::new(),
            status: ExportStatus::Ready,
            qso_count: 0,
        }
    }

    /// Prepares the export screen for the given log, computing the default
    /// export path and QSO count. Resets status to [`ExportStatus::Ready`].
    pub fn prepare(&mut self, log: Option<&Log>) {
        self.status = ExportStatus::Ready;
        match log {
            Some(log) => {
                self.qso_count = log.qsos.len();
                self.path = default_export_path(log)
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|e| format!("<error: {e}>"));
            }
            None => {
                self.qso_count = 0;
                self.path = String::new();
            }
        }
    }

    /// Handles a key event, returning an [`Action`] for the app to apply.
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match self.status {
            ExportStatus::Ready => match key.code {
                KeyCode::Enter => Action::ExportLog,
                KeyCode::Esc | KeyCode::Char('q') => Action::Navigate(Screen::QsoEntry),
                _ => Action::None,
            },
            ExportStatus::Success | ExportStatus::Error(_) => Action::Navigate(Screen::QsoEntry),
        }
    }

    /// Marks the export as successful.
    pub fn set_success(&mut self) {
        self.status = ExportStatus::Success;
    }

    /// Marks the export as failed with the given error message.
    pub fn set_error(&mut self, msg: String) {
        self.status = ExportStatus::Error(msg);
    }

    /// Returns the export file path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Sets the export file path.
    pub fn set_path(&mut self, path: String) {
        self.path = path;
    }

    /// Returns the current export status.
    pub fn status(&self) -> &ExportStatus {
        &self.status
    }

    /// Returns the number of QSOs that will be exported.
    pub fn qso_count(&self) -> usize {
        self.qso_count
    }
}

/// Renders the export confirmation screen.
#[mutants::skip]
pub fn draw_export(state: &ExportState, log: Option<&Log>, frame: &mut Frame, area: Rect) {
    let [status_area, content_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);

    let ctx = log
        .map(|l| StatusBarContext {
            callsign: l.station_callsign.clone(),
            park_ref: l.park_ref.clone(),
            qso_count: l.qso_count_today(),
            is_activated: l.is_activated(),
        })
        .unwrap_or_default();
    draw_status_bar(&ctx, frame, status_area);

    let block = Block::default()
        .title(" Export ADIF ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(content_area);
    frame.render_widget(block, content_area);

    let [info_area, status_area, footer_area] = Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .areas(inner);

    // Station info and export details
    let mut lines = Vec::new();

    if let Some(log) = log {
        let callsign = &log.station_callsign;
        let park = log
            .park_ref
            .as_deref()
            .map(|p| format!("  Park: {p}"))
            .unwrap_or_default();
        lines.push(Line::from(Span::styled(
            format!("Station: {callsign}{park}"),
            Style::default().fg(Color::White),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("QSOs: {}", state.qso_count()),
        Style::default().fg(Color::White),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("Path: {}", state.path()),
        Style::default().fg(Color::Yellow),
    )));

    frame.render_widget(Paragraph::new(lines), info_area);

    // Status message
    let (status_text, status_color) = match state.status() {
        ExportStatus::Ready => ("Press Enter to export.", Color::White),
        ExportStatus::Success => ("Export complete!", Color::Green),
        ExportStatus::Error(msg) => (msg.as_str(), Color::Red),
    };
    let status_line = Line::from(Span::styled(status_text, Style::default().fg(status_color)));
    frame.render_widget(
        Paragraph::new(vec![Line::from(""), status_line]),
        status_area,
    );

    // Footer
    let footer_text = match state.status() {
        ExportStatus::Ready => "Enter: export  Esc: back",
        ExportStatus::Success | ExportStatus::Error(_) => "Press any key to return",
    };
    let footer =
        Paragraph::new(Line::from(footer_text)).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    use super::*;
    use crate::model::{Band, Mode, Qso};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn make_log() -> Log {
        let mut log = Log::new(
            "W1AW".to_string(),
            None,
            Some("K-0001".to_string()),
            "FN31".to_string(),
        )
        .unwrap();
        log.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        log
    }

    fn make_qso() -> Qso {
        Qso::new(
            "KD9XYZ".to_string(),
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

    mod construction {
        use super::*;

        #[test]
        fn defaults() {
            let state = ExportState::new();
            assert_eq!(state.path(), "");
            assert_eq!(state.status(), &ExportStatus::Ready);
            assert_eq!(state.qso_count(), 0);
        }

        #[test]
        fn default_trait() {
            let state = ExportState::default();
            assert_eq!(state.qso_count(), 0);
        }
    }

    mod prepare {
        use super::*;

        #[test]
        fn populates_path_and_count() {
            let mut state = ExportState::new();
            let mut log = make_log();
            log.add_qso(make_qso());
            log.add_qso(make_qso());

            state.prepare(Some(&log));
            assert_eq!(state.qso_count(), 2);
            assert!(state.path().contains("duklog-K-0001"));
            assert!(state.path().ends_with(".adif"));
        }

        #[test]
        fn resets_status_to_ready() {
            let mut state = ExportState::new();
            state.set_success();
            assert_eq!(state.status(), &ExportStatus::Success);

            state.prepare(Some(&make_log()));
            assert_eq!(state.status(), &ExportStatus::Ready);
        }

        #[test]
        fn none_log_clears_state() {
            let mut state = ExportState::new();
            state.prepare(Some(&make_log()));
            assert!(!state.path().is_empty());

            state.prepare(None);
            assert_eq!(state.path(), "");
            assert_eq!(state.qso_count(), 0);
        }

        #[test]
        fn path_without_park_uses_callsign() {
            let mut state = ExportState::new();
            let mut log = make_log();
            log.park_ref = None;

            state.prepare(Some(&log));
            assert!(state.path().contains("duklog-W1AW"));
        }
    }

    mod handle_key {
        use super::*;

        #[test]
        fn enter_when_ready_returns_export() {
            let mut state = ExportState::new();
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::ExportLog);
        }

        #[test]
        fn esc_when_ready_returns_to_qso_entry() {
            let mut state = ExportState::new();
            let action = state.handle_key(press(KeyCode::Esc));
            assert_eq!(action, Action::Navigate(Screen::QsoEntry));
        }

        #[test]
        fn q_when_ready_returns_to_qso_entry() {
            let mut state = ExportState::new();
            let action = state.handle_key(press(KeyCode::Char('q')));
            assert_eq!(action, Action::Navigate(Screen::QsoEntry));
        }

        #[test]
        fn unhandled_key_when_ready_returns_none() {
            let mut state = ExportState::new();
            let action = state.handle_key(press(KeyCode::Char('x')));
            assert_eq!(action, Action::None);
        }

        #[test]
        fn any_key_after_success_returns_to_qso_entry() {
            let mut state = ExportState::new();
            state.set_success();
            let action = state.handle_key(press(KeyCode::Char('x')));
            assert_eq!(action, Action::Navigate(Screen::QsoEntry));
        }

        #[test]
        fn any_key_after_error_returns_to_qso_entry() {
            let mut state = ExportState::new();
            state.set_error("boom".into());
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::Navigate(Screen::QsoEntry));
        }
    }

    mod status_setters {
        use super::*;

        #[test]
        fn set_success() {
            let mut state = ExportState::new();
            state.set_success();
            assert_eq!(state.status(), &ExportStatus::Success);
        }

        #[test]
        fn set_error() {
            let mut state = ExportState::new();
            state.set_error("disk full".into());
            assert_eq!(state.status(), &ExportStatus::Error("disk full".into()));
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

        fn render_export(
            state: &ExportState,
            log: Option<&Log>,
            width: u16,
            height: u16,
        ) -> String {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| {
                    draw_export(state, log, frame, frame.area());
                })
                .unwrap();
            buffer_to_string(terminal.backend().buffer())
        }

        #[test]
        fn renders_title() {
            let state = ExportState::new();
            let output = render_export(&state, None, 80, 15);
            assert!(output.contains("Export ADIF"), "should show title");
        }

        #[test]
        fn renders_station_info() {
            let mut state = ExportState::new();
            let log = make_log();
            state.prepare(Some(&log));
            let output = render_export(&state, Some(&log), 80, 15);
            assert!(output.contains("W1AW"), "should show station callsign");
            assert!(output.contains("K-0001"), "should show park ref");
        }

        #[test]
        fn renders_qso_count() {
            let mut state = ExportState::new();
            let mut log = make_log();
            log.add_qso(make_qso());
            state.prepare(Some(&log));
            let output = render_export(&state, Some(&log), 80, 15);
            assert!(output.contains("QSOs: 1"), "should show QSO count");
        }

        #[test]
        fn renders_path() {
            let mut state = ExportState::new();
            let log = make_log();
            state.prepare(Some(&log));
            let output = render_export(&state, Some(&log), 120, 15);
            assert!(output.contains("Path:"), "should show path label");
            assert!(output.contains(".adif"), "should show adif extension");
        }

        #[test]
        fn renders_ready_status() {
            let state = ExportState::new();
            let output = render_export(&state, None, 80, 15);
            assert!(
                output.contains("Enter to export"),
                "should show ready prompt"
            );
        }

        #[test]
        fn renders_success_status() {
            let mut state = ExportState::new();
            state.set_success();
            let output = render_export(&state, None, 80, 15);
            assert!(
                output.contains("Export complete!"),
                "should show success message"
            );
        }

        #[test]
        fn renders_error_status() {
            let mut state = ExportState::new();
            state.set_error("disk full".into());
            let output = render_export(&state, None, 80, 15);
            assert!(output.contains("disk full"), "should show error message");
        }

        #[test]
        fn renders_footer_when_ready() {
            let state = ExportState::new();
            let output = render_export(&state, None, 80, 15);
            assert!(
                output.contains("Enter: export"),
                "should show export keybinding"
            );
            assert!(output.contains("Esc: back"), "should show back keybinding");
        }

        #[test]
        fn renders_footer_after_completion() {
            let mut state = ExportState::new();
            state.set_success();
            let output = render_export(&state, None, 80, 15);
            assert!(
                output.contains("any key to return"),
                "should show return prompt"
            );
        }

        #[test]
        fn renders_without_log() {
            let state = ExportState::new();
            let output = render_export(&state, None, 80, 15);
            assert!(output.contains("Export ADIF"), "should still show title");
            assert!(output.contains("QSOs: 0"), "should show zero count");
        }

        #[test]
        fn renders_without_park_ref() {
            let mut state = ExportState::new();
            let mut log = make_log();
            log.park_ref = None;
            state.prepare(Some(&log));
            let output = render_export(&state, Some(&log), 80, 15);
            assert!(output.contains("W1AW"), "should show callsign");
            assert!(!output.contains("Park:"), "should not show park label");
        }
    }
}
