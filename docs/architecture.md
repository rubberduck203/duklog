# Architecture

## Overview

duklog is an offline ham radio logging TUI for POTA activations. It is a single-binary Rust application with no network dependencies.

## Module Layout

```
src/
  main.rs       Terminal setup/teardown, panic hook
  lib.rs        Module re-exports, run() entry point
  model/        Domain types: Log, Qso, Band, Mode, validation
  adif/         ADIF file format writer (pure functions, no I/O)
  storage/      JSONL persistence to XDG paths, ADIF file export
  tui/          Application state, event loop, UI rendering
    screens/    Individual screen implementations
    widgets/    Reusable UI components
```

## Data Flow

```
User Input → TUI Event Loop → Model Mutation → Auto-Save (Storage)
                                    ↓
                              ADIF Export (on demand)
```

1. **Input**: Crossterm captures keyboard events
2. **Dispatch**: TUI event loop routes events to the active screen
3. **Model**: Screen handlers mutate the domain model (Log, Qso)
4. **Persistence**: After every model mutation, storage layer auto-saves to JSONL
5. **Export**: User-triggered ADIF export calls the pure ADIF writer, then writes to disk

## Domain Model

`Log` is an enum over concrete log types, each carrying a shared `LogHeader` plus type-specific fields:

```
LogHeader          — station_callsign, operator, grid_square, qsos, created_at, log_id
GeneralLog         — header: LogHeader  (no type-specific fields)
PotaLog            — header: LogHeader, park_ref: Option<String>
FieldDayLog        — header: LogHeader, tx_count: u8, class: FdClass, section: String, power: FdPowerCategory
WfdLog             — header: LogHeader, tx_count: u8, class: WfdClass, section: String
Log enum           — General(GeneralLog) | Pota(PotaLog) | FieldDay(FieldDayLog) | WinterFieldDay(WfdLog)
```

Qso carries two optional contest fields: `exchange_rcvd: Option<String>` (received exchange verbatim, e.g. `"3A CT"`) and `frequency: Option<u32>` (kHz, required for WFD ADIF). Both default to `None` for POTA and General logs; they are populated during QSO entry for contest log types (Phase 4.3).

Serialization for storage lives in `src/storage/manager.rs` via a flat `LogMetadata` struct with optional FD/WFD fields and a `StoredLogType` discriminant. Existing JSONL files without `log_type` default to `Log::Pota` for backward compatibility. Shared fields are accessed via `log.header()` / `log.header_mut()`.

### Duplicate detection scoping

`Log::find_duplicates` is type-aware:
- **POTA / General**: scoped to today (UTC) — matches the existing behaviour
- **FieldDay / WinterFieldDay**: scoped across the entire log — these events span two UTC calendar days

## Screen Architecture

The TUI uses explicit `match self.screen` dispatch in `App`, with an `Action` enum for screen-to-app communication. Each screen module owns its state struct and a free draw function.

### Dispatch Pattern

`App::handle_key` routes key events to the active screen via a match:

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

`QsoListState::handle_key` takes an explicit `qso_count: usize` parameter because the list needs to know the count to clamp cursor movement, and the count lives on the `Log` in `App` — not on the screen state. Passing it explicitly at the call site makes the dependency visible; caching it on the struct would hide it.

`navigate()` handles screen transitions with per-screen initialization logic (reset form, reload log list, prepare export path). Adding a new screen means one arm in `handle_key`, one arm in `navigate`, and one arm in `draw`.

### Action Enum

Each screen module exports a state struct whose `handle_key` method returns an `Action`:

```
Action::None              — no state change
Action::Navigate(s)       — switch to screen s
Action::SelectLog(l)      — open existing log l
Action::CreateLog(l)      — persist and open new log l; `LogManager::create_log` checks for duplicates (same callsign + operator + park_ref + grid square on the same UTC day) before saving and returns `StorageError::DuplicateLog` if found; different park refs are never duplicates; the error is displayed inline on the LogCreate screen
Action::AddQso(q)         — append QSO to the current log; checks for same-day duplicates (call+band+mode) and shows a warning if found (QSO is still saved)
Action::EditQso(idx)      — load QSO at index into entry form for editing
Action::UpdateQso(idx, q) — replace QSO at index with updated version
Action::ExportLog         — trigger ADIF export of the current log
Action::DeleteLog(log_id) — delete the log with the given ID from storage; clears current_log if it matches
Action::Quit              — exit the application
```

