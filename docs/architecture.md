# Architecture

## Overview

duklog is an offline ham radio logging TUI for POTA activations. It is a single-binary Rust application with no network dependencies.

## Module Layout

```
src/
  main.rs       Terminal setup/teardown, panic hook
  lib.rs        Module re-exports, run() entry point
  model/        Domain types: Log, Qso, Band, Mode, validation
  adif/         ADIF format writer and reader (pure formatting + async reader, no I/O in writer)
  storage/      ADIF persistence to XDG paths, file-copy export
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
4. **Persistence**: After every model mutation, storage layer auto-saves to ADIF (`.adif` files in `~/.local/share/duklog/logs/`)
5. **Export**: User-triggered export copies the internal ADIF file to `~/Documents/duklog/` — no reformatting required

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

Qso carries two optional fields: `exchange_rcvd: Option<String>` (received contest exchange verbatim, e.g. `"3A CT"`; contest logs only) and `frequency: Option<u32>` (kHz; required for FD/WFD, optional for General and POTA). Both default to `None`. They are populated during QSO entry for all log types that expose them.

Persistence uses ADIF as the single storage format. Log metadata is encoded in the ADIF header via standard fields (`STATION_CALLSIGN`, `MY_GRIDSQUARE`, `CREATED_TIMESTAMP`, `MY_SIG`/`MY_SIG_INFO` for POTA) and `APP_DUKLOG_*` app-extension fields (`APP_DUKLOG_LOG_TYPE`, `APP_DUKLOG_LOG_ID`, `APP_DUKLOG_FD_CLASS`, `APP_DUKLOG_SECTION`, etc.). The async `difa::RecordStream` reader is invoked via a `tokio::runtime::Runtime` (current-thread) held by `LogManager`, keeping the public API synchronous. Legacy `.jsonl` files are auto-migrated to ADIF on startup. Shared fields are accessed via `log.header()` / `log.header_mut()`.

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
Action::DeleteQso(idx)    — remove QSO at index from the active log and persist; clamps qso_list selection
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

`draw_form` renders fields vertically in a single column. `draw_form_field` renders a single field at a caller-supplied `Rect`, used by screens that need custom spatial layouts (e.g. multi-column rows).

### QSO Entry Screen

`screens/qso_entry.rs` holds `QsoEntryState`, which includes a `QsoFormType` field tracking the log type in use. `build_form_for_type(form_type, mode) -> Form` constructs a `Form` with the correct field set (3–6 fields depending on type). `set_log_context` derives the form type from the `Log` variant and rebuilds the form when the type changes.

`try_auto_set_band_from_frequency` parses the frequency field and calls `Band::from_frequency_khz` to auto-select the band. It fires when Tab or BackTab leaves the frequency field (FD/WFD forms only) and again during submit. Out-of-band or non-parseable values are silently ignored — the operator's manual band selection is preserved.

The screen uses `draw_qso_entry_form` (a private `#[mutants::skip]` function) to render the form in a two-row horizontal layout rather than calling `draw_form`. Row 1 always shows the three core fields in equal thirds. Row 2 varies by type: General puts Comments on the right half; POTA/FD put the type-specific field left and Comments right; WFD uses three equal thirds for exchange, frequency, and comments. This frees ~9 lines of vertical space compared to the previous single-column layout.

### Status Bar Widget

`widgets/status_bar.rs` provides `StatusBarContext` and `draw_status_bar`. The context holds:

- `context_label: String` — what goes in brackets: park ref for POTA, sent exchange (`"1B EPA"`) for FD/WFD, callsign for General
- `qso_count: usize` — today's QSO count for POTA; total count for all other types
- `pota_mode: bool` — if `true`, format as `N/10 QSOs`; otherwise `N QSOs`
- `is_activated: bool` — POTA only: show `ACTIVATED` instead of the count

`StatusBarContext::from_log(log: &Log)` constructs the correct context for any log type. Callers at the three display sites (QSO Entry, QSO List, Export) call `.map(StatusBarContext::from_log).unwrap_or_default()` on the optional current log.

