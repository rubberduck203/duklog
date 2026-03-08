# ADIF Native Storage — Implementation Plan (Phase 5.5)

## Context

duklog currently uses two formats for the same data:

- **Internal storage** (`~/.local/share/duklog/logs/{id}.jsonl`): JSON Lines via serde_json
- **Export** (`~/Documents/duklog/{filename}.adif`): ADIF via difa

ADIF is the canonical ham radio exchange format. Storing data as JSON to later convert to ADIF is accidental complexity. This phase makes ADIF the single storage format, so internal storage files are immediately usable by external tools (e.g. drag-and-drop to POTA upload portal) without an explicit export step.

**This is a breaking change** — existing `.jsonl` files cannot be read by the new storage backend without migration.

---

## Architecture After Change

**Storage files:**
```
~/.local/share/duklog/logs/{log_id}.adif    # internal working copy
~/Documents/duklog/{human-filename}.adif    # user-exported copy
```

**ADIF header encodes log metadata** using standard fields where they exist, and `APP_DUKLOG_*` fields for app-specific metadata. The `APP_` prefix is the ADIF-spec-blessed convention for application-specific extensions; external tools silently ignore them.

Example POTA log header:
```adif
<ADIF_VER:5>3.1.6
<PROGRAMID:6>duklog
<PROGRAMVERSION:5>0.5.0
<CREATED_TIMESTAMP:15>20260307 120000
<STATION_CALLSIGN:6>W1AW/P
<OPERATOR:4>W1AW
<MY_GRIDSQUARE:4>FN20
<MY_SIG:4>POTA
<MY_SIG_INFO:6>K-1234
<APP_DUKLOG_LOG_TYPE:4>pota
<eoh>
```

Field mapping for all log types:

| Metadata | Standard field | APP field |
|---|---|---|
| station_callsign | STATION_CALLSIGN | — |
| operator | OPERATOR | — |
| grid_square | MY_GRIDSQUARE | — |
| created_at | CREATED_TIMESTAMP | — |
| park_ref (POTA) | MY_SIG=POTA + MY_SIG_INFO | — |
| log_type | — | APP_DUKLOG_LOG_TYPE (pota/general/field_day/wfd) |
| tx_count (FD/WFD) | — | APP_DUKLOG_TX_COUNT |
| fd_class (FD) | — | APP_DUKLOG_FD_CLASS |
| wfd_class (WFD) | — | APP_DUKLOG_WFD_CLASS |
| section (FD/WFD) | — | APP_DUKLOG_SECTION |
| power (FD) | — | APP_DUKLOG_POWER |

The `log_id` is already the filename — no need to store it in the header.

**QSO records** are unchanged — all fields already map to standard ADIF fields (CALL, QSO_DATE, TIME_ON, BAND, MODE, RST_SENT, RST_RCVD, COMMENT, FREQ, SIG, SIG_INFO, SRX_STRING).

**Export** becomes a file copy (`std::fs::copy`) from the internal path to `~/Documents/duklog/{friendly-name}.adif`. No format conversion needed since internal storage IS valid ADIF.

---

## I/O Strategy

**Writing (header + full log):** Synchronous `std::fs` — call `adif::format_adif(log)`, write to file. No tokio needed.

**Appending a QSO:** ADIF records are self-terminating (`<eor>`) and there is no file-level length header or checksum, so appending is a true O(1) append — no reading required.

```rust
// append_qso: open in append mode, write formatted record, done
let mut file = OpenOptions::new().append(true).open(&path)?;
file.write_all(adif::format_qso(log, qso).as_bytes())?;
```

This mirrors the existing JSONL append behavior exactly.

**Reading (loading a log):** difa's `RecordStream` is async. Add tokio as a direct dependency; `LogManager` holds a `tokio::runtime::Runtime` (created once at construction) and calls `runtime.block_on(...)` for each load operation. This keeps all call sites synchronous.

```toml
# Cargo.toml addition
tokio = { version = "1", features = ["rt", "fs", "io-util"] }
```

Note: `tokio-util` is already a transitive dependency (used by difa). The roadmap note in Phase 6 about "no async runtime" becomes outdated — update it when this phase lands.

---

## Implementation Steps

### Step 1: Add tokio dependency

`Cargo.toml`: add `tokio = { version = "1", features = ["rt", "fs", "io-util"] }`.

### Step 2: Update `src/adif/mod.rs` — write metadata into ADIF header

Extend `format_header(log: &Log)` to emit:

- `MY_SIG` + `MY_SIG_INFO` in header for POTA logs
- `APP_DUKLOG_LOG_TYPE` for all log types
- FD-specific: `APP_DUKLOG_TX_COUNT`, `APP_DUKLOG_FD_CLASS`, `APP_DUKLOG_SECTION`, `APP_DUKLOG_POWER`
- WFD-specific: `APP_DUKLOG_TX_COUNT`, `APP_DUKLOG_WFD_CLASS`, `APP_DUKLOG_SECTION`

### Step 3: Add `src/adif/reader.rs` — async ADIF log reader

New module wrapping difa's `RecordStream`:

```rust
/// Parses an ADIF file and reconstructs a Log.
pub async fn read_log(path: &Path, log_id: &str) -> Result<Log, AdifError>
```

Implementation:

- Open file with `tokio::fs::File`
- Create `difa::RecordStream::new(reader, ignore_partial: true)`
- First record (header): extract all fields, construct the appropriate `Log` variant
- Subsequent records (QSOs): map each `Record` to a `Qso`
- Return fully-reconstructed `Log` (with QSOs populated)

Field parsing:

- `APP_DUKLOG_LOG_TYPE` → determines which `Log` variant to construct
- `STATION_CALLSIGN`, `OPERATOR`, `MY_GRIDSQUARE`, `CREATED_TIMESTAMP` → `LogHeader`
- `MY_SIG_INFO` → POTA `park_ref`
- `APP_DUKLOG_*` → FD/WFD-specific fields
- Per-QSO: `CALL` → `their_call`, `QSO_DATE`+`TIME_ON` → `timestamp`, `BAND` → `Band::from_str()`, `MODE` → `Mode::from_str()`, `RST_SENT`, `RST_RCVD`, `COMMENT`, `FREQ` (MHz → kHz), `SIG_INFO` → `their_park`, `SRX_STRING` → `exchange_rcvd`

### Step 4: Update `src/storage/manager.rs`

- Add `runtime: tokio::runtime::Runtime` to `LogManager`; initialize in `new()` and `with_path()`
- Change `log_file_path()` extension from `.jsonl` to `.adif`
- `save_log()`: call `adif::format_adif(log)`, write to `.adif` (sync `std::fs`)
- `append_qso()`: `OpenOptions::new().append(true).open(path)`, write `format_qso()` output — **no reading**
- `load_log()`: `self.runtime.block_on(adif::reader::read_log(path, log_id))`
- `list_logs()`: glob `.adif` files, call `load_log()` for each
- Remove `LogMetadata`, `StoredLogType` structs
- Add `migrate_jsonl_files()` (see Migration below)

### Step 5: Update `src/storage/export.rs`

Simplify `export_adif()`: use `std::fs::copy(internal_adif_path, export_path)`. Remove the `adif::format_adif()` + write call path.

### Step 6: Update `src/storage/error.rs`

- Remove `Json(#[from] serde_json::Error)`

### Step 7: Audit and remove serde dependencies

- Audit `Qso` — if `#[derive(Serialize, Deserialize)]` is no longer needed for storage, remove it (check tests for any JSON-based assertions)
- Remove `serde_json` from `Cargo.toml`
- Keep `serde` if still needed (chrono's serde feature, etc.)

### Step 8: Update roadmap note in Phase 6

Phase 6 notes "tokio-util is only used for the difa ADIF encoder trait; background sync uses std::thread." Once Phase 5.5 lands, tokio is a direct runtime dependency — update the note; Phase 6 background sync can use `tokio::spawn` or stay as `std::thread`.

---

## Migration

Auto-migrate existing `.jsonl` files on first startup. In `LogManager::new()`, call `migrate_jsonl_files()`:

1. Glob `*.jsonl` in the logs directory
2. For each: read and parse using the existing JSONL deserialization code (kept temporarily)
3. Write ADIF using `adif::format_adif()`
4. Delete the `.jsonl` file

Keep the JSONL parsing path behind a `// Migration only — delete after 1.0` comment. Remove it (and `serde_json`) once the migration window closes.

---

## What Does NOT Change

- Domain model (`Log`, `LogHeader`, `Qso`, `Band`, `Mode`) — untouched
- ADIF QSO field mapping — same fields, no changes
- TUI layer — no changes
- Export filename convention — unchanged
- Storage directory location (`~/.local/share/duklog/logs/`) — unchanged

---

## Verification

1. `make ci` passes
2. Create a POTA log, add a QSO — verify `~/.local/share/duklog/logs/{id}.adif` is created and is valid ADIF
3. Open the `.adif` file in a text editor or external tool — verify it's readable
4. Restart duklog — verify log and QSOs reload correctly
5. Export — verify `~/Documents/duklog/{friendly-name}.adif` appears
6. Drop a legacy `.jsonl` file in the logs dir — verify auto-migration runs and produces correct ADIF
7. `cargo tree | grep serde_json` → empty