The `App` calls `apply_action` to interpret these, keeping all global state transitions in one place.

### Key Handling

`F1` (help) is a global key intercepted by `App` before delegation on every screen. It captures the current screen as the origin, resets the help state, and switches to the Help screen. Pressing `q` or `Esc` on the Help screen returns to that origin screen. Each screen owns its own state and key bindings.

### QSO Editing Flow

The QSO list screen dispatches `EditQso(index)` when the user presses Enter on a row. `App::apply_action` populates the QSO entry form with the selected QSO's data and switches to QsoEntry in edit mode. On submit, the entry screen returns `UpdateQso(index, qso)` instead of `AddQso(qso)`. The app replaces the QSO in-memory, saves the full log, and returns to the QSO list.

### Log Create Screen

`screens/log_create.rs` manages `LogCreateState`, which holds:

- `log_type: LogType` — `General | Pota | FieldDay | WinterFieldDay`
- `focus_area: FocusArea` — `TypeSelector | Fields`; determines whether keyboard input targets the type selector row or the form
- `form: Form` — rebuilt whenever the type changes; pre-populated from per-type value buffers
- Per-type value buffers (`callsign_buf`, `grid_square_buf`, `park_ref_buf`, etc.) — persisted across type switches so the user's typing is not lost

`Left`/`Right` cycle the log type when `TypeSelector` is focused; `Tab` moves from `TypeSelector` to the first form field, and wraps from the last form field back to `TypeSelector`. `BackTab` reverses the direction. Typing is ignored while `TypeSelector` is focused. On submit, `submit()` dispatches to `submit_general`, `submit_pota`, `submit_field_day`, or `submit_wfd` based on the active type; each validates its type-specific fields and returns `Action::CreateLog(Log::*)`.

### Form Widget

`widgets/form.rs` provides a reusable `Form` with `FormField` entries. It handles focus cycling, character insert/delete, per-field errors, and rendering. Screens like LogCreate wrap a `Form` and add validation logic on submit.

### Status Bar Widget

`widgets/status_bar.rs` provides `StatusBarContext` (a plain data struct) and `draw_status_bar`. It renders a one-line context bar at the top of the QSO Entry, QSO List, and Export screens showing the active log's callsign, park reference, today's QSO count, and activation status. The widget is decoupled from `Log` — callers construct a `StatusBarContext` from whatever log type is active. This keeps Phase 4 multi-logbook changes confined to context construction rather than the widget itself.

## Design Decisions

- **No async runtime**: The TUI is synchronous. Crossterm's event polling is sufficient for a keyboard-driven logger. No need for tokio/async-std complexity.
- **`difa` crate for ADIF**: Uses the `difa` crate with `TagEncoder` and `BytesMut` for spec-compliant ADIF encoding.
- **Pure ADIF module**: `src/adif/` contains only pure formatting functions with no I/O. The storage module handles file writes. This makes ADIF logic fully unit-testable.
- **JSONL storage**: Each log is a single `.jsonl` file in `~/.local/share/duklog/logs/` (XDG). Line 1 is log metadata, lines 2+ are QSO records. Appending a new QSO is a single-line file append. Editing a QSO triggers a full file rewrite via `save_log`.
- **Auto-save**: Every model mutation triggers a save. No explicit "save" action needed — prevents data loss during field operation.
- **PostToolUse hooks**: `cargo check` and `cargo clippy` run automatically after every `.rs` file edit, providing immediate compilation and lint feedback. Tests and mutation testing are too slow for hooks and run explicitly via `make` targets.
- **Adversarial code review**: `code-review` subagent (Sonnet) runs before every PR to catch issues the developer is blind to.
- **Token-optimized CLAUDE.md**: Only always-needed content (62 lines) lives in CLAUDE.md. Domain knowledge, testing rules, and ADIF specs are in `.claude/rules/` with path-scoped loading. Coding standards are a skill preloaded into the code-review subagent.
- **Continuous learning**: `/learn-from-feedback` skill processes PR comments and user corrections into the appropriate knowledge store (rules, skills, or auto memory) so mistakes don't recur.

