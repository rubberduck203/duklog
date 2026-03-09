# Architecture

## Overview

duklog is an offline ham radio logging TUI. It is a single-binary Rust application with no network dependencies.

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
5. **Export**: User-triggered export copies the internal ADIF file to `~/Documents/duklog/` — no reformatting

## Domain Model

`Log` is an enum over concrete log types, each carrying a shared `LogHeader` plus type-specific fields. See [ADR-0001](adr/0001-log-enum-model.md) for the structural rationale.

`Qso` carries two optional fields: `exchange_rcvd: Option<String>` (received contest exchange; contest logs only) and `frequency: Option<u32>` (kHz; required for FD/WFD, optional otherwise).

Persistence uses ADIF as the single storage format. Log metadata is encoded in the ADIF header via standard fields and `APP_DUKLOG_*` app-extension fields. The async `difa::RecordStream` reader is invoked via a `tokio::runtime::Runtime` (current-thread) held by `LogManager`, keeping the public API synchronous. Legacy `.jsonl` files are auto-migrated to ADIF on startup.

## Screen Architecture

The TUI uses explicit `match self.screen` dispatch in `App`, with an `Action` enum for screen-to-app communication. Each screen module owns its state struct and a free draw function. See [ADR-0002](adr/0002-explicit-screen-dispatch.md).

## Design Principles

- **General purpose is the default** — no activation threshold, no park reference required; POTA is one log type among several
- **Minimal async footprint** — the TUI event loop is synchronous; a single `tokio::runtime::Runtime` (current-thread) lives on `LogManager` solely to drive `difa::RecordStream`
- **ADIF as single storage format** — internal `.adif` files are immediately usable by external tools; export is a file copy
- **Auto-save** — every model mutation triggers a save; no explicit "save" action
- **PostToolUse hooks** — `cargo check` and `cargo clippy` run automatically after every `.rs` file edit
- **Adversarial code review** — `code-review` subagent runs before every PR
- **Continuous learning** — `/learn-from-feedback` skill processes PR comments into the appropriate knowledge store

## Architecture Decision Records

| ADR | Decision | Modules |
|-----|----------|---------|
| [ADR-0001](adr/0001-log-enum-model.md) | Log enum over LogConfig-on-struct | `model/` |
| [ADR-0002](adr/0002-explicit-screen-dispatch.md) | Explicit screen dispatch over ScreenState trait | `tui/` |
| [ADR-0003](adr/0003-dynamic-form-construction.md) | Dynamic form construction for QSO entry | `tui/screens/` |
| [ADR-0004](adr/0004-hand-written-adif-reader.md) | Hand-written ADIF reader over serde ADIF format | `adif/`, `storage/` |

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
