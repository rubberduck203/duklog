//! Reusable form widget for text input screens.

use std::fmt;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

/// A single input field within a [`Form`].
pub trait Field: fmt::Debug {
    /// Display label shown above/beside the input.
    fn label(&self) -> &str;
    /// Current text value.
    fn value(&self) -> &str;
    /// Whether the field must be non-empty on submit.
    fn required(&self) -> bool;
    /// Validation error message, if any.
    fn error(&self) -> Option<&str>;
    /// Sets a validation error on this field.
    fn set_error(&mut self, msg: String);
    /// Clears the validation error.
    fn clear_error(&mut self);
    /// Appends a character to the field value.
    fn insert_char(&mut self, ch: char);
    /// Removes the last character from the field value.
    fn delete_char(&mut self);
    /// Explicitly sets the field value (e.g. loading an existing record for editing).
    fn set_value(&mut self, value: &str);
    /// Clears the field value.
    fn clear_value(&mut self);
    /// Resets the field to its initial state (empty value for text fields; default
    /// report for RST fields).
    fn reset(&mut self);
    /// Updates the displayed default and marks the field as unedited.
    ///
    /// Used by mode-cycling to update RST defaults without overwriting user edits.
    /// For non-RST fields this is a no-op.
    fn set_mode_default(&mut self, _default: &str) {}
}

/// A generic single-line text input field.
#[derive(Debug)]
pub struct FormField {
    label: String,
    value: String,
    error: Option<String>,
    required: bool,
}

impl FormField {
    /// Creates a new text field with the given label and required flag.
    pub fn new(label: impl Into<String>, required: bool) -> Self {
        Self {
            label: label.into(),
            value: String::new(),
            error: None,
            required,
        }
    }
}

impl Field for FormField {
    fn label(&self) -> &str {
        &self.label
    }
    fn value(&self) -> &str {
        &self.value
    }
    fn required(&self) -> bool {
        self.required
    }
    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    fn set_error(&mut self, msg: String) {
        self.error = Some(msg);
    }
    fn clear_error(&mut self) {
        self.error = None;
    }
    fn insert_char(&mut self, ch: char) {
        self.value.push(ch);
    }
    fn delete_char(&mut self) {
        self.value.pop();
    }
    fn set_value(&mut self, value: &str) {
        self.value = value.to_string();
    }
    fn clear_value(&mut self) {
        self.value.clear();
    }
    fn reset(&mut self) {
        self.value.clear();
        self.error = None;
    }
}

/// An RST signal-report input field.
///
/// Pre-populated with the mode's default report (e.g. `"59"` for SSB). The first
/// keystroke — insert or backspace — replaces the default entirely, so operators
/// who want to change it start typing immediately without backspacing. Operators
/// who accept the default Tab past the field and the value is preserved unchanged.
///
/// When the mode changes (via [`Field::set_mode_default`]), the displayed value and
/// stored default update automatically if the operator has not yet edited the field.
/// Once edited, the operator's report is preserved across mode changes.
#[derive(Debug)]
pub struct RstField {
    label: String,
    value: String,
    default: String,
    edited: bool,
    error: Option<String>,
}

impl RstField {
    /// Creates a new RST field pre-populated with `default`.
    pub fn new(label: impl Into<String>, default: impl Into<String>) -> Self {
        let default = default.into();
        Self {
            label: label.into(),
            value: default.clone(),
            default,
            edited: false,
            error: None,
        }
    }
}

impl Field for RstField {
    fn label(&self) -> &str {
        &self.label
    }
    fn value(&self) -> &str {
        &self.value
    }
    fn required(&self) -> bool {
        true
    }
    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    fn set_error(&mut self, msg: String) {
        self.error = Some(msg);
    }
    fn clear_error(&mut self) {
        self.error = None;
    }
    fn insert_char(&mut self, ch: char) {
        if !self.edited {
            self.value.clear();
            self.edited = true;
        }
        self.value.push(ch);
    }
    fn delete_char(&mut self) {
        if !self.edited {
            self.value.clear();
            self.edited = true;
        } else {
            self.value.pop();
        }
    }
    fn set_value(&mut self, value: &str) {
        self.value = value.to_string();
        self.edited = true;
    }
    fn clear_value(&mut self) {
        self.value.clear();
        self.edited = false;
    }
    fn reset(&mut self) {
        self.value = self.default.clone();
        self.edited = false;
        self.error = None;
    }
    fn set_mode_default(&mut self, default: &str) {
        if !self.edited {
            self.default = default.to_string();
            self.value = self.default.clone();
        }
    }
}

