# ARRL Field Day Notes

References:
- https://www.arrl.org/field-day-rules
- https://adif.org/316/ADIF_316.htm (CONTEST_ID enumeration)

## What is Field Day?

ARRL Field Day is an annual operating event held the last full weekend of June (18:00 UTC Saturday through 20:59 UTC Sunday — 27 hours). It emphasizes emergency preparedness: stations set up portable, temporary operations, ideally on emergency power.

It is **not a traditional contest** — there is no trophy for high score, and logs are not required for submission. However, it uses contest-style logging (exchange per QSO) and produces a summary score.

## Operating Classes (sent as part of exchange)

| Class | Description |
|-------|-------------|
| A     | Club or non-club group, 3+ persons, portable (most common) |
| B     | 1–2 person portable |
| C     | Mobile (vehicle, maritime, aeronautical) |
| D     | Home station, commercial power |
| E     | Home station, emergency/alternative power only |
| F     | Emergency Operations Center (EOC) |

Classes A and F may also operate a GOTA (Get-On-The-Air) station with a separate callsign.

The class letter is preceded by the number of simultaneously transmitting stations (transmitter count), e.g., `3A` = three-transmitter Class A group.

Battery (QRP) variants: `A-Battery` and `B-Battery` use ≤5W non-commercial power.

## Exchange Format

Every QSO exchange consists of:

```
<transmitter count><class letter> <ARRL/RAC section>
```

**Example**: `3A CT` — three-transmitter Class A station in Connecticut section
**Example**: `1B EPA` — one-transmitter Class B station in Eastern Pennsylvania
**DX stations**: send class + `DX` instead of a section (e.g., `1A DX`)
**Phone**: spoken as "Three Alpha Connecticut"

### What to log per QSO

| Field | Description |
|-------|-------------|
| Their callsign | Station worked |
| Band | Operating band |
| Mode category | Phone / CW / Digital |
| Their exchange | Their class + section (e.g., `3A CT`) |
| Your exchange | Your class + section (e.g., `1B EPA`) |
| Timestamp | UTC |

## Scoring

| Mode | Points per QSO |
|------|---------------|
| Phone | 1 |
| CW | 2 |
| Digital | 2 |

Power multipliers applied to total QSO points:
- ≤5W + non-commercial power: ×5
- ≤5W with commercial power OR ≤100W any source: ×2
- >100W: ×1

Up to ~18 bonus point categories (100–500 pts each): emergency power, public location, media coverage, satellite QSO, youth participation, etc.

**Final score** = (QSO points × power multiplier) + bonus points

## ADIF Mapping

| ADIF Field | Value |
|------------|-------|
| `CONTEST_ID` | `ARRL-FIELD-DAY` |
| `STX_STRING` | Sent exchange, e.g., `1B EPA` |
| `SRX_STRING` | Received exchange, e.g., `3A CT` |
| `STATION_CALLSIGN` | Your callsign |
| `CALL` | Their callsign |
| `QSO_DATE` | YYYYMMDD |
| `TIME_ON` | HHMMSS |
| `BAND` | Band (e.g., `20M`) |
| `MODE` | Mode (e.g., `SSB`, `CW`, `FT8`) |

Note: `SRX_STRING` / `STX_STRING` are free-text string fields — no numeric serial number required. The full exchange string (e.g., `3A CT`) is stored verbatim.

## Log-Level Setup Fields

These are known when the log is created and apply to every QSO in the log:

| Field | Example | Notes |
|-------|---------|-------|
| Station callsign | `W1AW` | Required |
| Operator | `W1AW` | Optional if same as station |
| Transmitter count | `1` | Number of simultaneous transmitters (1–20) |
| Operating class | `B` | A/B/C/D/E/F |
| ARRL/RAC section | `EPA` | See section list below |
| Power category | `100W` | For multiplier: QRP / Low (≤100W) / High (>100W) |

Sent exchange = `{transmitter_count}{class} {section}`, e.g., `1B EPA`

## ARRL/RAC Section Abbreviations (partial)

### US Sections (ARRL)
`CT` `EMA` `ME` `NH` `RI` `VT` `WMA` `ENY` `NLI` `NNJ` `SNJ` `WNY` `DE` `EPA` `MDC` `WPA` `AL` `GA` `KY` `NC` `NFL` `SFL` `TN` `VA` `PR` `VI` `WCF` `AR` `LA` `MS` `NM` `NTX` `OK` `STX` `WTX` `EB` `LAX` `ORG` `SB` `SCV` `SDG` `SF` `SJV` `SV` `PAC` `AZ` `EWA` `ID` `MT` `NV` `OR` `UT` `WWA` `WY` `AK` `CO` `MN` `ND` `NE` `SD` `WI` `IA` `IL` `IN` `KS` `MI` `MO` `OH`

### Canadian Sections (RAC)
`AB` `BC` `GH` `MB` `NB` `NL` `NS` `ONE` `ONN` `ONS` `PE` `QC` `SK` `TER`

Full list: approximately 71 ARRL + 17 RAC sections. DX stations substitute `DX` for the section.

## Key Differences from POTA

| Aspect | POTA | Field Day |
|--------|------|-----------|
| Activation threshold | 10 QSOs | N/A (score-based) |
| Park reference | Required (optional in duklog) | N/A |
| Exchange per QSO | None | Class + section |
| ADIF SIG fields | `MY_SIG=POTA` | N/A |
| ADIF CONTEST_ID | N/A | `ARRL-FIELD-DAY` |
| Duration | Open (UTC day) | 27 hours (June) |
| Submission | pota.app | ARRL website |
