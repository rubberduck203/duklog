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
4. **Persistence**: After every model mutation, storage layer auto-saves to JSON
5. **Export**: User-triggered ADIF export calls the pure ADIF writer, then writes to disk

## Design Decisions

- **No async runtime**: The TUI is synchronous. Crossterm's event polling is sufficient for a keyboard-driven logger. No need for tokio/async-std complexity.
- **`difa` crate for ADIF**: Uses the `difa` crate with `TagEncoder` and `BytesMut` for spec-compliant ADIF encoding.
- **Pure ADIF module**: `src/adif/` contains only pure formatting functions with no I/O. The storage module handles file writes. This makes ADIF logic fully unit-testable.
- **JSONL storage**: Each log is a single `.jsonl` file in `~/.local/share/duklog/logs/` (XDG). Line 1 is log metadata, lines 2+ are QSO records. Appending a QSO is a single-line file append — no read/rewrite needed.
- **Auto-save**: Every model mutation triggers a save. No explicit "save" action needed — prevents data loss during field operation.
- **PostToolUse hooks**: `cargo check` and `cargo clippy` run automatically after every `.rs` file edit, providing immediate compilation and lint feedback. Tests and mutation testing are too slow for hooks and run explicitly via `make` targets.
- **Adversarial code review**: `code-review` subagent (Sonnet) runs before every PR to catch issues the developer is blind to.
- **Token-optimized CLAUDE.md**: Only always-needed content (62 lines) lives in CLAUDE.md. Domain knowledge, testing rules, and ADIF specs are in `.claude/rules/` with path-scoped loading. Coding standards are a skill preloaded into the code-review subagent.
- **Continuous learning**: `/learn-from-feedback` skill processes PR comments and user corrections into the appropriate knowledge store (rules, skills, or auto memory) so mistakes don't recur.

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