/// A multi-field text form with focus management.
///
/// Fields are stored as `Box<dyn Field>` because a form always contains a heterogeneous
/// mix of [`FormField`] and [`RstField`] values. A generic type parameter `F: Field`
/// would require all fields to share the same concrete type, which forces the boxing
/// back to the call site anyway (as `Form<Box<dyn Field>>`). The `Box<dyn Field>`
/// approach is idiomatic for owned, mixed-type collections in Rust.
#[derive(Debug)]
pub struct Form {
    fields: Vec<Box<dyn Field>>,
    focus: usize,
}

impl Form {
    /// Creates a new form with the given fields. Focus starts on the first field.
    pub fn new(fields: Vec<Box<dyn Field>>) -> Self {
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

    /// Inserts a character into the focused field.
    pub fn insert_char(&mut self, ch: char) {
        if let Some(field) = self.fields.get_mut(self.focus) {
            field.insert_char(ch);
        }
    }

    /// Deletes the last character from the focused field.
    pub fn delete_char(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focus) {
            field.delete_char();
        }
    }

    /// Sets an error message on a field by index.
    pub fn set_error(&mut self, index: usize, error: String) {
        if let Some(field) = self.fields.get_mut(index) {
            field.set_error(error);
        }
    }

    /// Clears all field errors.
    pub fn clear_errors(&mut self) {
        for field in &mut self.fields {
            field.clear_error();
        }
    }

    /// Returns `true` if any field has an error set.
    pub fn has_errors(&self) -> bool {
        self.fields.iter().any(|f| f.error().is_some())
    }

    /// Explicitly sets the value of the field at `index`.
    ///
    /// For [`RstField`]s this marks the field as edited, so subsequent mode
    /// changes will not overwrite the operator's value.
    /// No-op if `index` is out of bounds.
    pub fn set_value(&mut self, index: usize, value: impl AsRef<str>) {
        if let Some(field) = self.fields.get_mut(index) {
            field.set_value(value.as_ref());
        }
    }

    /// Clears the value of the field at `index`. No-op if out of bounds.
    pub fn clear_value(&mut self, index: usize) {
        if let Some(field) = self.fields.get_mut(index) {
            field.clear_value();
        }
    }

    /// Returns the value of the field at `index`, or an empty string if out of bounds.
    pub fn value(&self, index: usize) -> &str {
        self.fields.get(index).map(|f| f.value()).unwrap_or("")
    }

    /// Returns all field values as a vector of string slices.
    pub fn values(&self) -> Vec<&str> {
        self.fields.iter().map(|f| f.value()).collect()
    }

    /// Forwards a mode-default update to the field at `index`.
    ///
    /// For [`RstField`]s, updates the displayed value and stored default only if
    /// the operator has not yet edited the field. For all other field types this
    /// is a no-op.
    /// No-op if `index` is out of bounds.
    pub fn set_mode_default(&mut self, index: usize, default: &str) {
        if let Some(field) = self.fields.get_mut(index) {
            field.set_mode_default(default);
        }
    }

    /// Resets the field at `index` to its initial state. No-op if out of bounds.
    ///
    /// For [`RstField`]s this restores the current default report and clears the
    /// edited flag. For [`FormField`]s this clears the value.
    pub fn reset_field(&mut self, index: usize) {
        if let Some(field) = self.fields.get_mut(index) {
            field.reset();
        }
    }

    /// Resets all fields and returns focus to the first field.
    pub fn reset(&mut self) {
        for field in &mut self.fields {
            field.reset();
        }
        self.focus = 0;
    }

