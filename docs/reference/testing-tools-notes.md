# Testing Tools Reference

## cargo-llvm-cov (Code Coverage)

Reference: https://github.com/taiki-e/cargo-llvm-cov

### What It Does
Instruments compiled Rust code via LLVM to measure which lines, branches, and functions are exercised by tests. Generates coverage reports in multiple formats.

### Installation
```bash
rustup component add llvm-tools    # required LLVM component
cargo install cargo-llvm-cov       # the cargo subcommand
```

### Commands

| Command | Purpose |
|---|---|
| `cargo llvm-cov` | Run tests, print coverage summary |
| `cargo llvm-cov --html` | Generate HTML report in `target/llvm-cov/html/` |
| `cargo llvm-cov --open` | Generate and open HTML report |
| `cargo llvm-cov --text` | Plain text report to stdout |
| `cargo llvm-cov --json --output-path cov.json` | Machine-readable JSON |
| `cargo llvm-cov --lcov --output-path lcov.info` | LCOV format (CI tools) |
| `cargo llvm-cov --fail-under-lines 90` | Fail if line coverage < 90% |
| `cargo llvm-cov --fail-under-functions 90` | Fail if function coverage < 90% |

### Excluding Code from Coverage

For functions that can't be meaningfully unit-tested (TUI rendering, terminal setup):

```rust
// At crate root:
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

// On specific functions:
#[cfg_attr(coverage_nightly, coverage(off))]
fn terminal_setup() { ... }
```

Suppress lint warning in Cargo.toml:
```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(coverage_nightly)'] }
```

### Merging Multiple Coverage Runs

Useful if testing with different feature flags:
```bash
cargo llvm-cov clean --workspace
cargo llvm-cov --no-report --features feature_a
cargo llvm-cov --no-report --features feature_b
cargo llvm-cov report --html
```

---

## cargo-mutants (Mutation Testing)

Reference: https://mutants.rs/ (especially https://mutants.rs/cautions.html)

### What It Does
Goes beyond coverage to verify tests actually **check behavior**. It introduces bugs (mutants) into source code and verifies the test suite catches them. A "missed" mutant means your tests didn't notice the bug — indicating a testing gap.

Coverage tells you what code is *reached* by tests. Mutation testing tells you whether tests *actually verify* the code's behavior.

### How It Works
1. Copies the source tree to a temporary directory (never modifies originals)
2. Applies one mutation at a time (e.g., replace function body with default value)
3. Runs `cargo test` against the mutated code
4. Reports whether each mutant was caught or missed

### Result Categories

| Result | Meaning | Action |
|---|---|---|
| **Caught** | Test suite failed (detected the bug) | Good — no action needed |
| **Missed** | Tests still passed despite the bug | **Bad** — improve tests |
| **Unviable** | Mutant didn't compile | Neutral — skip |
| **Timeout** | Tests hung on this mutant | Investigate — may indicate infinite loop path |

### Mutation Types Generated

- **Replace function body** with `Default::default()`, `Ok(Default::default())`, `""`, `0`, `true`, `false`, `vec![]`, `None`
- **Swap binary operators**: `+` → `-`, `==` → `!=`, `<` → `<=`, `&&` → `||`
- **Delete statements** (especially early returns and guard clauses)
- **Replace constants and literals**

### Commands

| Command | Purpose |
|---|---|
| `cargo mutants --list` | Preview all mutants without running |
| `cargo mutants --list --diff` | Preview with source diffs |
| `cargo mutants` | Run all mutants |
| `cargo mutants -f src/model/` | Run only in model module |
| `cargo mutants -e src/tui/` | Exclude TUI module |
| `cargo mutants --timeout 60` | Per-mutant timeout (seconds) |
| `cargo mutants -j 4` | Parallel jobs |

### Writing Mutation-Resistant Tests

**1. Assert exact values, not just types or existence:**
```rust
// BAD — any string satisfies this:
assert!(!format_field("CALL", "W1AW").is_empty());

// GOOD — exact output required:
assert_eq!(format_field("CALL", "W1AW"), "<CALL:4>W1AW");
```

**2. Test both branches of every condition:**
```rust
// For: if callsign.is_empty() { return Err(...) }
assert!(Log::new("", "K-0001").is_err());    // empty → rejected
assert!(Log::new("W1AW", "K-0001").is_ok()); // valid → accepted
```

**3. Test boundary values:**
```rust
// Activation requires 10 QSOs:
assert_eq!(log_with_n_qsos(9).needs_for_activation(), 1);
assert_eq!(log_with_n_qsos(10).needs_for_activation(), 0);
assert_eq!(log_with_n_qsos(11).needs_for_activation(), 0);
assert!(!log_with_n_qsos(9).is_activated());
assert!(log_with_n_qsos(10).is_activated());
```

**4. Assert computed values, not just properties:**
```rust
// BAD — mutant changing count logic passes:
assert!(log.qso_count_today() > 0);

// GOOD — exact count:
assert_eq!(log.qso_count_today(), 3);
```

**5. Use quickcheck for string-processing functions:**
```rust
#[quickcheck]
fn valid_callsign_roundtrips(call: String) -> bool {
    match validate_callsign(&call) {
        Ok(()) => !call.is_empty(),
        Err(_) => call.is_empty() || has_invalid_chars(&call),
    }
}
```

### Skipping Functions (`#[mutants::skip]`)

Add `mutants = "0.0.3"` to `[dependencies]` in Cargo.toml (provides only the attribute macro).

**Valid reasons to skip:**
- TUI `render()` methods (visual output, no testable return value)
- `main()` and terminal setup/teardown
- `Display` / `Debug` impls that are purely cosmetic

**Never skip:**
- Validation logic
- Data transformation / computation
- ADIF formatting
- Anything with a meaningful return value

### Cautions (from mutants.rs/cautions.html)

- **Test side effects**: Mutations can cause unexpected I/O. If tests write to real paths, a mutant could corrupt data. Always use `tempfile::tempdir()` for filesystem tests.
- **Never use `--in-place`**: It modifies the real source tree. If interrupted, mutations persist. Always use the default copy mode.
- **Never use production credentials in tests**: Mutations might alter code paths that touch external systems.
- **`mutants.out/` directory**: Created in project root during runs. Add to `.gitignore`.
- **Mutation marker**: All mutations contain the comment `/* ~ changed by cargo-mutants ~ */` — useful for debugging if you need to inspect a mutant.

---

## quickcheck (Property-Based Testing)

Reference: https://github.com/BurntSushi/quickcheck

### What It Does
Generates random inputs for test functions and checks that properties hold across all of them. If a property fails, quickcheck shrinks the input to find the minimal failing case.

### Usage
```rust
use quickcheck_macros::quickcheck;

#[quickcheck]
fn callsign_validation_never_panics(input: String) -> bool {
    // Property: validation always returns Ok or Err, never panics
    validate_callsign(&input).is_ok() || validate_callsign(&input).is_err()
}

#[quickcheck]
fn adif_field_length_matches_value(name: String, value: String) -> bool {
    let field = format_field(&name, &value);
    // Property: the length tag matches the actual value length
    field.contains(&format!(":{}>{}", value.len(), value))
}
```

### When to Use
- Any function that processes arbitrary string input (callsigns, park refs, grid squares)
- Serialization/deserialization round-trips
- Numeric computations where edge cases matter (QSO counts, thresholds)
- Validation functions (property: valid inputs accepted, invalid rejected)

### When NOT to Use
- Functions with complex setup requirements (prefer targeted unit tests)
- Functions with small, enumerable input spaces (just test all cases)
