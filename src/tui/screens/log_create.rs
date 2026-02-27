//! Log creation screen — form for entering new log session details.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::model::{
    Log, PotaLog, normalize_grid_square, normalize_park_ref, validate_callsign,
    validate_grid_square, validate_park_ref,
};
use crate::tui::action::Action;
use crate::tui::app::Screen;
use crate::tui::widgets::form::{Form, FormField, draw_form};

/// Field index for station callsign.
const CALLSIGN: usize = 0;
/// Field index for operator callsign.
const OPERATOR: usize = 1;
/// Field index for POTA park reference.
const PARK_REF: usize = 2;
/// Field index for Maidenhead grid square.
const GRID_SQUARE: usize = 3;

/// State for the log creation screen.
#[derive(Debug, Clone)]
pub struct LogCreateState {
    form: Form,
    general_error: Option<String>,
}

impl Default for LogCreateState {
    fn default() -> Self {
        Self::new()
    }
}

impl LogCreateState {
    /// Creates a new log creation form with empty fields.
    pub fn new() -> Self {
        Self {
            form: Form::new(vec![
                FormField::new("Station Callsign", true),
                FormField::new("Operator", false),
                FormField::new("Park Ref (e.g. K-0001)", false),
                FormField::new("Grid Square (e.g. FN31)", true),
            ]),
            general_error: None,
        }
    }

    /// Handles a key event, returning an [`Action`] for the app to apply.
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Tab => {
                self.form.focus_next();
                Action::None
            }
            KeyCode::BackTab => {
                self.form.focus_prev();
                Action::None
            }
            KeyCode::Char(ch) => {
                let should_uppercase = self.form.focus() == CALLSIGN
                    || self.form.focus() == OPERATOR
                    || self.form.focus() == PARK_REF;
                let ch = if should_uppercase {
                    ch.to_ascii_uppercase()
                } else {
                    ch
                };
                self.form.insert_char(ch);
                Action::None
            }
            KeyCode::Backspace => {
                self.form.delete_char();
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
        self.form.reset();
        self.general_error = None;
    }

    /// Validates all fields and attempts to create a [`Log`].
    fn submit(&mut self) -> Action {
        self.form.clear_errors();
        self.general_error = None;

        let callsign = self.form.value(CALLSIGN).to_string();
        let operator_str = self.form.value(OPERATOR).to_string();
        let operator = (!operator_str.is_empty()).then_some(operator_str);
        let park_ref_str = normalize_park_ref(self.form.value(PARK_REF));
        let grid_square = normalize_grid_square(self.form.value(GRID_SQUARE));

        // Validate each field individually to show all errors at once.
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
            self.form.set_error(PARK_REF, e.to_string());
        }
        if let Err(e) = validate_grid_square(&grid_square) {
            self.form.set_error(GRID_SQUARE, e.to_string());
        }

        if self.form.has_errors() {
            return Action::None;
        }

        let park_ref = (!park_ref_str.is_empty()).then_some(park_ref_str);

