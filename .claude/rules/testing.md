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
- **Be aggressive**: any new pure function should get at least one quickcheck property — default to quickcheck, not hand-written examples
- **Idempotency**: every normalization/transform function must have a `fn foo_is_idempotent(s: String) -> bool` property
- **Normalize → validate roundtrip**: when a normalizer feeds into a validator, add a property that constructs invalid-case-but-structurally-valid input, normalizes it, and asserts validation passes
- **ASCII guard**: when a function is documented to operate on ASCII inputs (e.g., callsigns, grid squares), add `if !s.is_ascii() { return true; }` at the top of idempotency properties — this avoids spurious failures from Unicode expansion edge cases and documents the domain
- Assert on **specific values**, not just `is_ok()` / `is_empty()` — critical for mutation testing
- Use `tempfile::tempdir()` for all storage tests — never write to real paths
- For storage deserialization: test corrupt-but-parseable inputs (e.g., `tx_count: 0`, empty section), not just missing fields — these bypass `None` guards but still violate domain invariants
- Tests must be deterministic and fast
- **Organize tests with submodules** (`mod typing { ... }`, `mod validation { ... }`), not section comments (`// --- Typing ---`)
- **Tests that exercise only one enum variant belong in that variant's submodule file.** If a test creates only a `Log::FieldDay(...)` and asserts on `Log`-level behaviour specific to that variant, move it to `field_day.rs`, not `mod.rs`. Tests that compare multiple variants or test shared/generic behaviour stay in the enum's module.
- **Extract test helpers** to reduce repetition — tests are code too; refactor shared setup into helper functions. When touching a file that has duplicated helpers (render helpers, fixture builders, common setup), move them to `src/tui/test_utils.rs` or a module-local shared helper as part of that same PR — don't defer it.
- Prefer `.expect("descriptive message")` over bare `.unwrap()` in tests — the message surfaces in failure output and makes failures easier to diagnose
- Minimum 90% line coverage enforced by `make coverage`

## TUI Render Testing

Two approaches are in active use — see **ADR-0005** for the ongoing decision.

### Pattern A — `buffer_to_string` + `.contains()` (baseline)

```rust
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use crate::tui::test_utils::buffer_to_string; // shared — do NOT redefine locally

let backend = TestBackend::new(80, 24);
let mut terminal = Terminal::new(backend).unwrap();
terminal.draw(|frame| {
    draw_my_widget(&state, frame, frame.area());
}).unwrap();
let content = buffer_to_string(terminal.backend().buffer());
assert!(content.contains("expected text"));
```

`buffer_to_string` is defined once in `src/tui/test_utils.rs`. Do not copy-paste it into individual test modules.

Tests semantic presence; cannot distinguish correct vs. incorrect column position.

### Pattern B — `insta` snapshots (introduced Phase 5.6, experimental)

```rust
use insta::assert_snapshot;

#[test]
fn recent_qsos_pota_no_park() {
    let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
    terminal.draw(|frame| draw_recent_qsos(&state, frame, frame.area())).unwrap();
    assert_snapshot!(terminal.backend()); // stores .snap file — a literal picture of the layout
}
```

First run writes a `.snap` file in a `snapshots/` directory adjacent to the test file.
Update after intentional layout changes: `cargo insta review`.

Use snapshots for **layout components where column/row position matters** — they catch
positional bugs that `.contains()` cannot. Use `.contains()` for **semantic assertions**
(field present/absent by log type, specific text visible).

Keep `#[mutants::skip]` on draw functions — mutation testing visual layout isn't productive.

**Required coverage for screen draw functions:**
- Every `draw_*` function in `src/tui/screens/` must have a `mod rendering` block with at least one render test **at 80×24** (standard terminal) asserting all field labels and UI elements are visible.
- For screens with multiple variants (e.g. log types, states), add one render test per variant — the test renders at 80×24 and asserts the variant-specific label appears. This prevents truncation/overflow bugs that only appear at wide or narrow terminal widths.
- Form-based screens must constrain form area width with `Constraint::Max(N)` centered via `Constraint::Fill(1)` on each side so fields don't span the full terminal width on wide displays. 60–80 chars is appropriate for short-label forms.

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
