//! Log creation screen — form for entering new log session details.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::model::{
    FdPowerCategory, FieldDayLog, GeneralLog, Log, PotaLog, WfdLog, normalize_grid_square,
    parse_fd_class, parse_wfd_class, validate_callsign, validate_grid_square, validate_park_ref,
    validate_section, validate_tx_count,
};
use crate::tui::action::Action;
use crate::tui::app::Screen;
use crate::tui::widgets::form::{Form, FormField, draw_form};

// --- Field index constants ---

/// Field index for station callsign (all types).
const CALLSIGN: usize = 0;
/// Field index for operator callsign (all types).
const OPERATOR: usize = 1;

// General log fields
/// Field index for grid square in the General log form.
const GENERAL_GRID: usize = 2;

// POTA log fields
/// Field index for POTA park reference.
const POTA_PARK_REF: usize = 2;
/// Field index for grid square in the POTA log form.
const POTA_GRID: usize = 3;

// Contest (FD / WFD) log fields
/// Field index for grid square in the contest log form.
const CONTEST_GRID: usize = 2;
/// Field index for transmitter count in the contest log form.
const CONTEST_TX_COUNT: usize = 3;
/// Field index for operating class in the contest log form.
const CONTEST_CLASS: usize = 4;
/// Field index for ARRL section in the contest log form.
const CONTEST_SECTION: usize = 5;

// --- Local enums ---

/// Which log type is selected in the type-selector row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum LogType {
    #[default]
    General,
    Pota,
    FieldDay,
    WinterFieldDay,
}

impl LogType {
    fn next(self) -> Self {
        match self {
            Self::General => Self::Pota,
            Self::Pota => Self::FieldDay,
            Self::FieldDay => Self::WinterFieldDay,
            Self::WinterFieldDay => Self::General,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::General => Self::WinterFieldDay,
            Self::Pota => Self::General,
            Self::FieldDay => Self::Pota,
            Self::WinterFieldDay => Self::FieldDay,
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Pota => "POTA",
            Self::FieldDay => "Field Day",
            Self::WinterFieldDay => "Winter FD",
        }
    }
}

/// Which UI area currently holds keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum FocusArea {
    #[default]
    TypeSelector,
    Fields,
}

// --- State ---

/// State for the log creation screen.
#[derive(Debug, Clone)]
pub struct LogCreateState {
    log_type: LogType,
    focus_area: FocusArea,
    form: Form,
    general_error: Option<String>,
    // Value buffers — persisted across type switches so the user's typing is preserved
    callsign_buf: String,
    operator_buf: String,
    grid_square_buf: String,
    park_ref_buf: String,
    tx_count_buf: String,
    class_buf: String,
    section_buf: String,
}

impl Default for LogCreateState {
    fn default() -> Self {
        Self::new()
    }
}

impl LogCreateState {
    /// Creates a new log creation form in its initial state (General type, TypeSelector focused).
    pub fn new() -> Self {
        Self {
            log_type: LogType::General,
            focus_area: FocusArea::TypeSelector,
            form: Form::new(vec![
                FormField::new("Station Callsign", true),
                FormField::new("Operator", false),
                FormField::new("Grid Square (e.g. FN31)", true),
            ]),
            general_error: None,
            callsign_buf: String::new(),
            operator_buf: String::new(),
            grid_square_buf: String::new(),
            park_ref_buf: String::new(),
            tx_count_buf: String::new(),
            class_buf: String::new(),
            section_buf: String::new(),
        }
    }

