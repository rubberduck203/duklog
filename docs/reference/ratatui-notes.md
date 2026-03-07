# Ratatui Notes

Reference: https://ratatui.rs/

## Core Model

Ratatui is an **immediate-mode** terminal UI library. There is no retained widget tree — you redraw the entire UI every frame in response to events.

## Architecture

```
main loop:
  1. terminal.draw(|frame| render(frame))   // draw UI
  2. crossterm::event::read()                // block for input
  3. match event -> update app state         // handle event
  4. if quit flag set, break                 // exit check
```

- `Terminal` wraps a backend (use `CrosstermBackend`)
- `Frame` is passed to the render function — call `frame.render_widget(widget, area)`
- `Layout` splits areas: `Direction::Horizontal/Vertical`, constraints like `Length`, `Percentage`, `Min`, `Max`, `Fill`

## Key Widgets

| Widget | Use Case | Stateful? |
|---|---|---|
| `Block` | Bordered/titled container | No |
| `Paragraph` | Text display, wrapping | No |
| `List` | Scrollable item list | Yes (`ListState`) |
| `Table` | Tabular data with columns | Yes (`TableState`) |
| `Tabs` | Tab bar navigation | No |
| `Gauge` | Progress bar | No |
| `Scrollbar` | Scroll indicator | Yes (`ScrollbarState`) |

## Styling

```rust
Style::default()
    .fg(Color::Green)
    .bg(Color::Black)
    .add_modifier(Modifier::BOLD)
```

Supports true color, 256-color, and basic 16-color terminals.

## Crossterm Integration

```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

// Blocking read (no async needed):
match event::read()? {
    Event::Key(key) => handle_key(key),
    Event::Resize(w, h) => handle_resize(w, h),
    _ => {}
}
```

## Terminal Setup/Teardown

```rust
// Setup
crossterm::terminal::enable_raw_mode()?;
crossterm::execute!(stdout, EnterAlternateScreen)?;
let terminal = Terminal::new(CrosstermBackend::new(stdout))?;

// Teardown (MUST happen even on panic)
crossterm::terminal::disable_raw_mode()?;
crossterm::execute!(stdout, LeaveAlternateScreen)?;
```

Register a panic hook to restore the terminal before printing the panic message, otherwise the terminal is left in raw mode.

## Layout Example

```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3),      // status bar
        Constraint::Min(0),         // main content
        Constraint::Length(1),      // bottom bar
    ])
    .split(frame.area());
```

## No Async Needed

Ratatui is synchronous. The blocking `event::read()` is exactly what a terminal UI wants — it sleeps until the user does something. No tokio, no async runtime.

## Text Input / Cursor Editing

Ratatui has **no built-in text input widget** with cursor tracking. The official third-party widget list (`https://ratatui.rs/showcase/third-party-widgets/`) recommends:

### `tui-textarea` (recommended)

- Crate: `tui-textarea = "0.7"` — requires `ratatui ^0.29.0` (compatible with duklog's pinned version)
- Multi-line editor widget; also supports **single-line input** via a dedicated example (`single_line`)
- Features: cursor, undo/redo, Emacs-like keybindings (`Ctrl+F/B`, `Ctrl+K`), text selection, yank buffer, search (optional feature), mouse scroll
- Key API:
  ```rust
  let mut textarea = TextArea::default();
  textarea.input(key_event);   // feeds crossterm KeyEvent directly
  textarea.lines()             // &[String] — current text
  textarea.into_lines()        // Vec<String> — consume
  ```
- Backend-agnostic; works with crossterm

### Hand-rolled cursor (current approach in `export.rs`)

Track `cursor: usize` byte offset; render before/at/after as separate `Span`s; `Modifier::REVERSED` on the char under cursor; `█` only at end-of-text. ~50 lines of logic per widget; prefer `tui-textarea` for future editable fields.

> **PR #32 feedback**: check ratatui third-party widget list before hand-rolling cursor logic. Consider adopting `tui-textarea` for any future editable field screens.

## Dependencies

ratatui 0.29 uses crossterm 0.28 by default. Pin both to match:
```toml
ratatui = "0.29"
crossterm = "0.28"
```
