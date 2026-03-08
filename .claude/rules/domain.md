---
paths:
  - "src/model/**"
  - "src/adif/**"
  - "src/storage/**"
---

# Domain Reference

## Data Model

Three categories of data:

### Per-Log (set at log creation, rarely changes)
- `station_callsign` — callsign used on air
- `operator` — individual operator callsign (may equal station_callsign)
- `park_ref` — optional POTA park reference (format: `[A-Z]{1,3}-\d{4,5}`, e.g. `K-0001`)
- `grid_square` — Maidenhead locator
- `log_id` — unique identifier
- `created_at` — UTC timestamp

### Slow-Moving (changes occasionally during operation)
- `band` — amateur band (e.g. `20M`, `40M`)
- `mode` — operating mode (SSB, CW, FT8, etc.)

### Fast-Moving (changes every QSO)
- `their_call` — other station's callsign
- `rst_sent` / `rst_rcvd` — signal reports
- `comments` — optional notes
- `their_park` — other station's park reference (P2P contacts)
- `timestamp` — UTC date/time of contact

## ADIF Reference

### Field Format
```
<FIELDNAME:length>value
```
Where `length` is the byte length of `value`.

### Required POTA Fields
`STATION_CALLSIGN`, `CALL`, `QSO_DATE` (YYYYMMDD), `TIME_ON` (HHMMSS), `BAND`, `MODE`

### Recommended POTA Fields
`OPERATOR`, `MY_SIG` (always `POTA`), `MY_SIG_INFO` (park ref), `RST_SENT`, `RST_RCVD`, `SIG`/`SIG_INFO` (for P2P)

### Activation Threshold
10 QSOs from a single park within one UTC day.

### RST Defaults by Mode
- SSB/FM/AM: `59` (2-digit)
- CW/PSK31/RTTY: `599` (3-digit)
- FT8/FT4/JS8: `-10` (dB)

## Storage

- XDG path: `~/.local/share/duklog/logs/` with one `.adif` file per log (ADIF is the single storage format)
- Export is a file copy (`std::fs::copy`) — no reformatting; the internal file is already valid ADIF
- Auto-save after every mutation

## ADIF Header Field Taxonomy

True ADIF header fields (per spec): `ADIF_VER`, `CREATED_TIMESTAMP`, `PROGRAMID`, `PROGRAMVERSION`.

Fields placed in the header for convenience (defined as QSO record fields in spec, but valid anywhere):
`STATION_CALLSIGN`, `OPERATOR`, `MY_GRIDSQUARE` — apply uniformly to every QSO; no need to repeat per record.

App-extension fields (`APP_DUKLOG_*`): encode log metadata not expressible in standard fields
(`APP_DUKLOG_LOG_TYPE`, `APP_DUKLOG_LOG_ID`, `APP_DUKLOG_PARK_REF`, `APP_DUKLOG_FD_CLASS`,
`APP_DUKLOG_SECTION`, `APP_DUKLOG_POWER`, `APP_DUKLOG_TX_COUNT`, `APP_DUKLOG_WFD_CLASS`).

## Offline References

Consult `docs/reference/` during implementation instead of fetching from the web:
- `adif-spec-notes.md` — ADIF v3.1.6 format, field syntax, band/mode values
- `pota-rules-notes.md` — POTA activation rules, required fields, park reference format
- `ratatui-notes.md` — Ratatui architecture, widgets, terminal setup pattern
- `testing-tools-notes.md` — cargo-llvm-cov and cargo-mutants usage
