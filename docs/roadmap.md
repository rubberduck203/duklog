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

---

## Remaining Work

### Phase 5.1 — Optional frequency for General and POTA logs

**Priority: High | Effort: Small | Depends on: —**

**Why**: General and POTA logs currently have no way to record operating frequency. WFD already has a required frequency field; General/POTA should have an optional one so the full ADIF FREQ field can be populated for any log type.

**What already exists** (no model changes needed):
- `Qso.frequency: Option<u32>` is already in the model (kHz)
- `Band::from_frequency_khz()` auto-detect is already implemented (PR #30)
- `try_auto_set_band_from_frequency()` exists in `qso_entry.rs` but only triggers for FD/WFD (`has_contest_exchange()` guard)
- ADIF `FREQ` is already emitted when `qso.frequency.is_some()` for FD/WFD, but `Log::General` and `Log::Pota` arms in `encode_type_specific_fields` skip it

**Files**: `src/tui/screens/qso_entry.rs`, `src/adif/writer.rs`

**QSO entry form changes** (`src/tui/screens/qso_entry.rs`):

Current General form (4 fields):
```
0: Their Callsign  1: RST Sent  2: RST Rcvd  3: Comments
```

New General form (5 fields):
```
0: Their Callsign  1: RST Sent  2: RST Rcvd  3: Frequency (kHz)  4: Comments
```

Current POTA form (5 fields):
```
0: Their Callsign  1: RST Sent  2: RST Rcvd  3: Their Park  4: Comments
```

New POTA form (6 fields):
```
0: Their Callsign  1: RST Sent  2: RST Rcvd  3: Their Park  4: Frequency (kHz)  5: Comments
```

- Add `GENERAL_FREQUENCY: usize = 3` and `POTA_FREQUENCY: usize = 4` constants
- Update `comments_idx()`: General 3→4, Pota 4→5
- Frequency field is **optional** (not required) — empty input → `None`
- Tab/BackTab leaving the frequency field → call `try_auto_set_band_from_frequency()` (currently only fires for contest forms; extend to General/POTA)
- `try_auto_set_band_from_frequency()` currently uses `CONTEST_FREQUENCY` directly — make it use the correct index for the current form type
- `submit()`: parse frequency for General/POTA (optional; non-empty must be valid positive integer, invalid → field error)
- `clear_fast_fields()`: clear the frequency field for General/POTA
- `start_editing()`: populate frequency field from `qso.frequency`

**ADIF writer changes** (`src/adif/writer.rs`):

Move `FREQ` emission out of the FD/WFD-specific arms and into a shared post-match block that applies to all log types:

```rust
if let Some(freq) = qso.frequency {
    let mhz = format!("{:.3}", f64::from(freq) / 1000.0);
    encode(encoder, buf, field_tag("FREQ", mhz.as_str()))?;
}
```

This replaces the duplicated freq blocks in both `Log::FieldDay` and `Log::WinterFieldDay` and makes FREQ work for General and POTA too.

**Tests to add/update**:
- ADIF writer: `general_qso_with_frequency_emits_freq`, `pota_qso_with_frequency_emits_freq`, `general_qso_without_frequency_omits_freq`
- QSO entry: update render tests for new field counts (General 5 fields, POTA 6 fields); add submit tests for optional frequency

---

### Phase 5.3 — Log-type-aware recent QSO display

**Priority: Medium | Effort: Small | Depends on: 5.1 (General/POTA freq column only meaningful after 5.1)**

**Why**: The recent QSOs table on the QSO entry screen is hard-coded to show RST and P2P park columns for all log types. FD/WFD QSOs display a meaningless "59/59" RST (just the stored default — not exchanged) and never show the received exchange or frequency. After Phase 5.1, General/POTA QSOs will also have an optional frequency that isn't reflected anywhere.

**Current rendering** (`src/tui/screens/qso_entry.rs`, `draw_recent_qsos`):
```
Time | Their Call | Band | Mode | RST Sent/Rcvd | P2P park ref
```
Columns are static; `state.form_type` is accessible from this function (same module) but is not used.

**Files**: `src/tui/screens/qso_entry.rs` (`draw_recent_qsos` only — `#[mutants::skip]`)

**Proposed column layout by log type**:

| Log type | Col 1 | Col 2 | Col 3 | Col 4 | Col 5 | Col 6 |
|----------|-------|-------|-------|-------|-------|-------|
| General  | Time  | Call  | Band  | Mode  | RST   | Freq (if set) |
| POTA     | Time  | Call  | Band  | Mode  | RST   | Their Park / Freq |
| FD / WFD | Time  | Call  | Band  | Mode  | Exchange | Freq |

For General/POTA: last column shows their park for POTA (empty for General without park data), or frequency if set and no park. For FD/WFD: replace RST with `exchange_rcvd` (e.g. `"3A CT"`); last column shows `frequency` as kHz (e.g. `"14225"`).

**Implementation**:
- `draw_recent_qsos` branches on `state.form_type`:
  - `has_rst()` branch: existing columns, but col 6 changes from "P2P {park}" to just `their_park.as_deref().unwrap_or("")` and, after 5.1 lands, adds frequency info
  - `has_contest_exchange()` branch: swap RST col for exchange col; last col shows frequency
- Column widths stay the same (6 fixed cols); only cell content changes
- No changes outside `draw_recent_qsos`; no model or logic changes needed

**Tests**: Draw functions are `#[mutants::skip]`; no new tests required. If render tests exist for this function, update expected strings.

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

