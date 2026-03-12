//! QSO entry screen — the core data entry form for logging contacts.

use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use crate::model::{
    Band, Log, Mode, Qso, normalize_park_ref, validate_callsign, validate_fd_exchange,
    validate_park_ref, validate_section, validate_wfd_exchange,
};
use crate::tui::action::Action;
use crate::tui::app::Screen;
use crate::tui::widgets::form::{Form, FormField, RstField, draw_form_field};
use crate::tui::widgets::{StatusBarContext, draw_status_bar};

/// Field index for the other station's callsign (all form types).
const THEIR_CALL: usize = 0;

// General / POTA form field indices
/// Field index for RST sent (General and POTA only).
const RST_SENT: usize = 1;
/// Field index for RST received (General and POTA only).
const RST_RCVD: usize = 2;
/// Field index for the other station's park reference (POTA form).
const POTA_THEIR_PARK: usize = 3;
/// Field index for optional frequency in kHz (General form).
const GENERAL_FREQUENCY: usize = 3;
/// Field index for optional frequency in kHz (POTA form; after Their Park).
const POTA_FREQUENCY: usize = 4;

// FD / WFD contest form field indices (no RST; exchange split into class + section)
/// Field index for the other station's contest class (FD and WFD).
const CONTEST_THEIR_CLASS: usize = 1;
/// Field index for the other station's contest section (FD and WFD).
const CONTEST_THEIR_SECTION: usize = 2;
/// Field index for frequency in kHz (FD and WFD; index 3 in contest forms).
const CONTEST_FREQUENCY: usize = 3;
// FD: Frequency at index 3, Comments at index 4
// WFD: Frequency at index 3, Comments at index 4
// Comments is always at form_type.comments_idx()

/// The form variant in use for the current log type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum QsoFormType {
    #[default]
    General,
    Pota,
    FieldDay,
    WinterFieldDay,
}

impl QsoFormType {
    /// Returns `true` for General and POTA (forms include RST Sent/Rcvd fields).
    fn has_rst(self) -> bool {
        matches!(self, Self::General | Self::Pota)
    }

    /// Returns `true` for FD and WFD (forms use class + section instead of RST).
    fn has_contest_exchange(self) -> bool {
        matches!(self, Self::FieldDay | Self::WinterFieldDay)
    }

    /// Index of the Comments field for this form type.
    fn comments_idx(self) -> usize {
        match self {
            Self::General => 4,
            Self::Pota => 5,
            Self::FieldDay | Self::WinterFieldDay => 4,
        }
    }

    /// Index of the optional/required frequency field for this form type.
    fn frequency_field_idx(self) -> usize {
        match self {
            Self::General => GENERAL_FREQUENCY,
            Self::Pota => POTA_FREQUENCY,
            Self::FieldDay | Self::WinterFieldDay => CONTEST_FREQUENCY,
        }
    }
}

