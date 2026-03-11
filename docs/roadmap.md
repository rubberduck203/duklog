# duklog Roadmap

## Completed Phases

- **Phase 1: Technical Guardrails** (`setup/tooling`) — Done
- **Phase 2: Claude Code Autonomy** (`setup/claude-code`) — Done
- **3.1 Data Model** (`feature/data-model`, PR #3) — Done
- **3.2 ADIF Export** (`feature/adif-writer`, PR #4) — Done
- **3.3 Storage** (`feature/storage`, PR #5, #6) — Done
- **3.4 TUI Shell** (`feature/tui-shell`, PR #7) — Done
- **3.5 Log Management Screens** (`feature/log-management`, PR #7) — Done
- **3.5.1 Optional Operator** (`feature/optional-operator`, PR #8) — Done
- **3.6 QSO Entry Screen** (`feature/qso-entry`, PR #9, #10) — Done
- **3.7 QSO List Screen** (`feature/qso-entry`, PR #10) — Done
- **3.7b QSO Editing** (`feature/qso-editing`, PR #12) — Done
- **3.8 Export Screen** (`feature/export-screen`, PR #11) — Done
- **3.9 Delete Log** (`feature/delete-log`) — Done
- **3.10 Duplicate QSO Detection** (`feature/duplicate-qso-detection`) — Done
- **3.11 Duplicate Log Prevention** (`feature/duplicate-log-prevention`) — Done
- **3.12 Polish** (`feature/polish`) — Done
- **4.0 Log enum refactor** (`feature/polish`) — Done
- **4.1 FieldDay and WFD model types** (`feature/log-types-model`) — Done
- **4.1.5 Refactor: submodule extraction and function decomposition** (`feature/refactor-structure`) — Done
- **4.1.6 Validation bug fixes** (`feature/validation-fixes`) — Done
- **4.2 Log type selection in log create flow** — Done
- **4.3 Field Day QSO entry + form layout redesign** — Done
- **4.3.1 Log create form layout fixes** — Done
- **4.3.2 FD/WFD exchange-only forms** — Done
- **4.4 Log select and status bar updates** — Done
- **4.4.5 QSO deletion** (`feature/qso-deletion`) — Done
- **Phase 4: Multiple Logbook Types** — Done
- **Export to Documents dir + log-type-aware filenames** (`feature/export-to-documents-dir`) — Done: ADIF exports written to `~/Documents/duklog/` (auto-created); filenames are log-type-specific: POTA → `{CALL}@{PARK}-{DATE}.adif`; General → `{CALL}-{DATE}.adif`; FD → `{CALL}-FD-{DATE}.adif`; WFD → `{CALL}-WFD-{DATE}.adif`; `/` in callsigns replaced with `_`; export path is editable on the export screen; `park_ref` made required on `PotaLog`; `DefaultFilename` trait added
- **Phase 5.1 — Optional frequency for General and POTA logs** — Done
- **Phase 5.3 — Log-type-aware recent QSO display** — Done: `draw_recent_qsos` branches on form type (superseded by Phase 5.6 column layout)
- **Phase 5.6 — Log-type-aware Recent QSOs display** — Done: fully branched column sets per log type; POTA has separate Their Park and Frequency columns (no fallback mixing, fixes #37/#39); row count adapts to `Rect` height (fixes #40); `insta` snapshot tests added; `buffer_to_string` consolidated into `src/tui/test_utils.rs`; ADR-0005 added for rendering test strategy
- **Phase 5.4 — `q`-key / `Esc` consistency audit** — Done: removed `q` as navigation key from Log Select, QSO List, and Help screens; `Esc` is the sole navigation/quit key everywhere
- **Phase 5.5 — ADIF native storage** — Done: internal storage switched from `.jsonl` to `.adif`; log metadata encoded in ADIF header via standard and `APP_DUKLOG_*` fields; QSO appends remain O(1) file appends; reads use `difa::RecordStream` via a current-thread tokio runtime held by `LogManager`; export simplified to `std::fs::copy`; legacy `.jsonl` files auto-migrated on startup; `tokio` and `futures` added as direct dependencies

---

## Remaining Work

### Phase 5 dependency order

```
5.1 ──► 5.3
5.1 ──► Phase 10 (US license privilege checker)
Phase 6 ──► (future) Geographic QSO analysis / county/state tallies
```

---

### Post-1.0 — Remove JSONL migration code

**Priority: Low | Effort: Tiny | Depends on: 1.0 release**

**Why**: The JSONL-to-ADIF migration code in `src/storage/manager.rs` (`load_jsonl_from_path`, `reconstruct_field_day`, `reconstruct_wfd`, `LogMetadata`, `StoredLogType`, `migrate_jsonl_files`) is marked `// Migration only — delete after 1.0`. Once 1.0 ships, it is safe to assume all users have migrated. Remove the dead code, the `serde_json` dependency, and the `Serialize`/`Deserialize` derives on model types that were only needed for JSONL.

**Files**: `src/storage/manager.rs`, `Cargo.toml`, model derive annotations.

---

### Refactor: QsoFormType enum variants carrying display types

**Priority: Low | Effort: Small | Depends on: —**

**Why**: The `RecentQsoDisplay` trait in `draw_recent_qsos` is currently implemented via free functions (`recent_qso_row_general`, etc.) and a `build_recent_rows` helper. A more idiomatic Rust design would embed the display struct into each `QsoFormType` variant:

```rust
enum QsoFormType {
    General(GeneralDisplay),
    Pota(PotaDisplay),
    FieldDay(ContestDisplay),
    WinterFieldDay(ContestDisplay),
}
```

This would let `draw_recent_qsos` extract the `&dyn RecentQsoDisplay` directly from the variant without a separate conversion match — the association between variant and display type is encoded in the enum itself. Each display type's `impl RecentQsoDisplay` groups `recent_row` and `column_widths` together, and the compiler enforces completeness per type.

**Trade-off**: `QsoFormType` is constructed and matched throughout `qso_entry.rs` (form building, submit handlers, key handlers). All construction sites become `QsoFormType::General(GeneralDisplay)` and matches use `QsoFormType::General(_)`. Mechanical but broad. Worth revisiting once there is a second `draw_*` function with a similar branching pattern — that's when the shared display-type infrastructure starts to pay for itself.

**Files**: `src/tui/screens/qso_entry.rs`.

---

### Snapshot coverage expansion

**Priority: Low | Effort: Small | Depends on: Phase 5.6 (done)**

**Why**: Phase 5.6 introduced `insta` snapshot tests for `draw_recent_qsos`, demonstrating that `.snap` files are human-readable text renderings that make layout regressions immediately visible as diffs. The same coverage should extend to all `draw_*` functions in `src/tui/screens/`. Currently only `qso_entry`'s recent-QSOs panel has snapshots; the log select table, log create form, QSO list, export screen, and help screen have render tests using only `.contains()` assertions.

**Scope**: For each `src/tui/screens/*/` screen, add at least one `assert_snapshot!(terminal.backend())` test at 80×24. Screens with multiple variants (log types, states) need one snapshot per variant. The `.snap` files serve as a living visual reference for the current UI layout — inspect them by reading the snapshot files directly.

**Files**: `src/tui/screens/` (all screen modules), `src/tui/screens/snapshots/`.

---

### Phase 5.7 — RST entry UX

**Priority: Medium | Effort: Small | Depends on: —**

**Why**: RST fields are pre-populated with "59" as a convenience, but the cursor lands at the end of the text. Operators who give real signal reports must backspace twice before typing, which is clunky. (#38)

**Scope**: Either (a) place the cursor at the start of the field in overwrite mode so the first keystroke replaces the default, or (b) leave fields empty and rely on the "59" default only at submit time. Evaluate both options when starting the phase; the goal is to make changing the default fast without penalising operators who accept "59" as-is.

**Files**: `src/tui/screens/qso_entry/` (field initialisation).

---

### Phase 6 — POTA park database

**Priority: High | Effort: Large | Depends on: —**

**Why**: Park refs are entered manually with no feedback. Having a local copy of the POTA park database enables name display, autocomplete, and soft validation — improving accuracy without requiring network access during logging.

This is a large feature. The scope here is the roadmap description; full implementation details are deferred until this phase is started. Before starting Phase 6, discuss what other features the park database might unlock (e.g., geographic QSO analysis, county/state worked tallies).

**Sync mechanism** (transparent / automatic):

The app uses a minimal tokio runtime (current-thread, no worker threads) inside `LogManager` for ADIF reads. Background sync uses a separate `std::thread` spawned during `duklog::run()` startup — no additional async complexity needed. The TUI starts immediately; sync happens concurrently. Results are delivered via a `std::sync::mpsc` channel polled in the event loop alongside crossterm events.

The "No network access, ever" project principle is revised to:
> No network access during logging. An optional background sync at startup fetches the POTA park database when connectivity is available.

**Data source**: POTA publishes a parks CSV at `https://pota.app/all_parks_ext.csv` (park_reference, name, active status, country, state). This is the dataset to fetch and cache.

**Local cache**: `~/.local/share/duklog/parks/pota_parks.json` (same XDG data dir as logs). Include a `fetched_at` timestamp; re-fetch when older than 7 days or absent.

**New dependency**: An HTTP client crate is needed. Prefer `ureq` (sync, minimal, no async runtime required) over `reqwest` (async-first).

**Sub-phases**:

#### 6.1 — Sync infrastructure
- `src/parks/` module: `ParkDatabase` struct, `ParkRecord { park_ref, name, active }`, load/save as JSON, `fetch()` function using `ureq`
- `src/storage/` or `src/parks/`: cache path helper (`data_dir/parks/pota_parks.json`)
- Spawn background thread in `lib.rs::run()`; send `ParkSyncEvent` (Started, Done(count), Failed(msg)) via `mpsc::channel`
- `App` holds `Option<Arc<ParkDatabase>>` (None until sync delivers it) and `park_sync_status: ParkSyncStatus` (Idle, Syncing, Ready(count), Failed)
- Status bar or footer shows a subtle indicator while syncing; clears on completion

#### 6.2 — Park lookup in log create and status bar
- Log create screen: when focus leaves the Park Ref field on a POTA form, look up the ref in `ParkDatabase` and display the park name below the field (or as a subtitle in the field block)
- Status bar: when a POTA log is active and the database is loaded, show park name alongside the ref: `[K-0001 Valley Forge NHP] 7/10 QSOs`

#### 6.3 — Autocomplete and soft validation in QSO entry
- "Their Park" field in POTA QSO entry: pressing Tab offers completion from `ParkDatabase` (filter by prefix of entered text); confirm with Tab or Enter
- Soft validation at submit: if park ref is not found in the database, show a warning (not a blocking error) — "Park not found in local database; may be stale"

---

### Phase 7 — Auto-generated screenshots

**Priority: Low | Effort: Small | Depends on: —**

**Why**: Screenshots in `docs/user-guide.md` drift out of sync whenever a screen layout changes. Keeping them accurate requires manual effort that is easy to skip.

**Scope**: Extend the existing `TestBackend` test infrastructure (already used for `draw_*` unit tests) to render each screen into a text buffer and write the result to `docs/screenshots/<screen>.txt` as part of a `make screenshots` target. No new test framework required — the pattern is already established in `src/tui/screens/*/tests`.

---

### Phase 8 — Field Day bonus points tracker

**Priority: Medium | Effort: Medium | Depends on: Phase 4 FD model (done)**

**Why**: ARRL Field Day scoring includes bonus points (100% emergency power, media contact, satellite QSO, natural power, W1AW bulletin, etc.) that can rival or exceed the QSO-based score. There is currently no way to record or total bonus points in duklog, so operators must track them separately on paper.

**Scope**: New screen accessible from the FD QSO list or log select screen. Displays the ARRL-defined bonus categories (from `docs/reference/arrl-field-day-notes.md`) as a checklist with boolean toggles or count fields (e.g., number of media contacts). Stores claimed bonuses in the `FieldDayLog` header (new field, backward-compatible via `#[serde(default)]`). Displays a running total: `QSO points + bonus points = raw score` alongside the power multiplier. No ADIF impact — bonus data is duklog-internal.

**Files**: `src/model/field_day.rs`, new `src/tui/screens/fd_bonus.rs`, `src/storage/manager.rs` (schema addition)

---

### Phase 9 — WFD objectives tracker

**Priority: Medium | Effort: Small–Medium | Depends on: Phase 4 WFD model (done)**

**Why**: WFD scoring uses a ×1/×2/×3 multiplier based on completed objectives (natural power, satellite contact, digital mode, etc.). Operators need to see which objectives they have claimed and how they affect final score.

**Scope**: Similar to the FD bonus tracker but simpler — WFD objectives are boolean flags (each completed objective adds +1 to the multiplier, capped at ×3). New screen showing the objective list from `docs/reference/winter-field-day-notes.md` with toggles. Stores claimed objectives in the `WfdLog` header (backward-compatible). Displays `QSO count × multiplier` alongside the objective checklist.

**Files**: `src/model/wfd.rs`, new `src/tui/screens/wfd_objectives.rs`, `src/storage/manager.rs` (schema addition)

---

### Phase 10 — US license privilege checker

**Priority: Medium | Effort: Medium | Depends on: Phase 5.1 (optional freq for General/POTA)**

**Why**: It is easy to accidentally log a QSO on a frequency outside your license class privileges, especially when band-hopping during an activation. A soft warning at QSO entry catches this without blocking logging.

**Scope**: At QSO submit (and optionally on Tab leaving the frequency field), compare `qso.frequency` against the operator's configured license class. Show a non-blocking warning if the frequency falls in a restricted segment. License class stored in user preferences (`~/.local/share/duklog/config.json`, new). Reference `docs/reference/fcc-us-band-privileges.md` for per-band sub-range details (General class has disjoint allowed segments on 80m, 40m, 20m, 15m; 60m is channelized). US-only initially; could be extended to other ITU regions.

**Depends on Phase 5.1** — General and POTA logs have no frequency field until 5.1 lands, so the checker is only useful once all log types can record frequency.

