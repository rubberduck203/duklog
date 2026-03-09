# CLAUDE.md — duklog Project Reference

## Project Overview

**duklog** is an offline ham radio logging TUI for general, POTA, and contest (Field Day / WFD) operation. No network access, ever. Built with Rust, Ratatui, and Crossterm.

## Module Structure

```
src/
  main.rs              # Terminal setup/teardown, panic hook, calls duklog::run()
  lib.rs               # Re-exports modules
  model/               # Log, Qso, Band, Mode, ValidationError
  adif/                # ADIF writer — pure formatting functions, no I/O
  storage/             # ADIF persistence (XDG), file-copy export
  tui/                 # App state, event loop
    screens/           # log_select, log_create, qso_entry, qso_list, export, help
    widgets/           # status_bar, form
```

## Dev Commands

Always use `make` targets, not raw `cargo` commands:

| Command | Purpose |
|---|---|
| `make ci` | fmt + lint + test + coverage (run before every commit) |
| `make test` | Run test suite |
| `make lint` | Clippy with `-D warnings` |
| `make coverage` | HTML coverage report, fails if < 90% line coverage |
| `make mutants` | Run mutation testing across entire codebase |
| `make mutants-module MOD=src/model/` | Run mutation testing on one module |

## Coding Style

- Prefer **functional**: iterators over loops, `map`/`filter`/`fold` over manual accumulation
- Prefer **expressions** over explicit `return`
- Prefer **`match`** over `if let` chains
- No `.unwrap()` or `.expect()` in library code — use `?` or proper error handling
- `.unwrap()` and `.expect("message")` are acceptable in tests and `main.rs`; prefer `.expect()` in tests since it produces better failure output
- Derive order: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`, `Serialize`, `Deserialize`
- Group imports: std, external crates, crate-internal — separated by blank lines
- Use specific imports, not glob imports

## Error Handling

Each module defines its own error type using `thiserror`. Errors propagate with `?`. TUI layer handles display.

## Documentation

- Rustdoc (`///`) on all `pub` items
- No feature is complete without documentation updates — update these as part of every feature:
  - `docs/user-guide.md` — screen descriptions, keybindings, user-facing workflows
  - `docs/architecture.md` — module layout, design principles, ADR index
  - `docs/adr/` — add a new ADR when making a non-obvious structural decision; update existing ADRs when decisions evolve
  - `docs/roadmap.md` — move completed phases, update remaining work
  - `docs/adif-format.md` — if ADIF fields or export format changes

## Reference Docs

Offline domain and framework research in `docs/reference/` — consult these instead of fetching from the web:

- `adif-spec-notes.md` — ADIF v3.1.6 file format, field syntax, band/mode values, header format
- `adif-band-frequencies.md` — frequency ranges (MHz and kHz) for all 13 bands in `Band` enum
- `fcc-us-band-privileges.md` — US FCC Part 97 band privileges by license class; General class sub-ranges, 60m channelization
- `pota-rules-notes.md` — POTA activation rules, required/recommended ADIF fields, park reference format, P2P contacts
- `arrl-field-day-notes.md` — Field Day exchange format, classes, sections, scoring, ADIF mapping
- `winter-field-day-notes.md` — WFD exchange format, classes, scoring, ADIF mapping, differences from Field Day
- `ratatui-notes.md` — Ratatui architecture, widget inventory, crossterm integration, terminal setup/teardown pattern
- `testing-tools-notes.md` — cargo-llvm-cov and cargo-mutants setup, commands, test writing guidance

## Git Workflow

- Feature branches off `main`, one per implementation step
- Run `make ci` before every commit
- Run the `code-review` subagent before creating PRs
- PRs to `main` with descriptive titles and summaries
