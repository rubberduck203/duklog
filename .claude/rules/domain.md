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
- `park_ref` — POTA park reference (format: `[A-Z]{1,3}-\d{4,5}`, e.g. `K-0001`); `String` on `PotaLog`, not `Option`
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

## Log Enum Model (ADR-0001)

```
LogHeader    — station_callsign, operator, grid_square, qsos, created_at, log_id  (all pub(crate))
GeneralLog   — header: LogHeader
PotaLog      — header: LogHeader, park_ref: String
FieldDayLog  — header: LogHeader, tx_count: u8, class: FdClass, section: String, power: FdPowerCategory
WfdLog       — header: LogHeader, tx_count: u8, class: WfdClass, section: String
Log enum     — General(GeneralLog) | Pota(PotaLog) | FieldDay(FieldDayLog) | WinterFieldDay(WfdLog)
```

Access shared fields via `log.header()` / `log.header_mut()`. Type-specific fields via pattern match.
`Log` does not derive `Serialize`/`Deserialize` — storage is ADIF. See [ADR-0001](../../docs/adr/0001-log-enum-model.md).

`Qso` carries: `exchange_rcvd: Option<String>` (received contest exchange, contest logs only) and `frequency: Option<u32>` (kHz; required for FD/WFD, optional otherwise). Both default to `None`.

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

- XDG path: `~/.local/share/duklog/logs/` with one `.adif` file per log
- Export is a file copy (`std::fs::copy`) — no reformatting
- Auto-save after every mutation

## Offline References

Consult `docs/reference/` during implementation instead of fetching from the web:
- `adif-spec-notes.md` — ADIF v3.1.6 format, field syntax, band/mode values
- `pota-rules-notes.md` — POTA activation rules, required fields, park reference format
- `ratatui-notes.md` — Ratatui architecture, widgets, terminal setup pattern
- `testing-tools-notes.md` — cargo-llvm-cov and cargo-mutants usage