Display format: `[context_label]  N/10 QSOs` (POTA) / `[context_label]  ACTIVATED` (POTA activated) / `[context_label]  N QSOs` (all other types).

## Design Decisions

- **Minimal async footprint**: The TUI event loop is synchronous. A single `tokio::runtime::Runtime` (current-thread, no worker threads) lives on `LogManager` solely to drive `difa::RecordStream` during log reads. All public storage APIs are synchronous (`block_on` internally). Crossterm event polling is unaffected.
- **`difa` crate for ADIF**: Uses the `difa` crate with `TagEncoder` and `BytesMut` for spec-compliant ADIF encoding, and `RecordStream` for async record-by-record reading.
- **ADIF as single storage format**: `src/adif/` contains pure formatting functions (writer) and an async reader. No I/O in the writer; storage module handles file writes. This makes ADIF logic fully unit-testable. Internal `.adif` files are immediately usable by external tools without export.
- **ADIF storage**: Each log is a single `.adif` file in `~/.local/share/duklog/logs/` (XDG). The file header encodes all log metadata; subsequent records encode QSOs. Appending a QSO is an O(1) pure file append. Editing a QSO rewrites the full file via `save_log`.
- **File-copy export**: `export_adif` copies the internal `.adif` file to the export path — no reformatting. This is O(file size) but simpler and provably correct.
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
- Storage uses ADIF; the `Log` enum does not derive `Serialize`/`Deserialize`. Log metadata is encoded in ADIF header fields; `APP_DUKLOG_LOG_TYPE` is the discriminant. Legacy `.jsonl` files are auto-migrated on startup via a serde-based JSONL reader kept in `manager.rs` until v1.0.

**Tradeoff accepted:** `Log` methods that delegate to `LogHeader` (e.g., `header()`, `header_mut()`, `add_qso()`) require one match arm per variant. This is mechanical boilerplate that grows linearly with log types — acceptable given the benefits.

---

### ADR-3: Dynamic form construction for QSO entry (Phase 4.3)

**Decision:** `QsoEntryState` holds a `form_type: QsoFormType` field and rebuilds the `Form` from scratch (via `build_form_for_type`) when the log type changes. Field indices are numeric constants (`THEIR_CALL=0`, `RST_SENT=1`, `RST_RCVD=2`); type-specific fields occupy index 3 (and 4 for WFD). A private `draw_qso_entry_form` function renders the form in a two-row layout using `draw_form_field` for per-cell placement.

**Rejected alternative:** Separate layout paths in `draw_qso_entry` branching purely on log type, with a fixed Form field set and conditional rendering.

**Rationale:**
- Follows the same pattern as `log_create.rs`, keeping both form-owning screens consistent.
- The `Form` struct is the canonical state container (field values, focus, errors); rebuilding it when the type changes ensures the field set is always self-consistent rather than conditionally hiding/showing fixed fields.
- `draw_form_field` — added specifically for this phase — makes it trivial to place individual fields in arbitrary `Rect`s without re-implementing the border/error rendering.
- Type-specific rendering is isolated in `draw_qso_entry_form`, which is a small `#[mutants::skip]` function tested via `TestBackend` renders.

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
| serde / serde_json | Serde derives (model types) + JSON (JSONL migration path only) |
| dirs | XDG Base Directory paths for platform-native storage |
| difa | ADIF v3.1.6 tag encoding and async record streaming |
| tokio | Async runtime for driving `difa::RecordStream` in `LogManager` |
| futures | `StreamExt` trait for `.next()` on `RecordStream` |
| thiserror | Ergonomic error types per module |
| mutants | `#[mutants::skip]` attribute for untestable functions |

## Design Notes: General-Purpose vs. POTA Focus

The original design treated duklog as a POTA-first logger with general logging as a fallback. The multi-logbook-type direction inverts this:

- **General purpose is the default** — no activation threshold, no park reference required
- **POTA is one logbook type** among several, not the primary identity
- **Contest logs** (FD, WFD) are first-class: they have their own creation fields, exchange capture, and ADIF output
- duklog is a **general offline ham radio logging TUI** with POTA and field day support

Existing data: logs without a `log_type` field should default to `Pota` during deserialization to preserve behaviour for current users.
