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
- **Phase 5.3 — Log-type-aware recent QSO display** — Done: `draw_recent_qsos` branches on form type; General shows RST + freq; POTA shows RST + their_park (park takes priority over freq when both set); FD/WFD show exchange_rcvd + freq

---

## Remaining Work

### Phase 5.4 — `q`-key / `Esc` consistency audit

**Priority: Low | Effort: Small | Depends on: —**

**Why**: On screens with editable text fields, pressing `q` inserts the character `q` into the field rather than navigating away, which is surprising to users who expect `q` to quit. `Esc` is the correct and consistent key for cancelling/navigating back in editing contexts. Some screens currently bind both `q` and `Esc` for navigation; screens with free-text input should only use `Esc`.

**Scope**: Audit all screens for `q`-as-navigation bindings. Remove `q` from any screen that has free-text input fields. Keep `q` on list/selection screens (Log Select, QSO List) where there is no text entry. Update `docs/user-guide.md` keybinding tables accordingly.

**Files**: `src/tui/screens/*.rs`, `docs/user-guide.md`

### Phase 5 dependency order

```
5.1 ──► 5.3
5.1 ──► Phase 10 (US license privilege checker)
Phase 6 ──► (future) Geographic QSO analysis / county/state tallies
```

---

### Phase 6 — POTA park database

**Priority: High | Effort: Large | Depends on: —**

**Why**: Park refs are entered manually with no feedback. Having a local copy of the POTA park database enables name display, autocomplete, and soft validation — improving accuracy without requiring network access during logging.

This is a large feature. The scope here is the roadmap description; full implementation details are deferred until this phase is started. Before starting Phase 6, discuss what other features the park database might unlock (e.g., geographic QSO analysis, county/state worked tallies).

**Sync mechanism** (transparent / automatic):

The app is fully synchronous (no async runtime; `tokio-util` is only used for the `difa` ADIF encoder trait). Background sync uses a `std::thread` spawned during `duklog::run()` startup. The TUI starts immediately; sync happens concurrently. Results are delivered via a `std::sync::mpsc` channel polled in the event loop alongside crossterm events.

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

