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
}

impl FormField {
    /// Creates a new form field.
    pub fn new(label: impl Into<String>, required: bool) -> Self {
        Self {
            label: label.into(),
            value: String::new(),
            error: None,
            required,
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
    pub fn insert_char(&mut self, ch: char) {
        if let Some(field) = self.fields.get_mut(self.focus) {
            field.value.push(ch);
        }
    }

    /// Deletes the last character from the focused field.
    pub fn delete_char(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focus) {
            field.value.pop();
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
        }
        self.focus = 0;
    }

    /// Returns a reference to the fields.
    pub fn fields(&self) -> &[FormField] {
        &self.fields
    }
}

/// Renders a form within the given area.
#[cfg_attr(coverage_nightly, coverage(off))]
#[mutants::skip]
pub fn draw_form(form: &Form, frame: &mut Frame, area: Rect) {
    let row_height = 3_u16;
    let constraints: Vec<Constraint> = form
        .fields
        .iter()
        .map(|_| Constraint::Length(row_height))
        .collect();

    let rows = Layout::vertical(constraints).split(area);

    for (i, field) in form.fields.iter().enumerate() {
        let is_focused = i == form.focus;

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
        frame.render_widget(paragraph, rows[i]);

        // Draw error below the field if there's space
        if let Some(ref err) = field.error {
            let error_line = Paragraph::new(Span::styled(err, Style::default().fg(Color::Red)));
            // Render error overlapping the bottom of the row area
            let err_area = Rect {
                x: rows[i].x + 2,
                y: rows[i].y + row_height.saturating_sub(1),
                width: rows[i].width.saturating_sub(4),
                height: 1,
            };
            frame.render_widget(error_line, err_area);
        }
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

    // --- Focus management ---

    #[test]
    fn focus_starts_at_zero() {
        let form = make_form();
        assert_eq!(form.focus(), 0);
    }

    #[test]
    fn focus_next_advances() {
        let mut form = make_form();
        form.focus_next();
        assert_eq!(form.focus(), 1);
        form.focus_next();
        assert_eq!(form.focus(), 2);
    }

    #[test]
    fn focus_next_wraps() {
        let mut form = make_form();
        form.focus_next();
        form.focus_next();
        form.focus_next();
        assert_eq!(form.focus(), 0);
    }

    #[test]
    fn focus_prev_wraps() {
        let mut form = make_form();
        form.focus_prev();
        assert_eq!(form.focus(), 2);
    }

    #[test]
    fn focus_prev_decrements() {
        let mut form = make_form();
        form.focus_next();
        form.focus_next();
        form.focus_prev();
        assert_eq!(form.focus(), 1);
    }

    #[test]
    fn focus_next_empty_form_is_noop() {
        let mut form = Form::new(vec![]);
        form.focus_next();
        assert_eq!(form.focus(), 0);
    }

    #[test]
    fn focus_prev_empty_form_is_noop() {
        let mut form = Form::new(vec![]);
        form.focus_prev();
        assert_eq!(form.focus(), 0);
    }

    // --- Character insert/delete ---

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

    // --- Error management ---

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

    // --- Values ---

    #[test]
    fn values_returns_all_field_values() {
        let mut form = make_form();
        form.insert_char('A');
        form.focus_next();
        form.insert_char('B');
        assert_eq!(form.values(), vec!["A", "B", ""]);
    }

    #[test]
    fn value_out_of_bounds_returns_empty() {
        let form = make_form();
        assert_eq!(form.value(99), "");
    }

    // --- Reset ---

    #[test]
    fn reset_clears_values_errors_and_focus() {
        let mut form = make_form();
        form.insert_char('X');
        form.focus_next();
        form.set_error(0, "err".into());
        form.reset();
        assert_eq!(form.value(0), "");
        assert_eq!(form.focus(), 0);
        assert!(!form.has_errors());
    }

    // --- Fields accessor ---

    #[test]
    fn fields_returns_correct_labels() {
        let form = make_form();
        let labels: Vec<&str> = form.fields().iter().map(|f| f.label.as_str()).collect();
        assert_eq!(labels, vec!["Callsign", "Operator", "Park Ref"]);
    }

    #[test]
    fn field_required_flags() {
        let form = make_form();
        assert!(form.fields()[0].required);
        assert!(form.fields()[1].required);
        assert!(!form.fields()[2].required);
    }
}
