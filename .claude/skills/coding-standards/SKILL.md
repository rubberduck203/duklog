---
name: coding-standards
description: duklog coding standards, testing requirements, and review checklist. Use when reviewing code, writing tests, or checking quality.
user-invocable: false
---

# duklog Coding Standards

## Style
- Iterators over loops, expressions over `return`, `match` over `if let`
- No `.unwrap()`/`.expect()` in lib code (only tests and `main.rs`)
- Derive order: Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize
- Specific imports (no globs), grouped: std / external / crate-internal
- Each module has its own `thiserror` error type; propagate with `?`

## Testing
- Every `pub fn` tested with success and failure paths
- Assert on **specific values**, not `is_ok()`/`is_empty()` — critical for mutation testing
- Test **boundary values** (e.g., activation threshold at 9, 10, 11 QSOs)
- Use **quickcheck** for string-input functions
- Use `tempfile::tempdir()` for storage tests — never real paths
- Deterministic and fast; no surviving mutants per module
- 90% minimum line coverage

## Coverage Exclusions
- Allowed: `main.rs` setup, TUI `render()` methods, `#[mutants::skip]` functions
- Never exclude: `src/model/`, `src/adif/`, `src/storage/`, `handle_key()` methods

## ADIF/POTA Correctness
- Field format: `<FIELDNAME:length>value` (length = byte length)
- Required: `STATION_CALLSIGN`, `CALL`, `QSO_DATE` (YYYYMMDD), `TIME_ON` (HHMMSS), `BAND`, `MODE`
- Park ref format: `[A-Z]{1,3}-\d{4,5}`
- Activation threshold: 10 QSOs, single park, one UTC day

## Documentation
- Rustdoc (`///`) on all `pub` items
- Update `docs/` for related features