        // All individual validations passed, so PotaLog::new should succeed.
        // Phase 4.2 will add log-type selection; for now all logs are POTA.
        match PotaLog::new(callsign, operator, park_ref, grid_square) {
            Ok(log) => Action::CreateLog(Log::Pota(log)),
            Err(e) => {
                // Shouldn't happen since we validated above, but handle gracefully.
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

    let [form_area, error_area, _spacer, footer_area] = Layout::vertical([
        Constraint::Length(12),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(inner);

    draw_form(state.form(), frame, form_area);

    if let Some(err) = state.general_error() {
        let error = Paragraph::new(Line::from(Span::styled(
            err,
            Style::default().fg(Color::Red),
        )));
        frame.render_widget(error, error_area);
    }

    let footer = Paragraph::new(Line::from(
        "Tab/Shift+Tab: next/prev  Enter: create  Esc: cancel",
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

    fn fill_valid_form(state: &mut LogCreateState) {
        // Callsign: W1AW
        for ch in "W1AW".chars() {
            state.handle_key(press(KeyCode::Char(ch)));
        }
        // Tab to operator
        state.handle_key(press(KeyCode::Tab));
        // Operator: W1AW
        for ch in "W1AW".chars() {
            state.handle_key(press(KeyCode::Char(ch)));
        }
        // Tab to park ref (skip)
        state.handle_key(press(KeyCode::Tab));
        // Tab to grid square
        state.handle_key(press(KeyCode::Tab));
        // Grid: FN31
        for ch in "FN31".chars() {
            state.handle_key(press(KeyCode::Char(ch)));
        }
    }

    fn type_string(state: &mut LogCreateState, s: &str) {
        for ch in s.chars() {
            state.handle_key(press(KeyCode::Char(ch)));
        }
    }

    fn fill_form_with_park_ref(state: &mut LogCreateState, park_ref: &str) {
        type_string(state, "W1AW");
        state.handle_key(press(KeyCode::Tab));
        type_string(state, "W1AW");
        state.handle_key(press(KeyCode::Tab));
        type_string(state, park_ref);
        state.handle_key(press(KeyCode::Tab));
        type_string(state, "FN31");
    }

    mod typing {
        use super::*;

        #[test]
        fn chars_fill_focused_field() {
            let mut state = LogCreateState::new();
            state.handle_key(press(KeyCode::Char('W')));
            state.handle_key(press(KeyCode::Char('1')));
            assert_eq!(state.form().value(CALLSIGN), "W1");
        }

        #[test]
        fn backspace_deletes_char() {
            let mut state = LogCreateState::new();
            state.handle_key(press(KeyCode::Char('A')));
            state.handle_key(press(KeyCode::Char('B')));
            state.handle_key(press(KeyCode::Backspace));
            assert_eq!(state.form().value(CALLSIGN), "A");
        }

        #[test]
        fn callsign_auto_uppercased() {
            let mut state = LogCreateState::new();
            for ch in "w3duk".chars() {
                state.handle_key(press(KeyCode::Char(ch)));
            }
            assert_eq!(state.form().value(CALLSIGN), "W3DUK");
        }

        #[test]
        fn operator_auto_uppercased() {
            let mut state = LogCreateState::new();
            state.handle_key(press(KeyCode::Tab)); // move to operator
            for ch in "w3duk".chars() {
                state.handle_key(press(KeyCode::Char(ch)));
            }
            assert_eq!(state.form().value(OPERATOR), "W3DUK");
        }

        #[test]
        fn park_ref_auto_uppercased() {
            let mut state = LogCreateState::new();
            state.handle_key(press(KeyCode::Tab)); // operator
            state.handle_key(press(KeyCode::Tab)); // park ref
            for ch in "k-0001".chars() {
                state.handle_key(press(KeyCode::Char(ch)));
            }
            assert_eq!(state.form().value(PARK_REF), "K-0001");
        }

        #[test]
        fn grid_square_not_auto_uppercased() {
            // Grid square is normalised at submit time (not at input time) because
            // subsquare chars must be stored lowercase; auto-uppercasing would
            // prevent typing the correct mixed-case form.
            let mut state = LogCreateState::new();
            state.handle_key(press(KeyCode::Tab)); // operator
            state.handle_key(press(KeyCode::Tab)); // park ref
            state.handle_key(press(KeyCode::Tab)); // grid square
            for ch in "fn31pr".chars() {
                state.handle_key(press(KeyCode::Char(ch)));
            }
            assert_eq!(state.form().value(GRID_SQUARE), "fn31pr");
        }
    }

    mod tab_cycling {
        use super::*;

        #[test]
        fn tab_cycles_focus_forward() {
            let mut state = LogCreateState::new();
            assert_eq!(state.form().focus(), CALLSIGN);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), OPERATOR);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), PARK_REF);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), GRID_SQUARE);
            state.handle_key(press(KeyCode::Tab));
            assert_eq!(state.form().focus(), CALLSIGN);
        }

        #[test]
        fn backtab_cycles_focus_backward() {
            let mut state = LogCreateState::new();
            state.handle_key(shift_press(KeyCode::BackTab));
            assert_eq!(state.form().focus(), GRID_SQUARE);
        }
    }

    mod navigation {
        use super::*;

        #[test]
        fn esc_navigates_back() {
            let mut state = LogCreateState::new();
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
        fn creates_log_without_park_ref() {
            let mut state = LogCreateState::new();
            fill_valid_form(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(log) => {
                    assert_eq!(log.header().station_callsign, "W1AW");
                    assert_eq!(log.header().operator, Some("W1AW".to_string()));
                    assert_eq!(log.park_ref(), None);
                    assert_eq!(log.header().grid_square, "FN31");
                }
                other => panic!("expected CreateLog, got {other:?}"),
            }
        }

        #[test]
        fn empty_operator_creates_log_with_none() {
            let mut state = LogCreateState::new();
            // Fill callsign
            for ch in "W1AW".chars() {
                state.handle_key(press(KeyCode::Char(ch)));
            }
            // Tab to operator, then past it (leave empty)
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            // Tab past park ref
            state.handle_key(press(KeyCode::Tab));
            // Fill grid square
            for ch in "FN31".chars() {
                state.handle_key(press(KeyCode::Char(ch)));
            }
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(log) => {
                    assert_eq!(log.header().operator, None);
                }
                other => panic!("expected CreateLog, got {other:?}"),
            }
        }

