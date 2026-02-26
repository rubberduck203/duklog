# ADIF Export Format

## Overview

duklog exports logs in ADIF (Amateur Data Interchange Format) v3.1.6, the standard format accepted by POTA for log submission. Field encoding uses the [`difa`](https://crates.io/crates/difa) crate, which outputs lowercase markers (`<eoh>`, `<eor>`) per the case-insensitive ADIF spec.

## File Structure

Each exported file contains a header followed by QSO records:

```
<ADIF_VER:5>3.1.6
<PROGRAMID:6>duklog
<PROGRAMVERSION:5>0.1.0
<CREATED_TIMESTAMP:15>20260216 120000
<eoh>

<STATION_CALLSIGN:4>W1AW<OPERATOR:4>W1AW<CALL:6>KD9XYZ<QSO_DATE:8>20260216<TIME_ON:6>143000<BAND:3>20M<MODE:3>SSB<RST_SENT:2>59<RST_RCVD:2>59<MY_GRIDSQUARE:4>FN31<MY_SIG:4>POTA<MY_SIG_INFO:6>K-0001<eor>
```

## Fields Written

### Header Fields

| Field | Description |
|---|---|
| `ADIF_VER` | Always `3.1.6` |
| `PROGRAMID` | Always `duklog` |
| `PROGRAMVERSION` | Current application version |
| `CREATED_TIMESTAMP` | UTC timestamp of export (YYYYMMDD HHMMSS) |

### Per-QSO Fields

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
| `MY_GRIDSQUARE` | Activator's Maidenhead grid square | Yes |
| `MY_SIG` | `POTA` (only when log has a park ref) | No |
| `MY_SIG_INFO` | Activator's park reference | No (with `MY_SIG`) |
| `SIG` | `POTA` (only for P2P contacts) | No |
| `SIG_INFO` | Other station's park ref (P2P) | No (with `SIG`) |
| `COMMENT` | QSO comments/notes | No (when non-empty) |

## Log Type Support

| Log Type | Export Supported | Notes |
|---|---|---|
| POTA | Yes | Includes `MY_SIG`/`MY_SIG_INFO` when park ref is set |
| General | Yes | No POTA-specific fields |
| Field Day | Planned (Phase 4.3) | Will include `CONTEST_ID`, `STX_STRING`, `SRX_STRING` |
| Winter Field Day | Planned (Phase 4.3) | Will include `CONTEST_ID`, `STX_STRING`, `SRX_STRING`, `FREQ` |

The `Qso` struct carries `exchange_rcvd: Option<String>` (received contest exchange) and `frequency: Option<u32>` (kHz) for use by Field Day and WFD export in Phase 4.3. These fields are not written to ADIF by the current POTA/General exporter.

## POTA Submission

Upload the exported `.adif` file at https://pota.app under activator tools. One file per activation (one park, one UTC day).
