# duklog Implementation Plan

## Context

duklog is an offline ham radio logging TUI for POTA activations. No network access, ever. Built with Rust, Ratatui, and Crossterm. XDG storage (`~/.local/share/duklog/`), multiple saved logs, feature branches + PRs.

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

---

## Remaining Work

### 3.9 Delete Log (`feature/delete-log`)
**Files**: `src/tui/screens/log_select.rs`, `src/tui/app.rs`, `src/tui/action.rs`

- Add `d` keybinding on Log Select screen to delete the highlighted log
- Show confirmation prompt before deleting (e.g. "Delete K-0001 2026-02-16? y/n")
- Call `LogManager::delete_log()` on confirmation, reload the log list
- Handle edge cases: empty list (no-op), deleting the only log (selection becomes `None`)
- Tests: delete updates list, cancel preserves list, empty list no-op

### ~~3.10 Duplicate QSO Detection~~ — Done (`feature/duplicate-qso-detection`)

- `Log::find_duplicates(&self, qso: &Qso) -> Vec<&Qso>` — case-insensitive match on callsign + band + mode
- `App::apply_action` checks for duplicates before saving; sets warning message after `clear_fast_fields()`
- QSO is still logged — operator may intentionally work the same station on the same band/mode

### 3.11 Duplicate Log Prevention (`feature/duplicate-log-prevention`)
**Files**: `src/model/log.rs`, `src/storage/manager.rs`, `src/tui/screens/log_create.rs`

- Prevent creating a second log with the same station callsign, operator, and park ref on the same UTC day
- `LogManager` checks existing logs at creation time and returns an error if a match exists
- TUI LogCreate screen displays the error inline (same as other validation errors)
- Tests: duplicate blocked, different day allowed, different station/operator/park allowed

### 3.12 Polish (`feature/polish`)
**Files**: `src/tui/widgets/status_bar.rs`, `src/tui/screens/help.rs`, various

- Status bar widget on all screens: park, callsign, QSO count, activation status (green when activated)
- Help screen: full keybinding reference (including `d` for delete on log select)
- Error handling polish: all errors display gracefully, no panics in normal operation
- Final `make mutants` pass across entire codebase
- Complete all `docs/` files

#### Future enhancements (post-3.12)

- **Editable export path**: Allow user to edit the export file path on the export confirmation screen before exporting (use existing `FormState` text input widget)
- **Auto-generated screenshots**: Use `TestBackend` to render each screen into a text buffer and output them as documentation assets (e.g. for `docs/user-guide.md`), keeping screenshots in sync with the actual UI automatically

---

## Dependency Graph (remaining)

```
3.9 Delete Log
3.10 Duplicate QSO Detection
3.11 Duplicate Log Prevention
    ↓
3.12 Polish (after all above)
```

Steps 3.9–3.11 are independent of each other and can be developed in any order.

## Reference Documentation

Domain and framework research has been saved to `docs/reference/`:

- `docs/reference/adif-spec-notes.md` — ADIF v3.1.6 file format, field syntax, band/mode values, header format
- `docs/reference/pota-rules-notes.md` — POTA activation rules, required/recommended ADIF fields, park reference format, P2P contacts
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
