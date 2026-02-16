# ADIF Specification Notes

Reference: https://www.adif.org/316/ADIF_316.htm (ADIF v3.1.6)

## File Structure

An ADI file has two sections separated by `<EOH>` (end-of-header):

```
<ADIF_VER:5>3.1.6
<PROGRAMID:6>duklog
<PROGRAMVERSION:5>0.1.0
<CREATED_TIMESTAMP:15>20260216 120000
<EOH>

<CALL:4>W1AW <QSO_DATE:8>20260216 <TIME_ON:6>143000 <BAND:3>20M <MODE:3>SSB <RST_SENT:2>59 <RST_RCVD:2>59 <EOR>
<CALL:6>KD9XYZ <QSO_DATE:8>20260216 <TIME_ON:6>144500 <BAND:3>20M <MODE:2>CW <RST_SENT:3>599 <RST_RCVD:3>599 <EOR>
```

## Field Syntax

```
<FIELDNAME:length>value
```

- The integer after the colon is the **byte length** of the value
- Field names are case-insensitive
- Whitespace between fields is ignored
- Records end with `<EOR>`
- Header ends with `<EOH>`

## Header Fields

| Field | Example | Notes |
|---|---|---|
| `ADIF_VER` | `3.1.6` | Spec version |
| `PROGRAMID` | `duklog` | Application name |
| `PROGRAMVERSION` | `0.1.0` | Application version |
| `CREATED_TIMESTAMP` | `20260216 120000` | YYYYMMDD HHMMSS UTC |

## QSO Fields Used by duklog

### Required (POTA submission)

| Field | Format | Example | Notes |
|---|---|---|---|
| `STATION_CALLSIGN` | string | `W1AW` | Callsign used on air |
| `CALL` | string | `KD9XYZ` | Other station's callsign |
| `QSO_DATE` | YYYYMMDD | `20260216` | UTC date |
| `TIME_ON` | HHMMSS | `143000` | UTC start time |
| `BAND` | string | `20M` | See band values below |
| `MODE` | string | `SSB` | See mode values below |

### Recommended (POTA)

| Field | Format | Example | Notes |
|---|---|---|---|
| `OPERATOR` | string | `W1AW` | Individual operator callsign (if different from station) |
| `MY_SIG` | string | `POTA` | Always "POTA" for our use |
| `MY_SIG_INFO` | string | `K-0001` | POTA park reference |
| `SIG` | string | `POTA` | Set when other station is also in a park (P2P) |
| `SIG_INFO` | string | `K-1234` | Other station's park reference (P2P) |
| `RST_SENT` | string | `59` | Signal report sent |
| `RST_RCVD` | string | `59` | Signal report received |

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

Common modes for POTA:

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
