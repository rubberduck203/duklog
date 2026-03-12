//! Reusable form widget for text input screens.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

/// A single field within a [`Form`].
#[derive(Debug, Clone)]
pub struct FormField {
    /// Display label shown to the left of the input.
    pub label: String,
    /// Current text value.
    pub value: String,
    /// Validation error message, if any.
    pub error: Option<String>,
    /// Whether the field must be non-empty on submit.
    pub required: bool,
    /// If true, the next `insert_char` or `delete_char` clears the current value before acting.
    /// Used for pre-populated defaults (e.g. RST "59") that should be replaced on first keystroke.
    pub clear_on_first_input: bool,
}

impl FormField {
    /// Creates a new form field.
    pub fn new(label: impl Into<String>, required: bool) -> Self {
        Self {
            label: label.into(),
            value: String::new(),
            error: None,
            required,
            clear_on_first_input: false,
        }
    }
}

/// A multi-field text form with focus management.
#[derive(Debug, Clone)]
pub struct Form {
    fields: Vec<FormField>,
    focus: usize,
}

impl Form {
    /// Creates a new form with the given fields. Focus starts on the first field.
    pub fn new(fields: Vec<FormField>) -> Self {
        Self { fields, focus: 0 }
    }

    /// Returns the index of the currently focused field.
    pub fn focus(&self) -> usize {
        self.focus
    }

    /// Sets focus to the given field index. No-op if out of bounds.
    pub fn set_focus(&mut self, index: usize) {
        if index < self.fields.len() {
            self.focus = index;
        }
    }

    /// Moves focus to the next field, wrapping around.
    pub fn focus_next(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        self.focus = (self.focus + 1) % self.fields.len();
    }

    /// Moves focus to the previous field, wrapping around.
    pub fn focus_prev(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        self.focus = (self.focus + self.fields.len() - 1) % self.fields.len();
    }

    /// Inserts a character at the end of the focused field.
    ///
    /// If `clear_on_first_input` is set on the focused field, the current value is cleared
    /// before inserting (replacing the pre-populated default on first keystroke).
    pub fn insert_char(&mut self, ch: char) {
        if let Some(field) = self.fields.get_mut(self.focus) {
            if field.clear_on_first_input {
                field.value.clear();
                field.clear_on_first_input = false;
            }
            field.value.push(ch);
        }
    }

    /// Deletes the last character from the focused field.
    ///
    /// If `clear_on_first_input` is set, the entire value is cleared instead of popping
    /// one character (treats the backspace as "replace default").
    pub fn delete_char(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focus) {
            if field.clear_on_first_input {
                field.value.clear();
                field.clear_on_first_input = false;
            } else {
                field.value.pop();
            }
        }
    }

    /// Sets an error message on a field by index.
    pub fn set_error(&mut self, index: usize, error: String) {
        if let Some(field) = self.fields.get_mut(index) {
            field.error = Some(error);
        }
    }

    /// Clears all field errors.
    pub fn clear_errors(&mut self) {
        for field in &mut self.fields {
            field.error = None;
        }
    }

    /// Returns `true` if any field has an error set.
    pub fn has_errors(&self) -> bool {
        self.fields.iter().any(|f| f.error.is_some())
    }

    /// Sets the value of the field at `index` and disarms `clear_on_first_input`.
    ///
    /// Use this for explicit values (e.g. loading an existing QSO for editing).
    /// The first keystroke after `set_value` appends normally rather than replacing.
    /// No-op if `index` is out of bounds.
    pub fn set_value(&mut self, index: usize, value: impl Into<String>) {
        if let Some(field) = self.fields.get_mut(index) {
            field.value = value.into();
            field.clear_on_first_input = false;
        }
    }

    /// Sets a pre-populated default value on the field at `index` and arms `clear_on_first_input`.
    ///
    /// The value is shown to the operator but the first keystroke (insert or delete) replaces it
    /// entirely, so changing the default requires no backspacing. Operators who accept the default
    /// can Tab past the field without typing — the value is preserved as-is.
    ///
    /// No-op if `index` is out of bounds.
    pub fn set_default(&mut self, index: usize, value: impl Into<String>) {
        if let Some(field) = self.fields.get_mut(index) {
            field.value = value.into();
            field.clear_on_first_input = true;
        }
    }

    /// Clears the value of the field at `index` and disarms `clear_on_first_input`.
    /// No-op if out of bounds.
    pub fn clear_value(&mut self, index: usize) {
        if let Some(field) = self.fields.get_mut(index) {
            field.value.clear();
            field.clear_on_first_input = false;
        }
    }

    /// Returns the value of the field at `index`, or an empty string if out of bounds.
    pub fn value(&self, index: usize) -> &str {
        self.fields
            .get(index)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    /// Returns all field values as a vector of string slices.
    pub fn values(&self) -> Vec<&str> {
        self.fields.iter().map(|f| f.value.as_str()).collect()
    }

    /// Resets all field values and errors.
    pub fn reset(&mut self) {
        for field in &mut self.fields {
            field.value.clear();
            field.error = None;
            field.clear_on_first_input = false;
        }
        self.focus = 0;
    }

    /// Returns a reference to the fields.
    pub fn fields(&self) -> &[FormField] {
        &self.fields
    }
}

