//! Status bar widget — persistent one-line log context display.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Data passed to the status bar widget; decoupled from `Log` for Phase 4 extensibility.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusBarContext {
    /// The active station callsign.
    pub callsign: String,
    /// The active park reference, if any.
    pub park_ref: Option<String>,
    /// Today's QSO count (from `Log::qso_count_today`).
    pub qso_count: usize,
    /// Whether the log is activated (from `Log::is_activated`).
    pub is_activated: bool,
}

/// Renders a one-line status bar showing the active log context.
///
/// Display format (left-aligned, Cyan):
/// - Activated, with park:     `[K-0001] W1AW  ACTIVATED`  (ACTIVATED in Green)
/// - Not activated, with park: `[K-0001] W1AW  7/10 QSOs`
/// - No park ref:              `W1AW  5 QSOs`
/// - No park, activated:       `W1AW  ACTIVATED`
///
/// Renders nothing if `ctx.callsign` is empty (no active log).
#[mutants::skip]
pub fn draw_status_bar(ctx: &StatusBarContext, frame: &mut Frame, area: Rect) {
    if ctx.callsign.is_empty() {
        return;
    }

    let cyan = Style::default().fg(Color::Cyan);
    let green = Style::default().fg(Color::Green);

    let mut spans: Vec<Span> = Vec::new();

    if let Some(park) = &ctx.park_ref {
        spans.push(Span::styled(format!("[{park}] "), cyan));
    }
    spans.push(Span::styled(ctx.callsign.clone(), cyan));
    spans.push(Span::styled("  ", cyan));

    if ctx.is_activated {
        spans.push(Span::styled("ACTIVATED", green));
    } else if ctx.park_ref.is_some() {
        spans.push(Span::styled(format!("{}/10 QSOs", ctx.qso_count), cyan));
    } else {
        spans.push(Span::styled(format!("{} QSOs", ctx.qso_count), cyan));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[cfg(test)]
mod tests {
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

    fn render_status_bar(ctx: &StatusBarContext, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                draw_status_bar(ctx, frame, frame.area());
            })
            .unwrap();
        buffer_to_string(terminal.backend().buffer())
    }

    #[test]
    fn renders_activated_with_park() {
        let ctx = StatusBarContext {
            callsign: "W1AW".to_string(),
            park_ref: Some("K-0001".to_string()),
            qso_count: 10,
            is_activated: true,
        };
        let output = render_status_bar(&ctx, 40, 1);
        assert!(
            output.contains("[K-0001]"),
            "should show park ref in brackets"
        );
        assert!(output.contains("ACTIVATED"), "should show ACTIVATED");
    }

    #[test]
    fn renders_count_with_park() {
        let ctx = StatusBarContext {
            callsign: "W1AW".to_string(),
            park_ref: Some("K-0001".to_string()),
            qso_count: 7,
            is_activated: false,
        };
        let output = render_status_bar(&ctx, 40, 1);
        assert!(
            output.contains("[K-0001]"),
            "should show park ref in brackets"
        );
        assert!(output.contains("7/10"), "should show count out of 10");
    }

    #[test]
    fn renders_without_park() {
        let ctx = StatusBarContext {
            callsign: "W1AW".to_string(),
            park_ref: None,
            qso_count: 5,
            is_activated: false,
        };
        let output = render_status_bar(&ctx, 40, 1);
        assert!(output.contains("W1AW"), "should show callsign");
        assert!(output.contains("5 QSOs"), "should show QSO count");
        assert!(
            !output.contains('['),
            "should not show brackets without park"
        );
    }

    #[test]
    fn renders_no_log() {
        let ctx = StatusBarContext::default();
        // Empty callsign → renders blank, no panic.
        let output = render_status_bar(&ctx, 40, 1);
        assert!(
            !output.contains("ACTIVATED"),
            "blank context should not show ACTIVATED"
        );
    }
}
