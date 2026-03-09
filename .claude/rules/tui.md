---
paths:
  - "src/tui/**"
---

# TUI Architecture

## Screen Dispatch

`App::handle_key` dispatches via explicit `match self.screen` (see ADR-0002):

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

`QsoListState::handle_key` takes `qso_count: usize` as an explicit parameter (the count lives on `Log` in `App`). Adding a screen = one arm in `handle_key` + `navigate` + `draw`; the compiler catches omissions.

## Action Enum

Each screen's `handle_key` returns an `Action`. `App::apply_action` interprets all of them, keeping global state transitions in one place:

```
Action::None
Action::Navigate(s)
Action::SelectLog(l)
Action::CreateLog(l)      — LogManager checks for duplicate (same callsign+operator+park_ref+grid on same UTC day) and returns StorageError::DuplicateLog
Action::AddQso(q)         — checks same-day duplicates (call+band+mode), shows warning but still saves
Action::EditQso(idx)      — loads QSO into entry form for editing
Action::UpdateQso(idx, q) — replaces QSO, saves full log, returns to QSO list
Action::ExportLog
Action::DeleteLog(log_id) — clears current_log if it matches
Action::DeleteQso(idx)    — removes QSO, persists, clamps selection
Action::Quit
```

## Key Handling

`F1` is a global key intercepted by `App` before delegation on every screen. It captures the current screen as origin, resets help state, and switches to Help. `q` or `Esc` on Help returns to origin.

**`q` as navigation key**: only valid on screens with no editable text field. When a screen has an editable field, `q` types into it — `Esc` is the only escape key in editing contexts. Audit new screens before adding `q` as a nav key.

## QSO Editing Flow

QSO list dispatches `EditQso(index)` on Enter. `App::apply_action` populates the QSO entry form with the selected QSO's data and switches to QsoEntry in edit mode. On submit, the entry screen returns `UpdateQso(index, qso)` instead of `AddQso`.

## Log Create Screen (`screens/log_create.rs`)

`LogCreateState` holds:
- `log_type: LogType` — `General | Pota | FieldDay | WinterFieldDay`
- `focus_area: FocusArea` — `TypeSelector | Fields`
- `form: Form` — rebuilt on type change, pre-populated from per-type value buffers
- Per-type value buffers (`callsign_buf`, `grid_square_buf`, `park_ref_buf`, etc.) — preserved across type switches

`Left`/`Right` cycle log type when `TypeSelector` focused. `Tab` moves TypeSelector→first field, last field→TypeSelector. `BackTab` reverses. Submit dispatches to `submit_general`, `submit_pota`, `submit_field_day`, or `submit_wfd`.

## QSO Entry Screen (`screens/qso_entry.rs`)

`QsoEntryState` holds `form_type: QsoFormType`. `build_form_for_type(form_type, mode) -> Form` constructs the correct field set (see ADR-0003):

- General/POTA: `THEIR_CALL=0`, `RST_SENT=1`, `RST_RCVD=2`; POTA adds Their Park at 3, Comments at 4; General Comments at 3
- FD/WFD: `THEIR_CALL=0`, `CONTEST_THEIR_CLASS=1`, `CONTEST_THEIR_SECTION=2`; FD Comments at 3; WFD adds `CONTEST_FREQUENCY=3`, Comments at 4
- FD/WFD QSOs store `rst_sent="59"` / `rst_rcvd="59"` as defaults (no RST exchange in contests)

`draw_qso_entry_form` (private, `#[mutants::skip]`) renders a two-row horizontal layout. Row 1: three core fields in equal thirds. Row 2 varies by type: General puts Comments on right half; POTA/FD put type-specific field left and Comments right; WFD uses three equal thirds.

`try_auto_set_band_from_frequency` fires when Tab/BackTab leaves the frequency field (FD/WFD only) and during submit. Out-of-band or non-parseable values are silently ignored.

## Form Widget (`widgets/form.rs`)

`Form` with `FormField` entries handles focus cycling, character insert/delete, per-field errors, and rendering.

- `draw_form` — renders all fields vertically in a single column
- `draw_form_field` — renders a single field at a caller-supplied `Rect`; used by screens needing custom spatial layouts (multi-column rows)

## Status Bar Widget (`widgets/status_bar.rs`)

`StatusBarContext` fields:
- `context_label: String` — park ref for POTA, sent exchange (`"1B EPA"`) for FD/WFD, callsign for General
- `qso_count: usize` — today's count for POTA; total count for all other types
- `pota_mode: bool` — if true, format as `N/10 QSOs`
- `is_activated: bool` — POTA only, shows `ACTIVATED` instead of count

`StatusBarContext::from_log(log: &Log)` constructs the correct context. Callers at the three display sites (QsoEntry, QsoList, Export):

```rust
app.current_log
    .as_ref()
    .map(StatusBarContext::from_log)
    .unwrap_or_default()
```

Display format: `[label]  N/10 QSOs` (POTA) / `[label]  ACTIVATED` (POTA activated) / `[label]  N QSOs` (all other).

## Key ADRs

- [ADR-0002](../../docs/adr/0002-explicit-screen-dispatch.md) — Why explicit match dispatch over ScreenState trait
- [ADR-0003](../../docs/adr/0003-dynamic-form-construction.md) — Why dynamic form reconstruction over fixed fields with conditional rendering
