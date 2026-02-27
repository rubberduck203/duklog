---
name: coding-standards
description: duklog coding standards, testing requirements, and review checklist. Use when reviewing code, writing tests, or checking quality.
---

# duklog Coding Standards

## Style
- Iterators over loops, expressions over `return`, `match` over `if let`
- No `.unwrap()`/`.expect()` in lib code (only tests and `main.rs`)
- Derive order: Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize
- Specific imports (no globs), grouped: std / external / crate-internal
- Each module has its own `thiserror` error type; propagate with `?`

## Domain Invariants
- **Never construct domain structs with struct literal syntax from outside the module** — always go through `::new()` or a dedicated constructor that enforces validation. Struct literal construction in the storage layer silently bypasses invariants (e.g., `tx_count >= 1`) that `::new()` enforces.
- **Loading from storage must enforce the same invariants as creation** — if a constructor validates a field, the deserialization path must validate it too. Check for `ok_or_else` guards that catch `None` but silently accept invalid `Some(0)` or empty-string values.

## Testing
- Every `pub fn` tested with success and failure paths
- Assert on **specific values**, not `is_ok()`/`is_empty()` — critical for mutation testing
- Test **boundary values** (e.g., activation threshold at 9, 10, 11 QSOs)
- Use **quickcheck** aggressively — default to it for any new pure function, not just validators
- Every normalization/transform function needs an idempotency property: `fn foo_is_idempotent(s: String) -> bool`
- Every normalize → validate pipeline needs a roundtrip property (construct invalid-case-but-valid-structure input, normalize, assert validates)
- Add `if !s.is_ascii() { return true; }` guard in quickcheck properties for ASCII-domain functions (callsigns, grid squares) to avoid Unicode expansion false failures
- Use `tempfile::tempdir()` for storage tests — never real paths
- Deterministic and fast; no surviving mutants per module
- 90% minimum line coverage
- For storage deserialization: test corrupt-but-parseable inputs (e.g., `tx_count: 0`, empty section) — not just missing fields

## Coverage Exclusions
- Allowed: `main.rs` setup, event loop methods requiring a real terminal
- Test `draw_*` functions with `TestBackend` render tests (not excluded from coverage)
- Keep `#[mutants::skip]` on draw functions (mutation testing visual layout isn't productive)
- Never exclude: `src/model/`, `src/adif/`, `src/storage/`, `handle_key()` methods

## ADIF/POTA Correctness
- Field format: `<FIELDNAME:length>value` (length = byte length)
- Required: `STATION_CALLSIGN`, `CALL`, `QSO_DATE` (YYYYMMDD), `TIME_ON` (HHMMSS), `BAND`, `MODE`
- Park ref format: `[A-Z]{1,3}-\d{4,5}`
- Activation threshold: 10 QSOs, single park, one UTC day

## Documentation
- Rustdoc (`///`) on all `pub` items
- Update `docs/` when implementing or changing features:
  - `docs/user-guide.md` — screen descriptions, keybindings, workflows
  - `docs/architecture.md` — module layout, Action enum, design decisions
  - `docs/implementation-plan.md` — move completed phases, update remaining work
  - `docs/adif-format.md` — if ADIF fields or format changes
- No feature is complete without documentation updates
