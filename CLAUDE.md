# CLAUDE.md — duklog Project Reference

## Project Overview

**duklog** is an offline ham radio logging TUI for POTA (Parks on the Air) activations. It runs entirely offline — no network access, ever. Built with Rust, Ratatui, and Crossterm.

## Module Structure

```
src/
  main.rs              # Terminal setup/teardown, panic hook, calls duklog::run()
  lib.rs               # Re-exports modules
  model/               # Log, Qso, Band, Mode, ValidationError
  adif/                # ADIF writer — pure formatting functions, no I/O
  storage/             # JSON persistence (XDG), ADIF file export
  tui/                 # App state, event loop
    screens/           # log_select, log_create, qso_entry, qso_list, export, help
    widgets/           # status_bar, form
```

## Dev Commands

Always use `make` targets, not raw `cargo` commands:

| Command | Purpose |
|---|---|
| `make build` | Compile |
| `make check` | Type check without building |
| `make test` | Run test suite |
| `make fmt` | Check formatting |
| `make lint` | Clippy with `-D warnings` |
| `make coverage` | HTML coverage report, fails if < 90% line coverage |
| `make coverage-report` | Open coverage report in browser |
| `make mutants` | Run mutation testing across entire codebase |
| `make mutants-module MOD=src/model/` | Run mutation testing on one module |
| `make mutants-list` | Preview mutants without running |
| `make ci` | fmt + lint + test + coverage (run before every commit) |
| `make doc` | Build and open rustdoc |
| `make clean` | Remove build artifacts and mutants output |

## Coding Style

### General

- Prefer a **functional approach**: iterators over loops, `map`/`filter`/`fold` over manual accumulation
- Prefer **expressions** over explicit `return` — the last expression in a block is the return value
- Prefer **`match`** over `if let` chains
- No `.unwrap()` or `.expect()` in library code (`src/lib.rs` and submodules) — use `?` or proper error handling
- `.unwrap()` is acceptable only in tests and `main.rs`

### Derive Order Convention

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
```

Always in this order: `Debug`, `Clone`, `Copy` (if applicable), `PartialEq`, `Eq`, `Hash` (if applicable), `PartialOrd`, `Ord` (if applicable), then `Serialize`, `Deserialize`.

### Imports

- Group imports: std, external crates, crate-internal — separated by blank lines
- Use specific imports, not glob imports

## Error Handling

Each module defines its own error type using `thiserror`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("callsign must not be empty")]
    EmptyCallsign,
    // ...
}
```

Errors propagate with `?`. TUI layer handles display to user.

## Testing Requirements

- Every `pub fn` must have tests covering both success and failure paths
- Validation functions must test both valid and invalid inputs
- Use **quickcheck** for functions that accept string inputs
- Assert on **specific values**, not just `is_ok()` / `is_empty()` — this is critical for mutation testing
- Test **boundary values** (e.g., 9, 10, 11 QSOs for activation threshold)
- Use `tempfile::tempdir()` for all storage tests — never write to real paths
- Tests must be deterministic and fast
- After implementing a module: `make mutants-module MOD=src/<module>/` — no surviving mutants allowed
- Minimum 90% line coverage enforced by `make coverage`

### Coverage Exclusions

Use `#[cfg_attr(coverage_nightly, coverage(off))]` for:
- `main.rs` terminal setup/teardown
- TUI `render()` methods (visual output, tested by eye)
- Functions also marked `#[mutants::skip]`

Never exclude from coverage:
- `src/model/` — 100% coverage target
- `src/adif/` — pure functions, fully testable
- `src/storage/` — use tempfile for isolation
- `handle_key()` methods in TUI — contains real logic

## Documentation Requirements

- Rustdoc (`///`) on all `pub` items
- Update files in `docs/` when implementing related features
- No feature is complete without documentation updates

## Data Model

Three categories of data:

### Per-Log (set at log creation, rarely changes)
- `station_callsign` — callsign used on air
- `operator` — individual operator callsign (may equal station_callsign)
- `park_ref` — POTA park reference (format: `[A-Z]{1,3}-\d{4,5}`, e.g. `K-0001`)
- `grid_square` — Maidenhead locator
- `log_id` — unique identifier
- `created_at` — UTC timestamp

### Slow-Moving (changes occasionally during operation)
- `band` — amateur band (e.g. `20M`, `40M`)
- `mode` — operating mode (SSB, CW, FT8, etc.)

### Fast-Moving (changes every QSO)
- `their_call` — other station's callsign
- `rst_sent` / `rst_rcvd` — signal reports
- `comments` — optional notes
- `their_park` — other station's park reference (P2P contacts)
- `timestamp` — UTC date/time of contact

## ADIF Reference

### Field Format
```
<FIELDNAME:length>value
```
Where `length` is the byte length of `value`.

### Required POTA Fields
`STATION_CALLSIGN`, `CALL`, `QSO_DATE` (YYYYMMDD), `TIME_ON` (HHMMSS), `BAND`, `MODE`

### Recommended POTA Fields
`OPERATOR`, `MY_SIG` (always `POTA`), `MY_SIG_INFO` (park ref), `RST_SENT`, `RST_RCVD`, `SIG`/`SIG_INFO` (for P2P)

### Activation Threshold
10 QSOs from a single park within one UTC day.

### RST Defaults by Mode
- SSB/FM/AM: `59` (2-digit)
- CW/PSK31/RTTY: `599` (3-digit)
- FT8/FT4: `-10` (dB)

## Storage

- XDG path: `~/.local/share/duklog/logs/` with one JSON file per log
- ADIF export default path: `~/duklog-{PARK}-{YYYYMMDD}.adif`
- Auto-save after every mutation

## Git Workflow

- Feature branches off `main`, one per implementation step
- Run `make ci` before every commit
- Run `make mutants` per module after implementation
- Run `/code-review` before creating PRs
- PRs to `main` with descriptive titles and summaries

## Reference Documentation

Offline domain and framework references are in `docs/reference/`:
- `adif-spec-notes.md` — ADIF v3.1.6 format, field syntax, band/mode values
- `pota-rules-notes.md` — POTA activation rules, required fields, park reference format
- `ratatui-notes.md` — Ratatui architecture, widgets, terminal setup pattern
- `testing-tools-notes.md` — cargo-llvm-cov and cargo-mutants usage

Consult these during implementation instead of fetching from the web.
