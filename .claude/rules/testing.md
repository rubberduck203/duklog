---
paths:
  - "src/**/*.rs"
  - "tests/**/*.rs"
---

# Testing Requirements

- Every `pub fn` must have tests covering both success and failure paths
- Validation functions must test both valid and invalid inputs
- Use **quickcheck** for functions that accept string inputs
- Use **quickcheck** for numeric threshold/boundary logic (prefer properties over hand-written boundary values)
- Assert on **specific values**, not just `is_ok()` / `is_empty()` — critical for mutation testing
- Use `tempfile::tempdir()` for all storage tests — never write to real paths
- For storage deserialization: test corrupt-but-parseable inputs (e.g., `tx_count: 0`, empty section), not just missing fields — these bypass `None` guards but still violate domain invariants
- Tests must be deterministic and fast
- **Organize tests with submodules** (`mod typing { ... }`, `mod validation { ... }`), not section comments (`// --- Typing ---`)
- **Extract test helpers** to reduce repetition — tests are code too; refactor shared setup into helper functions
- Prefer `.expect("descriptive message")` over bare `.unwrap()` in tests — the message surfaces in failure output and makes failures easier to diagnose
- Minimum 90% line coverage enforced by `make coverage`

## TUI Render Testing

Test `draw_*` functions by rendering into a `TestBackend` buffer and asserting key content appears:

```rust
use ratatui::backend::TestBackend;
use ratatui::Terminal;

let backend = TestBackend::new(80, 24);
let mut terminal = Terminal::new(backend).unwrap();
terminal.draw(|frame| {
    draw_my_widget(&state, frame, frame.area());
}).unwrap();
let content = buffer_to_string(terminal.backend().buffer());
assert!(content.contains("expected text"));
```

Use a `buffer_to_string` helper (in `mod rendering` test submodules) to extract text from `Buffer`.

Keep `#[mutants::skip]` on draw functions — mutation testing visual layout isn't productive.

## Coverage Exclusions

Use `#[cfg_attr(coverage_nightly, coverage(off))]` only for:
- `main.rs` terminal setup/teardown
- Event loop methods that call `event::read()` (require a real terminal)

Never exclude from coverage:
- `src/model/` — 100% coverage target
- `src/adif/` — pure functions, fully testable
- `src/storage/` — use tempfile for isolation
- `handle_key()` methods in TUI — contains real logic
- `draw_*` functions — use `TestBackend` render tests
