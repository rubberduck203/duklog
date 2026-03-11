/// Converts a ratatui `Buffer` to a string for use in rendering assertions.
///
/// Each row is written at full terminal width (padded with spaces), separated by
/// newlines. This preserves the 2D layout, so callers can inspect specific rows
/// or use `.contains()` to assert that text appears somewhere on screen.
///
/// # Usage
///
/// ```rust,ignore
/// let output = buffer_to_string(terminal.backend().buffer());
/// assert!(output.contains("W1AW"));
/// ```
pub fn buffer_to_string(buf: &ratatui::buffer::Buffer) -> String {
    let mut s = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            s.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
        }
        s.push('\n');
    }
    s
}