        #[test]
        fn creates_log_with_park_ref() {
            let mut state = LogCreateState::new();
            fill_form_with_park_ref(&mut state, "K-0001");

            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(log) => {
                    assert_eq!(log.park_ref(), Some("K-0001"));
                }
                other => panic!("expected CreateLog, got {other:?}"),
            }
        }

        #[test]
        fn empty_park_ref_is_accepted() {
            let mut state = LogCreateState::new();
            fill_valid_form(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            assert!(matches!(action, Action::CreateLog(_)));
        }

        #[test]
        fn grid_square_lowercase_normalised_on_submit() {
            let mut state = LogCreateState::new();
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
        fn grid_square_all_uppercase_subsquare_normalised_on_submit() {
            let mut state = LogCreateState::new();
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
        fn grid_square_all_lowercase_six_char_normalised_on_submit() {
            let mut state = LogCreateState::new();
            type_string(&mut state, "W1AW");
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            state.handle_key(press(KeyCode::Tab));
            type_string(&mut state, "fn31pr");
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(log) => assert_eq!(log.header().grid_square, "FN31pr"),
                other => panic!("expected CreateLog, got {other:?}"),
            }
        }

        #[test]
        fn lowercase_park_ref_normalised_on_submit() {
            let mut state = LogCreateState::new();
            fill_form_with_park_ref(&mut state, "k-0001");
            let action = state.handle_key(press(KeyCode::Enter));
            match action {
                Action::CreateLog(log) => assert_eq!(log.park_ref(), Some("K-0001")),
                other => panic!("expected CreateLog, got {other:?}"),
            }
        }
    }

    mod invalid_submit {
        use super::*;

        #[test]
        fn empty_submit_shows_all_errors() {
            let mut state = LogCreateState::new();
            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().has_errors());
            assert!(state.form().fields()[CALLSIGN].error.is_some());
            assert!(state.form().fields()[OPERATOR].error.is_none()); // optional
            assert!(state.form().fields()[PARK_REF].error.is_none()); // optional
            assert!(state.form().fields()[GRID_SQUARE].error.is_some());
        }

        #[test]
        fn invalid_park_ref_shows_error() {
            let mut state = LogCreateState::new();
            fill_valid_form(&mut state);
            state.handle_key(shift_press(KeyCode::BackTab));
            assert_eq!(state.form().focus(), PARK_REF);
            for ch in "bad".chars() {
                state.handle_key(press(KeyCode::Char(ch)));
            }

            let action = state.handle_key(press(KeyCode::Enter));
            assert_eq!(action, Action::None);
            assert!(state.form().fields()[PARK_REF].error.is_some());
            assert!(state.form().fields()[CALLSIGN].error.is_none());
        }

        #[test]
        fn errors_cleared_on_resubmit() {
            let mut state = LogCreateState::new();
            state.handle_key(press(KeyCode::Enter));
            assert!(state.form().has_errors());
            fill_valid_form(&mut state);
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
            let output = render_log_create(&state, 60, 20);
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
        fn renders_footer() {
            let state = LogCreateState::new();
            let output = render_log_create(&state, 60, 20);
            assert!(
                output.contains("Enter: create"),
                "should show footer keybindings"
            );
        }

        #[test]
        fn renders_field_values() {
            let mut state = LogCreateState::new();
            fill_valid_form(&mut state);
            let output = render_log_create(&state, 60, 20);
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
    }

    mod reset {
        use super::*;

        #[test]
        fn clears_form() {
            let mut state = LogCreateState::new();
            state.handle_key(press(KeyCode::Char('X')));
            state.reset();
            assert_eq!(state.form().value(CALLSIGN), "");
            assert_eq!(state.form().focus(), 0);
        }

        #[test]
        fn clears_general_error() {
            let mut state = LogCreateState::new();
            state.set_error("some error".into());
            state.reset();
            assert_eq!(state.general_error(), None);
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
            fill_valid_form(&mut state);
            let action = state.handle_key(press(KeyCode::Enter));
            assert!(matches!(action, Action::CreateLog(_)));
            assert_eq!(state.general_error(), None);
        }
    }
}
