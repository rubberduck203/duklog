# duklog Implementation Plan

## Context

duklog is a new offline ham radio logging TUI for POTA activations. The project is a blank slate — just a hello world `main.rs` and no dependencies. The user wants to be maximally hands-off, so we need to set up Claude Code to be autonomous before writing features.

Three phases: (1) dev tooling & project structure, (2) Claude Code autonomy setup, (3) feature implementation.

**User decisions**: XDG storage (`~/.local/share/duklog/`), multiple saved logs, feature branches + PRs.

---

## Phase 1: Technical Guardrails

**Branch**: `setup/tooling`

### 1.1 Install Dev Tools
```
rustup component add llvm-tools
cargo install cargo-llvm-cov
cargo install cargo-mutants
```

### 1.2 Project Structure
Split into lib + bin crate. All testable logic in `src/lib.rs` and submodules:

```
src/
  main.rs              # thin: terminal setup/teardown, calls duklog::run()
  lib.rs               # re-exports, run()
  model/               # Log, Qso, Band, Mode, ValidationError
  adif/                # ADIF writer (pure functions, no I/O)
  storage/             # JSON persistence (XDG), ADIF file export
  tui/                 # App state, event loop, screens, widgets
    screens/           # log_create, qso_entry, qso_list, export, help
    widgets/           # status_bar, form
```

### 1.3 Dependencies (`Cargo.toml`)
```toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"

[dev-dependencies]
quickcheck = "1"
quickcheck_macros = "1"
tempfile = "3"
```

No async runtime. No `difa` crate — ADIF writing is trivial string formatting.

### 1.4 Code Coverage Setup (cargo-llvm-cov)

cargo-llvm-cov instruments the compiled code via LLVM to measure which lines/branches/functions are exercised by tests.

**Installation requires two steps:**
```
rustup component add llvm-tools    # LLVM instrumentation support
cargo install cargo-llvm-cov       # the cargo subcommand
```

**Key commands:**
- `cargo llvm-cov` — run tests and print summary to stdout
- `cargo llvm-cov --html` — generate HTML report in `target/llvm-cov/html/`
- `cargo llvm-cov --fail-under-lines 90` — exit code 1 if line coverage < 90%
- `cargo llvm-cov --fail-under-functions 90` — same for function coverage

**Excluding code from coverage** (for TUI rendering, main.rs setup, etc.):
```rust
// At crate root (lib.rs):
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

// On functions that can't be meaningfully unit-tested:
#[cfg_attr(coverage_nightly, coverage(off))]
fn terminal_setup() { ... }
```

Add to `Cargo.toml` to suppress lint warnings about the custom cfg:
```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(coverage_nightly)'] }
```

**What to exclude from coverage:**
- `src/main.rs` — terminal setup/teardown, panic hooks (test via manual run)
- TUI `render()` methods — visual output, tested by eye not assertion
- `#[mutants::skip]` functions (see below) should also be `coverage(off)`

**What must NOT be excluded:**
- All `src/model/` logic — this is the core, 100% coverage target
- All `src/adif/` logic — pure functions, fully testable
- All `src/storage/` logic — use tempfile for isolation
- All `handle_key()` methods in TUI — these contain real logic

### 1.5 Mutation Testing Setup (cargo-mutants)

Mutation testing goes beyond coverage: it verifies that tests actually *check behavior*, not just *execute code*. cargo-mutants introduces bugs (mutants) into the source and checks whether the test suite catches them.