    /// Handles a key event, returning an [`Action`] for the app to apply.
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Tab => {
                match self.focus_area {
                    FocusArea::TypeSelector => {
                        self.focus_area = FocusArea::Fields;
                        self.form.set_focus(0);
                    }
                    FocusArea::Fields => {
                        if self.form.focus() == self.form.fields().len().saturating_sub(1) {
                            self.focus_area = FocusArea::TypeSelector;
                        } else {
                            self.form.focus_next();
                        }
                    }
                }
                Action::None
            }
            KeyCode::BackTab => {
                match self.focus_area {
                    FocusArea::TypeSelector => {
                        self.focus_area = FocusArea::Fields;
                        self.form
                            .set_focus(self.form.fields().len().saturating_sub(1));
                    }
                    FocusArea::Fields => {
                        if self.form.focus() == 0 {
                            self.focus_area = FocusArea::TypeSelector;
                        } else {
                            self.form.focus_prev();
                        }
                    }
                }
                Action::None
            }
            KeyCode::Left => {
                if self.focus_area == FocusArea::TypeSelector {
                    self.sync_buffers_from_form();
                    self.log_type = self.log_type.prev();
                    let form = self.build_form_for_type();
                    self.form = form;
                }
                Action::None
            }
            KeyCode::Right => {
                if self.focus_area == FocusArea::TypeSelector {
                    self.sync_buffers_from_form();
                    self.log_type = self.log_type.next();
                    let form = self.build_form_for_type();
                    self.form = form;
                }
                Action::None
            }
            KeyCode::Char(ch) => {
                if self.focus_area == FocusArea::Fields {
                    let focus = self.form.focus();
                    let should_uppercase = focus == CALLSIGN
                        || focus == OPERATOR
                        || (self.log_type == LogType::Pota && focus == POTA_PARK_REF)
                        || (matches!(self.log_type, LogType::FieldDay | LogType::WinterFieldDay)
                            && (focus == CONTEST_CLASS || focus == CONTEST_SECTION));
                    let ch = if should_uppercase {
                        ch.to_ascii_uppercase()
                    } else {
                        ch
                    };
                    self.form.insert_char(ch);
                }
                Action::None
            }
            KeyCode::Backspace => {
                if self.focus_area == FocusArea::Fields {
                    self.form.delete_char();
                }
                Action::None
            }
            KeyCode::Esc => Action::Navigate(Screen::LogSelect),
            KeyCode::Enter => self.submit(),
            _ => Action::None,
        }
    }

    /// Returns a reference to the form for rendering.
    pub fn form(&self) -> &Form {
        &self.form
    }

    /// Sets a general error message not tied to any specific field.
    ///
    /// Used to display storage-level errors (e.g. duplicate log) inline.
    pub fn set_error(&mut self, msg: String) {
        self.general_error = Some(msg);
    }

    /// Returns the general error message, if any.
    pub fn general_error(&self) -> Option<&str> {
        self.general_error.as_deref()
    }

    /// Resets the form to its initial empty state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Syncs buffer fields from the current form values for the active log type.
    ///
    /// Call this before switching log type to preserve the user's typing.
    fn sync_buffers_from_form(&mut self) {
        self.callsign_buf = self.form.value(CALLSIGN).to_string();
        self.operator_buf = self.form.value(OPERATOR).to_string();
        match self.log_type {
            LogType::General => {
                self.grid_square_buf = self.form.value(GENERAL_GRID).to_string();
            }
            LogType::Pota => {
                self.park_ref_buf = self.form.value(POTA_PARK_REF).to_string();
                self.grid_square_buf = self.form.value(POTA_GRID).to_string();
            }
            LogType::FieldDay | LogType::WinterFieldDay => {
                self.grid_square_buf = self.form.value(CONTEST_GRID).to_string();
                self.tx_count_buf = self.form.value(CONTEST_TX_COUNT).to_string();
                self.class_buf = self.form.value(CONTEST_CLASS).to_string();
                self.section_buf = self.form.value(CONTEST_SECTION).to_string();
            }
        }
    }

    /// Builds a new form for the current log type, pre-populated from buffers.
    fn build_form_for_type(&self) -> Form {
        let mut form = match self.log_type {
            LogType::General => Form::new(vec![
                FormField::new("Station Callsign", true),
                FormField::new("Operator", false),
                FormField::new("Grid Square (e.g. FN31)", true),
            ]),
            LogType::Pota => Form::new(vec![
                FormField::new("Station Callsign", true),
                FormField::new("Operator", false),
                FormField::new("Park Ref (e.g. K-0001)", false),
                FormField::new("Grid Square (e.g. FN31)", true),
            ]),
            LogType::FieldDay => Form::new(vec![
                FormField::new("Station Callsign", true),
                FormField::new("Operator", false),
                FormField::new("Grid Square (e.g. FN31)", true),
                FormField::new("Tx Count", true),
                FormField::new("FD Class (A–F)", true),
                FormField::new("Section", true),
            ]),
            LogType::WinterFieldDay => Form::new(vec![
                FormField::new("Station Callsign", true),
                FormField::new("Operator", false),
                FormField::new("Grid Square (e.g. FN31)", true),
                FormField::new("Tx Count", true),
                FormField::new("WFD Class (H/I/O/M)", true),
                FormField::new("Section", true),
            ]),
        };

        form.set_value(CALLSIGN, &self.callsign_buf);
        form.set_value(OPERATOR, &self.operator_buf);

        match self.log_type {
            LogType::General => {
                form.set_value(GENERAL_GRID, &self.grid_square_buf);
            }
            LogType::Pota => {
                form.set_value(POTA_PARK_REF, &self.park_ref_buf);
                form.set_value(POTA_GRID, &self.grid_square_buf);
            }
            LogType::FieldDay | LogType::WinterFieldDay => {
                form.set_value(CONTEST_GRID, &self.grid_square_buf);
                form.set_value(CONTEST_TX_COUNT, &self.tx_count_buf);
                form.set_value(CONTEST_CLASS, &self.class_buf);
                form.set_value(CONTEST_SECTION, &self.section_buf);
            }
        }

        form
    }

    /// Validates all fields and attempts to create a [`Log`].
    fn submit(&mut self) -> Action {
        self.form.clear_errors();
        self.general_error = None;
        match self.log_type {
            LogType::General => self.submit_general(),
            LogType::Pota => self.submit_pota(),
            LogType::FieldDay => self.submit_field_day(),
            LogType::WinterFieldDay => self.submit_wfd(),
        }
    }

    fn submit_general(&mut self) -> Action {
        let callsign = self.form.value(CALLSIGN).to_string();
        let operator_str = self.form.value(OPERATOR).to_string();
        let operator = (!operator_str.is_empty()).then_some(operator_str);
        let grid_square = normalize_grid_square(self.form.value(GENERAL_GRID));

        if let Err(e) = validate_callsign(&callsign) {
            self.form.set_error(CALLSIGN, e.to_string());
        }
        if let Some(ref op) = operator
            && let Err(e) = validate_callsign(op)
        {
            self.form.set_error(OPERATOR, e.to_string());
        }
        if let Err(e) = validate_grid_square(&grid_square) {
            self.form.set_error(GENERAL_GRID, e.to_string());
        }

        if self.form.has_errors() {
            return Action::None;
        }

        match GeneralLog::new(callsign, operator, grid_square) {
            Ok(log) => Action::CreateLog(Log::General(log)),
            Err(e) => {
                self.form.set_error(CALLSIGN, e.to_string());
                Action::None
            }
        }
    }

    fn submit_pota(&mut self) -> Action {
        let callsign = self.form.value(CALLSIGN).to_string();
        let operator_str = self.form.value(OPERATOR).to_string();
        let operator = (!operator_str.is_empty()).then_some(operator_str);
        // POTA_PARK_REF is auto-uppercased at input time
        let park_ref_str = self.form.value(POTA_PARK_REF).to_string();
        let grid_square = normalize_grid_square(self.form.value(POTA_GRID));

        if let Err(e) = validate_callsign(&callsign) {
            self.form.set_error(CALLSIGN, e.to_string());
        }
        if let Some(ref op) = operator
            && let Err(e) = validate_callsign(op)
        {
            self.form.set_error(OPERATOR, e.to_string());
        }
        if !park_ref_str.is_empty()
            && let Err(e) = validate_park_ref(&park_ref_str)
        {
            self.form.set_error(POTA_PARK_REF, e.to_string());
        }
        if let Err(e) = validate_grid_square(&grid_square) {
            self.form.set_error(POTA_GRID, e.to_string());
        }

        if self.form.has_errors() {
            return Action::None;
        }

        let park_ref = (!park_ref_str.is_empty()).then_some(park_ref_str);
        match PotaLog::new(callsign, operator, park_ref, grid_square) {
            Ok(log) => Action::CreateLog(Log::Pota(log)),
            Err(e) => {
                self.form.set_error(CALLSIGN, e.to_string());
                Action::None
            }
        }
    }

    fn submit_field_day(&mut self) -> Action {
        let callsign = self.form.value(CALLSIGN).to_string();
        let operator_str = self.form.value(OPERATOR).to_string();
        let operator = (!operator_str.is_empty()).then_some(operator_str);
        let grid_square = normalize_grid_square(self.form.value(CONTEST_GRID));
        let tx_count_str = self.form.value(CONTEST_TX_COUNT).to_string();
        // CONTEST_CLASS is auto-uppercased at input time
        let class_str = self.form.value(CONTEST_CLASS).to_string();
        // CONTEST_SECTION is auto-uppercased at input time
        let section = self.form.value(CONTEST_SECTION).to_string();

        if let Err(e) = validate_callsign(&callsign) {
            self.form.set_error(CALLSIGN, e.to_string());
        }
        if let Some(ref op) = operator
            && let Err(e) = validate_callsign(op)
        {
            self.form.set_error(OPERATOR, e.to_string());
        }
        if let Err(e) = validate_grid_square(&grid_square) {
            self.form.set_error(CONTEST_GRID, e.to_string());
        }
        let tx_count = match tx_count_str.parse::<u8>() {
            Ok(n) => {
                if let Err(e) = validate_tx_count(n) {
                    self.form.set_error(CONTEST_TX_COUNT, e.to_string());
                    0
                } else {
                    n
                }
            }
            Err(_) => {
                self.form.set_error(
                    CONTEST_TX_COUNT,
                    "must be a number 1\u{2013}255".to_string(),
                );
                0
            }
        };
        let class_result = parse_fd_class(&class_str);
        if let Err(ref e) = class_result {
            self.form.set_error(CONTEST_CLASS, e.to_string());
        }
        if let Err(e) = validate_section(&section) {
            self.form.set_error(CONTEST_SECTION, e.to_string());
        }

        if self.form.has_errors() {
            return Action::None;
        }

        // Phase 4.3 will add a power category selector; default to Low for now.
        let power = FdPowerCategory::Low;
        let Ok(class) = class_result else {
            return Action::None; // unreachable: class error already set above
        };
        match FieldDayLog::new(
            callsign,
            operator,
            tx_count,
            class,
            section,
            power,
            grid_square,
        ) {
            Ok(log) => Action::CreateLog(Log::FieldDay(log)),
            Err(e) => {
                self.form.set_error(CALLSIGN, e.to_string());
                Action::None
            }
        }
    }

    fn submit_wfd(&mut self) -> Action {
        let callsign = self.form.value(CALLSIGN).to_string();
        let operator_str = self.form.value(OPERATOR).to_string();
        let operator = (!operator_str.is_empty()).then_some(operator_str);
        let grid_square = normalize_grid_square(self.form.value(CONTEST_GRID));
        let tx_count_str = self.form.value(CONTEST_TX_COUNT).to_string();
        // CONTEST_CLASS is auto-uppercased at input time
        let class_str = self.form.value(CONTEST_CLASS).to_string();
        // CONTEST_SECTION is auto-uppercased at input time
        let section = self.form.value(CONTEST_SECTION).to_string();

        if let Err(e) = validate_callsign(&callsign) {
            self.form.set_error(CALLSIGN, e.to_string());
        }
        if let Some(ref op) = operator
            && let Err(e) = validate_callsign(op)
        {
            self.form.set_error(OPERATOR, e.to_string());
        }
        if let Err(e) = validate_grid_square(&grid_square) {
            self.form.set_error(CONTEST_GRID, e.to_string());
        }
        let tx_count = match tx_count_str.parse::<u8>() {
            Ok(n) => {
                if let Err(e) = validate_tx_count(n) {
                    self.form.set_error(CONTEST_TX_COUNT, e.to_string());
                    0
                } else {
                    n
                }
            }
            Err(_) => {
                self.form.set_error(
                    CONTEST_TX_COUNT,
                    "must be a number 1\u{2013}255".to_string(),
                );
                0
            }
        };
        let class_result = parse_wfd_class(&class_str);
        if let Err(ref e) = class_result {
            self.form.set_error(CONTEST_CLASS, e.to_string());
        }
        if let Err(e) = validate_section(&section) {
            self.form.set_error(CONTEST_SECTION, e.to_string());
        }

        if self.form.has_errors() {
            return Action::None;
        }

        let Ok(class) = class_result else {
            return Action::None; // unreachable: class error already set above
        };
        match WfdLog::new(callsign, operator, tx_count, class, section, grid_square) {
            Ok(log) => Action::CreateLog(Log::WinterFieldDay(log)),
            Err(e) => {
                self.form.set_error(CALLSIGN, e.to_string());
                Action::None
            }
        }
    }
}

