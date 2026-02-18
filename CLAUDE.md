# CLAUDE.md — duklog Project Reference

## Project Overview

**duklog** is an offline ham radio logging TUI for POTA (Parks on the Air) activations. No network access, ever. Built with Rust, Ratatui, and Crossterm.

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
- `.unwrap()` is acceptable only in tests and `main.rs`
- Derive order: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`, `Serialize`, `Deserialize`
- Group imports: std, external crates, crate-internal — separated by blank lines
- Use specific imports, not glob imports

## Error Handling

Each module defines its own error type using `thiserror`. Errors propagate with `?`. TUI layer handles display.

## Documentation

- Rustdoc (`///`) on all `pub` items
- No feature is complete without documentation updates — update these as part of every feature:
  - `docs/user-guide.md` — screen descriptions, keybindings, user-facing workflows
  - `docs/architecture.md` — Action enum, module layout, design decisions
  - `docs/implementation-plan.md` — move completed phases, update remaining work
  - `docs/adif-format.md` — if ADIF fields or export format changes

## Git Workflow

- Feature branches off `main`, one per implementation step
- Run `make ci` before every commit
- Run `make mutants` per module after implementation
- Run the `code-review` subagent before creating PRs
- PRs to `main` with descriptive titles and summaries
