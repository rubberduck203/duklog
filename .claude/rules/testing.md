---
paths:
  - "src/**/*.rs"
  - "tests/**/*.rs"
---

# Testing Requirements

- Every `pub fn` must have tests covering both success and failure paths
- Validation functions must test both valid and invalid inputs
- Use **quickcheck** for functions that accept string inputs
- Assert on **specific values**, not just `is_ok()` / `is_empty()` — critical for mutation testing
- Test **boundary values** (e.g., 9, 10, 11 QSOs for activation threshold)
- Use `tempfile::tempdir()` for all storage tests — never write to real paths
- Tests must be deterministic and fast
- After implementing a module: `make mutants-module MOD=src/<module>/` — no surviving mutants
- Minimum 90% line coverage enforced by `make coverage`

## Coverage Exclusions

Use `#[cfg_attr(coverage_nightly, coverage(off))]` for:
- `main.rs` terminal setup/teardown
- TUI `render()` methods (visual output, tested by eye)
- Functions also marked `#[mutants::skip]`

Never exclude from coverage:
- `src/model/` — 100% coverage target
- `src/adif/` — pure functions, fully testable
- `src/storage/` — use tempfile for isolation
- `handle_key()` methods in TUI — contains real logic