/// State for the QSO entry screen.
#[derive(Debug)]
pub struct QsoEntryState {
    form: Form,
    form_type: QsoFormType,
    band: Band,
    mode: Mode,
    recent_qsos: Vec<Qso>,
    error: Option<String>,
    /// When editing an existing QSO: `(index, original_timestamp)`.
    editing: Option<(usize, DateTime<Utc>)>,
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
        let form_type = QsoFormType::default();
        let form = Self::build_form_for_type(form_type, mode);
        Self {
            form,
            form_type,
            band: Band::default(),
            mode,
            recent_qsos: Vec::new(),
            error: None,
            editing: None,
        }
    }

    /// Constructs a [`Form`] with the correct fields for the given type and mode.
    ///
    /// - General: Their Callsign | RST Sent | RST Rcvd | Frequency (kHz) | Comments
    /// - POTA: Their Callsign | RST Sent | RST Rcvd | Their Park | Frequency (kHz) | Comments
    /// - FD / WFD: Their Callsign | Their Class | Their Section | Frequency | Comments  (no RST)
    fn build_form_for_type(form_type: QsoFormType, mode: Mode) -> Form {
        let rst = mode.default_rst();
        match form_type {
            QsoFormType::General => Form::new(vec![
                Box::new(FormField::new("Their Callsign", true)),
                Box::new(RstField::new("RST Sent", rst)),
                Box::new(RstField::new("RST Rcvd", rst)),
                Box::new(FormField::new("Frequency (kHz)", false)),
                Box::new(FormField::new("Comments", false)),
            ]),
            QsoFormType::Pota => Form::new(vec![
                Box::new(FormField::new("Their Callsign", true)),
                Box::new(RstField::new("RST Sent", rst)),
                Box::new(RstField::new("RST Rcvd", rst)),
                Box::new(FormField::new("Their Park", false)),
                Box::new(FormField::new("Frequency (kHz)", false)),
                Box::new(FormField::new("Comments", false)),
            ]),
            QsoFormType::FieldDay => Form::new(vec![
                Box::new(FormField::new("Their Callsign", true)),
                Box::new(FormField::new("Their Class (e.g. 3A)", true)),
                Box::new(FormField::new("Their Section", true)),
                Box::new(FormField::new("Frequency (kHz)", true)),
                Box::new(FormField::new("Comments", false)),
            ]),
            QsoFormType::WinterFieldDay => Form::new(vec![
                Box::new(FormField::new("Their Callsign", true)),
                Box::new(FormField::new("Their Class (e.g. 2H)", true)),
                Box::new(FormField::new("Their Section", true)),
                Box::new(FormField::new("Frequency (kHz)", true)),
                Box::new(FormField::new("Comments", false)),
            ]),
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
                KeyCode::Char('x') => {
                    return Action::Navigate(Screen::Export);
                }
                KeyCode::Char('e') => {
                    return Action::Navigate(Screen::QsoList);
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
                if self.form.focus() == self.form_type.frequency_field_idx() {
                    self.try_auto_set_band_from_frequency();
                }
                self.form.focus_next();
                Action::None
            }
            KeyCode::BackTab => {
                if self.form.focus() == self.form_type.frequency_field_idx() {
                    self.try_auto_set_band_from_frequency();
                }
                self.form.focus_prev();
                Action::None
            }
            KeyCode::Backspace => {
                self.form.delete_char();
                Action::None
            }
            KeyCode::Esc => {
                if self.editing.is_some() {
                    self.editing = None;
                    Action::Navigate(Screen::QsoList)
                } else {
                    Action::Navigate(Screen::LogSelect)
                }
            }
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

    /// Populates recent QSOs from a log (newest first) and rebuilds the form for the log type.
    ///
    /// Stores up to 20 QSOs; `draw_recent_qsos` limits display to what fits in the
    /// available `Rect` height so the row count adapts to the terminal size.
    pub fn set_log_context(&mut self, log: &Log) {
        self.recent_qsos = log.header().qsos.iter().rev().take(20).cloned().collect();
        let new_type = match log {
            Log::General(_) => QsoFormType::General,
            Log::Pota(_) => QsoFormType::Pota,
            Log::FieldDay(_) => QsoFormType::FieldDay,
            Log::WinterFieldDay(_) => QsoFormType::WinterFieldDay,
        };
        if new_type != self.form_type {
            self.form_type = new_type;
            self.form = Self::build_form_for_type(new_type, self.mode);
        }
    }

    /// Adds a QSO to the recent list (newest first), capped at 20.
    ///
    /// Display is further limited by `draw_recent_qsos` based on available `Rect` height.
    pub fn add_recent_qso(&mut self, qso: Qso) {
        self.recent_qsos.insert(0, qso);
        self.recent_qsos.truncate(20);
    }

    /// Returns `true` if the form is in edit mode.
    pub fn is_editing(&self) -> bool {
        self.editing.is_some()
    }

    /// Clears edit mode without resetting the rest of the form.
    pub fn clear_editing(&mut self) {
        self.editing = None;
    }

    /// Enters edit mode: populates the form from an existing QSO.
    pub fn start_editing(&mut self, index: usize, qso: &Qso) {
        self.form.set_value(THEIR_CALL, &qso.their_call);
        match self.form_type {
            QsoFormType::General => {
                self.form.set_value(RST_SENT, &qso.rst_sent);
                self.form.set_value(RST_RCVD, &qso.rst_rcvd);
                self.form.set_value(
                    GENERAL_FREQUENCY,
                    qso.frequency
                        .map(|f| f.to_string())
                        .unwrap_or_default()
                        .as_str(),
                );
            }
            QsoFormType::Pota => {
                self.form.set_value(RST_SENT, &qso.rst_sent);
                self.form.set_value(RST_RCVD, &qso.rst_rcvd);
                self.form
                    .set_value(3, qso.their_park.as_deref().unwrap_or(""));
                self.form.set_value(
                    POTA_FREQUENCY,
                    qso.frequency
                        .map(|f| f.to_string())
                        .unwrap_or_default()
                        .as_str(),
                );
            }
            QsoFormType::FieldDay | QsoFormType::WinterFieldDay => {
                // Parse exchange_rcvd ("CLASS SECTION") into the two separate fields.
                let exchange = qso.exchange_rcvd.as_deref().unwrap_or("");
                if let Some((class, section)) = exchange.split_once(' ') {
                    self.form.set_value(CONTEST_THEIR_CLASS, class);
                    self.form.set_value(CONTEST_THEIR_SECTION, section);
                } else {
                    self.form.set_value(CONTEST_THEIR_CLASS, exchange);
                    self.form.set_value(CONTEST_THEIR_SECTION, "");
                }
                self.form.set_value(
                    CONTEST_FREQUENCY,
                    qso.frequency
                        .map(|f| f.to_string())
                        .unwrap_or_default()
                        .as_str(),
                );
            }
        }
        let comments_idx = self.form_type.comments_idx();
        self.form.set_value(comments_idx, &qso.comments);
        self.band = qso.band;
        self.mode = qso.mode;
        self.form.clear_errors();
        self.error = None;
        self.form.set_focus(THEIR_CALL);
        self.editing = Some((index, qso.timestamp));
    }

    /// Clears fast-moving fields and repopulates RST defaults for the current mode.
    ///
    /// For General/POTA: resets Their Callsign, RST fields, type-specific field, and Comments.
    /// For FD/WFD: resets Their Callsign, Their Class, Their Section, Frequency, and Comments.
    pub fn clear_fast_fields(&mut self) {
        self.form.clear_value(THEIR_CALL);
        if self.form_type.has_rst() {
            self.form.reset_field(RST_SENT);
            self.form.reset_field(RST_RCVD);
            match self.form_type {
                QsoFormType::General => {
                    self.form.clear_value(GENERAL_FREQUENCY);
                }
                QsoFormType::Pota => {
                    self.form.clear_value(POTA_THEIR_PARK);
                    self.form.clear_value(POTA_FREQUENCY);
                }
                _ => unreachable!(),
            }
        } else {
            self.form.clear_value(CONTEST_THEIR_CLASS);
            self.form.clear_value(CONTEST_THEIR_SECTION);
            self.form.clear_value(CONTEST_FREQUENCY);
        }
        self.form.clear_value(self.form_type.comments_idx());
        self.form.clear_errors();
        self.error = None;
        self.editing = None;
        self.form.set_focus(THEIR_CALL);
    }

    /// Resets all state back to initial defaults.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Handles a printable character: inserts into the focused field.
    ///
    /// Callsign, contest class/section (FD/WFD), and park ref (POTA) are auto-uppercased.
    fn handle_char(&mut self, ch: char) -> Action {
        let focus = self.form.focus();
        let should_uppercase = focus == THEIR_CALL
            || (self.form_type.has_contest_exchange()
                && (focus == CONTEST_THEIR_CLASS || focus == CONTEST_THEIR_SECTION))
            || (self.form_type == QsoFormType::Pota && focus == POTA_THEIR_PARK);
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

    /// If the frequency field contains a parseable kHz value that maps to a known band,
    /// auto-selects that band. Called when leaving the frequency field.
    fn try_auto_set_band_from_frequency(&mut self) {
        let idx = self.form_type.frequency_field_idx();
        if let Ok(freq) = self.form.value(idx).parse::<u32>()
            && let Some(band) = Band::from_frequency_khz(freq)
        {
            self.band = band;
        }
    }

    /// Cycles the mode forward or backward, wrapping around.
    ///
    /// When the mode changes, RST fields are updated to the new mode's default
    /// only if they still contain the previous mode's default.
    /// FD/WFD forms have no RST fields, so RST updates are skipped for those.
    fn cycle_mode(&mut self, forward: bool) {
        self.mode = cycle(Mode::all(), self.mode, forward);
        let new_rst = self.mode.default_rst();

        if self.form_type.has_rst() {
            self.form.set_mode_default(RST_SENT, new_rst);
            self.form.set_mode_default(RST_RCVD, new_rst);
        }
    }

    /// Validates the form and constructs a QSO.
    fn submit(&mut self) -> Action {
        self.form.clear_errors();
        self.error = None;

        let their_call = self.form.value(THEIR_CALL).to_string();
        let comments_idx = self.form_type.comments_idx();
        let comments = self.form.value(comments_idx).to_string();

        if let Err(e) = validate_callsign(&their_call) {
            self.form.set_error(THEIR_CALL, e.to_string());
        }

        let rst_sent: String;
        let rst_rcvd: String;
        let mut their_park: Option<String> = None;
        let mut exchange_rcvd: Option<String> = None;
        let mut frequency: Option<u32> = None;

        match self.form_type {
            QsoFormType::General => {
                rst_sent = self.form.value(RST_SENT).to_string();
                rst_rcvd = self.form.value(RST_RCVD).to_string();
                if rst_sent.is_empty() {
                    self.form.set_error(RST_SENT, "RST sent is required".into());
                }
                if rst_rcvd.is_empty() {
                    self.form
                        .set_error(RST_RCVD, "RST received is required".into());
                }
                let freq_str = self.form.value(GENERAL_FREQUENCY).to_string();
                if !freq_str.is_empty() {
                    match freq_str.parse::<u32>() {
                        Ok(f) if f > 0 => {
                            self.try_auto_set_band_from_frequency();
                            frequency = Some(f);
                        }
                        _ => self.form.set_error(
                            GENERAL_FREQUENCY,
                            "frequency must be a positive integer (kHz)".into(),
                        ),
                    }
                }
            }
            QsoFormType::Pota => {
                rst_sent = self.form.value(RST_SENT).to_string();
                rst_rcvd = self.form.value(RST_RCVD).to_string();
                if rst_sent.is_empty() {
                    self.form.set_error(RST_SENT, "RST sent is required".into());
                }
                if rst_rcvd.is_empty() {
                    self.form
                        .set_error(RST_RCVD, "RST received is required".into());
                }
                // Normalize even though handle_char auto-uppercases at input: start_editing sets
                // the form value directly (bypassing handle_char), so stored lowercase park refs
                // loaded from pre-fix log files would reach submit without auto-uppercase.
                let park_str = normalize_park_ref(self.form.value(3));
                if !park_str.is_empty() {
                    if let Err(e) = validate_park_ref(&park_str) {
                        self.form.set_error(3, e.to_string());
                    } else {
                        their_park = Some(park_str);
                    }
                }
                let freq_str = self.form.value(POTA_FREQUENCY).to_string();
                if !freq_str.is_empty() {
                    match freq_str.parse::<u32>() {
                        Ok(f) if f > 0 => {
                            self.try_auto_set_band_from_frequency();
                            frequency = Some(f);
                        }
                        _ => self.form.set_error(
                            POTA_FREQUENCY,
                            "frequency must be a positive integer (kHz)".into(),
                        ),
                    }
                }
            }
            QsoFormType::FieldDay => {
                // FD does not exchange RST; use conventional default
                rst_sent = "59".to_string();
                rst_rcvd = "59".to_string();
                let class_str = self.form.value(CONTEST_THEIR_CLASS).to_string();
                let section_str = self.form.value(CONTEST_THEIR_SECTION).to_string();
                if class_str.is_empty() {
                    self.form
                        .set_error(CONTEST_THEIR_CLASS, "class is required".into());
                }
                if let Err(e) = validate_section(&section_str) {
                    self.form.set_error(CONTEST_THEIR_SECTION, e.to_string());
                }
                if !class_str.is_empty() && !section_str.is_empty() {
                    let assembled = format!("{class_str} {section_str}");
                    match validate_fd_exchange(&assembled) {
                        Ok(()) => exchange_rcvd = Some(assembled),
                        Err(e) => self.form.set_error(CONTEST_THEIR_CLASS, e.to_string()),
                    }
                }
                let freq_str = self.form.value(CONTEST_FREQUENCY).to_string();
                match freq_str.parse::<u32>() {
                    Ok(f) if f > 0 => {
                        // Auto-select band; no-op if frequency is between amateur allocations
                        self.try_auto_set_band_from_frequency();
                        frequency = Some(f);
                    }
                    _ => self.form.set_error(
                        CONTEST_FREQUENCY,
                        "frequency must be a positive integer (kHz)".into(),
                    ),
                }
            }
            QsoFormType::WinterFieldDay => {
                // WFD does not exchange RST; use conventional default
                rst_sent = "59".to_string();
                rst_rcvd = "59".to_string();
                let class_str = self.form.value(CONTEST_THEIR_CLASS).to_string();
                let section_str = self.form.value(CONTEST_THEIR_SECTION).to_string();
                if class_str.is_empty() {
                    self.form
                        .set_error(CONTEST_THEIR_CLASS, "class is required".into());
                }
                if let Err(e) = validate_section(&section_str) {
                    self.form.set_error(CONTEST_THEIR_SECTION, e.to_string());
                }
                if !class_str.is_empty() && !section_str.is_empty() {
                    let assembled = format!("{class_str} {section_str}");
                    match validate_wfd_exchange(&assembled) {
                        Ok(()) => exchange_rcvd = Some(assembled),
                        Err(e) => self.form.set_error(CONTEST_THEIR_CLASS, e.to_string()),
                    }
                }
                let freq_str = self.form.value(CONTEST_FREQUENCY).to_string();
                match freq_str.parse::<u32>() {
                    Ok(f) if f > 0 => {
                        // Auto-select band; no-op if frequency is between amateur allocations
                        self.try_auto_set_band_from_frequency();
                        frequency = Some(f);
                    }
                    _ => self.form.set_error(
                        CONTEST_FREQUENCY,
                        "frequency must be a positive integer (kHz)".into(),
                    ),
                }
            }
        }

        if self.form.has_errors() {
            return Action::None;
        }

        let timestamp = self.editing.map(|(_, ts)| ts).unwrap_or_else(Utc::now);

        match Qso::new(
            their_call,
            rst_sent,
            rst_rcvd,
            self.band,
            self.mode,
            timestamp,
            comments,
            their_park,
            exchange_rcvd,
            frequency,
        ) {
            Ok(qso) => match self.editing {
                Some((idx, _)) => Action::UpdateQso(idx, qso),
                None => Action::AddQso(qso),
            },
            Err(e) => {
                self.form.set_error(THEIR_CALL, e.to_string());
                Action::None
            }
        }
    }
}

/// Cycles through a slice to find the next or previous element.
///
/// # Panics
///
/// Panics if `current` is not found in `items`.
fn cycle<T: PartialEq + Copy>(items: &[T], current: T, forward: bool) -> T {
    let pos = items
        .iter()
        .position(|&x| x == current)
        .expect("current must be in items");
    let next = if forward {
        (pos + 1) % items.len()
    } else {
        (pos + items.len() - 1) % items.len()
    };
    items[next]
}

/// Renders the QSO entry screen.
#[mutants::skip]
pub fn draw_qso_entry(state: &QsoEntryState, log: Option<&Log>, frame: &mut Frame, area: Rect) {
    let title = if state.is_editing() {
        " Edit QSO "
    } else {
        " QSO Entry "
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [
        status_area,
        header_area,
        form_area,
        recent_area,
        footer_area,
    ] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(6),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .areas(inner);

    let ctx = log.map(StatusBarContext::from_log).unwrap_or_default();
    draw_status_bar(&ctx, frame, status_area);

    draw_header(state, log, frame, header_area);

    // Form fields
    draw_qso_entry_form(state, frame, form_area);

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

    draw_recent_qsos(state, frame, recent_area);

    // Footer
    let footer_text = if state.is_editing() {
        "Tab/Shift+Tab: next/prev  Alt+b/m: band/mode (Shift: reverse)  Enter: save  Esc: cancel"
    } else {
        "Tab/Shift+Tab: next/prev  Alt+b/m: band/mode (Shift: reverse)  Alt+e: edit  Alt+x: export  Enter: log  Esc: back"
    };
    let footer =
        Paragraph::new(Line::from(footer_text)).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}

/// Renders the QSO entry form in a two-row horizontal layout.
///
/// Row 1: index 0 | index 1 | index 2 (always three equal columns)
///   - General / POTA: Their Callsign | RST Sent | RST Rcvd
///   - FD / WFD:       Their Callsign | Their Class | Their Section
///
/// Row 2: varies by log type
///   - General:        Frequency (3)  | Comments (4) — two halves
///   - POTA:           Their Park (3) | Frequency (4) | Comments (5) — three thirds
///   - FD / WFD:       Frequency (3)  | Comments (4) — two halves
#[mutants::skip]
fn draw_qso_entry_form(state: &QsoEntryState, frame: &mut Frame, area: Rect) {
    use ratatui::layout::Constraint::Ratio;
    let form = state.form();
    let form_type = state.form_type;

    // Split into two rows of 3 lines each
    let [row1_area, row2_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Length(3)]).areas(area);

    // Row 1: always three equal columns (indices 0, 1, 2)
    let [col0, col1, col2] =
        Layout::horizontal([Ratio(1, 3), Ratio(1, 3), Ratio(1, 3)]).areas(row1_area);
    draw_form_field(form, 0, frame, col0);
    draw_form_field(form, 1, frame, col1);
    draw_form_field(form, 2, frame, col2);

    // Row 2: layout depends on form type
    match form_type {
        QsoFormType::General | QsoFormType::FieldDay | QsoFormType::WinterFieldDay => {
            // Frequency on left half, Comments on right half
            let [freq_area, comments_area] =
                Layout::horizontal([Ratio(1, 2), Ratio(1, 2)]).areas(row2_area);
            draw_form_field(form, 3, frame, freq_area);
            draw_form_field(form, 4, frame, comments_area);
        }
        QsoFormType::Pota => {
            // Their Park | Frequency | Comments — three equal columns
            let [park_area, freq_area, comments_area] =
                Layout::horizontal([Ratio(1, 3), Ratio(1, 3), Ratio(1, 3)]).areas(row2_area);
            draw_form_field(form, 3, frame, park_area);
            draw_form_field(form, POTA_FREQUENCY, frame, freq_area);
            draw_form_field(form, 5, frame, comments_area);
        }
    }
}

/// Renders the station info, band/mode, and activation progress header.
#[mutants::skip]
fn draw_header(state: &QsoEntryState, log: Option<&Log>, frame: &mut Frame, area: Rect) {
    if let Some(log) = log {
        let callsign = &log.header().station_callsign;
        let park = log.park_ref().unwrap_or("-");
        let grid = &log.header().grid_square;
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

        frame.render_widget(Paragraph::new(vec![header_line1, header_line2]), area);
    }
}

fn format_timestamp(qso: &Qso) -> String {
    qso.timestamp.format("%H:%M").to_string()
}

fn format_rst(qso: &Qso) -> String {
    format!("{}/{}", qso.rst_sent, qso.rst_rcvd)
}

fn format_frequency(qso: &Qso) -> String {
    qso.frequency.map(|f| f.to_string()).unwrap_or_default()
}

fn recent_qso_row_general(qso: &Qso) -> Row<'static> {
    // Time | Call | Band | Mode | RST | Freq
    Row::new(vec![
        format_timestamp(qso),
        qso.their_call.clone(),
        qso.band.to_string(),
        qso.mode.to_string(),
        format_rst(qso),
        format_frequency(qso),
    ])
}

fn recent_qso_row_pota(qso: &Qso) -> Row<'static> {
    // Time | Call | Band | Mode | RST | Park | Freq
    // Park and Freq are always distinct columns — no fallback mixing.
    Row::new(vec![
        format_timestamp(qso),
        qso.their_call.clone(),
        qso.band.to_string(),
        qso.mode.to_string(),
        format_rst(qso),
        qso.their_park.clone().unwrap_or_default(),
        format_frequency(qso),
    ])
}

