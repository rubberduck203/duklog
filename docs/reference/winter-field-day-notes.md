# Winter Field Day Notes

References:
- https://winterfieldday.org/sop.php
- https://winterfieldday.org/downloads/2026-rules-v3.pdf
- https://adif.org/316/ADIF_316.htm (CONTEST_ID enumeration)

## What is Winter Field Day?

Winter Field Day (WFD) is an annual operating event held the last full weekend of January (18:00 UTC Saturday through 21:00 UTC Sunday — 27 hours), organized by the Winter Field Day Association (WFDA). It mirrors ARRL Field Day's emergency preparedness spirit but in winter conditions.

Unlike ARRL Field Day, WFD **requires log submission** (Cabrillo or ADIF format, by March 1st UTC) for an official score.

## Operating Classes (sent as part of exchange)

| Class | Description |
|-------|-------------|
| H     | Home — inside a permanent livable residence |
| I     | Indoor — weather-protected building on permanent foundation (clubhouse, cabin) |
| O     | Outdoor — partly or fully exposed building/shelter without typical utilities |
| M     | Mobile — RV, car, van, boat, cargo trailer, or similar mobile structure |

## Exchange Format

Every QSO exchange consists of:

```
<transmitter count><class> <ARRL/RAC section>
```

The transmitter count is the number of transceivers capable of transmitting simultaneously.

**Example**: `2M EPA` — two-transmitter Mobile station in Eastern Pennsylvania
**Example**: `1H GA` — one-transmitter Home station in Georgia

### What to log per QSO

| Field | Description |
|-------|-------------|
| Their callsign | Station worked |
| Frequency | kHz for HF (rounded to nearest kHz); band designation for VHF+ (e.g., `144`) |
| Mode | CW / PH / FM / RY / DG |
| Their exchange | Their class + section (e.g., `2M EPA`) |
| Your exchange | Your class + section (e.g., `1H GA`) |
| Timestamp | UTC |

**Contact limit**: Each pair of stations may work each other a maximum of 3 times per band (once per mode: CW, phone, digital).

Repeater contacts are explicitly **prohibited** — only direct amateur RF counts.

## Scoring

| Mode | Points per QSO |
|------|---------------|
| Phone | 1 |
| CW | 2 |
| Digital | 2 |

Objectives (bonus multipliers): completing a list of optional operating objectives increases the final multiplier.

**Final score** = (Total QSO Points) × (Objectives multiplier + 1)

## ADIF Mapping

| ADIF Field | Value |
|------------|-------|
| `CONTEST_ID` | `WFD` |
| `STX_STRING` | Sent exchange, e.g., `1H EPA` |
| `SRX_STRING` | Received exchange, e.g., `2M CT` |
| `STATION_CALLSIGN` | Your callsign |
| `CALL` | Their callsign |
| `QSO_DATE` | YYYYMMDD |
| `TIME_ON` | HHMMSS |
| `FREQ` | Frequency in kHz (HF), band designation (VHF+) |
| `BAND` | Band (e.g., `20M`) |
| `MODE` | Mode (e.g., `SSB`, `CW`, `FT8`) |

Note: `SRX_STRING` / `STX_STRING` store the full exchange string verbatim (e.g., `1H EPA`).

## Log Submission

- Format: Cabrillo4 or ADIF
- Deadline: 23:59 UTC March 1st (approximately 5 weeks after the event)
- Submission: via the WFDA website
- Callsign in the filename must match the callsign used in the event

## Log-Level Setup Fields

These are known when the log is created and apply to every QSO in the log:

| Field | Example | Notes |
|-------|---------|-------|
| Station callsign | `W1AW` | Required |
| Operator | `W1AW` | Optional if same as station |
| Transmitter count | `1` | Number of simultaneous transmitters |
| Operating class | `H` | H / I / O / M |
| ARRL/RAC section | `EPA` | Same section list as Field Day |

Sent exchange = `{transmitter_count}{class} {section}`, e.g., `1H EPA`

## ARRL/RAC Sections

Same section abbreviations as ARRL Field Day — see `arrl-field-day-notes.md` for the full list.

## Key Differences from ARRL Field Day

| Aspect | ARRL Field Day | Winter Field Day |
|--------|---------------|-----------------|
| Time of year | Last full weekend June | Last full weekend January |
| Organizer | ARRL | WFDA (independent) |
| Log submission | Optional (recommended) | **Required** for official score |
| Operating classes | A/B/C/D/E/F | H/I/O/M |
| Contact limit | None stated | 3 per band (1 per mode) |
| Scoring multiplier | Power category (QRP/Low/High) | Objectives bonus |
| ADIF CONTEST_ID | `ARRL-FIELD-DAY` | `WFD` |

## Key Differences from POTA

| Aspect | POTA | WFD |
|--------|------|-----|
| Activation threshold | 10 QSOs | N/A (score-based) |
| Park reference | Required (optional in duklog) | N/A |
| Exchange per QSO | None | Class + section |
| ADIF SIG fields | `MY_SIG=POTA` | N/A |
| ADIF CONTEST_ID | N/A | `WFD` |
| Duration | Open (UTC day) | 27 hours (January) |
| Submission | pota.app | WFDA website |
