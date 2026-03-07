//! Status bar widget — persistent one-line log context display.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::model::Log;

/// Data passed to the status bar widget.
///
/// Construct via [`StatusBarContext::from_log`] or [`Default`] for an empty bar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusBarContext {
    /// Label shown in brackets: park ref for POTA, sent exchange for FD/WFD, callsign for General.
    pub context_label: String,
    /// QSO count: today's count for POTA; total count for all other log types.
    pub qso_count: usize,
    /// When `true`, format the count as `N/10 QSOs` (POTA activation threshold display).
    pub pota_mode: bool,
    /// When `true`, show `ACTIVATED` instead of the QSO count (POTA only).
    pub is_activated: bool,
}

impl StatusBarContext {
    /// Constructs a [`StatusBarContext`] from an active log.
    ///
    /// - POTA: `context_label` = park ref (or callsign if none); `qso_count` = today's QSOs; `pota_mode` = true
    /// - FD / WFD: `context_label` = sent exchange; `qso_count` = total QSOs; `pota_mode` = false
    /// - General: `context_label` = station callsign; `qso_count` = total QSOs; `pota_mode` = false
    pub fn from_log(log: &Log) -> Self {
        let is_pota = matches!(log, Log::Pota(_));
        Self {
            context_label: log.display_label(),
            qso_count: if is_pota {
                log.qso_count_today()
            } else {
                log.header().qsos.len()
            },
            pota_mode: is_pota,
            is_activated: log.is_activated(),
        }
    }
}

/// Renders a one-line status bar showing the active log context.
///
/// Display format (left-aligned):
/// - POTA activated:       `[K-0001]  ACTIVATED`  (ACTIVATED in Green)
/// - POTA not activated:   `[K-0001]  7/10 QSOs`
/// - FD / WFD:             `[1B EPA]  42 QSOs`
/// - General:              `[W1AW]  5 QSOs`
///
/// Renders nothing if `ctx.context_label` is empty (no active log).
#[mutants::skip]
pub fn draw_status_bar(ctx: &StatusBarContext, frame: &mut Frame, area: Rect) {
    if ctx.context_label.is_empty() {
        return;
    }

    let cyan = Style::default().fg(Color::Cyan);
    let green = Style::default().fg(Color::Green);

    let (count_str, count_style) = if ctx.is_activated {
        ("ACTIVATED".to_string(), green)
    } else if ctx.pota_mode {
        (format!("{}/10 QSOs", ctx.qso_count), cyan)
    } else {
        (format!("{} QSOs", ctx.qso_count), cyan)
    };

    let line = Line::from(vec![
        Span::styled(format!("[{}]  ", ctx.context_label), cyan),
        Span::styled(count_str, count_style),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::model::{
        FdClass, FdPowerCategory, FieldDayLog, GeneralLog, LogHeader, PotaLog, WfdClass, WfdLog,
    };

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

    fn make_header(callsign: &str) -> LogHeader {
        LogHeader {
            station_callsign: callsign.into(),
            operator: None,
            grid_square: "FN31".into(),
            qsos: vec![],
            created_at: chrono::Utc::now(),
            log_id: "test".into(),
        }
    }

    #[test]
    fn renders_activated_with_park() {
        let ctx = StatusBarContext {
            context_label: "K-0001".to_string(),
            qso_count: 10,
            pota_mode: true,
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
            context_label: "K-0001".to_string(),
            qso_count: 7,
            pota_mode: true,
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
    fn renders_general_log() {
        let ctx = StatusBarContext {
            context_label: "W1AW".to_string(),
            qso_count: 5,
            pota_mode: false,
            is_activated: false,
        };
        let output = render_status_bar(&ctx, 40, 1);
        assert!(
            output.contains("[W1AW]"),
            "should show callsign in brackets"
        );
        assert!(output.contains("5 QSOs"), "should show QSO count");
        assert!(!output.contains("/10"), "should not show POTA threshold");
    }

    #[test]
    fn renders_fd_exchange() {
        let ctx = StatusBarContext {
            context_label: "1B EPA".to_string(),
            qso_count: 42,
            pota_mode: false,
            is_activated: false,
        };
        let output = render_status_bar(&ctx, 40, 1);
        assert!(
            output.contains("[1B EPA]"),
            "should show exchange in brackets"
        );
        assert!(output.contains("42 QSOs"), "should show total QSO count");
        assert!(!output.contains("/10"), "should not show POTA threshold");
    }

    #[test]
    fn renders_no_log() {
        let ctx = StatusBarContext::default();
        let output = render_status_bar(&ctx, 40, 1);
        assert!(
            !output.contains("ACTIVATED"),
            "blank context should not show ACTIVATED"
        );
        assert!(
            !output.contains("QSOs"),
            "blank context should not show QSOs"
        );
    }

    mod from_log {
        use super::*;

        #[test]
        fn pota_with_park_uses_park_ref_as_label() {
            let log = Log::Pota(PotaLog {
                header: make_header("W1AW"),
                park_ref: "K-0001".into(),
            });
            let ctx = StatusBarContext::from_log(&log);
            assert_eq!(ctx.context_label, "K-0001");
            assert!(ctx.pota_mode, "should be pota_mode");
            assert!(!ctx.is_activated);
        }

        #[test]
        fn pota_uses_park_ref_as_label() {
            let log = Log::Pota(PotaLog {
                header: make_header("W1AW"),
                park_ref: "K-0001".into(),
            });
            let ctx = StatusBarContext::from_log(&log);
            assert_eq!(ctx.context_label, "K-0001");
            assert!(ctx.pota_mode, "should be pota_mode");
        }

        #[test]
        fn general_uses_callsign_as_label_and_total_count() {
            let log = Log::General(GeneralLog {
                header: make_header("W1AW"),
            });
            let ctx = StatusBarContext::from_log(&log);
            assert_eq!(ctx.context_label, "W1AW");
            assert!(!ctx.pota_mode, "should not be pota_mode");
            assert!(!ctx.is_activated);
        }

        #[test]
        fn field_day_uses_exchange_as_label_and_total_count() {
            let log = Log::FieldDay(
                FieldDayLog::new(
                    "W1AW".into(),
                    None,
                    1,
                    FdClass::B,
                    "EPA".into(),
                    FdPowerCategory::Low,
                    "FN31".into(),
                )
                .unwrap(),
            );
            let ctx = StatusBarContext::from_log(&log);
            assert_eq!(ctx.context_label, "1B EPA");
            assert!(!ctx.pota_mode, "should not be pota_mode");
        }

        #[test]
        fn wfd_uses_exchange_as_label_and_total_count() {
            let log = Log::WinterFieldDay(
                WfdLog::new(
                    "W1AW".into(),
                    None,
                    1,
                    WfdClass::H,
                    "EPA".into(),
                    "FN31".into(),
                )
                .unwrap(),
            );
            let ctx = StatusBarContext::from_log(&log);
            assert_eq!(ctx.context_label, "1H EPA");
            assert!(!ctx.pota_mode, "should not be pota_mode");
        }

        #[test]
        fn pota_mode_is_true_only_for_pota() {
            let general = Log::General(GeneralLog {
                header: make_header("W1AW"),
            });
            let pota = Log::Pota(PotaLog {
                header: make_header("W1AW"),
                park_ref: "K-0001".into(),
            });
            assert!(!StatusBarContext::from_log(&general).pota_mode);
            assert!(StatusBarContext::from_log(&pota).pota_mode);
        }
    }
}