/// Renders a single form field at the given area.
#[mutants::skip]
pub fn draw_form_field(form: &Form, field_idx: usize, frame: &mut Frame, area: Rect) {
    let Some(field) = form.fields.get(field_idx) else {
        return;
    };
    let row_height = 3_u16;
    let is_focused = field_idx == form.focus;

    let border_color = if field.error.is_some() {
        Color::Red
    } else if is_focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let label = if field.required {
        format!("{} *", field.label)
    } else {
        field.label.clone()
    };

    let block = Block::default()
        .title(label)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let mut spans = vec![Span::raw(&field.value)];
    if is_focused {
        spans.push(Span::styled(
            "\u{2588}",
            Style::default().add_modifier(Modifier::SLOW_BLINK),
        ));
    }

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    frame.render_widget(paragraph, area);

    // Draw error below the field if there's space
    if let Some(ref err) = field.error {
        let error_line = Paragraph::new(Span::styled(err, Style::default().fg(Color::Red)));
        let err_area = Rect {
            x: area.x + 2,
            y: area.y + row_height.saturating_sub(1),
            width: area.width.saturating_sub(4),
            height: 1,
        };
        frame.render_widget(error_line, err_area);
    }
}

/// Renders a form within the given area.
#[mutants::skip]
pub fn draw_form(form: &Form, frame: &mut Frame, area: Rect) {
    let row_height = 3_u16;
    let constraints: Vec<Constraint> = form
        .fields
        .iter()
        .map(|_| Constraint::Length(row_height))
        .collect();

    let rows = Layout::vertical(constraints).split(area);

    for i in 0..form.fields.len() {
        draw_form_field(form, i, frame, rows[i]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_form() -> Form {
        Form::new(vec![
            FormField::new("Callsign", true),
            FormField::new("Operator", true),
            FormField::new("Park Ref", false),
        ])
    }

    mod focus {
        use super::*;

        #[test]
        fn starts_at_zero() {
            let form = make_form();
            assert_eq!(form.focus(), 0);
        }

        #[test]
        fn next_advances() {
            let mut form = make_form();
            form.focus_next();
            assert_eq!(form.focus(), 1);
            form.focus_next();
            assert_eq!(form.focus(), 2);
        }

        #[test]
        fn next_wraps() {
            let mut form = make_form();
            form.focus_next();
            form.focus_next();
            form.focus_next();
            assert_eq!(form.focus(), 0);
        }

        #[test]
        fn prev_wraps() {
            let mut form = make_form();
            form.focus_prev();
            assert_eq!(form.focus(), 2);
        }

        #[test]
        fn prev_decrements() {
            let mut form = make_form();
            form.focus_next();
            form.focus_next();
            form.focus_prev();
            assert_eq!(form.focus(), 1);
        }

        #[test]
        fn next_empty_form_is_noop() {
            let mut form = Form::new(vec![]);
            form.focus_next();
            assert_eq!(form.focus(), 0);
        }

        #[test]
        fn prev_empty_form_is_noop() {
            let mut form = Form::new(vec![]);
            form.focus_prev();
            assert_eq!(form.focus(), 0);
        }

        #[test]
        fn set_focus_changes_focus() {
            let mut form = make_form();
            form.set_focus(2);
            assert_eq!(form.focus(), 2);
        }

        #[test]
        fn set_focus_out_of_bounds_is_noop() {
            let mut form = make_form();
            form.set_focus(99);
            assert_eq!(form.focus(), 0);
        }

        #[test]
        fn set_focus_at_len_is_noop() {
            let mut form = make_form();
            form.set_focus(form.fields().len());
            assert_eq!(form.focus(), 0);
        }
    }

    mod editing {
        use super::*;

        #[test]
        fn insert_char_appends_to_focused() {
            let mut form = make_form();
            form.insert_char('W');
            form.insert_char('1');
            assert_eq!(form.value(0), "W1");
            assert_eq!(form.value(1), "");
        }

        #[test]
        fn insert_char_on_different_focus() {
            let mut form = make_form();
            form.focus_next();
            form.insert_char('A');
            assert_eq!(form.value(0), "");
            assert_eq!(form.value(1), "A");
        }

        #[test]
        fn delete_char_removes_last() {
            let mut form = make_form();
            form.insert_char('A');
            form.insert_char('B');
            form.delete_char();
            assert_eq!(form.value(0), "A");
        }

        #[test]
        fn delete_char_on_empty_is_noop() {
            let mut form = make_form();
            form.delete_char();
            assert_eq!(form.value(0), "");
        }

        #[test]
        fn clear_on_first_input_clears_before_insert() {
            let mut form = make_form();
            form.set_default(0, "59");
            assert_eq!(form.value(0), "59");
            form.insert_char('5');
            assert_eq!(
                form.value(0),
                "5",
                "first char should replace default, not append"
            );
            form.insert_char('7');
            assert_eq!(form.value(0), "57");
        }

        #[test]
        fn delete_char_on_default_clears_value() {
            let mut form = make_form();
            form.set_default(0, "59");
            form.delete_char();
            assert_eq!(
                form.value(0),
                "",
                "backspace on default should clear entirely"
            );
        }

        #[test]
        fn clear_on_first_input_flag_disarmed_after_insert() {
            let mut form = make_form();
            form.set_default(0, "59");
            form.insert_char('5');
            // flag is now false; subsequent inserts append normally
            form.insert_char('9');
            assert_eq!(form.value(0), "59");
        }

        #[test]
        fn clear_on_first_input_rearms_on_set_default() {
            let mut form = make_form();
            form.set_default(0, "59");
            form.insert_char('5'); // disarms flag
            assert_eq!(form.value(0), "5");
            form.set_default(0, "59"); // re-arm
            form.insert_char('7');
            assert_eq!(
                form.value(0),
                "7",
                "set_default should re-arm the clear flag"
            );
        }

        #[test]
        fn normal_field_unaffected_by_set_default_on_other_field() {
            let mut form = make_form();
            form.set_default(1, "59"); // arm field 1
            // type into field 0 — should not clear anything
            form.insert_char('A');
            form.insert_char('B');
            assert_eq!(form.value(0), "AB");
            assert_eq!(form.value(1), "59"); // field 1 unchanged, still armed
        }
    }

    mod errors {
        use super::*;

        #[test]
        fn set_error_on_field() {
            let mut form = make_form();
            form.set_error(0, "bad callsign".into());
            assert!(form.has_errors());
            assert_eq!(form.fields()[0].error, Some("bad callsign".into()));
        }

        #[test]
        fn clear_errors_removes_all() {
            let mut form = make_form();
            form.set_error(0, "err1".into());
            form.set_error(1, "err2".into());
            assert!(form.has_errors());
            form.clear_errors();
            assert!(!form.has_errors());
        }

        #[test]
        fn has_errors_false_when_clean() {
            let form = make_form();
            assert!(!form.has_errors());
        }

        #[test]
        fn set_error_out_of_bounds_is_noop() {
            let mut form = make_form();
            form.set_error(99, "nope".into());
            assert!(!form.has_errors());
        }
    }

    mod values {
        use super::*;

        #[test]
        fn returns_all_field_values() {
            let mut form = make_form();
            form.insert_char('A');
            form.focus_next();
            form.insert_char('B');
            assert_eq!(form.values(), vec!["A", "B", ""]);
        }

        #[test]
        fn out_of_bounds_returns_empty() {
            let form = make_form();
            assert_eq!(form.value(99), "");
        }

        #[test]
        fn set_value_replaces_field() {
            let mut form = make_form();
            form.set_value(0, "W1AW");
            assert_eq!(form.value(0), "W1AW");
        }

        #[test]
        fn set_value_out_of_bounds_is_noop() {
            let mut form = make_form();
            form.set_value(99, "nope");
            assert_eq!(form.values(), vec!["", "", ""]);
        }

        #[test]
        fn clear_value_empties_field() {
            let mut form = make_form();
            form.set_value(1, "hello");
            form.clear_value(1);
            assert_eq!(form.value(1), "");
        }

        #[test]
        fn clear_value_out_of_bounds_is_noop() {
            let mut form = make_form();
            form.clear_value(99);
            assert_eq!(form.values(), vec!["", "", ""]);
        }

        #[test]
        fn clear_value_disarms_clear_on_first_input() {
            let mut form = make_form();
            form.set_default(0, "59");
            assert!(form.fields()[0].clear_on_first_input);
            form.clear_value(0);
            assert!(!form.fields()[0].clear_on_first_input);
            assert_eq!(form.value(0), "");
            // first insert now appends normally
            form.set_focus(0);
            form.insert_char('5');
            assert_eq!(form.value(0), "5");
        }

        #[test]
        fn set_default_sets_value_and_arms_flag() {
            let mut form = make_form();
            form.set_default(0, "59");
            assert_eq!(form.value(0), "59");
            assert!(form.fields()[0].clear_on_first_input);
        }

        #[test]
        fn set_default_out_of_bounds_is_noop() {
            let mut form = make_form();
            form.set_default(99, "59");
            assert_eq!(form.values(), vec!["", "", ""]);
        }

        #[test]
        fn set_value_disarms_clear_on_first_input() {
            let mut form = make_form();
            form.set_default(0, "59"); // arms flag
            assert!(form.fields()[0].clear_on_first_input);
            form.set_value(0, "57"); // explicit value — disarms
            assert!(!form.fields()[0].clear_on_first_input);
        }

        #[test]
        fn set_value_after_set_default_appends_normally() {
            let mut form = make_form();
            form.set_default(0, "59");
            form.set_value(0, "57"); // disarms
            form.insert_char('X');
            assert_eq!(
                form.value(0),
                "57X",
                "insert after set_value must append, not replace"
            );
        }
    }

    mod reset {
        use super::*;

        #[test]
        fn clears_values_errors_and_focus() {
            let mut form = make_form();
            form.insert_char('X');
            form.focus_next();
            form.set_error(0, "err".into());
            form.reset();
            assert_eq!(form.value(0), "");
            assert_eq!(form.focus(), 0);
            assert!(!form.has_errors());
        }

        #[test]
        fn reset_disarms_clear_on_first_input() {
            let mut form = make_form();
            form.set_default(0, "59");
            assert!(form.fields()[0].clear_on_first_input);
            form.reset();
            assert!(
                !form.fields()[0].clear_on_first_input,
                "reset must disarm the flag"
            );
        }

        #[test]
        fn after_reset_insert_appends_not_replaces() {
            let mut form = make_form();
            form.set_default(0, "59");
            form.reset();
            form.insert_char('A');
            assert_eq!(form.value(0), "A");
            form.insert_char('B');
            assert_eq!(
                form.value(0),
                "AB",
                "after reset, typing must append normally"
            );
        }
    }

    mod rendering {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        use super::*;

        use crate::tui::test_utils::buffer_to_string;

        fn render_form(form: &Form, width: u16, height: u16) -> String {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| {
                    draw_form(form, frame, frame.area());
                })
                .unwrap();
            buffer_to_string(terminal.backend().buffer())
        }

        #[test]
        fn renders_field_labels() {
            let form = make_form();
            let output = render_form(&form, 40, 9);
            assert!(output.contains("Callsign *"), "should show required label");
            assert!(output.contains("Operator *"), "should show required label");
            assert!(
                output.contains("Park Ref"),
                "should show optional label without asterisk"
            );
            assert!(
                !output.contains("Park Ref *"),
                "optional field should not have asterisk"
            );
        }

        fn render_single_field(form: &Form, field_idx: usize, width: u16, height: u16) -> String {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| {
                    draw_form_field(form, field_idx, frame, frame.area());
                })
                .unwrap();
            buffer_to_string(terminal.backend().buffer())
        }

        #[test]
        fn draw_form_field_renders_label() {
            let form = make_form();
            let output = render_single_field(&form, 0, 40, 3);
            assert!(
                output.contains("Callsign *"),
                "single field render should show label"
            );
        }

        #[test]
        fn draw_form_field_out_of_bounds_is_noop() {
            let form = make_form();
            // Should not panic for out-of-bounds index
            let output = render_single_field(&form, 99, 40, 3);
            // Buffer should be blank (no label rendered)
            assert!(!output.contains("Callsign"));
        }

        #[test]
        fn renders_field_values() {
            let mut form = make_form();
            form.set_value(0, "W1AW");
            form.set_value(1, "KD9XYZ");
            let output = render_form(&form, 40, 9);
            assert!(output.contains("W1AW"), "should render callsign value");
            assert!(output.contains("KD9XYZ"), "should render operator value");
        }

        #[test]
        fn renders_cursor_on_focused_field() {
            let form = make_form();
            let output = render_form(&form, 40, 9);
            // The focused field should contain the block cursor character
            assert!(output.contains('\u{2588}'), "should show cursor block");
        }

        #[test]
        fn renders_error_message() {
            let mut form = make_form();
            form.set_error(0, "invalid callsign".into());
            let output = render_form(&form, 40, 9);
            assert!(
                output.contains("invalid callsign"),
                "should render error text"
            );
        }
    }

    mod fields_accessor {
        use super::*;

        #[test]
        fn returns_correct_labels() {
            let form = make_form();
            let labels: Vec<&str> = form.fields().iter().map(|f| f.label.as_str()).collect();
            assert_eq!(labels, vec!["Callsign", "Operator", "Park Ref"]);
        }

        #[test]
        fn required_flags() {
            let form = make_form();
            assert!(form.fields()[0].required);
            assert!(form.fields()[1].required);
            assert!(!form.fields()[2].required);
        }
    }
}
