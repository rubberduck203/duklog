//! Help screen — scrollable keybinding reference.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::action::Action;
use crate::tui::app::Screen;

static LOG_SELECT_KEYS: &[(&str, &str)] = &[
    ("↑/↓", "navigate"),
    ("Enter", "open log"),
    ("n", "new log"),
    ("d", "delete log (y/n to confirm)"),
    ("q / Esc", "quit"),
    ("F1", "help"),
];

static LOG_CREATE_KEYS: &[(&str, &str)] = &[
    ("Tab / Shift-Tab", "next / prev field"),
    ("Enter", "create log"),
    ("Esc", "cancel"),
    ("F1", "help"),
];

static QSO_ENTRY_KEYS: &[(&str, &str)] = &[
    ("Tab / Shift-Tab", "next / prev field"),
    ("Enter", "log QSO"),
    ("Esc", "back to Log Select; in edit mode: cancel"),
    ("Alt+b", "next band"),
    ("Shift+Alt+B", "prev band"),
    ("Alt+m", "next mode"),
    ("Shift+Alt+M", "prev mode"),
    ("Alt+e", "open QSO list"),
    ("Alt+x", "export log"),
    ("F1", "help"),
];

static QSO_LIST_KEYS: &[(&str, &str)] = &[
    ("↑/↓", "navigate"),
    ("Home / End", "first / last"),
    ("Enter", "edit QSO"),
    ("q / Esc", "back"),
    ("F1", "help"),
];

static EXPORT_KEYS: &[(&str, &str)] = &[
    ("Enter", "export to ADIF"),
    ("q / Esc", "back"),
    ("F1", "help"),
];

static HELP_KEYS: &[(&str, &str)] = &[("↑/↓", "scroll"), ("q / Esc", "back")];

/// State for the help screen.
#[derive(Debug, Clone)]
pub struct HelpState {
    scroll: u16,
    origin: Screen,
}

impl Default for HelpState {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpState {
    /// Creates a new [`HelpState`] with scroll position at the top and origin [`Screen::LogSelect`].
    pub fn new() -> Self {
        Self {
            scroll: 0,
            origin: Screen::LogSelect,
        }
    }

    /// Returns the current scroll offset.
    pub fn scroll(&self) -> u16 {
        self.scroll
    }

    /// Returns the origin screen that opened help.
    pub fn origin(&self) -> Screen {
        self.origin
    }

    /// Sets the origin screen to return to when help is dismissed.
    pub fn set_origin(&mut self, screen: Screen) {
        self.origin = screen;
    }

    /// Resets the scroll position to the top.
    pub fn reset(&mut self) {
        self.scroll = 0;
    }

    /// Handles a key event, returning an [`Action`] for the app to apply.
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                Action::None
            }
            KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
                Action::None
            }
            KeyCode::Char('q') | KeyCode::Esc => Action::Navigate(self.origin),
            _ => Action::None,
        }
    }
}

fn screen_name(screen: Screen) -> &'static str {
    match screen {
        Screen::LogSelect => "Log Select",
        Screen::LogCreate => "Log Create",
        Screen::QsoEntry => "QSO Entry",
        Screen::QsoList => "QSO List",
        Screen::Export => "Export",
        Screen::Help => "Help",
    }
}

fn build_section(title: &'static str, keys: &[(&'static str, &'static str)]) -> Vec<Line<'static>> {
    let header_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default().fg(Color::Yellow);
    let dim_style = Style::default().fg(Color::DarkGray);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(title, header_style)),
    ];
    for (key, desc) in keys {
        lines.push(Line::from(vec![
            Span::styled(format!("  {key:<20}"), key_style),
            Span::styled(*desc, dim_style),
        ]));
    }
    lines
}

fn help_content(origin: Screen) -> Vec<Line<'static>> {
    match origin {
        Screen::LogSelect => build_section("Log Select", LOG_SELECT_KEYS),
        Screen::LogCreate => build_section("Log Create", LOG_CREATE_KEYS),
        Screen::QsoEntry => build_section("QSO Entry", QSO_ENTRY_KEYS),
        Screen::QsoList => build_section("QSO List", QSO_LIST_KEYS),
        Screen::Export => build_section("Export", EXPORT_KEYS),
        Screen::Help => build_section("Help", HELP_KEYS),
    }
}