## Architecture Decision Records

### ADR-1: Log enum over LogConfig-on-struct (Phase 4.0)

**Decision:** `Log` is an enum (`General(GeneralLog)`, `Pota(PotaLog)`, future types) wrapping concrete structs that each embed a `LogHeader` for shared fields.

**Rejected alternative:** A single `Log` struct with a `LogConfig` enum field carrying type-specific data.

**Rationale:**
- Type-specific fields are accessible without pattern matching through a config enum. `PotaLog.park_ref` is a direct field; `FieldDayLog.class` and `.section` will be direct fields.
- Type-specific methods live on the concrete type, not on `Log`. `PotaLog::is_activated()` exists; `GeneralLog` simply has no such method.
- The compiler enforces exhaustiveness at each dispatch point in `log.rs`. Adding a new log type surfaces all required method updates at compile time.
- Each concrete type can be unit-tested in isolation without constructing the `Log` enum wrapper.
- Serialization uses `#[serde(tag = "log_type")]`; existing JSONL files without `log_type` default to `Log::Pota` for backward compatibility.

**Tradeoff accepted:** `Log` methods that delegate to `LogHeader` (e.g., `header()`, `header_mut()`, `add_qso()`) require one match arm per variant. This is mechanical boilerplate that grows linearly with log types — acceptable given the benefits.

---

### ADR-2: Explicit screen dispatch over a ScreenState trait (Phase 4.0)

**Decision:** `App::handle_key` uses an explicit `match self.screen` to dispatch key events. Draw functions remain free functions called from a `match` in `App::draw`. Navigation initialization logic stays in `App::navigate`.

**Rejected alternative:** A `ScreenState` trait extended with `on_enter(&mut self, ctx: &ScreenContext)` and `draw(&self, ctx: &DrawContext, frame: &mut Frame, area: Rect)`, eliminating the dispatch matches by pushing logic into screen state structs.

**Rationale:**
- The explicit matches are the honest representation of the coupling. `QsoListState::handle_key` needs the QSO count, which lives on the `Log` in `App`. Passing it as a parameter at the call site makes that dependency visible. A `ScreenState` trait that hides it — by caching the count on the struct and requiring `App` to call `set_qso_count()` before navigating — trades explicitness for a silent invariant that can be violated without a compile error.
- `App::navigate` is the right place for initialization logic that reads from app-level state (`current_log`, `manager`). Moving it to `on_enter` methods would require a `ScreenContext` borrow that spans screen types — adding lifetime complexity without a behavioral benefit.
- The draw matches and navigate match are additive: adding a screen appends one arm to each. This is low cognitive load and the compiler catches omissions.
- With 6 screens, the ergonomic benefit of trait dispatch does not outweigh the hidden coupling it introduces.

**When to revisit:** If the number of screens grows large enough that the matches become a maintenance burden (>~10 screens), or if a screen's initialization logic grows complex enough to warrant encapsulation, a narrow `on_enter` trait becomes worth the tradeoff.

## Dependencies

| Crate | Purpose |
|---|---|
| ratatui | Terminal UI framework |
| crossterm | Terminal backend (input, raw mode) |
| chrono | UTC timestamps, date formatting |
| serde / serde_json | JSON serialization for log persistence |
| dirs | XDG Base Directory paths for platform-native storage |
| difa | ADIF v3.1.6 tag encoding |
| thiserror | Ergonomic error types per module |
| mutants | `#[mutants::skip]` attribute for untestable functions |