/// Renders the log creation screen.
#[mutants::skip]
pub fn draw_log_create(state: &LogCreateState, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Create New Log ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [type_row, form_area, error_area, _spacer, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(9),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(inner);

    // Type selector
    let type_text = format!("< {} >", state.log_type.display_name());
    let selector_border_color = if state.focus_area == FocusArea::TypeSelector {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let type_selector = Paragraph::new(type_text).block(
        Block::default()
            .title("Log Type")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(selector_border_color)),
    );
    frame.render_widget(type_selector, type_row);

    draw_form(state.form(), frame, form_area);

    if let Some(err) = state.general_error() {
        let error = Paragraph::new(Line::from(Span::styled(
            err,
            Style::default().fg(Color::Red),
        )));
        frame.render_widget(error, error_area);
    }

    let footer = Paragraph::new(Line::from(
        "Tab/Shift+Tab: next/prev  \u{2190}/\u{2192}: log type  Enter: create  Esc: cancel",
    ))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}

#[cfg(test)]
mod tests {
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

    fn type_string(state: &mut LogCreateState, s: &str) {
        for ch in s.chars() {
            state.handle_key(press(KeyCode::Char(ch)));
        }
    }

    /// Move from TypeSelector (initial) into the first form field.
    fn enter_fields(state: &mut LogCreateState) {
        state.handle_key(press(KeyCode::Tab));
    }

    /// Switch to POTA type (from any TypeSelector position, starting from General).
    fn switch_to_pota(state: &mut LogCreateState) {
        state.handle_key(press(KeyCode::Right));
    }

    /// Switch to Field Day type (from General: Right×2).
    fn switch_to_field_day(state: &mut LogCreateState) {
        state.handle_key(press(KeyCode::Right));
        state.handle_key(press(KeyCode::Right));
    }

    /// Switch to Winter FD type (from General: Right×3).
    fn switch_to_wfd(state: &mut LogCreateState) {
        state.handle_key(press(KeyCode::Right));
        state.handle_key(press(KeyCode::Right));
        state.handle_key(press(KeyCode::Right));
    }

    /// Fill a valid General log form (starts from a fresh state).
    fn fill_valid_general_form(state: &mut LogCreateState) {
        enter_fields(state); // TypeSelector → CALLSIGN
        type_string(state, "W1AW");
        state.handle_key(press(KeyCode::Tab)); // CALLSIGN → OPERATOR (leave empty)
        state.handle_key(press(KeyCode::Tab)); // OPERATOR → GENERAL_GRID
        type_string(state, "FN31");
    }

    /// Fill a valid POTA form (starts from a fresh state).
    fn fill_valid_pota_form(state: &mut LogCreateState) {
        switch_to_pota(state);
        enter_fields(state); // TypeSelector → CALLSIGN
        type_string(state, "W1AW");
        state.handle_key(press(KeyCode::Tab)); // CALLSIGN → OPERATOR (leave empty)
        state.handle_key(press(KeyCode::Tab)); // OPERATOR → POTA_PARK_REF (leave empty)
        state.handle_key(press(KeyCode::Tab)); // POTA_PARK_REF → POTA_GRID
        type_string(state, "FN31");
    }

    /// Fill a valid POTA form with operator (starts from a fresh state).
    fn fill_valid_pota_form_with_operator(state: &mut LogCreateState) {
        switch_to_pota(state);
        enter_fields(state);
        type_string(state, "W1AW");
        state.handle_key(press(KeyCode::Tab));
        type_string(state, "W1AW");
        state.handle_key(press(KeyCode::Tab));
        state.handle_key(press(KeyCode::Tab));
        type_string(state, "FN31");
    }

    /// Fill a valid Field Day form (starts from a fresh state).
    fn fill_valid_fd_form(state: &mut LogCreateState) {
        switch_to_field_day(state);
        enter_fields(state); // TypeSelector → CALLSIGN
        type_string(state, "W1AW");
        state.handle_key(press(KeyCode::Tab)); // → OPERATOR (leave empty)
        state.handle_key(press(KeyCode::Tab)); // → CONTEST_GRID
        type_string(state, "FN31");
        state.handle_key(press(KeyCode::Tab)); // → CONTEST_TX_COUNT
        type_string(state, "3");
        state.handle_key(press(KeyCode::Tab)); // → CONTEST_CLASS
        type_string(state, "B");
        state.handle_key(press(KeyCode::Tab)); // → CONTEST_SECTION
        type_string(state, "EPA");
    }

    /// Fill a valid WFD form (starts from a fresh state).
    fn fill_valid_wfd_form(state: &mut LogCreateState) {
        switch_to_wfd(state);
        enter_fields(state); // TypeSelector → CALLSIGN
        type_string(state, "W1AW");
        state.handle_key(press(KeyCode::Tab)); // → OPERATOR (leave empty)
        state.handle_key(press(KeyCode::Tab)); // → CONTEST_GRID
        type_string(state, "FN31");
        state.handle_key(press(KeyCode::Tab)); // → CONTEST_TX_COUNT
        type_string(state, "1");
        state.handle_key(press(KeyCode::Tab)); // → CONTEST_CLASS
        type_string(state, "H");
        state.handle_key(press(KeyCode::Tab)); // → CONTEST_SECTION
        type_string(state, "EPA");
    }

    mod log_type_selection {
        use super::*;

        #[test]
        fn initial_type_is_general() {
            let state = LogCreateState::new();
            assert_eq!(state.log_type, LogType::General);
        }

        #[test]
        fn right_cycles_to_pota() {
            let mut state = LogCreateState::new();
            state.handle_key(press(KeyCode::Right));
            assert_eq!(state.log_type, LogType::Pota);
        }

        #[test]
        fn right_cycles_general_through_all_types() {
            let mut state = LogCreateState::new();
            state.handle_key(press(KeyCode::Right));
            assert_eq!(state.log_type, LogType::Pota);
            state.handle_key(press(KeyCode::Right));
            assert_eq!(state.log_type, LogType::FieldDay);
            state.handle_key(press(KeyCode::Right));
            assert_eq!(state.log_type, LogType::WinterFieldDay);
            state.handle_key(press(KeyCode::Right));
            assert_eq!(state.log_type, LogType::General);
        }

        #[test]
        fn left_wraps_to_winter_fd() {
            let mut state = LogCreateState::new();
            state.handle_key(press(KeyCode::Left));
            assert_eq!(state.log_type, LogType::WinterFieldDay);
        }

        #[test]
        fn left_right_are_ignored_in_fields_mode() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state);
            state.handle_key(press(KeyCode::Right));
            assert_eq!(state.log_type, LogType::General); // unchanged
            state.handle_key(press(KeyCode::Left));
            assert_eq!(state.log_type, LogType::General); // unchanged
        }

        #[test]
        fn type_switch_rebuilds_form_fields() {
            let mut state = LogCreateState::new();
            assert_eq!(state.form().fields().len(), 3); // General: 3 fields
            switch_to_pota(&mut state);
            assert_eq!(state.form().fields().len(), 4); // POTA: 4 fields
            state.handle_key(press(KeyCode::Right)); // → FieldDay
            assert_eq!(state.form().fields().len(), 6); // FD: 6 fields
        }

        #[test]
        fn type_switch_preserves_callsign_and_operator() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state);
            type_string(&mut state, "W1AW");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "KD9XYZ");

            // Go back to TypeSelector then switch to POTA
            state.handle_key(shift_press(KeyCode::BackTab)); // OPERATOR → CALLSIGN
            state.handle_key(shift_press(KeyCode::BackTab)); // CALLSIGN → TypeSelector
            switch_to_pota(&mut state);

            assert_eq!(state.form().value(CALLSIGN), "W1AW");
            assert_eq!(state.form().value(OPERATOR), "KD9XYZ");
        }

        #[test]
        fn type_switch_preserves_grid_square() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state); // → CALLSIGN
            state.handle_key(press(KeyCode::Tab)); // → OPERATOR
            state.handle_key(press(KeyCode::Tab)); // → GENERAL_GRID
            type_string(&mut state, "FN31");

            // Return to TypeSelector: Tab at last field
            state.handle_key(press(KeyCode::Tab)); // GENERAL_GRID (last) → TypeSelector
            switch_to_pota(&mut state);

            assert_eq!(state.form().value(POTA_GRID), "FN31");
        }
    }

    mod focus_navigation {
        use super::*;

        #[test]
        fn initial_focus_is_type_selector() {
            let state = LogCreateState::new();
            assert_eq!(state.focus_area, FocusArea::TypeSelector);
        }

        #[test]
        fn tab_from_selector_enters_first_field() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state);
            assert_eq!(state.focus_area, FocusArea::Fields);
            assert_eq!(state.form().focus(), CALLSIGN);
        }

        #[test]
        fn backtab_from_selector_enters_last_field() {
            let mut state = LogCreateState::new();
            state.handle_key(shift_press(KeyCode::BackTab));
            assert_eq!(state.focus_area, FocusArea::Fields);
            assert_eq!(state.form().focus(), GENERAL_GRID); // last field of General form
        }

        #[test]
        fn tab_at_last_field_returns_to_selector() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state); // → CALLSIGN
            state.handle_key(press(KeyCode::Tab)); // → OPERATOR
            state.handle_key(press(KeyCode::Tab)); // → GENERAL_GRID (last)
            state.handle_key(press(KeyCode::Tab)); // → TypeSelector
            assert_eq!(state.focus_area, FocusArea::TypeSelector);
        }

        #[test]
        fn backtab_at_first_field_returns_to_selector() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state); // → CALLSIGN (first)
            state.handle_key(shift_press(KeyCode::BackTab)); // → TypeSelector
            assert_eq!(state.focus_area, FocusArea::TypeSelector);
        }

        #[test]
        fn tab_cycles_through_all_general_fields() {
            let mut state = LogCreateState::new();
            assert_eq!(state.focus_area, FocusArea::TypeSelector);
            enter_fields(&mut state);
            assert_eq!(state.form().focus(), CALLSIGN);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), OPERATOR);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), GENERAL_GRID);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.focus_area, FocusArea::TypeSelector);
        }

        #[test]
        fn pota_form_has_four_fields_in_tab_cycle() {
            let mut state = LogCreateState::new();
            switch_to_pota(&mut state);
            enter_fields(&mut state);
            assert_eq!(state.form().focus(), CALLSIGN);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), OPERATOR);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), POTA_PARK_REF);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), POTA_GRID);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.focus_area, FocusArea::TypeSelector);
        }

        #[test]
        fn backtab_cycles_backward_through_fields() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state); // → CALLSIGN
            state.handle_key(press(KeyCode::Tab)); // → OPERATOR
            state.handle_key(shift_press(KeyCode::BackTab)); // → CALLSIGN
            assert_eq!(state.form().focus(), CALLSIGN);
            state.handle_key(shift_press(KeyCode::BackTab)); // → TypeSelector
            assert_eq!(state.focus_area, FocusArea::TypeSelector);
        }
    }

    mod typing {
        use super::*;

        #[test]
        fn chars_ignored_in_type_selector_mode() {
            let mut state = LogCreateState::new();
            state.handle_key(press(KeyCode::Char('W')));
            state.handle_key(press(KeyCode::Char('1')));
            assert_eq!(state.form().value(CALLSIGN), "");
        }

        #[test]
        fn chars_fill_focused_field_after_entering_fields() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state);
            state.handle_key(press(KeyCode::Char('W')));
            state.handle_key(press(KeyCode::Char('1')));
            assert_eq!(state.form().value(CALLSIGN), "W1");
        }

        #[test]
        fn backspace_ignored_in_type_selector_mode() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state);
            type_string(&mut state, "W");
            // BackTab from first field → TypeSelector
            state.handle_key(shift_press(KeyCode::BackTab));
            assert_eq!(state.focus_area, FocusArea::TypeSelector);
            // Backspace in TypeSelector should be a no-op
            state.handle_key(press(KeyCode::Backspace));
            // Re-enter fields and verify 'W' is still there
            enter_fields(&mut state);
            assert_eq!(state.form().value(CALLSIGN), "W");
        }

        #[test]
        fn backspace_deletes_char_in_fields_mode() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state);
            state.handle_key(press(KeyCode::Char('A')));
            state.handle_key(press(KeyCode::Char('B')));
            state.handle_key(press(KeyCode::Backspace));
            assert_eq!(state.form().value(CALLSIGN), "A");
        }

        #[test]
        fn callsign_auto_uppercased() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state);
            for ch in "w3duk".chars() {
                state.handle_key(press(KeyCode::Char(ch)));
            }
            assert_eq!(state.form().value(CALLSIGN), "W3DUK");
        }

        #[test]
        fn operator_auto_uppercased() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state);
            state.handle_key(press(KeyCode::Tab)); // move to operator
            for ch in "w3duk".chars() {
                state.handle_key(press(KeyCode::Char(ch)));
            }
            assert_eq!(state.form().value(OPERATOR), "W3DUK");
        }

        #[test]
        fn pota_park_ref_auto_uppercased() {
            let mut state = LogCreateState::new();
            switch_to_pota(&mut state);
            enter_fields(&mut state);
            state.handle_key(press(KeyCode::Tab)); // operator
            state.handle_key(press(KeyCode::Tab)); // park ref
            for ch in "k-0001".chars() {
                state.handle_key(press(KeyCode::Char(ch)));
            }
            assert_eq!(state.form().value(POTA_PARK_REF), "K-0001");
        }

        #[test]
        fn grid_square_not_auto_uppercased() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state);
            state.handle_key(press(KeyCode::Tab)); // operator
            state.handle_key(press(KeyCode::Tab)); // grid square
            for ch in "fn31pr".chars() {
                state.handle_key(press(KeyCode::Char(ch)));
            }
            assert_eq!(state.form().value(GENERAL_GRID), "fn31pr");
        }

        #[test]
        fn fd_class_auto_uppercased() {
            let mut state = LogCreateState::new();
            switch_to_field_day(&mut state);
            enter_fields(&mut state);
            state.handle_key(press(KeyCode::Tab)); // operator
            state.handle_key(press(KeyCode::Tab)); // grid
            state.handle_key(press(KeyCode::Tab)); // tx count
            state.handle_key(press(KeyCode::Tab)); // class
            state.handle_key(press(KeyCode::Char('b')));
            assert_eq!(state.form().value(CONTEST_CLASS), "B");
        }

        #[test]
        fn fd_section_auto_uppercased() {
            let mut state = LogCreateState::new();
            switch_to_field_day(&mut state);
            enter_fields(&mut state);
            state.handle_key(press(KeyCode::Tab)); // operator
            state.handle_key(press(KeyCode::Tab)); // grid
            state.handle_key(press(KeyCode::Tab)); // tx count
            state.handle_key(press(KeyCode::Tab)); // class
            state.handle_key(press(KeyCode::Tab)); // section
            type_string(&mut state, "epa");
            assert_eq!(state.form().value(CONTEST_SECTION), "EPA");
        }

        #[test]
        fn fd_grid_square_not_auto_uppercased() {
            // Verify that grid square is NOT auto-uppercased even in FD mode —
            // the condition is (FD type AND focus==class/section), not (FD type OR ...)
            let mut state = LogCreateState::new();
            switch_to_field_day(&mut state);
            enter_fields(&mut state); // → CALLSIGN
            state.handle_key(press(KeyCode::Tab)); // → OPERATOR
            state.handle_key(press(KeyCode::Tab)); // → CONTEST_GRID
            for ch in "fn31".chars() {
                state.handle_key(press(KeyCode::Char(ch)));
            }
            assert_eq!(state.form().value(CONTEST_GRID), "fn31");
        }

        #[test]
        fn fd_tx_count_not_auto_uppercased() {
            let mut state = LogCreateState::new();
            switch_to_field_day(&mut state);
            enter_fields(&mut state);
            state.handle_key(press(KeyCode::Tab)); // operator
            state.handle_key(press(KeyCode::Tab)); // grid
            state.handle_key(press(KeyCode::Tab)); // tx count
            type_string(&mut state, "3");
            assert_eq!(state.form().value(CONTEST_TX_COUNT), "3");
        }
    }

    mod navigation {
        use super::*;

        #[test]
        fn esc_navigates_back_from_selector() {
            let mut state = LogCreateState::new();
            let action = state.handle_key(press(KeyCode::Esc));
            assert_eq!(action, Action::Navigate(Screen::LogSelect));
        }

        #[test]
        fn esc_navigates_back_from_fields() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state);
            let action = state.handle_key(press(KeyCode::Esc));
            assert_eq!(action, Action::Navigate(Screen::LogSelect));
        }

        #[test]
        fn unhandled_key_returns_none() {
            let mut state = LogCreateState::new();
            let action = state.handle_key(press(KeyCode::F(1)));
            assert_eq!(action, Action::None);
        }
    }

    mod valid_submit {
        use super::*;

        #[test]
        fn general_log_created() {
            let mut state = LogCreateState::new();
            fill_valid_general_form(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(Log::General(log)) => {
                    assert_eq!(log.header.station_callsign, "W1AW");
                    assert_eq!(log.header.operator, None);
                    assert_eq!(log.header.grid_square, "FN31");
                }
                other => panic!("expected CreateLog(General), got {other:?}"),
            }
        }

        #[test]
        fn pota_log_created_without_park_ref() {
            let mut state = LogCreateState::new();
            fill_valid_pota_form(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(log) => {
                    assert_eq!(log.header().station_callsign, "W1AW");
                    assert_eq!(log.header().operator, None);
                    assert_eq!(log.park_ref(), None);
                    assert_eq!(log.header().grid_square, "FN31");
                }
                other => panic!("expected CreateLog, got {other:?}"),
            }
        }

        #[test]
        fn pota_log_created_with_park_ref() {
            let mut state = LogCreateState::new();
            switch_to_pota(&mut state);
            enter_fields(&mut state);
            type_string(&mut state, "W1AW");
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab)); // skip operator
            type_string(&mut state, "K-0001");
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "FN31");
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(log) => assert_eq!(log.park_ref(), Some("K-0001")),
                other => panic!("expected CreateLog, got {other:?}"),
            }
        }

        #[test]
        fn pota_empty_operator_creates_log_with_none() {
            let mut state = LogCreateState::new();
            fill_valid_pota_form(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(log) => assert_eq!(log.header().operator, None),
                other => panic!("expected CreateLog, got {other:?}"),
            }
        }

        #[test]
        fn pota_with_operator() {
            let mut state = LogCreateState::new();
            fill_valid_pota_form_with_operator(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(log) => {
                    assert_eq!(log.header().operator, Some("W1AW".to_string()));
                }
                other => panic!("expected CreateLog, got {other:?}"),
            }
        }

        #[test]
        fn pota_grid_square_lowercase_normalised_on_submit() {
            let mut state = LogCreateState::new();
            switch_to_pota(&mut state);
            enter_fields(&mut state);
            type_string(&mut state, "W1AW");
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "fn31");
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(log) => assert_eq!(log.header().grid_square, "FN31"),
                other => panic!("expected CreateLog, got {other:?}"),
            }
        }

        #[test]
        fn pota_grid_square_uppercase_subsquare_normalised_on_submit() {
            let mut state = LogCreateState::new();
            switch_to_pota(&mut state);
            enter_fields(&mut state);
            type_string(&mut state, "W1AW");
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "FN31PR");
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(log) => assert_eq!(log.header().grid_square, "FN31pr"),
                other => panic!("expected CreateLog, got {other:?}"),
            }
        }

        #[test]
        fn pota_lowercase_park_ref_stored_uppercase() {
            let mut state = LogCreateState::new();
            switch_to_pota(&mut state);
            enter_fields(&mut state);
            type_string(&mut state, "W1AW");
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "k-0001"); // auto-uppercased on input
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "FN31");
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(log) => assert_eq!(log.park_ref(), Some("K-0001")),
                other => panic!("expected CreateLog, got {other:?}"),
            }
        }

        #[test]
        fn field_day_log_created() {
            let mut state = LogCreateState::new();
            fill_valid_fd_form(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(Log::FieldDay(log)) => {
                    assert_eq!(log.header.station_callsign, "W1AW");
                    assert_eq!(log.tx_count, 3);
                    assert_eq!(log.class, crate::model::FdClass::B);
                    assert_eq!(log.section, "EPA");
                    assert_eq!(log.power, crate::model::FdPowerCategory::Low);
                    assert_eq!(log.header.grid_square, "FN31");
                }
                other => panic!("expected CreateLog(FieldDay), got {other:?}"),
            }
        }

        #[test]
        fn wfd_log_created() {
            let mut state = LogCreateState::new();
            fill_valid_wfd_form(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(Log::WinterFieldDay(log)) => {
                    assert_eq!(log.header.station_callsign, "W1AW");
                    assert_eq!(log.tx_count, 1);
                    assert_eq!(log.class, crate::model::WfdClass::H);
                    assert_eq!(log.section, "EPA");
                    assert_eq!(log.header.grid_square, "FN31");
                }
                other => panic!("expected CreateLog(WinterFieldDay), got {other:?}"),
            }
        }
    }

    mod invalid_submit {
        use super::*;

        #[test]
        fn general_empty_submit_shows_errors() {
            let mut state = LogCreateState::new();
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().has_errors());
            assert!(state.form().fields()[CALLSIGN].error.is_some());
            assert!(state.form().fields()[OPERATOR].error.is_none()); // optional
            assert!(state.form().fields()[GENERAL_GRID].error.is_some());
        }

        #[test]
        fn pota_empty_submit_shows_errors() {
            let mut state = LogCreateState::new();
            switch_to_pota(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().has_errors());
            assert!(state.form().fields()[CALLSIGN].error.is_some());
            assert!(state.form().fields()[OPERATOR].error.is_none()); // optional
            assert!(state.form().fields()[POTA_PARK_REF].error.is_none()); // optional
            assert!(state.form().fields()[POTA_GRID].error.is_some());
        }

        #[test]
        fn fd_empty_submit_shows_errors() {
            let mut state = LogCreateState::new();
            switch_to_field_day(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().has_errors());
            assert!(state.form().fields()[CALLSIGN].error.is_some());
            assert!(state.form().fields()[OPERATOR].error.is_none()); // optional
            assert!(state.form().fields()[CONTEST_GRID].error.is_some());
            assert!(state.form().fields()[CONTEST_TX_COUNT].error.is_some()); // empty = parse error
            assert!(state.form().fields()[CONTEST_CLASS].error.is_some()); // empty = invalid class
            assert!(state.form().fields()[CONTEST_SECTION].error.is_some());
        }

        #[test]
        fn wfd_empty_submit_shows_errors() {
            let mut state = LogCreateState::new();
            switch_to_wfd(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().has_errors());
            assert!(state.form().fields()[CALLSIGN].error.is_some());
            assert!(state.form().fields()[OPERATOR].error.is_none()); // optional
            assert!(state.form().fields()[CONTEST_GRID].error.is_some());
            assert!(state.form().fields()[CONTEST_TX_COUNT].error.is_some());
            assert!(state.form().fields()[CONTEST_CLASS].error.is_some());
            assert!(state.form().fields()[CONTEST_SECTION].error.is_some());
        }

        #[test]
        fn pota_invalid_park_ref_shows_error() {
            let mut state = LogCreateState::new();
            fill_valid_pota_form(&mut state);
            // Navigate back to park ref and add a bad value
            // Easiest: go back to TypeSelector then re-enter fields
            // Actually easier: navigate using BackTab from last field
            // In the pota form after fill: focus is on POTA_GRID (last)
            // Actually after fill_valid_pota_form, focus is on POTA_GRID (we typed FN31 there)
            // BackTab to POTA_PARK_REF:
            state.handle_key(shift_press(KeyCode::BackTab));
            assert_eq!(state.form().focus(), POTA_PARK_REF);
            type_string(&mut state, "BAD");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().fields()[POTA_PARK_REF].error.is_some());
            assert!(state.form().fields()[CALLSIGN].error.is_none());
        }

        #[test]
        fn fd_invalid_class_shows_error() {
            let mut state = LogCreateState::new();
            fill_valid_fd_form(&mut state);
            // Clear the class field and type an invalid value
            state.handle_key(shift_press(KeyCode::BackTab)); // section → class
            state.handle_key(press(KeyCode::Backspace)); // remove "B"
            type_string(&mut state, "Z");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().fields()[CONTEST_CLASS].error.is_some());
        }

        #[test]
        fn fd_invalid_tx_count_shows_error() {
            let mut state = LogCreateState::new();
            fill_valid_fd_form(&mut state);
            // Navigate to tx_count field
            state.handle_key(shift_press(KeyCode::BackTab)); // section → class
            state.handle_key(shift_press(KeyCode::BackTab)); // class → tx_count
            state.handle_key(press(KeyCode::Backspace)); // remove "3"
            type_string(&mut state, "0");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().fields()[CONTEST_TX_COUNT].error.is_some());
        }

        #[test]
        fn fd_non_numeric_tx_count_shows_error() {
            let mut state = LogCreateState::new();
            fill_valid_fd_form(&mut state);
            state.handle_key(shift_press(KeyCode::BackTab)); // section → class
            state.handle_key(shift_press(KeyCode::BackTab)); // class → tx_count
            state.handle_key(press(KeyCode::Backspace)); // remove "3"
            type_string(&mut state, "x");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().fields()[CONTEST_TX_COUNT].error.is_some());
        }

        #[test]
        fn wfd_invalid_class_shows_error() {
            let mut state = LogCreateState::new();
            fill_valid_wfd_form(&mut state);
            state.handle_key(shift_press(KeyCode::BackTab)); // section → class
            state.handle_key(press(KeyCode::Backspace)); // remove "H"
            type_string(&mut state, "Z");
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().fields()[CONTEST_CLASS].error.is_some());
        }

        #[test]
        fn errors_cleared_on_resubmit() {
            let mut state = LogCreateState::new();
            state.handle_key(press(KeyCode::Enter));
            assert!(state.form().has_errors());
            fill_valid_general_form(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            assert!(matches!(action, Action::CreateLog(_)));
            assert!(!state.form().has_errors());
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

        fn render_log_create(state: &LogCreateState, width: u16, height: u16) -> String {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| {
                    draw_log_create(state, frame, frame.area());
                })
                .unwrap();
            buffer_to_string(terminal.backend().buffer())
        }

        #[test]
        fn renders_title_and_fields() {
            let state = LogCreateState::new();
            let output = render_log_create(&state, 60, 22);
            assert!(output.contains("Create New Log"), "should show title");
            assert!(
                output.contains("Station Callsign"),
                "should show callsign field"
            );
            assert!(
                output.contains("Grid Square"),
                "should show grid square field"
            );
        }

        #[test]
        fn renders_type_selector() {
            let state = LogCreateState::new();
            let output = render_log_create(&state, 60, 22);
            assert!(
                output.contains("Log Type"),
                "should show type selector label"
            );
            assert!(output.contains("General"), "should show default log type");
        }

        #[test]
        fn renders_footer() {
            let state = LogCreateState::new();
            let output = render_log_create(&state, 80, 22);
            assert!(
                output.contains("Enter: create"),
                "should show footer keybindings"
            );
            assert!(output.contains("log type"), "should show type switch hint");
        }

        #[test]
        fn renders_field_values() {
            let mut state = LogCreateState::new();
            fill_valid_general_form(&mut state);
            let output = render_log_create(&state, 60, 22);
            assert!(output.contains("W1AW"), "should show typed callsign");
            assert!(output.contains("FN31"), "should show typed grid square");
        }

        #[test]
        fn renders_general_error() {
            let mut state = LogCreateState::new();
            state.set_error("A log already exists for W1AW on 2026-02-19 UTC".into());
            let output = render_log_create(&state, 70, 25);
            assert!(
                output.contains("A log already exists"),
                "should render general error"
            );
        }

        #[test]
        fn renders_pota_fields_when_type_is_pota() {
            let mut state = LogCreateState::new();
            switch_to_pota(&mut state);
            let output = render_log_create(&state, 60, 25);
            assert!(output.contains("POTA"), "should show POTA in type selector");
            assert!(output.contains("Park Ref"), "should show park ref field");
        }
    }

    mod reset {
        use super::*;

        #[test]
        fn clears_form_and_focus() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state);
            state.handle_key(press(KeyCode::Char('X')));
            state.reset();
            assert_eq!(state.form().value(CALLSIGN), "");
            assert_eq!(state.form().focus(), 0);
            assert_eq!(state.focus_area, FocusArea::TypeSelector);
        }

        #[test]
        fn resets_log_type_to_general() {
            let mut state = LogCreateState::new();
            switch_to_pota(&mut state);
            state.reset();
            assert_eq!(state.log_type, LogType::General);
        }

        #[test]
        fn clears_general_error() {
            let mut state = LogCreateState::new();
            state.set_error("some error".into());
            state.reset();
            assert_eq!(state.general_error(), None);
        }

        #[test]
        fn clears_buffers_so_they_dont_leak() {
            let mut state = LogCreateState::new();
            enter_fields(&mut state);
            type_string(&mut state, "W1AW"); // fills callsign
            state.reset();
            // After reset, switch to POTA and verify callsign buf is empty
            switch_to_pota(&mut state);
            assert_eq!(state.form().value(CALLSIGN), "");
        }
    }

    mod general_error {
        use super::*;

        #[test]
        fn set_error_stores_message() {
            let mut state = LogCreateState::new();
            state.set_error("duplicate log".into());
            assert_eq!(state.general_error(), Some("duplicate log"));
        }

        #[test]
        fn submit_clears_general_error() {
            let mut state = LogCreateState::new();
            state.set_error("old error".into());
            fill_valid_general_form(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            assert!(matches!(action, Action::CreateLog(_)));
            assert_eq!(state.general_error(), None);
        }
    }
}