fn recent_qso_row_contest(qso: &Qso) -> Row<'static> {
    // Time | Call | Band | Mode | Exchange | Freq
    Row::new(vec![
        format_timestamp(qso),
        qso.their_call.clone(),
        qso.band.to_string(),
        qso.mode.to_string(),
        qso.exchange_rcvd.clone().unwrap_or_default(),
        format_frequency(qso),
    ])
}

fn build_recent_rows<F: Fn(&Qso) -> Row<'static>>(
    state: &QsoEntryState,
    max_rows: usize,
    to_row: F,
) -> Vec<Row<'static>> {
    state
        .recent_qsos()
        .iter()
        .take(max_rows)
        .map(to_row)
        .collect()
}

/// Renders the recent QSOs table widget.
///
/// Row count adapts to the available `Rect` height — no hard-coded limit.
/// Column sets are fully branched on log type; park and frequency are always
/// separate columns so there is never any ambiguity about which value is shown.
#[mutants::skip]
fn draw_recent_qsos(state: &QsoEntryState, frame: &mut Frame, area: Rect) {
    let recent_block = Block::default()
        .title(" Recent QSOs ")
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));

    let recent_inner = recent_block.inner(area);
    frame.render_widget(recent_block, area);

    if state.recent_qsos().is_empty() {
        return;
    }

    // Limit rows to what actually fits — Rect height is the source of truth.
    let max_rows = recent_inner.height as usize;

    match state.form_type {
        QsoFormType::General => {
            let widths = [
                Constraint::Length(6),
                Constraint::Length(10),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(8),
                Constraint::Min(10),
            ];
            frame.render_widget(
                Table::new(
                    build_recent_rows(state, max_rows, recent_qso_row_general),
                    widths,
                ),
                recent_inner,
            );
        }
        QsoFormType::Pota => {
            let widths = [
                Constraint::Length(6),
                Constraint::Length(10),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(8),
                Constraint::Length(12),
                Constraint::Min(8),
            ];
            frame.render_widget(
                Table::new(
                    build_recent_rows(state, max_rows, recent_qso_row_pota),
                    widths,
                ),
                recent_inner,
            );
        }
        QsoFormType::FieldDay | QsoFormType::WinterFieldDay => {
            let widths = [
                Constraint::Length(6),
                Constraint::Length(10),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(10),
                Constraint::Min(10),
            ];
            frame.render_widget(
                Table::new(
                    build_recent_rows(state, max_rows, recent_qso_row_contest),
                    widths,
                ),
                recent_inner,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    use super::*;
    use crate::model::{Band, FdClass, FdPowerCategory, FieldDayLog, PotaLog, WfdClass, WfdLog};

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
            None,
            None,
        )
        .unwrap()
    }

    fn fill_valid_callsign(state: &mut QsoEntryState) {
        type_string(state, "KD9XYZ");
    }

    fn make_pota_log() -> Log {
        Log::Pota(
            PotaLog::new(
                "W1AW".to_string(),
                None,
                "K-0001".to_string(),
                "FN31".to_string(),
            )
            .unwrap(),
        )
    }

    fn make_fd_log() -> Log {
        Log::FieldDay(
            FieldDayLog::new(
                "W1AW".to_string(),
                None,
                1,
                FdClass::B,
                "EPA".to_string(),
                FdPowerCategory::Low,
                "FN31".to_string(),
            )
            .unwrap(),
        )
    }

    fn make_wfd_log() -> Log {
        Log::WinterFieldDay(
            WfdLog::new(
                "W1AW".to_string(),
                None,
                1,
                WfdClass::I,
                "EPA".to_string(),
                "FN31".to_string(),
            )
            .unwrap(),
        )
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
            // General form: Frequency at index 3, Comments at index 4
            assert_eq!(state.form().value(GENERAL_FREQUENCY), "");
            assert_eq!(state.form().value(4), "");
            assert!(state.recent_qsos().is_empty());
            assert_eq!(state.error(), None);
        }

        #[test]
        fn pota_context_adds_their_park_field() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            // POTA form: Their Park at index 3, Frequency at index 4, Comments at index 5
            assert_eq!(state.form().value(3), "");
            assert_eq!(state.form().value(POTA_FREQUENCY), "");
            assert_eq!(state.form().value(5), "");
            assert_eq!(
                state.form().fields()[3].label(),
                "Their Park",
                "index 3 should be Their Park"
            );
            assert_eq!(
                state.form().fields()[POTA_FREQUENCY].label(),
                "Frequency (kHz)",
                "index 4 should be Frequency"
            );
        }

        #[test]
        fn fd_contest_class_and_section_are_required() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            assert!(
                state.form().fields()[CONTEST_THEIR_CLASS].required(),
                "Their Class must be required for Field Day"
            );
            assert!(
                state.form().fields()[CONTEST_THEIR_SECTION].required(),
                "Their Section must be required for Field Day"
            );
        }

        #[test]
        fn wfd_contest_class_and_section_are_required() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_wfd_log());
            assert!(
                state.form().fields()[CONTEST_THEIR_CLASS].required(),
                "Their Class must be required for Winter Field Day"
            );
            assert!(
                state.form().fields()[CONTEST_THEIR_SECTION].required(),
                "Their Section must be required for Winter Field Day"
            );
        }

        #[test]
        fn pota_their_park_is_optional() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            assert!(
                !state.form().fields()[3].required(),
                "Their Park must be optional for POTA"
            );
            assert!(
                !state.form().fields()[POTA_FREQUENCY].required(),
                "Frequency must be optional for POTA"
            );
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
            state.set_log_context(&make_pota_log());
            // Tab to Their Park field (index 3 in POTA form)
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "k-0001");
            assert_eq!(state.form().value(3), "K-0001");
        }

        #[test]
        fn comments_not_uppercased() {
            let mut state = QsoEntryState::new();
            // General form: Comments at index 4; 4 tabs from THEIR_CALL
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "hello");
            assert_eq!(state.form().value(4), "hello");
        }

        #[test]
        fn general_rst_sent_not_auto_uppercased() {
            // RST fields are at indices 1 and 2 on General/POTA forms. They must NOT be
            // auto-uppercased. If has_contest_exchange() were mutated to return true, it would
            // cause these fields (matching contest indices 1/2) to be uppercased incorrectly.
            let mut state = QsoEntryState::new();
            state.handle_key(press(KeyCode::Tab)); // focus RST Sent (index 1)
            state.handle_key(press(KeyCode::Backspace));
            state.handle_key(press(KeyCode::Backspace));
            type_string(&mut state, "r5");
            assert_eq!(
                state.form().value(RST_SENT),
                "r5",
                "RST sent should not be auto-uppercased on General form"
            );
        }

        #[test]
        fn general_frequency_not_auto_uppercased() {
            // GENERAL_FREQUENCY is at index 3. The `handle_char` uppercase condition
            // checks `form_type == QsoFormType::Pota && focus == 3`. If `&&` were mutated
            // to `||`, a General form focused on index 3 would incorrectly uppercase input.
            let mut state = QsoEntryState::new();
            state.handle_key(press(KeyCode::Tab)); // RST Sent
            state.handle_key(press(KeyCode::Tab)); // RST Rcvd
            state.handle_key(press(KeyCode::Tab)); // Frequency (index 3)
            type_string(&mut state, "abc");
            assert_eq!(
                state.form().value(GENERAL_FREQUENCY),
                "abc",
                "General frequency field must not auto-uppercase input"
            );
        }

        #[test]
        fn backspace_deletes_char() {
            let mut state = QsoEntryState::new();
            type_string(&mut state, "W1AW");
            state.handle_key(press(KeyCode::Backspace));
            assert_eq!(state.form().value(THEIR_CALL), "W1A");
        }

        #[test]
        fn tab_cycles_focus_general() {
            // General form has 5 fields: THEIR_CALL, RST_SENT, RST_RCVD, Frequency(3), Comments(4)
            let mut state = QsoEntryState::new();
            assert_eq!(state.form().focus(), THEIR_CALL);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), RST_SENT);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), RST_RCVD);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), GENERAL_FREQUENCY);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), 4); // Comments in General form
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), THEIR_CALL);
        }

        #[test]
        fn tab_cycles_focus_pota() {
            // POTA form: THEIR_CALL, RST_SENT, RST_RCVD, Their Park(3), Frequency(4), Comments(5)
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            assert_eq!(state.form().focus(), THEIR_CALL);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), RST_SENT);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), RST_RCVD);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), 3); // Their Park
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), POTA_FREQUENCY);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), 5); // Comments
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), THEIR_CALL);
        }

        #[test]
        fn backtab_cycles_focus_backward() {
            // General form: last field is index 4 (Comments)
            let mut state = QsoEntryState::new();
            state.handle_key(shift_press(KeyCode::BackTab));
            assert_eq!(state.form().focus(), 4);
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
            let action = state.handle_key(alt_press(KeyCode::Char('z')));
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
            state.set_log_context(&make_pota_log());
            // Tab to Their Park (index 3 in POTA form)
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "mb-0001");
            assert_eq!(state.form().value(3), "MB-0001");
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

        #[test]
        fn mode_change_rearms_rst_replace_behavior_when_unedited() {
            // When a mode change updates an unedited RST field, the new default
            // should also be replaceable on first keystroke.
            let mut state = QsoEntryState::new();
            state.handle_key(alt_press(KeyCode::Char('m'))); // SSB → CW, value "59" → "599"
            assert_eq!(state.form().value(RST_SENT), "599");
            // First keystroke should replace "599", not append to it
            state.handle_key(press(KeyCode::Tab)); // → RST Sent
            state.handle_key(press(KeyCode::Char('5')));
            assert_eq!(
                state.form().value(RST_SENT),
                "5",
                "first key after mode-change must replace new default"
            );
        }

        #[test]
        fn mode_change_preserves_edited_rst_and_subsequent_input_appends() {
            // When the user has manually edited RST, a mode change must not overwrite it,
            // and subsequent input must append (not replace) since the field is "edited".
            let mut state = QsoEntryState::new();
            state.handle_key(press(KeyCode::Tab)); // → RST Sent
            state.handle_key(press(KeyCode::Char('5'))); // first char replaces default → "5"
            assert_eq!(state.form().value(RST_SENT), "5");
            state.handle_key(alt_press(KeyCode::Char('m'))); // SSB → CW
            assert_eq!(
                state.form().value(RST_SENT),
                "5",
                "mode change must not overwrite manually-edited RST"
            );
            // subsequent typing appends (not replaces) — the field stays in "edited" state
            state.handle_key(press(KeyCode::Char('9')));
            assert_eq!(state.form().value(RST_SENT), "59");
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
            state.set_log_context(&make_pota_log());
            fill_valid_callsign(&mut state);
            // Tab to Their Park (index 3 in POTA form)
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
            assert!(state.form().fields()[THEIR_CALL].error().is_some());
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
            assert!(state.form().fields()[RST_SENT].error().is_some());
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
            assert!(state.form().fields()[RST_RCVD].error().is_some());
        }

        #[test]
        fn invalid_park_ref_shows_error() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            fill_valid_callsign(&mut state);
            // Tab to Their Park (index 3 in POTA form)
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "BAD");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().fields()[3].error().is_some());
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
            // General form: Comments at index 4; 4 tabs from THEIR_CALL
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "nice signal");
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => {
                    assert_eq!(qso.comments, "nice signal");
                }
                other => panic!("expected AddQso, got {other:?}"),
            }
        }

        fn make_fd_log() -> Log {
            use crate::model::{FdClass, FdPowerCategory, FieldDayLog};
            Log::FieldDay(
                FieldDayLog::new(
                    "W1AW".to_string(),
                    None,
                    1,
                    FdClass::B,
                    "EPA".to_string(),
                    FdPowerCategory::Low,
                    "FN31".to_string(),
                )
                .unwrap(),
            )
        }

        fn make_wfd_log() -> Log {
            use crate::model::{WfdClass, WfdLog};
            Log::WinterFieldDay(
                WfdLog::new(
                    "W1AW".to_string(),
                    None,
                    1,
                    WfdClass::H,
                    "EPA".to_string(),
                    "FN31".to_string(),
                )
                .unwrap(),
            )
        }

        fn make_general_log() -> Log {
            use crate::model::GeneralLog;
            Log::General(GeneralLog::new("W1AW".to_string(), None, "FN31".to_string()).unwrap())
        }

        #[test]
        fn fd_valid_exchange_returns_add_qso() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            fill_valid_callsign(&mut state);
            // Tab to Their Class (index 1)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "3A");
            // Tab to Their Section (index 2)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "CT");
            // Tab to Frequency (index 3)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "14225");
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => {
                    assert_eq!(qso.exchange_rcvd, Some("3A CT".to_string()));
                    assert_eq!(qso.frequency, Some(14225));
                    assert_eq!(qso.their_park, None);
                }
                other => panic!("expected AddQso, got {other:?}"),
            }
        }

        #[test]
        fn fd_missing_class_shows_error() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            fill_valid_callsign(&mut state);
            // Leave class empty; section empty too
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(
                state.form().fields()[CONTEST_THEIR_CLASS].error().is_some(),
                "empty class should show error"
            );
        }

        #[test]
        fn fd_invalid_class_shows_error() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            fill_valid_callsign(&mut state);
            // Tab to Their Class (index 1)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "Z"); // Z is not a valid FD class
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "CT");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(
                state.form().fields()[CONTEST_THEIR_CLASS].error().is_some(),
                "invalid FD class should show error at class field"
            );
        }

        #[test]
        fn fd_missing_section_shows_error() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            fill_valid_callsign(&mut state);
            // Fill class but leave section empty
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "3A");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(
                state.form().fields()[CONTEST_THEIR_SECTION]
                    .error()
                    .is_some(),
                "empty section should show error at section field"
            );
        }

        #[test]
        fn fd_valid_class_empty_section_no_class_error() {
            // When class is valid but section is empty, only the section field gets an error.
            // If the &&→|| mutation were present, the class field would also receive a spurious
            // exchange validation error after the code assembled "<class> " and validated it.
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            fill_valid_callsign(&mut state);
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "3A"); // valid class
            // Leave section empty
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(
                state.form().fields()[CONTEST_THEIR_SECTION]
                    .error()
                    .is_some(),
                "empty section should show error at section field"
            );
            assert!(
                state.form().fields()[CONTEST_THEIR_CLASS].error().is_none(),
                "class field must not have an error when class is valid"
            );
        }

        #[test]
        fn wfd_valid_class_empty_section_no_class_error() {
            // Same invariant as fd_valid_class_empty_section_no_class_error, for WFD.
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_wfd_log());
            fill_valid_callsign(&mut state);
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "2H"); // valid WFD class
            // Leave section and frequency empty (we're only testing section validation)
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(
                state.form().fields()[CONTEST_THEIR_SECTION]
                    .error()
                    .is_some(),
                "empty section should show error at section field"
            );
            assert!(
                state.form().fields()[CONTEST_THEIR_CLASS].error().is_none(),
                "class field must not have an error when class is valid"
            );
        }

        #[test]
        fn wfd_valid_exchange_and_frequency_returns_add_qso() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_wfd_log());
            fill_valid_callsign(&mut state);
            // Tab to Their Class (index 1)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "2H");
            // Tab to Their Section (index 2)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "EPA");
            // Tab to Frequency (index 3)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "14225");
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => {
                    assert_eq!(qso.exchange_rcvd, Some("2H EPA".to_string()));
                    assert_eq!(qso.frequency, Some(14225));
                }
                other => panic!("expected AddQso, got {other:?}"),
            }
        }

        #[test]
        fn fd_frequency_auto_sets_band_on_submit() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            fill_valid_callsign(&mut state);
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "3A");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "CT");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "7200"); // 40M
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => assert_eq!(qso.band, Band::M40),
                other => panic!("expected AddQso, got {other:?}"),
            }
        }

        #[test]
        fn wfd_frequency_auto_sets_band_on_submit() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_wfd_log());
            fill_valid_callsign(&mut state);
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "2H");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "EPA");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "21200"); // 15M
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => assert_eq!(qso.band, Band::M15),
                other => panic!("expected AddQso, got {other:?}"),
            }
        }

        #[test]
        fn fd_frequency_auto_sets_band_on_tab() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            // Navigate to frequency field (index 3)
            state.handle_key(press(KeyCode::Tab)); // callsign → class
            state.handle_key(press(KeyCode::Tab)); // class → section
            state.handle_key(press(KeyCode::Tab)); // section → frequency
            type_string(&mut state, "7200"); // 40M (not the default 20M)
            assert_eq!(
                state.band(),
                Band::M20,
                "band should not change before tabbing away"
            );
            state.handle_key(press(KeyCode::Tab)); // frequency → comments (triggers auto-band)
            assert_eq!(state.band(), Band::M40, "band should auto-select to 40M");
        }

        #[test]
        fn fd_frequency_auto_sets_band_on_backtab() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            // Navigate to frequency field (index 3)
            state.handle_key(press(KeyCode::Tab)); // callsign → class
            state.handle_key(press(KeyCode::Tab)); // class → section
            state.handle_key(press(KeyCode::Tab)); // section → frequency
            type_string(&mut state, "3750"); // 80M (not the default 20M)
            assert_eq!(
                state.band(),
                Band::M20,
                "band should not change before tabbing away"
            );
            state.handle_key(press(KeyCode::BackTab)); // frequency → section (triggers auto-band)
            assert_eq!(state.band(), Band::M80, "band should auto-select to 80M");
        }

        #[test]
        fn general_form_tab_with_empty_frequency_does_not_change_band() {
            // Tabbing off an empty frequency field must not change the band.
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_general_log());
            // Tab to frequency field (index 3) and immediately tab away without typing
            state.handle_key(press(KeyCode::Tab)); // → rst_sent
            state.handle_key(press(KeyCode::Tab)); // → rst_rcvd
            state.handle_key(press(KeyCode::Tab)); // → frequency
            state.handle_key(press(KeyCode::Tab)); // → comments (triggers auto-band attempt)
            // Band stays at default — empty frequency field, no auto-band
            assert_eq!(state.band(), Band::M20);
        }

        #[test]
        fn fd_tab_from_non_frequency_field_does_not_trigger_auto_band() {
            // Guard: focus != CONTEST_FREQUENCY; tabbing off the class field must not fire
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            // Focus is on callsign (index 0); tab to class (index 1)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "3A");
            // Tab from class (index 1) to section (index 2) — must not trigger auto-band
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.band(), Band::M20);
        }

        #[test]
        fn fd_tab_off_non_frequency_does_not_override_manual_band() {
            // Kills the &&→|| mutant on the Tab guard:
            // auto-band fires correctly on Tab-off-frequency; a subsequent Tab off a
            // different field must NOT overwrite a manual band change.
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            state.handle_key(press(KeyCode::Tab)); // → class (1)
            state.handle_key(press(KeyCode::Tab)); // → section (2)
            state.handle_key(press(KeyCode::Tab)); // → frequency (3)
            type_string(&mut state, "7200"); // 40M
            state.handle_key(press(KeyCode::Tab)); // → comments (4): auto-band → M40
            assert_eq!(state.band(), Band::M40);
            // Manually step one band forward: M40 → M30
            state.handle_key(alt_press(KeyCode::Char('b')));
            assert_eq!(state.band(), Band::M30);
            // Tab from comments (4) to callsign (0) wrapping: must NOT fire auto-band
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(
                state.band(),
                Band::M30,
                "manual band selection must not be overwritten"
            );
        }

        #[test]
        fn fd_backtab_off_non_frequency_does_not_override_manual_band() {
            // Kills the &&→|| mutant on the BackTab guard.
            // After auto-band via BackTab-off-frequency, a subsequent BackTab from a
            // different field must NOT overwrite a manual band change.
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            state.handle_key(press(KeyCode::Tab)); // → class (1)
            state.handle_key(press(KeyCode::Tab)); // → section (2)
            state.handle_key(press(KeyCode::Tab)); // → frequency (3)
            type_string(&mut state, "7200"); // 40M
            // BackTab off frequency → section (2): auto-band → M40
            state.handle_key(press(KeyCode::BackTab));
            assert_eq!(state.band(), Band::M40);
            // Manually step one band forward: M40 → M30
            state.handle_key(alt_press(KeyCode::Char('b')));
            assert_eq!(state.band(), Band::M30);
            // BackTab from section (2) to class (1): must NOT fire auto-band
            state.handle_key(press(KeyCode::BackTab));
            assert_eq!(
                state.band(),
                Band::M30,
                "manual band selection must not be overwritten"
            );
        }

        #[test]
        fn fd_out_of_band_frequency_does_not_change_band() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "9999"); // not a ham band
            state.handle_key(press(KeyCode::Tab));
            // Band stays at default (20M) since 9999 kHz maps to nothing
            assert_eq!(state.band(), Band::M20);
        }

        #[test]
        fn fd_invalid_frequency_text_does_not_change_band() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "abc"); // not parseable
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.band(), Band::M20);
        }

        #[test]
        fn fd_invalid_frequency_shows_error() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            fill_valid_callsign(&mut state);
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "3A");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "CT");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "abc");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(
                state.form().fields()[CONTEST_FREQUENCY].error().is_some(),
                "invalid frequency should show error at frequency field"
            );
        }

        #[test]
        fn fd_zero_frequency_shows_error() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            fill_valid_callsign(&mut state);
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "3A");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "CT");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "0");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None, "zero frequency should be rejected");
            assert!(
                state.form().fields()[CONTEST_FREQUENCY].error().is_some(),
                "frequency field must show an error for 0"
            );
        }

        #[test]
        fn wfd_invalid_frequency_shows_error() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_wfd_log());
            fill_valid_callsign(&mut state);
            // Tab to Their Class (index 1)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "2H");
            // Tab to Their Section (index 2)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "EPA");
            // Tab to Frequency (index 3) - put invalid value
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "abc");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(
                state.form().fields()[3].error().is_some(),
                "invalid frequency should show error at frequency field (index 3)"
            );
        }

        #[test]
        fn wfd_zero_frequency_shows_error() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_wfd_log());
            fill_valid_callsign(&mut state);
            // Tab to Their Class (index 1)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "2H");
            // Tab to Their Section (index 2)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "EPA");
            // Tab to Frequency (index 3) - enter 0 which is not a valid frequency
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "0");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None, "zero frequency should be rejected");
            assert!(
                state.form().fields()[3].error().is_some(),
                "frequency field must show an error for 0"
            );
        }

        #[test]
        fn wfd_class_and_section_auto_uppercased() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_wfd_log());
            // Tab to Their Class (index 1)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "2h"); // lowercase
            assert_eq!(state.form().value(CONTEST_THEIR_CLASS), "2H");
            // Tab to Their Section (index 2)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "epa"); // lowercase
            assert_eq!(state.form().value(CONTEST_THEIR_SECTION), "EPA");
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

        // --- General / POTA frequency tests ---

        #[test]
        fn general_empty_frequency_is_optional() {
            // Submitting without entering a frequency yields frequency = None
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_general_log());
            fill_valid_callsign(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => assert_eq!(qso.frequency, None),
                other => panic!("expected AddQso, got {other:?}"),
            }
        }

        #[test]
        fn general_valid_frequency_is_stored() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_general_log());
            fill_valid_callsign(&mut state);
            // Tab to frequency field (index 3)
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "14225");
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => assert_eq!(qso.frequency, Some(14225)),
                other => panic!("expected AddQso, got {other:?}"),
            }
        }

        #[test]
        fn general_invalid_frequency_shows_error() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_general_log());
            fill_valid_callsign(&mut state);
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "abc");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(
                state.form().fields()[GENERAL_FREQUENCY].error().is_some(),
                "invalid general frequency must show error"
            );
        }

        #[test]
        fn general_zero_frequency_shows_error() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_general_log());
            fill_valid_callsign(&mut state);
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "0");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(
                state.form().fields()[GENERAL_FREQUENCY].error().is_some(),
                "zero general frequency must show error"
            );
        }

        #[test]
        fn general_frequency_auto_sets_band_on_submit() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_general_log());
            fill_valid_callsign(&mut state);
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "7200"); // 40M
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => assert_eq!(qso.band, Band::M40),
                other => panic!("expected AddQso, got {other:?}"),
            }
        }

        #[test]
        fn general_frequency_auto_sets_band_on_tab() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_general_log());
            state.handle_key(press(KeyCode::Tab)); // → rst_sent
            state.handle_key(press(KeyCode::Tab)); // → rst_rcvd
            state.handle_key(press(KeyCode::Tab)); // → frequency
            type_string(&mut state, "3750"); // 80M
            assert_eq!(
                state.band(),
                Band::M20,
                "band should not change before tabbing away"
            );
            state.handle_key(press(KeyCode::Tab)); // → comments: triggers auto-band
            assert_eq!(state.band(), Band::M80);
        }

        #[test]
        fn pota_empty_frequency_is_optional() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            fill_valid_callsign(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => assert_eq!(qso.frequency, None),
                other => panic!("expected AddQso, got {other:?}"),
            }
        }

        #[test]
        fn pota_valid_frequency_is_stored() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            fill_valid_callsign(&mut state);
            // Tab to Frequency field (index 4 in POTA form)
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "14225");
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => assert_eq!(qso.frequency, Some(14225)),
                other => panic!("expected AddQso, got {other:?}"),
            }
        }

        #[test]
        fn pota_invalid_frequency_shows_error() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            fill_valid_callsign(&mut state);
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "abc");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(
                state.form().fields()[POTA_FREQUENCY].error().is_some(),
                "invalid POTA frequency must show error"
            );
        }

        #[test]
        fn pota_zero_frequency_shows_error() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            fill_valid_callsign(&mut state);
            // Tab to Frequency field (index 4 in POTA form)
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "0");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None, "zero frequency should be rejected");
            assert!(
                state.form().fields()[POTA_FREQUENCY].error().is_some(),
                "POTA frequency field must show an error for 0"
            );
        }

        #[test]
        fn pota_frequency_auto_sets_band_on_submit() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            fill_valid_callsign(&mut state);
            // Tab to Frequency (index 4)
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "21200"); // 15M
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::AddQso(qso) => assert_eq!(qso.band, Band::M15),
                other => panic!("expected AddQso, got {other:?}"),
            }
        }

        #[test]
        fn pota_frequency_auto_sets_band_on_tab() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            // Tab to frequency field (index 4: after callsign, rst_sent, rst_rcvd, their_park)
            state.handle_key(press(KeyCode::Tab)); // → rst_sent
            state.handle_key(press(KeyCode::Tab)); // → rst_rcvd
            state.handle_key(press(KeyCode::Tab)); // → their_park
            state.handle_key(press(KeyCode::Tab)); // → frequency
            type_string(&mut state, "7200"); // 40M
            assert_eq!(
                state.band(),
                Band::M20,
                "band should not change before tabbing away"
            );
            state.handle_key(press(KeyCode::Tab)); // → comments: triggers auto-band
            assert_eq!(state.band(), Band::M40);
        }
    }

    mod clear_fast_fields {
        use super::*;

        #[test]
        fn clears_callsign_park_comments() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            fill_valid_callsign(&mut state);
            // Tab to Their Park (index 3) in POTA form
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "K-1234");
            // Tab to Frequency (index 4)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "14225");
            // Tab to Comments (index 5)
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "test comment");

            state.clear_fast_fields();
            assert_eq!(state.form().value(THEIR_CALL), "");
            assert_eq!(state.form().value(3), ""); // Their Park in POTA form
            assert_eq!(state.form().value(POTA_FREQUENCY), ""); // Frequency in POTA form
            assert_eq!(state.form().value(5), ""); // Comments in POTA form
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

        #[test]
        fn fd_clears_class_section_and_frequency() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            fill_valid_callsign(&mut state);
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "3A");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "CT");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "14225");

            state.clear_fast_fields();
            assert_eq!(state.form().value(THEIR_CALL), "");
            assert_eq!(state.form().value(CONTEST_THEIR_CLASS), "");
            assert_eq!(state.form().value(CONTEST_THEIR_SECTION), "");
            assert_eq!(state.form().value(CONTEST_FREQUENCY), "");
        }

        #[test]
        fn wfd_clears_class_section_and_frequency() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_wfd_log());
            fill_valid_callsign(&mut state);
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "2H");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "EPA");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "14225");

            state.clear_fast_fields();
            assert_eq!(state.form().value(THEIR_CALL), "");
            assert_eq!(state.form().value(CONTEST_THEIR_CLASS), "");
            assert_eq!(state.form().value(CONTEST_THEIR_SECTION), "");
            assert_eq!(state.form().value(CONTEST_FREQUENCY), "");
        }

        #[test]
        fn fd_does_not_uppercase_comments_field() {
            // Comments is at index 4 in FD form; it should NOT be auto-uppercased.
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            // Tab past callsign, class, section, frequency to comments (index 4)
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "nice signal");
            assert_eq!(
                state.form().value(4),
                "nice signal",
                "FD comments should not be uppercased"
            );
        }

        #[test]
        fn clear_fast_fields_rearms_rst_clear_on_first_input() {
            // After clear_fast_fields the RST flag must be re-armed so the next QSO's
            // operator can replace the default on first keystroke without backspacing.
            let mut state = QsoEntryState::new();
            // Edit RST Sent so it's no longer the default
            state.handle_key(press(KeyCode::Tab)); // → RST Sent
            state.handle_key(press(KeyCode::Char('5')));
            assert_eq!(state.form().value(RST_SENT), "5");

            state.clear_fast_fields();
            // After clear_fast_fields, RST should be restored to default
            assert_eq!(
                state.form().value(RST_SENT),
                "59",
                "clear_fast_fields must restore RST Sent default"
            );
            assert_eq!(
                state.form().value(RST_RCVD),
                "59",
                "clear_fast_fields must restore RST Rcvd default"
            );
        }

        #[test]
        fn clear_fast_fields_rst_is_replaced_on_first_keystroke() {
            // After clear_fast_fields the first keystroke into RST replaces the default.
            let mut state = QsoEntryState::new();
            state.clear_fast_fields();
            state.handle_key(press(KeyCode::Tab)); // → RST Sent
            state.handle_key(press(KeyCode::Char('5')));
            assert_eq!(
                state.form().value(RST_SENT),
                "5",
                "first char after clear_fast_fields must replace default"
            );
        }
    }

    mod recent_qsos {
        use super::*;

        #[test]
        fn set_log_context_populates_recent() {
            let mut state = QsoEntryState::new();
            let mut log = Log::Pota(
                PotaLog::new(
                    "W1AW".to_string(),
                    None,
                    "K-0001".to_string(),
                    "FN31".to_string(),
                )
                .unwrap(),
            );
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
        fn set_log_context_stores_up_to_20() {
            let mut state = QsoEntryState::new();
            let mut log = Log::Pota(
                PotaLog::new(
                    "W1AW".to_string(),
                    None,
                    "K-0001".to_string(),
                    "FN31".to_string(),
                )
                .unwrap(),
            );
            for i in 0..25 {
                log.add_qso(make_qso(&format!("W{i}AW"), Band::M20, Mode::Ssb));
            }

            state.set_log_context(&log);
            assert_eq!(state.recent_qsos().len(), 20);
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
        fn add_recent_qso_caps_at_20() {
            let mut state = QsoEntryState::new();
            for i in 0..25 {
                state.add_recent_qso(make_qso(&format!("W{i}AW"), Band::M20, Mode::Ssb));
            }
            assert_eq!(state.recent_qsos().len(), 20);
            // Newest (last added) is first
            assert_eq!(state.recent_qsos()[0].their_call, "W24AW");
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

        #[test]
        fn alt_x_navigates_to_export() {
            let mut state = QsoEntryState::new();
            let action = state.handle_key(alt_press(KeyCode::Char('x')));
            assert_eq!(action, Action::Navigate(Screen::Export));
        }

        #[test]
        fn alt_e_navigates_to_qso_list() {
            let mut state = QsoEntryState::new();
            let action = state.handle_key(alt_press(KeyCode::Char('e')));
            assert_eq!(action, Action::Navigate(Screen::QsoList));
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
            let log = Log::Pota(
                PotaLog::new(
                    "W1AW".to_string(),
                    None,
                    "K-0001".to_string(),
                    "FN31".to_string(),
                )
                .unwrap(),
            );
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

    mod editing {
        use super::*;

        fn make_test_qso() -> Qso {
            Qso::new(
                "W3ABC".to_string(),
                "57".to_string(),
                "55".to_string(),
                Band::M40,
                Mode::Cw,
                Utc.with_ymd_and_hms(2026, 1, 10, 12, 0, 0).unwrap(),
                "test comment".to_string(),
                Some("K-5678".to_string()),
                None,
                None,
            )
            .unwrap()
        }

        #[test]
        fn start_editing_populates_form() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            let qso = make_test_qso();
            state.start_editing(2, &qso);

            assert_eq!(state.form().value(THEIR_CALL), "W3ABC");
            assert_eq!(state.form().value(RST_SENT), "57");
            assert_eq!(state.form().value(RST_RCVD), "55");
            assert_eq!(state.form().value(3), "K-5678"); // Their Park in POTA form
            assert_eq!(state.form().value(POTA_FREQUENCY), ""); // no frequency on test QSO
            assert_eq!(state.form().value(5), "test comment"); // Comments in POTA form
            assert_eq!(state.band(), Band::M40);
            assert_eq!(state.mode(), Mode::Cw);
            assert_eq!(state.form().focus(), THEIR_CALL);
        }

        #[test]
        fn start_editing_without_park() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            let qso = make_qso("W3ABC", Band::M20, Mode::Ssb);
            state.start_editing(0, &qso);
            assert_eq!(state.form().value(3), ""); // Their Park empty in POTA form
        }

        #[test]
        fn is_editing_returns_correct_value() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            assert!(!state.is_editing());
            state.start_editing(0, &make_test_qso());
            assert!(state.is_editing());
        }

        #[test]
        fn submit_in_edit_mode_returns_update_qso() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            let qso = make_test_qso();
            let original_ts = qso.timestamp;
            state.start_editing(2, &qso);

            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::UpdateQso(idx, updated) => {
                    assert_eq!(idx, 2);
                    assert_eq!(updated.their_call, "W3ABC");
                    assert_eq!(updated.timestamp, original_ts);
                }
                other => panic!("expected UpdateQso, got {other:?}"),
            }
        }

        #[test]
        fn esc_in_edit_mode_navigates_to_qso_list() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            state.start_editing(0, &make_test_qso());
            let action = state.handle_key(press(KeyCode::Esc));
            assert_eq!(action, Action::Navigate(Screen::QsoList));
            assert!(!state.is_editing());
        }

        #[test]
        fn reset_clears_editing_state() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            state.start_editing(0, &make_test_qso());
            state.reset();
            assert!(!state.is_editing());
        }

        #[test]
        fn clear_fast_fields_clears_editing_state() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            state.start_editing(0, &make_test_qso());
            state.clear_fast_fields();
            assert!(!state.is_editing());
        }

        #[test]
        fn start_editing_clears_errors() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            state.handle_key(press(KeyCode::Enter)); // trigger validation errors
            assert!(state.form().has_errors());
            state.set_error("some error".into());

            state.start_editing(0, &make_test_qso());
            assert!(!state.form().has_errors());
            assert_eq!(state.error(), None);
        }

        #[test]
        fn start_editing_disarms_rst_clear_on_first_input() {
            // start_editing loads a real QSO's RST via set_value, which must disarm
            // the clear_on_first_input flag armed by set_default at form init. If it
            // doesn't, the first keystroke would silently wipe the loaded RST value.
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            let qso = make_test_qso(); // rst_sent="57", rst_rcvd="55"
            state.start_editing(0, &qso);

            // Tab to RST Sent and type a character — should append, not replace
            state.handle_key(press(KeyCode::Tab)); // → RST_SENT (index 1)
            state.handle_key(press(KeyCode::Char('8')));
            assert_eq!(state.form().value(RST_SENT), "578");
        }

        #[test]
        fn start_editing_fd_populates_class_section_and_frequency() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            let qso = Qso::new(
                "W3ABC".to_string(),
                "59".to_string(),
                "59".to_string(),
                Band::M20,
                Mode::Ssb,
                Utc.with_ymd_and_hms(2026, 1, 10, 12, 0, 0).unwrap(),
                String::new(),
                None,
                Some("3A CT".to_string()),
                Some(14225),
            )
            .unwrap();
            state.start_editing(0, &qso);

            assert_eq!(state.form().value(CONTEST_THEIR_CLASS), "3A");
            assert_eq!(state.form().value(CONTEST_THEIR_SECTION), "CT");
            assert_eq!(state.form().value(CONTEST_FREQUENCY), "14225");
        }

        #[test]
        fn start_editing_wfd_populates_class_section_and_frequency() {
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_wfd_log());
            let qso = Qso::new(
                "W3ABC".to_string(),
                "59".to_string(),
                "59".to_string(),
                Band::M20,
                Mode::Ssb,
                Utc.with_ymd_and_hms(2026, 1, 10, 12, 0, 0).unwrap(),
                String::new(),
                None,
                Some("2H EPA".to_string()),
                Some(14225),
            )
            .unwrap();
            state.start_editing(0, &qso);

            assert_eq!(state.form().value(CONTEST_THEIR_CLASS), "2H");
            assert_eq!(state.form().value(CONTEST_THEIR_SECTION), "EPA");
            assert_eq!(state.form().value(CONTEST_FREQUENCY), "14225");
        }

        #[test]
        fn start_editing_fd_exchange_without_space_puts_all_in_class() {
            // Handles corrupt/legacy data that has no space in exchange_rcvd.
            let mut state = QsoEntryState::new();
            state.set_log_context(&make_fd_log());
            let mut qso = make_qso("W3ABC", Band::M20, Mode::Ssb);
            qso.exchange_rcvd = Some("3A".to_string()); // no space — malformed
            state.start_editing(0, &qso);

            assert_eq!(state.form().value(CONTEST_THEIR_CLASS), "3A");
            assert_eq!(state.form().value(CONTEST_THEIR_SECTION), "");
        }

        #[test]
        fn editing_qso_with_stored_lowercase_park_normalises_on_resubmit() {
            // Simulates a pre-fix stored QSO whose their_park was saved in lowercase.
            // start_editing sets the form value directly (bypassing handle_char's auto-uppercase),
            // so normalize_park_ref in submit() is the only safeguard.
            let mut base = make_qso("W3ABC", Band::M20, Mode::Ssb);
            base.their_park = Some("k-1234".to_string()); // bypass Qso::new validation

            let mut state = QsoEntryState::new();
            state.set_log_context(&make_pota_log());
            state.start_editing(0, &base);

            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::UpdateQso(_, updated) => {
                    assert_eq!(updated.their_park, Some("K-1234".to_string()));
                }
                other => panic!("expected UpdateQso, got {other:?}"),
            }
        }
    }

    mod rendering {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        use super::*;

        use crate::tui::test_utils::buffer_to_string;

        fn render_qso_entry_terminal(
            state: &QsoEntryState,
            log: Option<&Log>,
            width: u16,
            height: u16,
        ) -> Terminal<TestBackend> {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| {
                    draw_qso_entry(state, log, frame, frame.area());
                })
                .unwrap();
            terminal
        }

        fn render_qso_entry(
            state: &QsoEntryState,
            log: Option<&Log>,
            width: u16,
            height: u16,
        ) -> String {
            buffer_to_string(
                render_qso_entry_terminal(state, log, width, height)
                    .backend()
                    .buffer(),
            )
        }

        fn make_log() -> Log {
            Log::Pota(
                PotaLog::new(
                    "W1AW".to_string(),
                    None,
                    "K-0001".to_string(),
                    "FN31".to_string(),
                )
                .unwrap(),
            )
        }

        #[test]
        fn renders_title_and_form_fields() {
            let state = QsoEntryState::new();
            let output = render_qso_entry(&state, None, 80, 30);
            assert!(output.contains("QSO Entry"), "should show title");
            assert!(
                output.contains("Their Callsign"),
                "should show callsign field"
            );
            assert!(output.contains("RST Sent"), "should show RST Sent field");
            assert!(output.contains("RST Rcvd"), "should show RST Rcvd field");
        }

        #[test]
        fn renders_header_with_log_context() {
            let state = QsoEntryState::new();
            let log = make_log();
            let output = render_qso_entry(&state, Some(&log), 80, 30);
            assert!(
                output.contains("W1AW"),
                "should show station callsign in header"
            );
            assert!(output.contains("K-0001"), "should show park ref in header");
            assert!(
                output.contains("Band:"),
                "should show band indicator in header"
            );
            assert!(
                output.contains("Mode:"),
                "should show mode indicator in header"
            );
        }

        #[test]
        fn renders_activation_progress() {
            let state = QsoEntryState::new();
            let log = make_log();
            let output = render_qso_entry(&state, Some(&log), 80, 30);
            assert!(
                output.contains("QSOs today: 0 / 10"),
                "should show activation progress"
            );
            assert!(output.contains("needed"), "should show needed count");
        }

        #[test]
        fn renders_activated_status() {
            let state = QsoEntryState::new();
            let mut log = make_log();
            for i in 0..10 {
                let mut qso = make_qso(&format!("W{i}AW"), Band::M20, Mode::Ssb);
                qso.timestamp = Utc::now();
                log.add_qso(qso);
            }
            let output = render_qso_entry(&state, Some(&log), 80, 30);
            assert!(
                output.contains("Activated!"),
                "should show activated status"
            );
        }

        #[test]
        fn renders_recent_qsos() {
            let mut state = QsoEntryState::new();
            state.add_recent_qso(make_qso("W3ABC", Band::M20, Mode::Ssb));
            state.add_recent_qso(make_qso("KD9XYZ", Band::M40, Mode::Cw));
            let output = render_qso_entry(&state, None, 80, 30);
            assert!(output.contains("Recent QSOs"), "should show recent section");
            assert!(output.contains("W3ABC"), "should show first recent QSO");
            assert!(output.contains("KD9XYZ"), "should show second recent QSO");
        }

        #[test]
        fn renders_error_message() {
            let mut state = QsoEntryState::new();
            state.set_error("save failed".into());
            let output = render_qso_entry(&state, None, 80, 30);
            assert!(output.contains("save failed"), "should show error message");
        }

        #[test]
        fn renders_footer_keybindings() {
            let state = QsoEntryState::new();
            let output = render_qso_entry(&state, None, 120, 30);
            assert!(
                output.contains("Alt+b/m"),
                "should show band/mode keybindings"
            );
            assert!(
                output.contains("Alt+e: edit"),
                "should show edit keybinding"
            );
            assert!(
                output.contains("Enter: log"),
                "should show submit keybinding"
            );
        }

        #[test]
        fn renders_their_park_in_recent_pota() {
            let log = make_log();
            let mut state = QsoEntryState::new();
            state.set_log_context(&log);
            let qso = Qso::new(
                "W3ABC".to_string(),
                "59".to_string(),
                "59".to_string(),
                Band::M20,
                Mode::Ssb,
                Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
                String::new(),
                Some("K-5678".to_string()),
                None,
                None,
            )
            .unwrap();
            state.add_recent_qso(qso);
            let output = render_qso_entry(&state, Some(&log), 80, 30);
            assert!(output.contains("K-5678"), "should show park reference");
        }

        #[test]
        fn renders_pota_park_and_freq_both_visible() {
            // Both park and freq have dedicated columns — neither displaces the other.
            let log = make_log();
            let mut state = QsoEntryState::new();
            state.set_log_context(&log);
            let qso = Qso::new(
                "W3ABC".to_string(),
                "59".to_string(),
                "59".to_string(),
                Band::M20,
                Mode::Ssb,
                Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
                String::new(),
                Some("K-5678".to_string()),
                None,
                Some(14_225),
            )
            .unwrap();
            state.add_recent_qso(qso);
            let output = render_qso_entry(&state, Some(&log), 80, 30);
            assert!(output.contains("K-5678"), "park should appear");
            assert!(
                output.contains("14225"),
                "freq should appear alongside park"
            );
        }

        #[test]
        fn renders_frequency_in_recent_general() {
            let mut state = QsoEntryState::new();
            let qso = Qso::new(
                "W3ABC".to_string(),
                "59".to_string(),
                "59".to_string(),
                Band::M20,
                Mode::Ssb,
                Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
                String::new(),
                None,
                None,
                Some(14_225),
            )
            .unwrap();
            state.add_recent_qso(qso);
            let output = render_qso_entry(&state, None, 80, 30);
            assert!(
                output.contains("14225"),
                "should show frequency in general recent QSOs"
            );
        }

        #[test]
        fn renders_exchange_in_recent_fd() {
            let mut state = QsoEntryState::new();
            let log = make_fd_log();
            state.set_log_context(&log);
            let qso = Qso::new(
                "W3ABC".to_string(),
                "59".to_string(),
                "59".to_string(),
                Band::M20,
                Mode::Ssb,
                Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
                String::new(),
                None,
                Some("3A CT".to_string()),
                None,
            )
            .unwrap();
            state.add_recent_qso(qso);
            let output = render_qso_entry(&state, Some(&log), 80, 30);
            assert!(
                output.contains("3A CT"),
                "should show exchange in FD recent QSOs"
            );
        }

        #[test]
        fn renders_exchange_in_recent_wfd() {
            let mut state = QsoEntryState::new();
            let log = make_wfd_log();
            state.set_log_context(&log);
            let qso = Qso::new(
                "W3ABC".to_string(),
                "59".to_string(),
                "59".to_string(),
                Band::M20,
                Mode::Ssb,
                Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
                String::new(),
                None,
                Some("2H EPA".to_string()),
                None,
            )
            .unwrap();
            state.add_recent_qso(qso);
            let output = render_qso_entry(&state, Some(&log), 80, 30);
            assert!(
                output.contains("2H EPA"),
                "should show exchange in WFD recent QSOs"
            );
        }

        #[test]
        fn renders_without_log_context() {
            let state = QsoEntryState::new();
            let output = render_qso_entry(&state, None, 80, 30);
            // Should render without crashing, just no header info
            assert!(output.contains("QSO Entry"), "should still show title");
            assert!(
                !output.contains("Band:"),
                "should not show band without log"
            );
        }

        #[test]
        fn renders_dash_for_non_pota_log() {
            let state = QsoEntryState::new();
            let log = make_general_log();
            let output = render_qso_entry(&state, Some(&log), 80, 30);
            assert!(
                output.contains("W1AW @ -"),
                "non-POTA logs show dash for park"
            );
        }

        fn make_fd_log() -> Log {
            use crate::model::{FdClass, FdPowerCategory, FieldDayLog};
            Log::FieldDay(
                FieldDayLog::new(
                    "W1AW".to_string(),
                    None,
                    1,
                    FdClass::B,
                    "EPA".to_string(),
                    FdPowerCategory::Low,
                    "FN31".to_string(),
                )
                .unwrap(),
            )
        }

        fn make_wfd_log() -> Log {
            use crate::model::{WfdClass, WfdLog};
            Log::WinterFieldDay(
                WfdLog::new(
                    "W1AW".to_string(),
                    None,
                    1,
                    WfdClass::H,
                    "EPA".to_string(),
                    "FN31".to_string(),
                )
                .unwrap(),
            )
        }

        fn make_general_log() -> Log {
            use crate::model::GeneralLog;
            Log::General(GeneralLog::new("W1AW".to_string(), None, "FN31".to_string()).unwrap())
        }

        fn render_with_log_type(log: &Log) -> String {
            let mut state = QsoEntryState::new();
            state.set_log_context(log);
            render_qso_entry(&state, Some(log), 80, 25)
        }

        #[test]
        fn renders_field_labels_row1() {
            let state = QsoEntryState::new();
            let output = render_qso_entry(&state, None, 80, 25);
            assert!(output.contains("Their Callsign"), "row1: callsign label");
            assert!(output.contains("RST Sent"), "row1: RST sent label");
            assert!(output.contains("RST Rcvd"), "row1: RST rcvd label");
        }

        #[test]
        fn renders_general_log_form() {
            let log = make_general_log();
            let output = render_with_log_type(&log);
            assert!(output.contains("Their Callsign"), "should show callsign");
            assert!(!output.contains("Their Park"), "general has no Their Park");
            assert!(
                !output.contains("Their Exchange"),
                "general has no Their Exchange"
            );
        }

        #[test]
        fn renders_pota_log_form() {
            let log = make_log();
            let output = render_with_log_type(&log);
            assert!(output.contains("Their Park"), "POTA should show Their Park");
            assert!(
                !output.contains("Their Exchange"),
                "POTA should not show Their Exchange"
            );
        }

        #[test]
        fn renders_fd_log_form() {
            let log = make_fd_log();
            let output = render_with_log_type(&log);
            assert!(
                output.contains("Their Class"),
                "FD should show Their Class field"
            );
            assert!(
                output.contains("Their Section"),
                "FD should show Their Section field"
            );
            assert!(output.contains('*'), "required field should show asterisk");
            assert!(
                !output.contains("Their Park"),
                "FD should not show Their Park"
            );
            assert!(output.contains("Frequency"), "FD should show Frequency");
            assert!(!output.contains("RST"), "FD should not show RST fields");
        }

        #[test]
        fn renders_wfd_log_form() {
            let log = make_wfd_log();
            let output = render_with_log_type(&log);
            assert!(
                output.contains("Their Class"),
                "WFD should show Their Class field"
            );
            assert!(
                output.contains("Their Section"),
                "WFD should show Their Section field"
            );
            assert!(output.contains("Frequency"), "WFD should show Frequency");
            assert!(!output.contains("RST"), "WFD should not show RST fields");
        }

        #[test]
        fn renders_default_form_without_log_context() {
            let state = QsoEntryState::new();
            let output = render_qso_entry(&state, None, 80, 25);
            assert!(!output.contains("Their Park"), "default has no Their Park");
            assert!(
                !output.contains("Their Exchange"),
                "default has no Their Exchange"
            );
        }

        // Snapshot tests for the full QSO entry screen (draw_qso_entry) at 80×24.
        //
        // These capture the complete layout — title, header, form fields, footer — so
        // that Phase 5.7 RST UX changes don't introduce silent regressions.
        mod snap_full_screen {
            use insta::assert_snapshot;

            use super::*;

            fn render_full(state: &QsoEntryState, log: Option<&Log>) -> Terminal<TestBackend> {
                render_qso_entry_terminal(state, log, 80, 24)
            }

            #[test]
            fn snap_general_default() {
                let mut state = QsoEntryState::new();
                let log = make_general_log();
                state.set_log_context(&log);
                let terminal = render_full(&state, Some(&log));
                assert_snapshot!(terminal.backend());
            }

            #[test]
            fn snap_pota_default() {
                let mut state = QsoEntryState::new();
                let log = make_log();
                state.set_log_context(&log);
                let terminal = render_full(&state, Some(&log));
                assert_snapshot!(terminal.backend());
            }

            #[test]
            fn snap_fd_default() {
                let mut state = QsoEntryState::new();
                let log = make_fd_log();
                state.set_log_context(&log);
                let terminal = render_full(&state, Some(&log));
                assert_snapshot!(terminal.backend());
            }

            #[test]
            fn snap_wfd_default() {
                let mut state = QsoEntryState::new();
                let log = make_wfd_log();
                state.set_log_context(&log);
                let terminal = render_full(&state, Some(&log));
                assert_snapshot!(terminal.backend());
            }

            #[test]
            fn snap_editing_mode() {
                let mut state = QsoEntryState::new();
                let log = make_log();
                state.set_log_context(&log);
                let qso = make_qso("W3ABC", Band::M20, Mode::Ssb);
                state.start_editing(0, &qso);
                let terminal = render_full(&state, Some(&log));
                assert_snapshot!(terminal.backend());
            }

            #[test]
            fn snap_with_error() {
                let mut state = QsoEntryState::new();
                let log = make_log();
                state.set_log_context(&log);
                state.set_error("duplicate contact: W3ABC already logged on 20M SSB".into());
                let terminal = render_full(&state, Some(&log));
                assert_snapshot!(terminal.backend());
            }
        }

        // Snapshot and targeted tests for draw_recent_qsos column layout.
        //
        // Snapshot tests (assert_snapshot!) capture the full rendered layout so
        // column-position bugs are immediately visible in the .snap diff.
        // Targeted tests (.contains()) assert semantic presence/absence.
        //
        // See ADR-0005 for the ongoing decision between these approaches.
        mod recent_qsos_rendering {
            use insta::assert_snapshot;

            use super::*;

            /// Render only `draw_recent_qsos` into a terminal of the given size.
            ///
            /// Returns the `Terminal` so callers can use `terminal.backend()` for
            /// `assert_snapshot!` (Display) or `terminal.backend().buffer()` for
            /// `buffer_to_string`.
            fn render_recent(
                state: &QsoEntryState,
                width: u16,
                height: u16,
            ) -> Terminal<TestBackend> {
                let backend = TestBackend::new(width, height);
                let mut terminal = Terminal::new(backend).unwrap();
                terminal
                    .draw(|frame| {
                        draw_recent_qsos(state, frame, frame.area());
                    })
                    .unwrap();
                terminal
            }

            fn pota_qso_with_park(call: &str, park: &str, freq: Option<u32>) -> Qso {
                Qso::new(
                    call.to_string(),
                    "59".to_string(),
                    "59".to_string(),
                    Band::M20,
                    Mode::Ssb,
                    Utc.with_ymd_and_hms(2026, 3, 10, 14, 30, 0).unwrap(),
                    String::new(),
                    Some(park.to_string()),
                    None,
                    freq,
                )
                .unwrap()
            }

            fn pota_qso_no_park(call: &str, freq: Option<u32>) -> Qso {
                Qso::new(
                    call.to_string(),
                    "59".to_string(),
                    "59".to_string(),
                    Band::M20,
                    Mode::Ssb,
                    Utc.with_ymd_and_hms(2026, 3, 10, 14, 30, 0).unwrap(),
                    String::new(),
                    None,
                    None,
                    freq,
                )
                .unwrap()
            }

            fn general_qso(call: &str, freq: Option<u32>) -> Qso {
                Qso::new(
                    call.to_string(),
                    "59".to_string(),
                    "59".to_string(),
                    Band::M20,
                    Mode::Ssb,
                    Utc.with_ymd_and_hms(2026, 3, 10, 14, 30, 0).unwrap(),
                    String::new(),
                    None,
                    None,
                    freq,
                )
                .unwrap()
            }

            fn fd_qso(call: &str, exchange: &str, freq: Option<u32>) -> Qso {
                Qso::new(
                    call.to_string(),
                    "59".to_string(),
                    "59".to_string(),
                    Band::M20,
                    Mode::Ssb,
                    Utc.with_ymd_and_hms(2026, 3, 10, 14, 30, 0).unwrap(),
                    String::new(),
                    None,
                    Some(exchange.to_string()),
                    freq,
                )
                .unwrap()
            }

            // --- Snapshot tests: verify full column layout ---

            #[test]
            fn snap_general_with_freq() {
                let mut state = QsoEntryState::new();
                state.add_recent_qso(general_qso("W3ABC", Some(14_225)));
                assert_snapshot!(render_recent(&state, 80, 4).backend());
            }

            #[test]
            fn snap_general_no_freq() {
                let mut state = QsoEntryState::new();
                state.add_recent_qso(general_qso("W3ABC", None));
                assert_snapshot!(render_recent(&state, 80, 4).backend());
            }

            #[test]
            fn snap_pota_park_and_freq() {
                let mut state = QsoEntryState::new();
                let log = Log::Pota(
                    PotaLog::new(
                        "W1AW".to_string(),
                        None,
                        "K-0001".to_string(),
                        "FN31".to_string(),
                    )
                    .unwrap(),
                );
                state.set_log_context(&log);
                state.add_recent_qso(pota_qso_with_park("W3ABC", "K-5678", Some(14_225)));
                assert_snapshot!(render_recent(&state, 80, 4).backend());
            }

            #[test]
            fn snap_pota_park_no_freq() {
                let mut state = QsoEntryState::new();
                let log = Log::Pota(
                    PotaLog::new(
                        "W1AW".to_string(),
                        None,
                        "K-0001".to_string(),
                        "FN31".to_string(),
                    )
                    .unwrap(),
                );
                state.set_log_context(&log);
                state.add_recent_qso(pota_qso_with_park("W3ABC", "K-5678", None));
                assert_snapshot!(render_recent(&state, 80, 4).backend());
            }

            /// Regression test for issue #37: POTA contact with no park and a frequency
            /// must show the frequency in the freq column, not the park column.
            #[test]
            fn snap_pota_freq_no_park() {
                let mut state = QsoEntryState::new();
                let log = Log::Pota(
                    PotaLog::new(
                        "W1AW".to_string(),
                        None,
                        "K-0001".to_string(),
                        "FN31".to_string(),
                    )
                    .unwrap(),
                );
                state.set_log_context(&log);
                state.add_recent_qso(pota_qso_no_park("W3ABC", Some(14_225)));
                assert_snapshot!(render_recent(&state, 80, 4).backend());
            }

            #[test]
            fn snap_pota_neither_park_nor_freq() {
                let mut state = QsoEntryState::new();
                let log = Log::Pota(
                    PotaLog::new(
                        "W1AW".to_string(),
                        None,
                        "K-0001".to_string(),
                        "FN31".to_string(),
                    )
                    .unwrap(),
                );
                state.set_log_context(&log);
                state.add_recent_qso(pota_qso_no_park("W3ABC", None));
                assert_snapshot!(render_recent(&state, 80, 4).backend());
            }

            #[test]
            fn snap_fd_exchange_and_freq() {
                let mut state = QsoEntryState::new();
                let log = make_fd_log();
                state.set_log_context(&log);
                state.add_recent_qso(fd_qso("W3ABC", "3A CT", Some(14_225)));
                assert_snapshot!(render_recent(&state, 80, 4).backend());
            }

            #[test]
            fn snap_fd_exchange_no_freq() {
                let mut state = QsoEntryState::new();
                let log = make_fd_log();
                state.set_log_context(&log);
                state.add_recent_qso(fd_qso("W3ABC", "3A CT", None));
                assert_snapshot!(render_recent(&state, 80, 4).backend());
            }

            #[test]
            fn snap_wfd_exchange_and_freq() {
                let mut state = QsoEntryState::new();
                let log = make_wfd_log();
                state.set_log_context(&log);
                state.add_recent_qso(fd_qso("W3ABC", "2H EPA", Some(14_225)));
                assert_snapshot!(render_recent(&state, 80, 4).backend());
            }

            #[test]
            fn snap_wfd_exchange_no_freq() {
                let mut state = QsoEntryState::new();
                let log = make_wfd_log();
                state.set_log_context(&log);
                state.add_recent_qso(fd_qso("W3ABC", "2H EPA", None));
                assert_snapshot!(render_recent(&state, 80, 4).backend());
            }

            /// Issue #40: row count adapts to terminal height.
            /// height=10 gives 9 inner rows; 10 QSOs fills all 9 visibly.
            #[test]
            fn snap_general_tall_terminal_shows_more_rows() {
                let mut state = QsoEntryState::new();
                for i in 0..10 {
                    state.add_recent_qso(general_qso(
                        &format!("W{i}AW"),
                        Some(14_000 + i as u32 * 25),
                    ));
                }
                assert_snapshot!(render_recent(&state, 80, 10).backend());
            }

            // --- Targeted semantic tests ---

            #[test]
            fn pota_freq_column_independent_of_park() {
                // When park is present, freq should also be visible (separate column).
                let mut state = QsoEntryState::new();
                let log = Log::Pota(
                    PotaLog::new(
                        "W1AW".to_string(),
                        None,
                        "K-0001".to_string(),
                        "FN31".to_string(),
                    )
                    .unwrap(),
                );
                state.set_log_context(&log);
                state.add_recent_qso(pota_qso_with_park("W3ABC", "K-5678", Some(14_225)));
                let output = buffer_to_string(render_recent(&state, 80, 4).backend().buffer());
                assert!(output.contains("K-5678"), "park should appear");
                assert!(
                    output.contains("14225"),
                    "freq should also appear alongside park"
                );
            }

            #[test]
            fn pota_no_park_freq_still_visible() {
                // Regression for #37: freq must appear even when park is absent.
                let mut state = QsoEntryState::new();
                let log = Log::Pota(
                    PotaLog::new(
                        "W1AW".to_string(),
                        None,
                        "K-0001".to_string(),
                        "FN31".to_string(),
                    )
                    .unwrap(),
                );
                state.set_log_context(&log);
                state.add_recent_qso(pota_qso_no_park("W3ABC", Some(14_225)));
                let output = buffer_to_string(render_recent(&state, 80, 4).backend().buffer());
                assert!(
                    output.contains("14225"),
                    "freq should appear in freq column"
                );
                assert!(!output.contains("K-"), "no park ref should appear");
            }

            #[test]
            fn row_count_adapts_to_height() {
                // Issue #40: row count is driven by Rect height, not a hard-coded constant.
                // height=4 → 3 inner rows (Borders::TOP costs 1); height=10 → 9 inner rows.
                let mut state = QsoEntryState::new();
                for i in 0..8 {
                    state.add_recent_qso(general_qso(&format!("W{i}AW"), None));
                }
                let short = buffer_to_string(render_recent(&state, 80, 4).backend().buffer());
                let tall = buffer_to_string(render_recent(&state, 80, 10).backend().buffer());
                let short_rows = (0..8)
                    .filter(|i| short.contains(&format!("W{i}AW")))
                    .count();
                let tall_rows = (0..8).filter(|i| tall.contains(&format!("W{i}AW"))).count();
                assert_eq!(short_rows, 3, "height=4 should show exactly 3 rows");
                assert_eq!(tall_rows, 8, "height=10 should show all 8 QSOs");
            }
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
