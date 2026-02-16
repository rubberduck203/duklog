# POTA Rules & Logging Requirements

References:
- https://docs.pota.app/docs/rules.html
- https://docs.pota.app/docs/activator_reference/submitting_logs.html
- https://docs.pota.app/docs/activator_reference/ADIF_for_POTA_reference.html

## Activation Requirements

- **Minimum 10 QSOs** from a designated park within a single UTC day
- Partial logs (failed activations with < 10 QSOs) should still be submitted so hunters get credit
- No time limit on log submission, but upload promptly for hunters

## Logging Rules

- No fully automated QSOs — both operators must participate directly
- No land repeaters allowed
- Satellites are permitted (log activator's TX band)
- No relayed contacts — both stations must directly copy each other
- Any band/mode combination counts, including WARC bands (30m, 17m, 12m)
- Each unique callsign+band+mode combination counts as one QSO

## Required ADIF Fields for POTA Submission

| Field | Required | Notes |
|---|---|---|
| `STATION_CALLSIGN` or `OPERATOR` | Yes | Call used on air; club activations need both |
| `CALL` | Yes | The hunter's callsign |
| `QSO_DATE` | Yes | UTC, YYYYMMDD format |
| `TIME_ON` | Yes | UTC, HHMM or HHMMSS. POTA tallies by start time |
| `BAND` | Yes | e.g. `20M`; use activator's TX band for satellite |
| `MODE` or `SUBMODE` | Yes | If both present, SUBMODE takes precedence |

## Recommended Fields

| Field | Value | Notes |
|---|---|---|
| `MY_SIG` | `POTA` | Identifies this as a POTA log |
| `MY_SIG_INFO` | e.g. `K-0001` | Your park reference — how POTA knows which park |
| `MY_STATE` | e.g. `OH` | Subdivision code for location clarity |
| `SIG` | `POTA` | Set when other station is also POTA (park-to-park) |
| `SIG_INFO` | e.g. `K-1234` | Other station's park reference (park-to-park) |

## Park Reference Format

- Format: `{ENTITY}-{NUMBER}` (e.g. `K-0001`, `VE-0001`, `JA-0001`)
- Entity prefix is 1-3 uppercase letters
- Number is 4-5 digits, zero-padded
- Regex: `[A-Z]{1,3}-\d{4,5}`
- Full list at https://pota.app (but we're offline, so validate format only)

## Park-to-Park (P2P) Contacts

When both stations are activating a POTA park:
- Set `SIG` to `POTA`
- Set `SIG_INFO` to the other station's park reference
- This counts for both activators
- The POTA system automatically matches P2P contacts

## Upload Format

- POTA accepts standard `.adi` (ADIF) files
- Upload at https://pota.app under activator tools
- One file per activation (one park, one UTC day)
- Multiple parks in a day = multiple files