**How it works:**
1. Copies the source tree (never modifies originals by default)
2. Applies mutations: replacing function bodies with `Default::default()`, swapping operators, changing return values, etc.
3. Runs `cargo test` against each mutant
4. Reports results per mutant: **caught** (test failed — good), **missed** (tests still passed — bad), **unviable** (didn't compile — neutral), **timeout** (hung — investigate)

**Mutation types cargo-mutants generates:**
- Replace function body with default return value (`Default::default()`, `Ok(Default::default())`, `""`, `0`, `true`, `false`, `vec![]`, `None`)
- Replace binary operators (`+` → `-`, `==` → `!=`, `<` → `<=`, `&&` → `||`, etc.)
- Delete statements (especially early returns, guard clauses)
- Replace constants and literals

**Key commands:**
- `cargo mutants --list` — preview all mutants without running (use to estimate scope)
- `cargo mutants` — run all mutants
- `cargo mutants -f src/model/` — run mutants only in model module
- `cargo mutants --timeout 60` — per-mutant timeout in seconds
- `cargo mutants -j 4` — parallel jobs (defaults to num CPUs)

**Writing tests that survive mutation testing — CRITICAL GUIDANCE:**

1. **Assert on specific values, not just "no error":**
   ```rust
   // BAD — mutant returning any String will pass:
   assert!(!format_field("CALL", "W1AW").is_empty());

   // GOOD — mutant must produce this exact string:
   assert_eq!(format_field("CALL", "W1AW"), "<CALL:4>W1AW");
   ```

2. **Test both branches of every condition:**
   ```rust
   // If the code has: if callsign.is_empty() { return Err(...) }
   // You need BOTH:
   assert!(Log::new("", "K-0001").is_err());    // empty rejected
   assert!(Log::new("W1AW", "K-0001").is_ok()); // valid accepted
   ```

3. **Test boundary values, not just happy paths:**
   ```rust
   // If activation requires 10 QSOs:
   assert_eq!(log_with_n_qsos(9).needs_for_activation(), 1);
   assert_eq!(log_with_n_qsos(10).needs_for_activation(), 0);
   assert_eq!(log_with_n_qsos(11).needs_for_activation(), 0);
   assert!(!log_with_n_qsos(9).is_activated());
   assert!(log_with_n_qsos(10).is_activated());
   ```

4. **Assert on computed values, not just types:**
   ```rust
   // BAD — mutant changing the count logic still passes:
   assert!(log.qso_count_today() > 0);

   // GOOD — exact value must match:
   assert_eq!(log.qso_count_today(), 3);
   ```

5. **Use quickcheck for functions with string inputs** to catch mutations that change character-level logic:
   ```rust
   #[quickcheck]
   fn valid_callsign_roundtrips(call: String) -> bool {
       // If it passes validation, it should be usable
       match validate_callsign(&call) {
           Ok(()) => !call.is_empty() && call.chars().all(|c| c.is_ascii_alphanumeric() || c == '/'),
           Err(_) => call.is_empty() || call.chars().any(|c| !c.is_ascii_alphanumeric() && c != '/'),
       }
   }
   ```

**Skipping functions with `#[mutants::skip]`:**

Use sparingly. Valid reasons to skip:
- TUI rendering functions (visual output, no testable return value)
- `main()` and terminal setup/teardown
- `Display` / `Debug` impls that are purely cosmetic
- Logging/tracing calls

**Never skip:**
- Validation logic
- Data transformation functions
- ADIF formatting
- Any function with a meaningful return value

To use, add to `Cargo.toml`:
```toml
[dependencies]
mutants = "0.0.3"  # only provides the #[mutants::skip] attribute
```

**Cautions from mutants.rs:**
- Tests must limit side effects to temporary resources — mutations can cause unexpected I/O behavior if tests touch real files/network
- Always use `tempfile::tempdir()` for storage tests, never write to real XDG paths
- Never use `--in-place` flag — always let cargo-mutants work on a copy
- The `mutants.out/` directory is created in the project root — add to `.gitignore`

### 1.6 Makefile

```makefile
.PHONY: build check test fmt lint coverage mutants ci doc clean

build:
	cargo build

check:
	cargo check

test:
	cargo test

fmt:
	cargo fmt --check

lint:
	cargo clippy -- -D warnings

coverage:
	cargo llvm-cov --html --fail-under-lines 90

coverage-report:
	cargo llvm-cov --open

mutants:
	cargo mutants --timeout 60

mutants-list:
	cargo mutants --list

mutants-module:
	@test -n "$(MOD)" || (echo "Usage: make mutants-module MOD=src/model/" && exit 1)
	cargo mutants -f "$(MOD)" --timeout 60

ci: fmt lint test coverage
	@echo "All CI checks passed"

doc:
	cargo doc --no-deps --open

clean:
	cargo clean
	rm -rf mutants.out/
```

Add to `.gitignore`:
```
/target
/mutants.out
```

### 1.7 Verification
- `make ci` passes on skeleton project with stub modules and at least one `#[test]`
- `make mutants-list` shows expected mutant count (small for skeleton)
- `make coverage` generates an HTML report

---

## Phase 2: Claude Code Autonomy

**Branch**: `setup/claude-code`

### 2.1 CLAUDE.md
The primary reference for all coding decisions. Key sections:

- **Project overview**: Offline POTA logger TUI, no network access ever
- **Module structure**: What lives where
- **Dev commands**: Always use `make` targets, not raw `cargo`
- **Coding style**: Iterators over loops, expressions over return, match over if let, no `.unwrap()` in lib code, derive order convention
- **Error handling**: Each module has its own `thiserror` error type
- **Testing requirements**: Every pub fn tested, pass + fail cases for validation, quickcheck for string inputs, `make mutants` per module, 90% coverage minimum
- **Documentation requirements**: Rustdoc on all pub items, update `docs/` per feature
- **Data model reference**: Three tiers (per-log, slow-moving, fast-moving) with field details
- **ADIF/POTA reference**: Required fields, format spec, activation threshold (10 QSOs)
- **Git workflow**: Feature branches, PRs, `make ci` before any commit

### 2.2 Hooks (`.claude/settings.local.json`)
PostToolUse hooks on Edit/Write of `.rs` files:
1. `cargo check` — immediate compilation feedback after every file edit
2. `cargo clippy -- -D warnings` — style enforcement after every edit

No hooks for tests (too slow) or mutants (way too slow). Those run explicitly via `make ci` / `make mutants`.

### 2.3 Code Review Agent

Create a custom `code-review` slash command (`.claude/commands/code-review.md`) that acts as an adversarial reviewer. This agent is invoked after completing a feature branch, before creating a PR. Its job is to find problems the developer (Claude) is blind to.

**The agent prompt will instruct it to**:
- Run `git diff main...HEAD` to see all changes on the branch
- Review against CLAUDE.md standards (style, testing, docs)
- Check for: missing tests, surviving mutants, uncovered branches, `.unwrap()` in lib code, loops that should be iterators, `if let` that should be `match`
- Check for: missing rustdoc, missing `docs/` updates, incorrect ADIF field names/formats
- Check for: error handling gaps, panics in non-test code, hardcoded paths, off-by-one errors
- Run `make ci` and `make mutants` and report any failures
- Output a structured review: **Blockers** (must fix), **Suggestions** (should fix), **Nits** (optional)
- Be adversarial: assume the code has bugs and look for them

**Usage workflow**: After completing a feature, run `/code-review` before creating the PR. Fix all blockers, then create the PR.

### 2.4 Documentation Skeleton
```
docs/
  architecture.md     # Module layout, data flow, design decisions
  user-guide.md       # Usage, keybindings, workflow
  adif-format.md      # ADIF output format, POTA field mapping
  development.md      # Dev setup, running tests, contributing
```

Created as stubs, filled in as features are implemented.

### 2.5 Verification
`make ci` passes with hooks firing correctly on edits. `/code-review` runs successfully on the setup branch itself.

---

## Phase 3: Feature Implementation

Each step is a separate feature branch + PR. Run `/code-review` before each PR.

### 3.1 Data Model (`feature/data-model`)
**Files**: `src/model/{band,mode,qso,log,validation}.rs`

- `Band` enum: all amateur bands, `adif_str()` method, `all()` for TUI lists
- `Mode` enum: SSB, CW, FT8, FT4, PSK31, RTTY, FM, AM, DIGI
- `Qso` struct: their_call, rst_sent/rcvd, band, mode, timestamp (UTC), comments, their_park (P2P)
- `Log` struct: station_callsign, operator, park_ref, grid_square, qsos vec, created_at, log_id
- `ValidationError` via thiserror: empty callsign, invalid park ref, invalid grid square
- `Log::new()` with validation, `add_qso()`, `qso_count_today()`, `needs_for_activation()`, `is_activated()`
- Tests: quickcheck for callsign/park validation, boundary tests for activation threshold across UTC midnight

### 3.2 ADIF Export (`feature/adif-writer`)
**Files**: `src/adif/{fields,writer}.rs`

- `format_field(name, value) -> String` — `<NAME:len>value`
- `format_qso(log, qso) -> String` — all POTA fields for one record
- `format_adif(log) -> String` — full file with header + EOH + records + EOR
- Pure functions, no I/O. Tested independently.
- Tests: field formatting, required POTA fields present, date/time format, quickcheck round-trip

### 3.3 Storage (`feature/storage`)
**Files**: `src/storage/{session,manager}.rs`

- XDG path: `~/.local/share/duklog/logs/` with JSON files per log
- `LogManager`: list_logs, save_log, load_log, delete_log
- `export_adif(log, path)` — calls adif writer, writes to file
- Default export path: `~/duklog-{PARK}-{YYYYMMDD}.adif`
- Tests with `tempfile::tempdir()`, round-trip save/load, export produces valid ADIF

### 3.4 TUI Shell (`feature/tui-shell`)
**Files**: `src/tui/{app,events}.rs`, `src/main.rs`, `src/lib.rs`

- `Screen` enum: LogSelect, LogCreate, QsoEntry, QsoList, Export, Help
- `App` struct with event loop: draw → read event → dispatch → act
- Terminal setup/teardown in main.rs with panic hook to restore terminal
- Auto-save after every mutation
- Global keys: q/Esc=quit/back, F1/?=help
- Tests: state transitions, initial screen selection

### 3.5 Log Management Screens (`feature/log-management`)
**Files**: `src/tui/screens/{log_select,log_create}.rs`, `src/tui/widgets/form.rs`

- **Log Select**: list existing logs or create new. Shows park ref, date, QSO count per log.
- **Log Create**: form for station_callsign, operator, park_ref, grid_square. Validation inline.
- **Form widget**: reusable labeled text input with focus state and error display
- Tab/Shift-Tab navigation, Enter to submit

### 3.5.1 Make Operator Optional (`feature/optional-operator`)
**Files**: `src/model/log.rs`, `src/adif/writer.rs`, `src/storage/manager.rs`, `src/tui/screens/log_create.rs`

Per ADIF spec, OPERATOR is "individual operator callsign (if different from station)" and POTA rules only require both fields for club activations. Make `Log.operator` an `Option<String>`:

- **Model**: Change `Log.operator` from `String` to `Option<String>`. `Log::new()` accepts `Option<String>`, validates only if `Some`. Skip operator validation when `None`.
- **ADIF writer**: Emit `OPERATOR` tag only when `Some` and different from `station_callsign`. When `None`, omit the tag entirely.
- **Storage**: Backward-compatible — existing JSONL files with a string `operator` field deserialize into `Some(operator)`. New logs with no operator serialize as `null`.
- **TUI LogCreate**: Change operator form field from required to optional. When left empty, pass `None` to `Log::new()`.
- **Tests**: Update all affected tests across model, ADIF, storage, and TUI modules.

### 3.6 QSO Entry Screen (`feature/qso-entry`)
**Files**: `src/tui/screens/qso_entry.rs`

- Status bar: park, callsign, QSO count / activation progress
- Slow-moving fields (Band, Mode): changed with `b`/`m` cycling, not Tab-focused
- Fast-moving fields (Their Call, RST Sent, RST Rcvd, Comments): Tab-navigated
- Enter: validate → create Qso → add to log → save → clear fast fields → keep slow fields
- Last 3 QSOs shown below form as confirmation
- RST defaults: "59" for SSB, "599" for CW/digital

### 3.7 QSO List Screen (`feature/qso-list`)
**Files**: `src/tui/screens/qso_list.rs`

- Scrollable ratatui `Table`: Time, Call, Band, Mode, RST S/R, Comments
- Up/Down scrolling, column headers
- Can be developed in parallel with 3.6

### 3.8 Export Screen (`feature/export`)
**Files**: `src/tui/screens/export.rs`

- Shows default export path, QSO count, confirmation prompt
- Calls `storage::export_adif()`, shows success/error

### 3.9 Polish (`feature/polish`)
**Files**: `src/tui/widgets/status_bar.rs`, `src/tui/screens/help.rs`, various

- Status bar widget on all screens: park, callsign, QSO count, activation status (green when activated)
- Help screen: full keybinding reference
- Error handling polish: all errors display gracefully, no panics in normal operation
- Final `make mutants` pass across entire codebase
- Complete all `docs/` files

---

## Dependency Graph

```
Phase 1 → Phase 2 → 3.1 Data Model → 3.2 ADIF Writer → 3.3 Storage
                                                              ↓
                                                         3.4 TUI Shell
                                                              ↓
                                                    3.5 Log Management
                                                              ↓
                                                  3.5.1 Optional Operator
                                                              ↓
                                              3.6 QSO Entry  ←→  3.7 QSO List (parallel)
                                                              ↓
                                                         3.8 Export
                                                              ↓
                                                         3.9 Polish
```

## Reference Documentation

Domain and framework research has been saved to `docs/reference/`:

- `docs/reference/adif-spec-notes.md` — ADIF v3.1.6 file format, field syntax, band/mode values, header format
- `docs/reference/pota-rules-notes.md` — POTA activation rules, required/recommended ADIF fields, park reference format, P2P contacts
- `docs/reference/ratatui-notes.md` — Ratatui architecture, widget inventory, crossterm integration, terminal setup/teardown pattern
- `docs/reference/testing-tools-notes.md` — cargo-llvm-cov and cargo-mutants setup, commands, test writing guidance for mutation testing

These are distilled from the official docs and should be consulted during implementation rather than fetching from the web (offline-first).

---

## Verification

**After each step**: `make ci` passes (fmt, lint, test, coverage)
**After each module**: `make mutants` — no surviving mutants
**Before each PR**: `/code-review` passes with no blockers
**End-to-end acceptance** (after 3.9):
1. Launch → select/create log → log 10 QSOs → see "ACTIVATED" → export ADIF
2. Quit and relaunch → session restored → switch between logs
3. Inspect ADIF file: all required POTA fields present, correct format
