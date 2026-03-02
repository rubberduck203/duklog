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
| `MY_SIG` | `POTA` (POTA logs only, when park ref is set) | No |
| `MY_SIG_INFO` | Activator's park reference | No (with `MY_SIG`) |
| `SIG` | `POTA` (POTA logs only, P2P contacts) | No |
| `SIG_INFO` | Other station's park ref (P2P) | No (with `SIG`) |
| `CONTEST_ID` | `ARRL-FIELD-DAY` (FD) or `WFD` (WFD) | No (contest logs) |
| `STX_STRING` | Sent exchange: `<tx_count><class> <section>` | No (contest logs) |
| `SRX_STRING` | Received exchange (verbatim from QSO entry) | No (contest logs, when present) |
| `FREQ` | Operating frequency in **MHz** (e.g. `14.225`) | No (WFD logs only) |
| `COMMENT` | QSO comments/notes | No (when non-empty) |

## Log Type Support

| Log Type | Export Supported | Notes |
|---|---|---|
| General | Yes | No contest or activation fields |
| POTA | Yes | `MY_SIG`/`MY_SIG_INFO` when park ref is set; `SIG`/`SIG_INFO` for P2P |
| Field Day | Yes | `CONTEST_ID=ARRL-FIELD-DAY`, `STX_STRING`, `SRX_STRING` |
| Winter Field Day | Yes | `CONTEST_ID=WFD`, `STX_STRING`, `SRX_STRING`, `FREQ` (MHz) |

The `Qso` struct carries `exchange_rcvd: Option<String>` (received contest exchange verbatim) and `frequency: Option<u32>` (kHz internally, converted to MHz on export). POTA-specific fields (`SIG`/`SIG_INFO`) are gated strictly on the POTA log type and are never emitted for other log types.

## POTA Submission

Upload the exported `.adif` file at https://pota.app under activator tools. One file per activation (one park, one UTC day).
