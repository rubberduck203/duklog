---
paths:
  - "src/adif/**"
  - "src/storage/**"
---

# ADIF and Storage Architecture

## Storage Design

Each log is a single `.adif` file in `~/.local/share/duklog/logs/` (XDG). The ADIF header encodes all log metadata; subsequent records encode QSOs.

- **Append QSO**: O(1) pure file append (no reformatting)
- **Edit QSO**: rewrites the full file via `save_log` — necessary because ADIF has no in-place edit primitive
- **Export**: `std::fs::copy` of the internal file to `~/Documents/duklog/` — no reformatting; the internal file is immediately valid ADIF

Auto-save after every model mutation — no explicit save action needed.

## ADIF Module Layout

`src/adif/` contains pure formatting functions (writer) and an async reader. **No I/O in the writer** — the storage module handles all file writes. This keeps ADIF logic fully unit-testable.

## Async Runtime

`LogManager` holds a `tokio::runtime::Runtime` (current-thread, no worker threads) solely to drive `difa::RecordStream` during log reads via `block_on`. All public storage APIs are synchronous. The TUI event loop is unaffected.

## difa Crate Usage

- Writer: `TagEncoder` with `BytesMut` — synchronous, spec-compliant ADIF encoding
- `Tag::Eoh` and `Tag::Eor` include trailing newlines
- Reader: `difa::RecordStream` (async) — invoked via `Runtime::block_on` in `LogManager`

## ADIF Header Field Taxonomy

True ADIF header fields (per spec): `ADIF_VER`, `CREATED_TIMESTAMP`, `PROGRAMID`, `PROGRAMVERSION`.

Fields placed in the header for convenience (QSO record fields in spec, but valid anywhere — apply uniformly to all QSOs):
- `STATION_CALLSIGN`, `OPERATOR` — always emitted
- `MY_GRIDSQUARE` — emitted only when `!log.header().grid_square.is_empty()`

App-extension fields (`APP_DUKLOG_*`) encode log metadata not expressible in standard fields:
`APP_DUKLOG_LOG_TYPE`, `APP_DUKLOG_LOG_ID`, `APP_DUKLOG_PARK_REF`, `APP_DUKLOG_FD_CLASS`,
`APP_DUKLOG_SECTION`, `APP_DUKLOG_POWER`, `APP_DUKLOG_TX_COUNT`, `APP_DUKLOG_WFD_CLASS`.

`APP_DUKLOG_LOG_TYPE` is the discriminant for the `Log` enum variant.

## FREQ Unit Convention

`FREQ` in ADIF is MHz: `format!("{:.3}", f64::from(freq) / 1000.0)`. Stored internally as kHz, exported as MHz.

## Reader Pattern (ADR-0004)

The ADIF reader is hand-written — **not** serde-based. Domain enums expose `adif_str()` / `from_adif_str()` as the explicit ADIF ↔ Rust conversion layer. Structural mismatches that make generic serde impractical:

- `QSO_DATE` + `TIME_ON` → single `DateTime<Utc>`
- `FREQ` MHz ↔ kHz unit conversion
- Optional fields absent from record when `None`

See [ADR-0004](../../docs/adr/0004-hand-written-adif-reader.md) for full rationale.

## Legacy Migration

`.jsonl` files from pre-5.5 storage are auto-migrated to ADIF on startup via a serde-based JSONL reader kept in `manager.rs`. Logs without `log_type` default to `Pota` during deserialization to preserve existing user data.

## Duplicate Detection Scoping

`Log::find_duplicates` is type-aware:
- **POTA / General**: scoped to today (UTC)
- **FieldDay / WinterFieldDay**: scoped across the entire log (events span two UTC calendar days)

`LogManager::create_log` checks for duplicate logs (same callsign + operator + park_ref + grid on same UTC day) and returns `StorageError::DuplicateLog` if found. For non-POTA log types (General, FD, WFD), `park_ref` is empty and is not a differentiating factor — two General logs with the same callsign+operator+grid on the same UTC day are duplicates.
