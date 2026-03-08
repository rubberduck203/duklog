# ADIF Storage and Export Format

## Overview

duklog uses ADIF (Amateur Data Interchange Format) v3.1.6 as its **single storage format** — the same file that is stored internally is also the file that gets exported. No reformatting is required at export time; `export_adif` is a file copy. Field encoding uses the [`difa`](https://crates.io/crates/difa) crate, which outputs lowercase markers (`<eoh>`, `<eor>`) per the case-insensitive ADIF spec.

Internal storage: `~/.local/share/duklog/logs/{log-id}.adif`

Export destination: `~/Documents/duklog/{filename}.adif` (falls back to `~/duklog/` if Documents is unavailable)

## File Structure

Each file contains a header followed by QSO records. The header encodes both standard ADIF metadata and duklog-specific log metadata via `APP_DUKLOG_*` application-extension fields.

```
<ADIF_VER:5>3.1.6
<CREATED_TIMESTAMP:15>20260216 120000
<PROGRAMID:6>duklog
<PROGRAMVERSION:5>0.6.0
<STATION_CALLSIGN:4>W1AW
<MY_GRIDSQUARE:4>FN31
<APP_DUKLOG_LOG_ID:24>K-0001-20260216-120000
<APP_DUKLOG_PARK_REF:6>K-0001
<APP_DUKLOG_LOG_TYPE:4>pota
<eoh>

<CALL:6>KD9XYZ<QSO_DATE:8>20260216<TIME_ON:6>143000<BAND:3>20M<MODE:3>SSB<RST_SENT:2>59<RST_RCVD:2>59<MY_SIG:4>POTA<MY_SIG_INFO:6>K-0001<eor>
```

## Header Fields

### Standard ADIF header fields

| Field | Description |
|---|---|
| `ADIF_VER` | Always `3.1.6` |
| `CREATED_TIMESTAMP` | UTC timestamp of log creation (YYYYMMDD HHMMSS) |
| `PROGRAMID` | Always `duklog` |
| `PROGRAMVERSION` | Current application version |
| `STATION_CALLSIGN` | Station callsign |
| `OPERATOR` | Operator callsign (omitted when same as `STATION_CALLSIGN`) |
| `MY_GRIDSQUARE` | Maidenhead grid square (omitted when not set; FD/WFD logs may not set it) |

### APP_DUKLOG_* metadata fields (all log types)

| Field | Description |
|---|---|
| `APP_DUKLOG_LOG_ID` | Unique log identifier; used by duklog to reload the log from disk |
| `APP_DUKLOG_LOG_TYPE` | Log variant: `general`, `pota`, `field_day`, or `wfd` |

### APP_DUKLOG_* metadata fields (POTA logs only)

| Field | Description |
|---|---|
| `APP_DUKLOG_PARK_REF` | POTA park reference (e.g. `K-0001`); used by duklog to reconstruct the log on load |

### APP_DUKLOG_* metadata fields (Field Day logs only)

| Field | Description |
|---|---|
| `APP_DUKLOG_TX_COUNT` | Number of transmitters |
| `APP_DUKLOG_FD_CLASS` | Field Day class (e.g. `B`) |
| `APP_DUKLOG_SECTION` | ARRL/RAC section (e.g. `EPA`) |
| `APP_DUKLOG_POWER` | Power category: `qrp`, `low`, or `high` |

### APP_DUKLOG_* metadata fields (Winter Field Day logs only)

| Field | Description |
|---|---|
| `APP_DUKLOG_TX_COUNT` | Number of transmitters |
| `APP_DUKLOG_WFD_CLASS` | WFD class (e.g. `H`) |
| `APP_DUKLOG_SECTION` | WFD section (e.g. `EPA`) |

## Per-QSO Fields

| Field | Source | Always Present |
|---|---|---|
| `STATION_CALLSIGN` | Log station callsign | Yes |
| `OPERATOR` | Operator callsign | No (omitted when same as station callsign) |
| `CALL` | Other station's callsign | Yes |
| `QSO_DATE` | QSO UTC date (YYYYMMDD) | Yes |
| `TIME_ON` | QSO UTC time (HHMMSS) | Yes |
| `BAND` | Operating band | Yes |
| `MODE` | Operating mode | Yes |
| `RST_SENT` | Signal report sent | Yes |
| `RST_RCVD` | Signal report received | Yes |
| `MY_GRIDSQUARE` | Activator's Maidenhead grid square | No (omitted when not set) |
| `MY_SIG` | `POTA` (POTA logs only, when park ref is set) | No |
| `MY_SIG_INFO` | Activator's park reference | No (with `MY_SIG`) |
| `SIG` | `POTA` (POTA logs only, P2P contacts) | No |
| `SIG_INFO` | Other station's park ref (P2P) | No (with `SIG`) |
| `CONTEST_ID` | `ARRL-FIELD-DAY` (FD) or `WFD` (WFD) | No (contest logs) |
| `STX_STRING` | Sent exchange: `<tx_count><class> <section>` | No (contest logs) |
| `SRX_STRING` | Received exchange (verbatim from QSO entry) | No (contest logs, when present) |
| `FREQ` | Operating frequency in **MHz** (e.g. `14.225`) | No (all log types, when frequency is set) |
| `COMMENT` | QSO comments/notes | No (when non-empty) |

Note: `MY_SIG`/`MY_SIG_INFO` appear in QSO records for POTA contacts. The park reference is also stored in `APP_DUKLOG_PARK_REF` in the header (as log metadata), so the two serve different purposes: QSO-level fields for external tool compatibility, header metadata for duklog internal round-trip.

## Log Type Support

| Log Type | Notes |
|---|---|
| General | `FREQ` (MHz) when frequency is set |
| POTA | `MY_SIG`/`MY_SIG_INFO` per QSO; `SIG`/`SIG_INFO` for P2P; `FREQ` when set; `APP_DUKLOG_PARK_REF` in header |
| Field Day | `CONTEST_ID=ARRL-FIELD-DAY`, `STX_STRING`, `SRX_STRING`, `FREQ`; FD metadata in `APP_DUKLOG_*` header fields |
| Winter Field Day | `CONTEST_ID=WFD`, `STX_STRING`, `SRX_STRING`, `FREQ`; WFD metadata in `APP_DUKLOG_*` header fields |

## POTA Submission

Upload the exported `.adif` file at https://pota.app under activator tools. Since internal and exported files are identical, you can also submit the internal file directly from `~/.local/share/duklog/logs/`. One file per activation (one park, one UTC day).
