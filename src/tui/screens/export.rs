//! Export confirmation screen — review path and QSO count, then write ADIF.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
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
    /// Byte offset of the cursor within `path`.
    cursor: usize,
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
            cursor: 0,
            status: ExportStatus::Ready,
            qso_count: 0,
        }
    }

    /// Prepares the export screen for the given log, computing the default
    /// export path and QSO count. Resets status to [`ExportStatus::Ready`].
    /// Cursor is placed at the end of the path.
    pub fn prepare(&mut self, log: Option<&Log>) {
        self.status = ExportStatus::Ready;
        match log {
            Some(log) => {
                self.qso_count = log.header().qsos.len();
                self.path = default_export_path(log)
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|e| format!("<error: {e}>"));
                self.cursor = self.path.len();
            }
            None => {
                self.qso_count = 0;
                self.path = String::new();
                self.cursor = 0;
            }
        }
    }

    /// Handles a key event, returning an [`Action`] for the app to apply.
    ///
    /// While the export is ready, the path is editable:
    /// - Printable characters are inserted at the cursor position.
    /// - `Backspace` removes the character before the cursor.
    /// - `Delete` removes the character at the cursor.
    /// - `Left` / `Right` move the cursor one character.
    /// - `Home` / `End` jump to the start or end of the path.
    /// - `Enter` exports to the current path; `Esc` cancels.
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match self.status {
            ExportStatus::Ready => match key.code {
                KeyCode::Enter => Action::ExportLog,
                KeyCode::Esc => Action::Navigate(Screen::QsoEntry),
                KeyCode::Backspace => {
                    if self.cursor > 0 {
                        let prev = self.prev_char_boundary();
                        self.path.remove(prev);
                        self.cursor = prev;
                    }
                    Action::None
                }
                KeyCode::Delete => {
                    if self.cursor < self.path.len() {
                        self.path.remove(self.cursor);
                    }
                    Action::None
                }
                KeyCode::Left => {
                    if self.cursor > 0 {
                        self.cursor = self.prev_char_boundary();
                    }
                    Action::None
                }
                KeyCode::Right => {
                    if let Some(c) = self.path[self.cursor..].chars().next() {
                        self.cursor += c.len_utf8();
                    }
                    Action::None
                }
                KeyCode::Home => {
                    self.cursor = 0;
                    Action::None
                }
                KeyCode::End => {
                    self.cursor = self.path.len();
                    Action::None
                }
                KeyCode::Char(c) => {
                    self.path.insert(self.cursor, c);
                    self.cursor += c.len_utf8();
                    Action::None
                }
                _ => Action::None,
            },
            ExportStatus::Success | ExportStatus::Error(_) => Action::Navigate(Screen::QsoEntry),
        }
    }

    /// Returns the byte offset of the character boundary before the cursor.
    fn prev_char_boundary(&self) -> usize {
        self.path[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0)
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

    /// Sets the export file path and moves the cursor to the end.
    pub fn set_path(&mut self, path: String) {
        self.cursor = path.len();
        self.path = path;
    }

    /// Returns the cursor position as a byte offset within the path.
    pub fn cursor(&self) -> usize {
        self.cursor
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

    let ctx = log.map(StatusBarContext::from_log).unwrap_or_default();
    draw_status_bar(&ctx, frame, status_area);

    let block = Block::default()
        .title(" Export ADIF ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(content_area);
    frame.render_widget(block, content_area);

    let [info_area, export_status_area, footer_area] = Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .areas(inner);

    // Station info and export details
    let mut lines = Vec::new();

    if let Some(log) = log {
        let callsign = &log.header().station_callsign;
        let park = log
            .park_ref()
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
    let mut path_spans = vec![Span::raw("Path: ")];
    if matches!(state.status(), ExportStatus::Ready) {
        let path = state.path();
        let cur = state.cursor();
        let before = &path[..cur];
        let rest = &path[cur..];
        // Find the end of the character at the cursor (if any).
        let char_end = rest
            .char_indices()
            .nth(1)
            .map(|(i, _)| cur + i)
            .unwrap_or(path.len());
        path_spans.push(Span::styled(before, Style::default().fg(Color::Yellow)));
        if cur < path.len() {
            // Overlay the character under the cursor with reversed video.
            path_spans.push(Span::styled(
                &path[cur..char_end],
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::REVERSED),
            ));
            path_spans.push(Span::styled(
                &path[char_end..],
                Style::default().fg(Color::Yellow),
            ));
        } else {
            // Cursor is past the end of the text — show the block.
            path_spans.push(Span::styled(
                "\u{2588}",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::SLOW_BLINK),
            ));
        }
    } else {
        path_spans.push(Span::styled(
            state.path(),
            Style::default().fg(Color::Yellow),
        ));
    }
    lines.push(Line::from(path_spans));

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
        export_status_area,
    );

    // Footer
    let footer_text = match state.status() {
        ExportStatus::Ready => "Enter: export  Esc: back  (edit path above)",
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
    use crate::model::{Band, Mode, PotaLog, Qso};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn make_log() -> Log {
        let mut log = PotaLog::new(
            "W1AW".to_string(),
            None,
            Some("K-0001".to_string()),
            "FN31".to_string(),
        )
        .unwrap();
        log.header.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        Log::Pota(log)
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
            None,
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
            assert!(state.path().contains("W1AW@K-0001"));
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
            assert!(state.path().contains("W1AW@K-0001"));

            state.prepare(None);
            assert_eq!(state.path(), "");
            assert_eq!(state.qso_count(), 0);
        }

        #[test]
        fn path_without_park_uses_callsign() {
            let mut state = ExportState::new();
            let mut log = make_log();
            if let Log::Pota(ref mut p) = log {
                p.park_ref = None;
            }

            state.prepare(Some(&log));
            assert!(state.path().contains("W1AW-"));
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
        fn unhandled_key_when_ready_returns_none() {
            let mut state = ExportState::new();
            let action = state.handle_key(press(KeyCode::F(2)));
            assert_eq!(action, Action::None);
        }

        #[test]
        fn typing_chars_inserts_at_cursor() {
            let mut state = ExportState::new();
            state.handle_key(press(KeyCode::Char('/')));
            state.handle_key(press(KeyCode::Char('t')));
            state.handle_key(press(KeyCode::Char('m')));
            state.handle_key(press(KeyCode::Char('p')));
            assert_eq!(state.path(), "/tmp");
            assert_eq!(state.cursor(), 4);
        }

        #[test]
        fn backspace_removes_char_before_cursor() {
            let mut state = ExportState::new();
            state.set_path("/tmp/foo.adif".into());
            state.handle_key(press(KeyCode::Backspace));
            assert_eq!(state.path(), "/tmp/foo.adi");
        }

        #[test]
        fn backspace_at_start_is_noop() {
            let mut state = ExportState::new();
            state.set_path("/tmp/foo.adif".into());
            state.handle_key(press(KeyCode::Home));
            state.handle_key(press(KeyCode::Backspace));
            assert_eq!(state.path(), "/tmp/foo.adif");
        }

        #[test]
        fn delete_removes_char_at_cursor() {
            let mut state = ExportState::new();
            state.set_path("/tmp/foo.adif".into());
            state.handle_key(press(KeyCode::Home));
            state.handle_key(press(KeyCode::Delete));
            assert_eq!(state.path(), "tmp/foo.adif");
        }

        #[test]
        fn delete_at_end_is_noop() {
            let mut state = ExportState::new();
            state.set_path("/tmp/foo.adif".into());
            // cursor is at end after set_path
            state.handle_key(press(KeyCode::Delete));
            assert_eq!(state.path(), "/tmp/foo.adif");
        }

        #[test]
        fn left_moves_cursor_back() {
            let mut state = ExportState::new();
            state.set_path("/tmp".into());
            state.handle_key(press(KeyCode::Left));
            assert_eq!(state.cursor(), 3);
        }

        #[test]
        fn left_at_start_is_noop() {
            let mut state = ExportState::new();
            state.handle_key(press(KeyCode::Left));
            assert_eq!(state.cursor(), 0);
        }

        #[test]
        fn right_moves_cursor_forward() {
            let mut state = ExportState::new();
            state.set_path("/tmp".into());
            state.handle_key(press(KeyCode::Home));
            state.handle_key(press(KeyCode::Right));
            assert_eq!(state.cursor(), 1);
        }

        #[test]
        fn right_at_end_is_noop() {
            let mut state = ExportState::new();
            state.set_path("/tmp".into());
            state.handle_key(press(KeyCode::Right));
            assert_eq!(state.cursor(), 4);
        }

        #[test]
        fn home_moves_cursor_to_start() {
            let mut state = ExportState::new();
            state.set_path("/tmp/foo.adif".into());
            state.handle_key(press(KeyCode::Home));
            assert_eq!(state.cursor(), 0);
        }

        #[test]
        fn end_moves_cursor_to_end() {
            let mut state = ExportState::new();
            state.set_path("/tmp/foo.adif".into());
            state.handle_key(press(KeyCode::Home));
            state.handle_key(press(KeyCode::End));
            assert_eq!(state.cursor(), 13);
        }

        #[test]
        fn insert_at_mid_cursor() {
            let mut state = ExportState::new();
            state.set_path("/tmp/foo.adif".into());
            state.handle_key(press(KeyCode::Home));
            state.handle_key(press(KeyCode::Right)); // cursor at 1
            state.handle_key(press(KeyCode::Char('X')));
            assert_eq!(state.path(), "/Xtmp/foo.adif");
        }

        #[test]
        fn q_appends_to_path() {
            let mut state = ExportState::new();
            state.handle_key(press(KeyCode::Char('q')));
            assert_eq!(state.path(), "q");
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
            let output = render_export(&state, None, 120, 15);
            assert!(
                output.contains("Enter: export"),
                "should show export keybinding"
            );
            assert!(output.contains("Esc: back"), "should show back keybinding");
            assert!(
                output.contains("edit path"),
                "should hint that path is editable"
            );
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
            if let Log::Pota(ref mut p) = log {
                p.park_ref = None;
            }
            state.prepare(Some(&log));
            let output = render_export(&state, Some(&log), 80, 15);
            assert!(output.contains("W1AW"), "should show callsign");
            assert!(!output.contains("Park:"), "should not show park label");
        }
    }
}
