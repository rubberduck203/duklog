# duklog Implementation Plan

## Context

duklog is an offline ham radio logger with POTA and field day support. No network access, ever. Built with Rust, Ratatui, and Crossterm. XDG storage (`~/.local/share/duklog/`), multiple saved logs, feature branches + PRs.

Standards and reference material are maintained in `CLAUDE.md`, `.claude/rules/`, and `docs/reference/`.

---

## Completed Phases

- **Phase 1: Technical Guardrails** (`setup/tooling`) — Done
- **Phase 2: Claude Code Autonomy** (`setup/claude-code`) — Done
- **3.1 Data Model** (`feature/data-model`, PR #3) — Done
- **3.2 ADIF Export** (`feature/adif-writer`, PR #4) — Done
- **3.3 Storage** (`feature/storage`, PR #5, #6) — Done
- **3.4 TUI Shell** (`feature/tui-shell`, PR #7) — Done
- **3.5 Log Management Screens** (`feature/log-management`, PR #7) — Done
- **3.5.1 Optional Operator** (`feature/optional-operator`, PR #8) — Done
- **3.6 QSO Entry Screen** (`feature/qso-entry`, PR #9, #10) — Done
- **3.7 QSO List Screen** (`feature/qso-entry`, PR #10) — Done
- **3.7b QSO Editing** (`feature/qso-editing`, PR #12) — Done
- **3.8 Export Screen** (`feature/export-screen`, PR #11) — Done
- **3.9 Delete Log** (`feature/delete-log`) — Done
- **3.10 Duplicate QSO Detection** (`feature/duplicate-qso-detection`) — Done
- **3.11 Duplicate Log Prevention** (`feature/duplicate-log-prevention`) — Done
- **3.12 Polish** (`feature/polish`) — Done
- **4.0 Log enum refactor** (`feature/polish`) — Done
- **4.1 FieldDay and WFD model types** (`feature/log-types-model`) — Done
- **4.1.5 Refactor: submodule extraction and function decomposition** (`feature/refactor-structure`) — Done
- **4.1.6 Validation bug fixes** (`feature/validation-fixes`) — Done: added `normalize_park_ref`/`normalize_grid_square`; PARK_REF auto-uppercased in `log_create.rs`; grid square normalised at submit; defensive normalize in `qso_entry.rs`; lockup investigated, not reproducible
- **4.2 Log type selection in log create flow** — Done: `LogType` enum (General/Pota/FieldDay/WinterFieldDay); type selector row above form with Left/Right cycling; `FocusArea` (TypeSelector/Fields); per-type form construction with buffer preservation across type switches; `submit_general`, `submit_pota`, `submit_field_day`, `submit_wfd` dispatch; `parse_fd_class`/`parse_wfd_class` added to model with quickcheck tests; `InvalidFdClass`/`InvalidWfdClass` `ValidationError` variants
- **4.3 Field Day QSO entry + form layout redesign** — Done: `validate_fd_exchange`/`validate_wfd_exchange` with quickcheck tests; `InvalidFdExchange`/`InvalidWfdExchange` `ValidationError` variants; `QsoFormType` enum drives dynamic form construction in `QsoEntryState`; two-row horizontal form layout via `draw_qso_entry_form` using `draw_form_field`; form area reduced from 15 to 6 lines; type-aware submit validates exchange (FD/WFD) and frequency (WFD); ADIF writer updated: `CONTEST_ID`/`STX_STRING`/`SRX_STRING` for FD/WFD, `FREQ` for WFD, `SIG`/`SIG_INFO` gated on POTA log type (ADR-3)
- **4.3.1 Log create form layout fixes** — Done: `draw_log_create` changed from `Constraint::Min(9)` to `Constraint::Length(fields.len() * 3)` so each field always gets exactly 3 lines; render tests updated to just-sufficient terminal heights (General=16, POTA=19, FD/WFD=25)
- **4.4 Log select and status bar updates** — Done: `Log::log_type_name()` added; `StatusBarContext` redesigned with `context_label`/`pota_mode` (replaces `callsign`/`park_ref`); `StatusBarContext::from_log` constructor; log select table shows "Type" column (General/POTA/FD/WFD) instead of park column; log select footer adds `F1: help`; status bar format unified to `[label]  N QSOs` / `[label]  N/10 QSOs` / `[label]  ACTIVATED`
- **4.3.2 FD/WFD exchange-only forms** — Done: grid square removed from FD/WFD log-create forms; `FieldDayLog`/`WfdLog` no longer call `validate_grid_square`; `MY_GRIDSQUARE` ADIF emission guarded on non-empty grid; QSO entry "Their Exchange" split into "Their Class" (e.g. `3A`) + "Their Section" (e.g. `CT`) fields; RST removed from FD/WFD QSO entry (default "59" stored in Qso model); class+section assembled and validated with `validate_fd/wfd_exchange` at submit; per-field errors shown on class field for invalid exchange format
- **4.4.5 QSO deletion** (`feature/qso-deletion`) — Done: `d` key on QSO list triggers y/n confirmation prompt; `Action::DeleteQso(idx)` dispatches to `apply_delete_qso`; `LogHeader::remove_qso` / `Log::remove_qso` added; `QsoListState` gains `pending_delete`, `error`, and `clamp_selection`; `reset()` clears transient state; storage errors surfaced in QSO list footer

---

## Remaining Work

### Phase 4: Multiple Logbook Types

duklog should support four logbook types, selectable when creating a new log. Each type shares the core QSO model but has type-specific setup fields, QSO exchange fields, ADIF output, and status display.

Reference docs: `docs/reference/arrl-field-day-notes.md`, `docs/reference/winter-field-day-notes.md`, `docs/reference/pota-rules-notes.md`

#### Logbook Types

| Type | Key fields | Exchange per QSO | ADIF extras |
|------|-----------|-----------------|-------------|
| **General** | callsign, operator, grid | none | none |
| **POTA** | callsign, operator, grid, park_ref | their_park (optional, P2P) | `MY_SIG=POTA`, `MY_SIG_INFO`, `SIG`, `SIG_INFO` |
| **Field Day** | callsign, operator, tx_count, fd_class, section | their_exchange (class+section) | `CONTEST_ID=ARRL-FIELD-DAY`, `STX_STRING`, `SRX_STRING` |
| **Winter Field Day** | callsign, operator, tx_count, wfd_class, section | their_exchange (class+section) | `CONTEST_ID=WFD`, `STX_STRING`, `SRX_STRING` |

#### 4.1 Add FieldDay and WFD types (`feature/log-types-model`)
**Files**: `src/model/log.rs`, `src/model/qso.rs`

The model now uses a `Log` enum (`General(GeneralLog)`, `Pota(PotaLog)`, future `FieldDay(FieldDayLog)`, `WinterFieldDay(WfdLog)`) backed by a shared `LogHeader`. See architecture.md for details.

- Add `FieldDayLog` struct: `header: LogHeader`, `tx_count: u8`, `class: FdClass`, `section: String`, `power: FdPowerCategory`
- Add `WfdLog` struct: `header: LogHeader`, `tx_count: u8`, `class: WfdClass`, `section: String`
- Add new `Log` enum variants: `FieldDay(FieldDayLog)`, `WinterFieldDay(WfdLog)`
- Add `FdClass` enum: `A`, `B`, `C`, `D`, `E`, `F`
- Add `WfdClass` enum: `H`, `I`, `O`, `M`
- Add `FdPowerCategory` enum: `Qrp` (≤5W non-commercial), `Low` (≤100W), `High` (>100W) — drives the ×5/×2/×1 multiplier
- Add `exchange_rcvd: Option<String>` to `Qso` — stores received contest exchange verbatim (e.g., `3A CT`); `None` for POTA and General logs
- Add `frequency: Option<u32>` to `Qso` — frequency in kHz; required for WFD ADIF (`FREQ` field); optional otherwise
- `their_park: Option<String>` stays on `Qso`; the ADIF writer gates `SIG`/`SIG_INFO` emission on log type being `Pota`, so non-POTA logs never accidentally emit POTA fields even if `their_park` is somehow set
- **`log_id` generation**: prefix contest log IDs for readability — `FD-{callsign}-{YYYYMMDD-HHMMSS}` for Field Day, `WFD-{callsign}-{YYYYMMDD-HHMMSS}` for WFD, `{park_ref_or_callsign}-{timestamp}` unchanged for POTA/General
- **Duplicate log check**: `LogManager::create_log` is type-aware. Two logs of *different* types for the same callsign on the same day are not duplicates; within a type, apply the existing field comparison
- **`find_duplicates` scope**: POTA logs use today-only scoping (existing). Field Day and WFD logs must scope across the entire log (events span two UTC calendar days; WFD also enforces a 3-contact-per-band limit across the whole event)
- Update `is_activated()` to be type-aware:
  - POTA: ≥10 QSOs today (existing logic)
  - Field Day / WFD / General: always `false` (score-based, no activation threshold)

#### 4.2 Log type selection in log create flow (`feature/log-type-selection`)
**Files**: `src/tui/screens/log_create.rs`, `src/tui/app.rs`

- Add a log type selector as the first step/field in log create (or a separate screen)
- Show type-appropriate fields based on selection:
  - General: callsign, operator, grid
  - POTA: callsign, operator, grid, park ref
  - Field Day: callsign, operator, tx count, FD class (A–F), ARRL/RAC section
  - WFD: callsign, operator, tx count, WFD class (H/I/O/M), ARRL/RAC section
- Section field (FD/WFD): permissive free-text, auto-uppercase; accepts any non-empty string (handles `DX`, unusual sections, and future additions without a hardcoded list)
- Update `CreateLog` action to carry `LogConfig`

#### 4.3.1 Log create form layout fixes (`feature/log-create-layout`)
**Files**: `src/tui/screens/log_create.rs`, `src/tui/widgets/form.rs`

Two rendering bugs observed during Phase 4.3 review:

**Bug 1 — Form field height**: FD/WFD forms have 6 fields × 3 lines each = 18 lines required. The vertical layout allocates `Constraint::Min(9)` to the form area, so on terminals shorter than ~25 lines some fields render at 2 lines (top/bottom border only, content row squeezed out). The fix is to make the form area height dynamic: `Constraint::Length(fields.len() as u16 * 3)` so the outer layout always allocates exactly the right height, and the scroll/truncation decision is made explicitly rather than silently.

- `draw_form` currently splits `form_area` with `Constraint::Ratio` — this must be changed to `Constraint::Length(3)` per field so each field always gets exactly 3 lines regardless of available space.
- `draw_log_create` must allocate `Constraint::Length(state.form().fields().len() as u16 * 3)` for `form_area` instead of `Constraint::Min(9)`.
- Render tests must assert at a height that is just sufficient for each form type (e.g., 20 lines for General/3 fields, 24 lines for FD/WFD/6 fields) so a regression causes an immediate test failure rather than a silent layout issue.

**Bug 2 — Already fixed (commit 40e99b1)**: Form fields were rendering at full terminal width on wide displays. Fixed by centering with `Constraint::Max(60)`.

**Bug 3 — Already fixed (commit 40e99b1)**: Typing while type selector was focused was silently ignored; now jumps immediately to Station Callsign.

#### 4.3.2 FD/WFD exchange-only forms (`feature/contest-exchange-only`)
**Files**: `src/tui/screens/log_create.rs`, `src/tui/screens/qso_entry.rs`

Cross-checking the reference docs reveals fields collected that are not part of either contest exchange:

**Log Create — remove Grid Square (FD and WFD)**

Neither FD nor WFD lists grid square as a log-level setup field. The spec requires only: callsign, operator, tx count, class, section (and power category for FD). Remove `CONTEST_GRID` constant, `grid_square_buf` sync for FD/WFD, and the grid square `FormField` from both arms of `build_form_for_type`. Update `submit_field_day` and `submit_wfd` accordingly. Update render tests for the new field counts (FD: 5 fields × 3 lines = 15; WFD: same).

**QSO Entry — remove RST, split exchange into class + section (FD and WFD)**

Neither contest exchange includes RST. Additionally, "Their Exchange" should be split into two fields — their class (e.g. `3A`) and their section (e.g. `CT`) — mirroring the separate class/section fields on the log create screen and enabling per-field validation via `parse_fd_class`/`parse_wfd_class` and `validate_section`. The `SRX_STRING` is then assembled as `"{class} {section}"` at submit time; `validate_fd_exchange`/`validate_wfd_exchange` are no longer needed at QSO entry. After removal and split the field layout becomes:

| Index | FD | WFD |
|-------|----|-----|
| 0 | Their Callsign | Their Callsign |
| 1 | Their Class (e.g. `3A`) | Their Class (e.g. `2M`) |
| 2 | Their Section (e.g. `CT`) | Their Section (e.g. `EPA`) |
| 3 | Comments | Frequency (kHz) |
| 4 | — | Comments |

Update index constants (or add per-type index helpers), `build_form_for_type`, submit logic, and render tests.

#### 4.4 Log select and status bar updates (`feature/log-type-ui`)
**Files**: `src/tui/screens/log_select.rs`, `src/tui/widgets/status_bar.rs`

- Log select table: show log type column instead of (or alongside) park column
- Status bar: show type-appropriate context
  - POTA: `[K-0001] 7/10 QSOs` or `[K-0001] ACTIVATED`
  - Field Day: `[1B EPA] 42 QSOs`
  - WFD: `[1H EPA] 18 QSOs`
  - General: `[W1AW] 5 QSOs`
- Add `F1: Help` to the bottom menu bar; audit current keymap for overlap with standard F1 conventions

---

## Dependency Graph (remaining)

```
[4.1 → 4.1.5 → 4.1.6 → 4.2 → 4.3 → 4.3.1 → 4.3.2 → 4.4 → 4.4.5 — all complete]
```

---

## Design Notes: General-Purpose vs. POTA Focus

The original design treated duklog as a POTA-first logger with general logging as a fallback. The multi-logbook-type direction inverts this:

- **General purpose is the default** — no activation threshold, no park reference required
- **POTA is one logbook type** among several, not the primary identity
- **Contest logs** (FD, WFD) are first-class: they have their own creation fields, exchange capture, and ADIF output
- duklog is a **general offline ham radio logging TUI** with POTA and field day support

Existing data: logs without a `log_type` field should default to `Pota` during deserialization to preserve behaviour for current users.

---

## Reference Documentation

Domain and framework research has been saved to `docs/reference/`:

- `docs/reference/adif-spec-notes.md` — ADIF v3.1.6 file format, field syntax, band/mode values, header format
- `docs/reference/adif-band-frequencies.md` — ADIF v3.1.4 band enumeration: frequency ranges (MHz and kHz) for all 13 bands in `Band` enum, plus full ADIF band table for reference
- `docs/reference/fcc-us-band-privileges.md` — US FCC Part 97 band privileges by license class; includes General class sub-ranges, gap segments reserved for Extra/Advanced, mode restrictions, and implementation notes for a future privilege-check feature
- `docs/reference/pota-rules-notes.md` — POTA activation rules, required/recommended ADIF fields, park reference format, P2P contacts
- `docs/reference/arrl-field-day-notes.md` — Field Day exchange format, classes, sections, scoring, ADIF mapping
- `docs/reference/winter-field-day-notes.md` — WFD exchange format, classes, scoring, ADIF mapping, differences from Field Day
- `docs/reference/ratatui-notes.md` — Ratatui architecture, widget inventory, crossterm integration, terminal setup/teardown pattern
- `docs/reference/testing-tools-notes.md` — cargo-llvm-cov and cargo-mutants setup, commands, test writing guidance for mutation testing

These are distilled from the official docs and should be consulted during implementation rather than fetching from the web (offline-first).

---

## Verification

**After each step**: `make ci` passes (fmt, lint, test, coverage)
**After each module**: `make mutants` — no surviving mutants
**Before each PR**: `/code-review` passes with no blockers
**End-to-end acceptance** (after 3.12):
1. Launch → select/create log → log 10 QSOs → see "ACTIVATED" → export ADIF
2. Quit and relaunch → session restored → switch between logs
3. Inspect ADIF file: all required POTA fields present, correct format

**End-to-end acceptance** (after Phase 4):
1. Create a Field Day log → log QSOs with exchanges → export ADIF → verify `CONTEST_ID=ARRL-FIELD-DAY` and `STX_STRING`/`SRX_STRING` fields
2. Create a WFD log → log QSOs with exchanges → export ADIF → verify `CONTEST_ID=WFD`
3. Create a General log → log QSOs → export ADIF → verify no POTA or contest fields present
4. Open an existing (pre-Phase-4) log → verify it loads as `Pota` type with correct behaviour

#### Future enhancements (post Phase 4)

- **Editable export path**: Allow user to edit the export file path on the export confirmation screen before exporting (use existing `FormState` text input widget)
- **Auto-generated screenshots**: Use `TestBackend` to render each screen into a text buffer and output them as documentation assets (e.g. for `docs/user-guide.md`), keeping screenshots in sync with the actual UI automatically
- **Field Day bonus points tracker**: Screen or sidebar to track claimed bonus points toward the FD summary sheet
- **WFD objectives tracker**: Track completed WFD objectives for the multiplier
- **Auto-determine band from frequency**: When a frequency is entered in the QSO form, automatically select the matching `Band` value (e.g., 14.225 MHz → 20M) so the operator doesn't have to set both
- **US license privilege checker**: Warn the operator when a logged frequency falls outside their license class privileges. Configurable license class (Technician/General/Advanced/Extra) stored in user preferences. General class requires per-band sub-range checks (80m, 40m, 20m, 15m each have disjoint General segments with gaps reserved for Extra/Advanced; 60m is channelized). Reference: `docs/reference/fcc-us-band-privileges.md`. US-only for now; could be extended to other ITU regions.
