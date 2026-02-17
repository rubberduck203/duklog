//! QSO entry screen — the core data entry form for logging contacts.

use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use crate::model::{Band, Log, Mode, Qso, validate_callsign, validate_park_ref};
use crate::tui::action::Action;
use crate::tui::app::Screen;
use crate::tui::widgets::form::{Form, FormField, draw_form};

/// Field index for the other station's callsign.
const THEIR_CALL: usize = 0;
/// Field index for RST sent.
const RST_SENT: usize = 1;
/// Field index for RST received.
const RST_RCVD: usize = 2;
/// Field index for the other station's POTA park reference.
const THEIR_PARK: usize = 3;
/// Field index for free-text comments.
const COMMENTS: usize = 4;

/// State for the QSO entry screen.
#[derive(Debug, Clone)]
pub struct QsoEntryState {
    form: Form,
    band: Band,
    mode: Mode,
    recent_qsos: Vec<Qso>,
    error: Option<String>,
}

impl Default for QsoEntryState {
    fn default() -> Self {
        Self::new()
    }
}

impl QsoEntryState {
    /// Creates a new QSO entry state with default band/mode and empty form.
    pub fn new() -> Self {
        let mode = Mode::default();
        let rst = mode.default_rst();
        let mut form = Form::new(vec![
            FormField::new("Their Callsign", true),
            FormField::new("RST Sent", true),
            FormField::new("RST Rcvd", true),
            FormField::new("Their Park", false),
            FormField::new("Comments", false),
        ]);
        form.set_value(RST_SENT, rst);
        form.set_value(RST_RCVD, rst);

        Self {
            form,
            band: Band::default(),
            mode,
            recent_qsos: Vec::new(),
            error: None,
        }
    }