    /// Returns a reference to the field list.
    pub fn fields(&self) -> &[Box<dyn Field>] {
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

    let border_color = if field.error().is_some() {
        Color::Red
    } else if is_focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let label = if field.required() {
        format!("{} *", field.label())
    } else {
        field.label().to_string()
    };

    let block = Block::default()
        .title(label)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let mut spans = vec![Span::raw(field.value())];
    if is_focused {
        spans.push(Span::styled(
            "\u{2588}",
            Style::default().add_modifier(Modifier::SLOW_BLINK),
        ));
    }

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    frame.render_widget(paragraph, area);

    // Draw error below the field if there's space
    if let Some(err) = field.error() {
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
            Box::new(FormField::new("Callsign", true)),
            Box::new(FormField::new("Operator", true)),
            Box::new(FormField::new("Park Ref", false)),
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
    }

    mod rst_field {
        use super::*;

        fn make_rst_form() -> Form {
            Form::new(vec![
                Box::new(FormField::new("Their Call", true)),
                Box::new(RstField::new("RST Sent", "59")),
                Box::new(RstField::new("RST Rcvd", "59")),
            ])
        }

        #[test]
        fn first_insert_replaces_default() {
            let mut form = make_rst_form();
            form.set_focus(1);
            form.insert_char('5');
            assert_eq!(
                form.value(1),
                "5",
                "first char should replace default, not append"
            );
            form.insert_char('7');
            assert_eq!(form.value(1), "57");
        }

        #[test]
        fn backspace_on_default_clears_entirely() {
            let mut form = make_rst_form();
            form.set_focus(1);
            form.delete_char();
            assert_eq!(
                form.value(1),
                "",
                "backspace on default should clear entirely"
            );
        }

        #[test]
        fn subsequent_inserts_append_after_first() {
            let mut form = make_rst_form();
            form.set_focus(1);
            form.insert_char('5');
            form.insert_char('9');
            assert_eq!(form.value(1), "59");
        }

        #[test]
        fn tab_past_preserves_default() {
            let form = make_rst_form();
            assert_eq!(form.value(1), "59");
            assert_eq!(form.value(2), "59");
        }

        #[test]
        fn set_value_marks_as_edited() {
            let mut form = make_rst_form();
            form.set_value(1, "57");
            form.set_focus(1);
            form.insert_char('X');
            assert_eq!(
                form.value(1),
                "57X",
                "set_value should mark as edited — insert must append"
            );
        }

        #[test]
        fn set_mode_default_updates_when_unedited() {
            let mut form = make_rst_form();
            form.set_mode_default(1, "599");
            assert_eq!(form.value(1), "599");
        }

        #[test]
        fn set_mode_default_preserves_when_edited() {
            let mut form = make_rst_form();
            form.set_focus(1);
            form.insert_char('5'); // now edited
            form.set_mode_default(1, "599");
            assert_eq!(form.value(1), "5", "edited RST must survive mode change");
        }

        #[test]
        fn reset_field_restores_default() {
            let mut form = make_rst_form();
            form.set_focus(1);
            form.insert_char('5'); // edited
            form.reset_field(1);
            assert_eq!(form.value(1), "59");
            // flag is cleared — next insert should replace again
            form.insert_char('7');
            assert_eq!(form.value(1), "7");
        }

        #[test]
        fn set_mode_default_updates_default_for_future_reset() {
            let mut form = make_rst_form();
            form.set_mode_default(1, "599");
            form.set_focus(1);
            form.insert_char('6'); // edit
            form.reset_field(1);
            assert_eq!(form.value(1), "599", "reset should restore updated default");
        }

        #[test]
        fn non_rst_field_unaffected_by_set_mode_default() {
            let mut form = make_rst_form();
            form.set_mode_default(0, "anything"); // no-op on FormField
            assert_eq!(form.value(0), "");
        }

        #[test]
        fn rst_required_is_always_true() {
            let form = make_rst_form();
            assert!(form.fields()[1].required());
            assert!(form.fields()[2].required());
        }
    }

    mod errors {
        use super::*;

        #[test]
        fn set_error_on_field() {
            let mut form = make_form();
            form.set_error(0, "bad callsign".into());
            assert!(form.has_errors());
            assert_eq!(form.fields()[0].error(), Some("bad callsign"));
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
        fn set_mode_default_out_of_bounds_is_noop() {
            let mut form = make_form();
            form.set_mode_default(99, "59");
            assert_eq!(form.values(), vec!["", "", ""]);
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
        fn rst_field_reset_restores_default_not_empty() {
            let mut form = Form::new(vec![Box::new(RstField::new("RST Sent", "59"))]);
            form.insert_char('5'); // edit
            form.reset();
            assert_eq!(
                form.value(0),
                "59",
                "reset on RstField must restore default"
            );
        }

        #[test]
        fn rst_field_after_reset_first_key_replaces() {
            let mut form = Form::new(vec![Box::new(RstField::new("RST Sent", "59"))]);
            form.insert_char('5'); // edit
            form.reset();
            form.insert_char('7'); // first key after reset should replace
            assert_eq!(form.value(0), "7");
        }
    }

    mod fields_accessor {
        use super::*;

        #[test]
        fn returns_correct_labels() {
            let form = make_form();
            let labels: Vec<&str> = form.fields().iter().map(|f| f.label()).collect();
            assert_eq!(labels, vec!["Callsign", "Operator", "Park Ref"]);
        }

        #[test]
        fn required_flags() {
            let form = make_form();
            assert!(form.fields()[0].required());
            assert!(form.fields()[1].required());
            assert!(!form.fields()[2].required());
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

        #[test]
        fn rst_field_renders_default_value() {
            let form = Form::new(vec![Box::new(RstField::new("RST Sent", "59"))]);
            let output = render_single_field(&form, 0, 40, 3);
            assert!(
                output.contains("RST Sent *"),
                "RST field is always required"
            );
            assert!(output.contains("59"), "should render the default RST value");
        }
    }
}
