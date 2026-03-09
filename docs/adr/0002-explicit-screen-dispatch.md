# ADR-0002: Explicit Screen Dispatch Over ScreenState Trait

**Status:** Accepted
**Phase:** 4.0

## Context

The TUI has multiple screens (LogSelect, LogCreate, QsoEntry, QsoList, Export, Help). Routing key events and draw calls to the active screen requires a dispatch mechanism. Some screen handlers need app-level state (e.g., QsoList needs the QSO count from the current log).

## Decision

`App::handle_key` uses an explicit `match self.screen` to dispatch key events. Draw functions are free functions called from a `match` in `App::draw`. Navigation initialization logic lives in `App::navigate`.

```rust
let action = match self.screen {
    Screen::LogSelect => self.log_select.handle_key(key),
    Screen::LogCreate => self.log_create.handle_key(key),
    Screen::QsoEntry  => self.qso_entry.handle_key(key),
    Screen::QsoList   => self.qso_list.handle_key(key, count),
    Screen::Export    => self.export.handle_key(key),
    Screen::Help      => self.help.handle_key(key),
};
```

`QsoListState::handle_key` takes `qso_count: usize` as an explicit parameter — the count lives on `Log` in `App`, not on the screen state.

Adding a new screen: one arm in `handle_key`, one in `navigate`, one in `draw`. The compiler catches omissions.

## Rejected Alternative

A `ScreenState` trait with `on_enter(&mut self, ctx: &ScreenContext)` and `draw(&self, ctx: &DrawContext, frame: &mut Frame, area: Rect)`, eliminating the dispatch matches.

Reference: checkpoint commit `41e939b` contains the ScreenState trait version.

## Rationale

- Explicit matches are the honest representation of coupling. `QsoListState::handle_key` needs the QSO count from `App`; passing it explicitly at the call site makes that dependency visible.
- A `ScreenState` trait hides it: caching the count on the struct requires `App` to call `set_qso_count()` before navigating — a silent invariant with no compile-time enforcement.
- `App::navigate` is the right place for initialization that reads app-level state (`current_log`, `manager`). Moving it into `on_enter` methods would require a `ScreenContext` borrow spanning screen types, adding lifetime complexity without behavioral benefit.
- With 6 screens, the ergonomic benefit of trait dispatch does not outweigh the hidden coupling.

## When to Revisit

If the number of screens grows beyond ~10, or if a screen's initialization logic becomes complex enough to warrant encapsulation.