    /// Handles a key event, returning an [`Action`] for the app to apply.
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        // Alt+B/M cycle band/mode forward; Shift+Alt+B/M cycle backward
        if key.modifiers == KeyModifiers::ALT {
            match key.code {
                KeyCode::Char('b') => {
                    self.cycle_band(true);
                    return Action::None;
                }
                KeyCode::Char('m') => {
                    self.cycle_mode(true);
                    return Action::None;
                }
                _ => {}
            }
        }
        const ALT_SHIFT: KeyModifiers = KeyModifiers::ALT.union(KeyModifiers::SHIFT);
        if key.modifiers == ALT_SHIFT {
            match key.code {
                KeyCode::Char('B') => {
                    self.cycle_band(false);
                    return Action::None;
                }
                KeyCode::Char('M') => {
                    self.cycle_mode(false);
                    return Action::None;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Tab => {
                self.form.focus_next();
                Action::None
            }
            KeyCode::BackTab => {
                self.form.focus_prev();
                Action::None
            }
            KeyCode::Backspace => {
                self.form.delete_char();
                Action::None
            }
            KeyCode::Esc => Action::Navigate(Screen::LogSelect),
            KeyCode::Enter => self.submit(),
            KeyCode::Char(ch) => self.handle_char(ch),
            _ => Action::None,
        }
    }

    /// Returns a reference to the form for rendering.
    pub fn form(&self) -> &Form {
        &self.form
    }

    /// Returns the current band.
    pub fn band(&self) -> Band {
        self.band
    }

    /// Returns the current mode.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Returns the recent QSOs list.
    pub fn recent_qsos(&self) -> &[Qso] {
        &self.recent_qsos
    }

    /// Returns the current error message, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Sets an error message to display.
    pub fn set_error(&mut self, msg: String) {
        self.error = Some(msg);
    }

    /// Populates recent QSOs from a log (last 3, newest first).
    pub fn set_log_context(&mut self, log: &Log) {
        self.recent_qsos = log.qsos.iter().rev().take(3).cloned().collect();
    }

    /// Adds a QSO to the recent list, keeping only the last 3 (newest first).
    pub fn add_recent_qso(&mut self, qso: Qso) {
        self.recent_qsos.insert(0, qso);
        self.recent_qsos.truncate(3);
    }

    /// Clears fast-moving fields and repopulates RST defaults for the current mode.
    pub fn clear_fast_fields(&mut self) {
        self.form.clear_value(THEIR_CALL);
        let rst = self.mode.default_rst();
        self.form.set_value(RST_SENT, rst);
        self.form.set_value(RST_RCVD, rst);
        self.form.clear_value(THEIR_PARK);
        self.form.clear_value(COMMENTS);
        self.form.clear_errors();
        self.error = None;
        self.form.set_focus(THEIR_CALL);
    }

    /// Resets all state back to initial defaults.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Handles a printable character: inserts into the focused field.
    ///
    /// Callsign and park ref fields are auto-uppercased.
    fn handle_char(&mut self, ch: char) -> Action {
        let should_uppercase = self.form.focus() == THEIR_CALL || self.form.focus() == THEIR_PARK;
        let ch = if should_uppercase {
            ch.to_ascii_uppercase()
        } else {
            ch
        };
        self.form.insert_char(ch);
        Action::None
    }

    /// Cycles the band forward or backward, wrapping around.
    fn cycle_band(&mut self, forward: bool) {
        self.band = cycle(Band::all(), self.band, forward);
    }

    /// Cycles the mode forward or backward, wrapping around.
    ///
    /// When the mode changes, RST fields are updated to the new mode's default
    /// only if they still contain the previous mode's default.
    fn cycle_mode(&mut self, forward: bool) {
        let old_rst = self.mode.default_rst();
        self.mode = cycle(Mode::all(), self.mode, forward);
        let new_rst = self.mode.default_rst();

        if self.form.value(RST_SENT) == old_rst {
            self.form.set_value(RST_SENT, new_rst);
        }
        if self.form.value(RST_RCVD) == old_rst {
            self.form.set_value(RST_RCVD, new_rst);
        }
    }

    /// Validates the form and constructs a QSO.
    fn submit(&mut self) -> Action {
        self.form.clear_errors();
        self.error = None;

        let their_call = self.form.value(THEIR_CALL).to_string();
        let rst_sent = self.form.value(RST_SENT).to_string();
        let rst_rcvd = self.form.value(RST_RCVD).to_string();
        let their_park_str = self.form.value(THEIR_PARK).to_string();
        let comments = self.form.value(COMMENTS).to_string();

        if let Err(e) = validate_callsign(&their_call) {
            self.form.set_error(THEIR_CALL, e.to_string());
        }
        if rst_sent.is_empty() {
            self.form.set_error(RST_SENT, "RST sent is required".into());
        }
        if rst_rcvd.is_empty() {
            self.form
                .set_error(RST_RCVD, "RST received is required".into());
        }
        if !their_park_str.is_empty()
            && let Err(e) = validate_park_ref(&their_park_str)
        {
            self.form.set_error(THEIR_PARK, e.to_string());
        }

        if self.form.has_errors() {
            return Action::None;
        }

        let their_park = (!their_park_str.is_empty()).then_some(their_park_str);

        match Qso::new(
            their_call,
            rst_sent,
            rst_rcvd,
            self.band,
            self.mode,
            Utc::now(),
            comments,
            their_park,
        ) {
            Ok(qso) => Action::AddQso(qso),
            Err(e) => {
                self.form.set_error(THEIR_CALL, e.to_string());
                Action::None
            }
        }
    }
}

/// Cycles through a slice to find the next or previous element.
fn cycle<T: PartialEq + Copy>(items: &[T], current: T, forward: bool) -> T {
    let pos = items.iter().position(|&x| x == current).unwrap_or(0);
    let next = if forward {
        (pos + 1) % items.len()
    } else {
        (pos + items.len() - 1) % items.len()
    };
    items[next]
}

/// Renders the QSO entry screen.
#[cfg_attr(coverage_nightly, coverage(off))]
#[mutants::skip]
pub fn draw_qso_entry(state: &QsoEntryState, log: Option<&Log>, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" QSO Entry ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [header_area, form_area, recent_area, footer_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(15),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .areas(inner);

    // Header: station info + band/mode
    if let Some(log) = log {
        let callsign = &log.station_callsign;
        let park = log.park_ref.as_deref().unwrap_or("-");
        let grid = &log.grid_square;
        let today = log.qso_count_today();
        let needed = log.needs_for_activation();

        let header_line1 = Line::from(vec![
            Span::styled(
                format!("{callsign} @ {park} ({grid})"),
                Style::default().fg(Color::White),
            ),
            Span::raw("    "),
            Span::styled(
                format!("Band: {}", state.band()),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Mode: {}", state.mode()),
                Style::default().fg(Color::Yellow),
            ),
        ]);

        let activation_info = if needed > 0 {
            format!("QSOs today: {today} / 10  [{needed} needed]")
        } else {
            format!("QSOs today: {today} / 10  [Activated!]")
        };
        let header_line2 = Line::from(Span::styled(
            activation_info,
            Style::default().fg(Color::DarkGray),
        ));

        frame.render_widget(
            Paragraph::new(vec![header_line1, header_line2]),
            header_area,
        );
    }

    // Form fields
    draw_form(state.form(), frame, form_area);

    // Error message
    if let Some(err) = state.error() {
        let err_paragraph = Paragraph::new(Span::styled(err, Style::default().fg(Color::Red)));
        // Render at bottom of form area
        let err_area = Rect {
            x: form_area.x,
            y: form_area.y + form_area.height.saturating_sub(1),
            width: form_area.width,
            height: 1,
        };
        frame.render_widget(err_paragraph, err_area);
    }

    // Recent QSOs table
    let recent_block = Block::default()
        .title(" Recent QSOs ")
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));

    let recent_inner = recent_block.inner(recent_area);
    frame.render_widget(recent_block, recent_area);

    if !state.recent_qsos().is_empty() {
        let rows: Vec<Row> = state
            .recent_qsos()
            .iter()
            .map(|qso| {
                let time = qso.timestamp.format("%H:%M").to_string();
                let p2p = qso
                    .their_park
                    .as_ref()
                    .map(|p| format!("P2P {p}"))
                    .unwrap_or_default();
                Row::new(vec![
                    time,
                    qso.their_call.clone(),
                    qso.band.to_string(),
                    qso.mode.to_string(),
                    format!("{}/{}", qso.rst_sent, qso.rst_rcvd),
                    p2p,
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(8),
            Constraint::Min(10),
        ];

        let table = Table::new(rows, widths);
        frame.render_widget(table, recent_inner);
    }

    // Footer
    let footer = Paragraph::new(Line::from(
        "Tab: next  Alt+b/m: band/mode  Shift+Alt: reverse  Enter: log  Esc: back",
    ))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
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

    fn shift_press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn alt_press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn shift_alt_press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::ALT | KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn type_string(state: &mut QsoEntryState, s: &str) {
        for ch in s.chars() {
            state.handle_key(press(KeyCode::Char(ch)));
        }
    }

    fn make_qso(call: &str, band: Band, mode: Mode) -> Qso {
        Qso::new(
            call.to_string(),
            mode.default_rst().to_string(),
            mode.default_rst().to_string(),
            band,
            mode,
            Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
            String::new(),
            None,
        )
        .unwrap()
    }

    fn fill_valid_callsign(state: &mut QsoEntryState) {
        type_string(state, "KD9XYZ");
    }

    mod construction {
        use super::*;

        #[test]
        fn defaults() {
            let state = QsoEntryState::new();
            assert_eq!(state.band(), Band::M20);
            assert_eq!(state.mode(), Mode::Ssb);
            assert_eq!(state.form().value(RST_SENT), "59");
            assert_eq!(state.form().value(RST_RCVD), "59");
            assert_eq!(state.form().value(THEIR_CALL), "");
            assert_eq!(state.form().value(THEIR_PARK), "");
            assert_eq!(state.form().value(COMMENTS), "");
            assert!(state.recent_qsos().is_empty());
            assert_eq!(state.error(), None);
        }

        #[test]
        fn default_trait() {
            let state = QsoEntryState::default();
            assert_eq!(state.band(), Band::M20);
        }
    }

    mod typing {
        use super::*;

        #[test]
        fn chars_fill_focused_field() {
            let mut state = QsoEntryState::new();
            state.handle_key(press(KeyCode::Char('W')));
            state.handle_key(press(KeyCode::Char('1')));
            assert_eq!(state.form().value(THEIR_CALL), "W1");
        }

        #[test]
        fn callsign_auto_uppercased() {
            let mut state = QsoEntryState::new();
            type_string(&mut state, "w1aw");
            assert_eq!(state.form().value(THEIR_CALL), "W1AW");
        }

        #[test]
        fn park_ref_auto_uppercased() {
            let mut state = QsoEntryState::new();
            // Tab to Their Park field
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "k-0001");
            assert_eq!(state.form().value(THEIR_PARK), "K-0001");
        }

        #[test]
        fn comments_not_uppercased() {
            let mut state = QsoEntryState::new();
            // Tab to Comments field
            for _ in 0..4 {
                state.handle_key(press(KeyCode::Tab));
            }
            type_string(&mut state, "hello");
            assert_eq!(state.form().value(COMMENTS), "hello");
        }

        #[test]
        fn backspace_deletes_char() {
            let mut state = QsoEntryState::new();
            type_string(&mut state, "W1AW");
            state.handle_key(press(KeyCode::Backspace));
            assert_eq!(state.form().value(THEIR_CALL), "W1A");
        }

        #[test]
        fn tab_cycles_focus() {
            let mut state = QsoEntryState::new();
            assert_eq!(state.form().focus(), THEIR_CALL);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), RST_SENT);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), RST_RCVD);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), THEIR_PARK);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), COMMENTS);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), THEIR_CALL);
        }

        #[test]
        fn backtab_cycles_focus_backward() {
            let mut state = QsoEntryState::new();
            state.handle_key(shift_press(KeyCode::BackTab));
            assert_eq!(state.form().focus(), COMMENTS);
        }

        #[test]
        fn unhandled_key_returns_none() {
            let mut state = QsoEntryState::new();
            let action = state.handle_key(press(KeyCode::F(1)));
            assert_eq!(action, Action::None);
        }
    }

    mod band_cycling {
        use super::*;

        #[test]
        fn alt_b_cycles_forward() {
            let mut state = QsoEntryState::new();
            assert_eq!(state.band(), Band::M20);
            state.handle_key(alt_press(KeyCode::Char('b')));
            assert_eq!(state.band(), Band::M17);
        }

        #[test]
        fn shift_alt_b_cycles_backward() {
            let mut state = QsoEntryState::new();
            assert_eq!(state.band(), Band::M20);
            state.handle_key(shift_alt_press(KeyCode::Char('B')));
            assert_eq!(state.band(), Band::M30);
        }

        #[test]
        fn wraps_forward() {
            let mut state = QsoEntryState::new();
            for _ in 0..Band::all().len() {
                state.handle_key(alt_press(KeyCode::Char('b')));
            }
            assert_eq!(state.band(), Band::M20);
        }

        #[test]
        fn wraps_backward() {
            let mut state = QsoEntryState::new();
            for _ in 0..Band::all().len() {
                state.handle_key(shift_alt_press(KeyCode::Char('B')));
            }
            assert_eq!(state.band(), Band::M20);
        }

        #[test]
        fn b_types_in_callsign() {
            let mut state = QsoEntryState::new();
            type_string(&mut state, "wb4");
            assert_eq!(state.form().value(THEIR_CALL), "WB4");
            assert_eq!(state.band(), Band::M20); // unchanged
        }

        #[test]
        fn unhandled_alt_falls_through() {
            let mut state = QsoEntryState::new();
            let action = state.handle_key(alt_press(KeyCode::Char('x')));
            assert_eq!(action, Action::None);
        }
    }

    mod mode_cycling {
        use super::*;

        #[test]
        fn alt_m_cycles_forward() {
            let mut state = QsoEntryState::new();
            assert_eq!(state.mode(), Mode::Ssb);
            state.handle_key(alt_press(KeyCode::Char('m')));
            assert_eq!(state.mode(), Mode::Cw);
        }

        #[test]
        fn shift_alt_m_cycles_backward() {
            let mut state = QsoEntryState::new();
            assert_eq!(state.mode(), Mode::Ssb);
            state.handle_key(shift_alt_press(KeyCode::Char('M')));
            assert_eq!(state.mode(), Mode::Digi);
        }

        #[test]
        fn wraps_forward() {
            let mut state = QsoEntryState::new();
            for _ in 0..Mode::all().len() {
                state.handle_key(alt_press(KeyCode::Char('m')));
            }
            assert_eq!(state.mode(), Mode::Ssb);
        }

        #[test]
        fn wraps_backward() {
            let mut state = QsoEntryState::new();
            for _ in 0..Mode::all().len() {
                state.handle_key(shift_alt_press(KeyCode::Char('M')));
            }
            assert_eq!(state.mode(), Mode::Ssb);
        }

        #[test]
        fn m_types_in_callsign() {
            let mut state = QsoEntryState::new();
            type_string(&mut state, "km4");
            assert_eq!(state.form().value(THEIR_CALL), "KM4");
            assert_eq!(state.mode(), Mode::Ssb); // unchanged
        }

        #[test]
        fn m_types_in_park_ref() {
            let mut state = QsoEntryState::new();
            // Tab to Their Park
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "mb-0001");
            assert_eq!(state.form().value(THEIR_PARK), "MB-0001");
            assert_eq!(state.mode(), Mode::Ssb);
        }
    }

    mod rst_defaults {
        use super::*;

        #[test]
        fn mode_change_updates_rst_when_unedited() {
            let mut state = QsoEntryState::new();
            assert_eq!(state.form().value(RST_SENT), "59");
            assert_eq!(state.form().value(RST_RCVD), "59");

            // Switch to CW
            state.handle_key(alt_press(KeyCode::Char('m')));
            assert_eq!(state.mode(), Mode::Cw);
            assert_eq!(state.form().value(RST_SENT), "599");
            assert_eq!(state.form().value(RST_RCVD), "599");
        }

        #[test]
        fn mode_change_preserves_edited_rst() {
            let mut state = QsoEntryState::new();
            // Edit RST Sent
            state.handle_key(press(KeyCode::Tab)); // focus RST Sent
            state.handle_key(press(KeyCode::Backspace));
            state.handle_key(press(KeyCode::Backspace));
            type_string(&mut state, "57");

            // Switch to CW — RST Sent should keep "57", RST Rcvd should update
            state.handle_key(alt_press(KeyCode::Char('m')));
            assert_eq!(state.form().value(RST_SENT), "57");
            assert_eq!(state.form().value(RST_RCVD), "599");
        }

        #[test]
        fn mode_change_preserves_both_edited() {
            let mut state = QsoEntryState::new();
            // Edit RST Sent
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Backspace));
            state.handle_key(press(KeyCode::Backspace));
            type_string(&mut state, "57");

            // Edit RST Rcvd
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Backspace));
            state.handle_key(press(KeyCode::Backspace));
            type_string(&mut state, "55");

            state.handle_key(alt_press(KeyCode::Char('m')));
            assert_eq!(state.form().value(RST_SENT), "57");
            assert_eq!(state.form().value(RST_RCVD), "55");
        }
    }

    mod submit {
        use super::*;

        #[test]
        fn valid_qso_returns_add_qso() {
            let mut state = QsoEntryState::new();
            fill_valid_callsign(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => {
                    assert_eq!(qso.their_call, "KD9XYZ");
                    assert_eq!(qso.rst_sent, "59");
                    assert_eq!(qso.rst_rcvd, "59");
                    assert_eq!(qso.band, Band::M20);
                    assert_eq!(qso.mode, Mode::Ssb);
                    assert_eq!(qso.their_park, None);
                    assert_eq!(qso.comments, "");
                }
                other => panic!("expected AddQso, got {other:?}"),
            }
        }

        #[test]
        fn valid_p2p_qso() {
            let mut state = QsoEntryState::new();
            fill_valid_callsign(&mut state);
            // Tab to Their Park
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "K-1234");
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => {
                    assert_eq!(qso.their_park, Some("K-1234".to_string()));
                }
                other => panic!("expected AddQso, got {other:?}"),
            }
        }

        #[test]
        fn empty_callsign_shows_error() {
            let mut state = QsoEntryState::new();
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().fields()[THEIR_CALL].error.is_some());
        }

        #[test]
        fn empty_rst_sent_shows_error() {
            let mut state = QsoEntryState::new();
            fill_valid_callsign(&mut state);
            // Clear RST Sent
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Backspace));
            state.handle_key(press(KeyCode::Backspace));
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().fields()[RST_SENT].error.is_some());
        }

        #[test]
        fn empty_rst_rcvd_shows_error() {
            let mut state = QsoEntryState::new();
            fill_valid_callsign(&mut state);
            // Clear RST Rcvd
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Backspace));
            state.handle_key(press(KeyCode::Backspace));
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().fields()[RST_RCVD].error.is_some());
        }

        #[test]
        fn invalid_park_ref_shows_error() {
            let mut state = QsoEntryState::new();
            fill_valid_callsign(&mut state);
            // Tab to Their Park
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "BAD");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().fields()[THEIR_PARK].error.is_some());
        }

        #[test]
        fn errors_cleared_on_resubmit() {
            let mut state = QsoEntryState::new();
            state.handle_key(press(KeyCode::Enter));
            assert!(state.form().has_errors());
            fill_valid_callsign(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            assert!(matches!(action, Action::AddQso(_)));
            assert!(!state.form().has_errors());
        }

        #[test]
        fn submit_with_comments() {
            let mut state = QsoEntryState::new();
            fill_valid_callsign(&mut state);
            // Tab to Comments
            for _ in 0..4 {
                state.handle_key(press(KeyCode::Tab));
            }
            type_string(&mut state, "nice signal");
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => {
                    assert_eq!(qso.comments, "nice signal");
                }
                other => panic!("expected AddQso, got {other:?}"),
            }
        }

        #[test]
        fn submit_with_different_band_and_mode() {
            let mut state = QsoEntryState::new();
            fill_valid_callsign(&mut state);
            state.handle_key(alt_press(KeyCode::Char('b'))); // 20M -> 17M
            state.handle_key(alt_press(KeyCode::Char('m'))); // SSB -> CW
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => {
                    assert_eq!(qso.band, Band::M17);
                    assert_eq!(qso.mode, Mode::Cw);
                }
                other => panic!("expected AddQso, got {other:?}"),
            }
        }
    }

    mod clear_fast_fields {
        use super::*;

        #[test]
        fn clears_callsign_park_comments() {
            let mut state = QsoEntryState::new();
            fill_valid_callsign(&mut state);
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "K-1234");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "test comment");

            state.clear_fast_fields();
            assert_eq!(state.form().value(THEIR_CALL), "");
            assert_eq!(state.form().value(THEIR_PARK), "");
            assert_eq!(state.form().value(COMMENTS), "");
        }

        #[test]
        fn repopulates_rst_defaults() {
            let mut state = QsoEntryState::new();
            // Change RST
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Backspace));
            state.handle_key(press(KeyCode::Backspace));
            type_string(&mut state, "57");

            state.clear_fast_fields();
            assert_eq!(state.form().value(RST_SENT), "59");
            assert_eq!(state.form().value(RST_RCVD), "59");
        }

        #[test]
        fn band_mode_persist() {
            let mut state = QsoEntryState::new();
            state.handle_key(alt_press(KeyCode::Char('b'))); // cycle band
            state.handle_key(alt_press(KeyCode::Char('m'))); // cycle mode
            let band = state.band();
            let mode = state.mode();

            state.clear_fast_fields();
            assert_eq!(state.band(), band);
            assert_eq!(state.mode(), mode);
        }

        #[test]
        fn rst_matches_current_mode_after_clear() {
            let mut state = QsoEntryState::new();
            state.handle_key(alt_press(KeyCode::Char('m'))); // SSB -> CW
            assert_eq!(state.mode(), Mode::Cw);

            state.clear_fast_fields();
            assert_eq!(state.form().value(RST_SENT), "599");
            assert_eq!(state.form().value(RST_RCVD), "599");
        }

        #[test]
        fn clears_errors() {
            let mut state = QsoEntryState::new();
            state.handle_key(press(KeyCode::Enter)); // trigger errors
            assert!(state.form().has_errors());
            state.clear_fast_fields();
            assert!(!state.form().has_errors());
        }

        #[test]
        fn clears_storage_error() {
            let mut state = QsoEntryState::new();
            state.set_error("storage failure".into());
            state.clear_fast_fields();
            assert_eq!(state.error(), None);
        }

        #[test]
        fn resets_focus_to_callsign() {
            let mut state = QsoEntryState::new();
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), RST_RCVD);
            state.clear_fast_fields();
            assert_eq!(state.form().focus(), THEIR_CALL);
        }
    }

    mod recent_qsos {
        use super::*;

        #[test]
        fn set_log_context_populates_recent() {
            let mut state = QsoEntryState::new();
            let mut log = Log::new(
                "W1AW".to_string(),
                None,
                Some("K-0001".to_string()),
                "FN31".to_string(),
            )
            .unwrap();
            log.add_qso(make_qso("W3ABC", Band::M20, Mode::Ssb));
            log.add_qso(make_qso("N0CALL", Band::M40, Mode::Cw));
            log.add_qso(make_qso("KD9XYZ", Band::M20, Mode::Ssb));

            state.set_log_context(&log);
            assert_eq!(state.recent_qsos().len(), 3);
            // Newest first
            assert_eq!(state.recent_qsos()[0].their_call, "KD9XYZ");
            assert_eq!(state.recent_qsos()[1].their_call, "N0CALL");
            assert_eq!(state.recent_qsos()[2].their_call, "W3ABC");
        }

        #[test]
        fn set_log_context_caps_at_3() {
            let mut state = QsoEntryState::new();
            let mut log = Log::new(
                "W1AW".to_string(),
                None,
                Some("K-0001".to_string()),
                "FN31".to_string(),
            )
            .unwrap();
            for i in 0..5 {
                log.add_qso(make_qso(&format!("W{i}AW"), Band::M20, Mode::Ssb));
            }

            state.set_log_context(&log);
            assert_eq!(state.recent_qsos().len(), 3);
        }

        #[test]
        fn add_recent_qso_newest_first() {
            let mut state = QsoEntryState::new();
            state.add_recent_qso(make_qso("W3ABC", Band::M20, Mode::Ssb));
            state.add_recent_qso(make_qso("KD9XYZ", Band::M20, Mode::Ssb));
            assert_eq!(state.recent_qsos()[0].their_call, "KD9XYZ");
            assert_eq!(state.recent_qsos()[1].their_call, "W3ABC");
        }

        #[test]
        fn add_recent_qso_caps_at_3() {
            let mut state = QsoEntryState::new();
            state.add_recent_qso(make_qso("A1A", Band::M20, Mode::Ssb));
            state.add_recent_qso(make_qso("B2B", Band::M20, Mode::Ssb));
            state.add_recent_qso(make_qso("C3C", Band::M20, Mode::Ssb));
            state.add_recent_qso(make_qso("D4D", Band::M20, Mode::Ssb));
            assert_eq!(state.recent_qsos().len(), 3);
            assert_eq!(state.recent_qsos()[0].their_call, "D4D");
            assert_eq!(state.recent_qsos()[2].their_call, "B2B");
        }
    }

    mod navigation {
        use super::*;

        #[test]
        fn esc_navigates_to_log_select() {
            let mut state = QsoEntryState::new();
            let action = state.handle_key(press(KeyCode::Esc));
            assert_eq!(action, Action::Navigate(Screen::LogSelect));
        }
    }

    mod error_display {
        use super::*;

        #[test]
        fn set_error_returns_message() {
            let mut state = QsoEntryState::new();
            state.set_error("test error".into());
            assert_eq!(state.error(), Some("test error"));
        }

        #[test]
        fn set_log_context_with_empty_log() {
            let mut state = QsoEntryState::new();
            let log = Log::new(
                "W1AW".to_string(),
                None,
                Some("K-0001".to_string()),
                "FN31".to_string(),
            )
            .unwrap();
            state.set_log_context(&log);
            assert!(state.recent_qsos().is_empty());
        }
    }

    mod reset {
        use super::*;

        #[test]
        fn resets_to_defaults() {
            let mut state = QsoEntryState::new();
            fill_valid_callsign(&mut state);
            state.handle_key(alt_press(KeyCode::Char('b')));
            state.handle_key(alt_press(KeyCode::Char('m')));
            state.add_recent_qso(make_qso("W1AW", Band::M20, Mode::Ssb));
            state.set_error("some error".into());

            state.reset();
            assert_eq!(state.band(), Band::M20);
            assert_eq!(state.mode(), Mode::Ssb);
            assert_eq!(state.form().value(THEIR_CALL), "");
            assert_eq!(state.form().value(RST_SENT), "59");
            assert!(state.recent_qsos().is_empty());
            assert_eq!(state.error(), None);
        }
    }

    mod cycle_helper {
        use super::*;

        #[test]
        fn cycle_forward() {
            assert_eq!(cycle(Band::all(), Band::M20, true), Band::M17);
        }

        #[test]
        fn cycle_backward() {
            assert_eq!(cycle(Band::all(), Band::M20, false), Band::M30);
        }

        #[test]
        fn cycle_forward_wraps_at_end() {
            assert_eq!(cycle(Band::all(), Band::Cm70, true), Band::M160);
        }

        #[test]
        fn cycle_backward_wraps_at_start() {
            assert_eq!(cycle(Band::all(), Band::M160, false), Band::Cm70);
        }

        #[test]
        fn cycle_modes_forward() {
            assert_eq!(cycle(Mode::all(), Mode::Ssb, true), Mode::Cw);
        }

        #[test]
        fn cycle_modes_backward_wraps() {
            assert_eq!(cycle(Mode::all(), Mode::Ssb, false), Mode::Digi);
        }
    }
}