/// Renders the help screen.
#[mutants::skip]
pub fn draw_help(state: &HelpState, frame: &mut Frame, area: Rect) {
    let title = format!(" Help – {} ", screen_name(state.origin()));
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [content_area, footer_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);

    let content_lines = help_content(state.origin());
    let total = content_lines.len() as u16;
    let height = content_area.height;
    let capped_scroll = state.scroll().min(total.saturating_sub(height));

    let paragraph = Paragraph::new(content_lines).scroll((capped_scroll, 0));
    frame.render_widget(paragraph, content_area);

    let footer =
        Paragraph::new("↑/↓: scroll  q/Esc: back").style(Style::default().fg(Color::DarkGray));
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

    mod construction {
        use super::*;

        #[test]
        fn new_initializes_scroll_to_zero() {
            let state = HelpState::new();
            assert_eq!(state.scroll(), 0);
        }

        #[test]
        fn new_initializes_origin_to_log_select() {
            let state = HelpState::new();
            assert_eq!(state.origin(), Screen::LogSelect);
        }

        #[test]
        fn default_works() {
            let state = HelpState::default();
            assert_eq!(state.scroll(), 0);
            assert_eq!(state.origin(), Screen::LogSelect);
        }
    }

    mod set_origin {
        use super::*;

        #[test]
        fn set_origin_stores_screen() {
            let mut state = HelpState::new();
            state.set_origin(Screen::QsoEntry);
            assert_eq!(state.origin(), Screen::QsoEntry);
        }

        #[test]
        fn origin_accessor_returns_value() {
            let mut state = HelpState::new();
            state.set_origin(Screen::Export);
            assert_eq!(state.origin(), Screen::Export);
        }
    }

    mod handle_key {
        use super::*;

        #[test]
        fn up_decrements_scroll() {
            let mut state = HelpState::new();
            state.scroll = 5;
            let action = state.handle_key(press(KeyCode::Up));
            assert_eq!(action, Action::None);
            assert_eq!(state.scroll(), 4);
        }

        #[test]
        fn up_at_zero_saturates() {
            let mut state = HelpState::new();
            let action = state.handle_key(press(KeyCode::Up));
            assert_eq!(action, Action::None);
            assert_eq!(state.scroll(), 0);
        }

        #[test]
        fn down_increments_scroll() {
            let mut state = HelpState::new();
            let action = state.handle_key(press(KeyCode::Down));
            assert_eq!(action, Action::None);
            assert_eq!(state.scroll(), 1);
        }

        #[test]
        fn q_navigates_to_log_select() {
            let mut state = HelpState::new();
            let action = state.handle_key(press(KeyCode::Char('q')));
            assert_eq!(action, Action::Navigate(Screen::LogSelect));
        }

        #[test]
        fn esc_navigates_to_log_select() {
            let mut state = HelpState::new();
            let action = state.handle_key(press(KeyCode::Esc));
            assert_eq!(action, Action::Navigate(Screen::LogSelect));
        }

        #[test]
        fn q_navigates_to_origin() {
            let mut state = HelpState::new();
            state.set_origin(Screen::QsoEntry);
            let action = state.handle_key(press(KeyCode::Char('q')));
            assert_eq!(action, Action::Navigate(Screen::QsoEntry));
        }

        #[test]
        fn esc_navigates_to_origin() {
            let mut state = HelpState::new();
            state.set_origin(Screen::QsoList);
            let action = state.handle_key(press(KeyCode::Esc));
            assert_eq!(action, Action::Navigate(Screen::QsoList));
        }

        #[test]
        fn unknown_key_returns_none() {
            let mut state = HelpState::new();
            let action = state.handle_key(press(KeyCode::Char('x')));
            assert_eq!(action, Action::None);
            assert_eq!(state.scroll(), 0);
        }
    }

    mod reset {
        use super::*;

        #[test]
        fn reset_sets_scroll_to_zero() {
            let mut state = HelpState::new();
            state.handle_key(press(KeyCode::Down));
            state.handle_key(press(KeyCode::Down));
            assert_eq!(state.scroll(), 2);
            state.reset();
            assert_eq!(state.scroll(), 0);
        }
    }

    mod screen_name_fn {
        use super::*;

        #[test]
        fn all_variants_have_expected_names() {
            assert_eq!(screen_name(Screen::LogSelect), "Log Select");
            assert_eq!(screen_name(Screen::LogCreate), "Log Create");
            assert_eq!(screen_name(Screen::QsoEntry), "QSO Entry");
            assert_eq!(screen_name(Screen::QsoList), "QSO List");
            assert_eq!(screen_name(Screen::Export), "Export");
            assert_eq!(screen_name(Screen::Help), "Help");
        }
    }

    mod help_content_fn {
        use super::*;

        fn content_text(screen: Screen) -> String {
            help_content(screen)
                .into_iter()
                .flat_map(|l| l.spans.into_iter())
                .map(|s| s.content.into_owned())
                .collect()
        }

        #[test]
        fn each_screen_returns_nonempty_content() {
            let screens = [
                Screen::LogSelect,
                Screen::LogCreate,
                Screen::QsoEntry,
                Screen::QsoList,
                Screen::Export,
                Screen::Help,
            ];
            for screen in screens {
                assert!(
                    !help_content(screen).is_empty(),
                    "{screen:?} should have content"
                );
            }
        }

        #[test]
        fn content_includes_section_title() {
            assert!(content_text(Screen::LogSelect).contains("Log Select"));
            assert!(content_text(Screen::QsoEntry).contains("QSO Entry"));
        }

        #[test]
        fn qso_entry_content_excludes_other_sections() {
            let text = content_text(Screen::QsoEntry);
            assert!(
                !text.contains("Log Create"),
                "should not include Log Create"
            );
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

        fn render_help(state: &HelpState, width: u16, height: u16) -> String {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| {
                    draw_help(state, frame, frame.area());
                })
                .unwrap();
            buffer_to_string(terminal.backend().buffer())
        }

        #[test]
        fn title_contains_help() {
            let state = HelpState::new();
            let output = render_help(&state, 80, 30);
            assert!(output.contains("Help"), "should show Help title");
        }

        #[test]
        fn title_includes_origin_screen_name() {
            let mut state = HelpState::new();
            state.set_origin(Screen::QsoEntry);
            let output = render_help(&state, 80, 30);
            assert!(
                output.contains("QSO Entry"),
                "title should include origin screen name"
            );
        }

        #[test]
        fn content_contains_log_select() {
            let state = HelpState::new();
            let output = render_help(&state, 80, 30);
            assert!(
                output.contains("Log Select"),
                "should show Log Select section"
            );
        }

        #[test]
        fn content_contains_qso_entry() {
            let mut state = HelpState::new();
            state.set_origin(Screen::QsoEntry);
            let output = render_help(&state, 80, 30);
            assert!(
                output.contains("QSO Entry"),
                "should show QSO Entry section"
            );
        }

        #[test]
        fn content_contains_qso_list() {
            let mut state = HelpState::new();
            state.set_origin(Screen::QsoList);
            let output = render_help(&state, 80, 30);
            assert!(output.contains("QSO List"), "should show QSO List section");
        }

        #[test]
        fn content_contains_export() {
            let mut state = HelpState::new();
            state.set_origin(Screen::Export);
            let output = render_help(&state, 80, 30);
            assert!(
                output.contains("export to ADIF"),
                "should show Export section content"
            );
        }

        #[test]
        fn renders_only_origin_screen_section() {
            let mut state = HelpState::new();
            state.set_origin(Screen::QsoEntry);
            let output = render_help(&state, 80, 30);
            assert!(
                output.contains("QSO Entry"),
                "should show QSO Entry section"
            );
            assert!(
                !output.contains("Log Create"),
                "should not show Log Create section"
            );
        }

        #[test]
        fn footer_contains_q_and_esc() {
            let state = HelpState::new();
            let output = render_help(&state, 80, 30);
            assert!(output.contains('q'), "footer should mention q");
            assert!(output.contains("Esc"), "footer should mention Esc");
        }
    }
}
