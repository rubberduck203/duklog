# ADIF Specification Notes

Reference: https://www.adif.org/316/ADIF_316.htm (ADIF v3.1.6)

## File Structure

An ADI file has two sections separated by `<eoh>` (end-of-header).
duklog uses the [`difa`](https://crates.io/crates/difa) crate for ADIF encoding,
which outputs lowercase markers (`<eoh>`, `<eor>`) per the case-insensitive spec:

```
<ADIF_VER:5>3.1.6
<PROGRAMID:6>duklog
<PROGRAMVERSION:5>0.1.0
<CREATED_TIMESTAMP:15>20260216 120000
<eoh>

<CALL:4>W1AW<QSO_DATE:8>20260216<TIME_ON:6>143000<BAND:3>20M<MODE:3>SSB<RST_SENT:2>59<RST_RCVD:2>59<eor>
<CALL:6>KD9XYZ<QSO_DATE:8>20260216<TIME_ON:6>144500<BAND:3>20M<MODE:2>CW<RST_SENT:3>599<RST_RCVD:3>599<eor>
```

## Field Syntax

```
<FIELDNAME:length>value
```

- The integer after the colon is the **byte length** of the value
- Field names are case-insensitive
- Whitespace between fields is ignored
- Records end with `<EOR>` (case-insensitive; difa writes `<eor>`)
- Header ends with `<EOH>` (case-insensitive; difa writes `<eoh>`)

## Header Fields

| Field | Example | Notes |
|---|---|---|
| `ADIF_VER` | `3.1.6` | Spec version |
| `PROGRAMID` | `duklog` | Application name |
| `PROGRAMVERSION` | `0.1.0` | Application version |
| `CREATED_TIMESTAMP` | `20260216 120000` | YYYYMMDD HHMMSS UTC |

## QSO Fields Used by duklog

### Core QSO fields (all log types)

| Field | Format | Example | Notes |
|---|---|---|---|
| `STATION_CALLSIGN` | string | `W1AW` | Callsign used on air |
| `CALL` | string | `KD9XYZ` | Other station's callsign |
| `QSO_DATE` | YYYYMMDD | `20260216` | UTC date |
| `TIME_ON` | HHMMSS | `143000` | UTC start time |
| `BAND` | string | `20M` | See band values below |
| `FREQ` | number (kHz) | `14225` | Frequency in kHz; emitted when captured; complements `BAND` |
| `MODE` | string | `SSB` | See mode values below |
| `RST_SENT` | string | `59` | Signal report sent |
| `RST_RCVD` | string | `59` | Signal report received |
| `OPERATOR` | string | `W1AW` | Individual operator callsign (if different from station) |
| `MY_GRIDSQUARE` | string | `FN31` | Maidenhead grid square |
| `COMMENT` | string | `nice op` | Free-text comment; emitted only if non-empty |

### POTA fields

| Field | Format | Example | Notes |
|---|---|---|---|
| `MY_SIG` | string | `POTA` | Always `POTA`; emitted only for POTA logs with a park ref |
| `MY_SIG_INFO` | string | `K-0001` | POTA park reference |
| `SIG` | string | `POTA` | Set when other station is also at a park (P2P) |
| `SIG_INFO` | string | `K-1234` | Other station's park reference (P2P) |

### Contest fields (Field Day and WFD)

| Field | Format | Example | Notes |
|---|---|---|---|
| `CONTEST_ID` | string | `ARRL-FIELD-DAY` | Contest identifier; see values below |
| `STX_STRING` | string | `1B EPA` | Sent exchange (your class + section); constant per log |
| `SRX_STRING` | string | `3A CT` | Received exchange (their class + section); per QSO |

**`CONTEST_ID` values used by duklog**:

| Log Type | `CONTEST_ID` |
|---|---|
| ARRL Field Day | `ARRL-FIELD-DAY` |
| Winter Field Day | `WFD` |

Note: `SPAR-WINTER-FD` is the deprecated identifier for WFD 2016 and earlier; duklog uses `WFD` for all WFD logs (2017+).

## ADIF Band Values

These are the string values used in the `BAND` field:

| Band | ADIF Value |
|---|---|
| 160m | `160M` |
| 80m | `80M` |
| 60m | `60M` |
| 40m | `40M` |
| 30m | `30M` |
| 20m | `20M` |
| 17m | `17M` |
| 15m | `15M` |
| 12m | `12M` |
| 10m | `10M` |
| 6m | `6M` |
| 2m | `2M` |
| 70cm | `70CM` |

## ADIF Mode Values

Common modes:

| Mode | ADIF Value | RST Format |
|---|---|---|
| SSB | `SSB` | 2-digit (e.g. `59`) |
| CW | `CW` | 3-digit (e.g. `599`) |
| FT8 | `FT8` | dB (e.g. `-10`) |
| FT4 | `FT4` | dB (e.g. `-10`) |
| JS8 | `JS8` | dB (e.g. `-10`) |
| FM | `FM` | 2-digit (e.g. `59`) |
| AM | `AM` | 2-digit (e.g. `59`) |
| PSK31 | `PSK31` | 3-digit (e.g. `599`) |
| RTTY | `RTTY` | 3-digit (e.g. `599`) |

Note: FT8/FT4/JS8 use `SUBMODE` in ADIF. For POTA, if both `MODE` and `SUBMODE` are present, POTA uses `SUBMODE`. For simplicity, duklog writes these as `MODE` since POTA accepts it.
