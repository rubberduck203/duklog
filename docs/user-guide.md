# User Guide

## Overview

duklog is an offline logging tool for ham radio activations. It runs entirely in your terminal with no internet connection required — perfect for field use.

## Installation

Download the latest release binary from the [releases page](https://github.com/rubberduck203/duklog/releases/latest) and place it somewhere on your `PATH`.

Alternatively, if you have Rust installed, you can build from source:

```bash
cargo build --release
# Binary at target/release/duklog
```

## Quick Start

1. Launch duklog from your terminal: `duklog`
2. Press `n` to create a new log
3. Use `←`/`→` to select a log type (General, POTA, Field Day, Winter FD), then `Tab` to move to the fields; fill in your callsign and grid square (plus any type-specific fields)
4. Press `Enter` to create the log
5. Enter your first contact's callsign, adjust RST if needed, and press `Enter` to log the QSO
6. Use `Alt+b` / `Alt+m` to change band and mode as needed
7. When done, press `Alt+x` to export your log as an ADIF file

## Screens

### Log Select

The home screen. Shows all saved logs in a table with columns for Callsign, Date, Park, Grid, and QSO count.

| Key | Action |
|---|---|
| `Up` / `Down` | Navigate the log list |
| `Enter` | Open the selected log |
| `n` | Create a new log |
| `d` | Delete the selected log (asks for confirmation; `y` to confirm, `n`/`Esc` to cancel) |
| `q` / `Esc` | Quit duklog |
| `F1` | Show help |

### Log Create

A form for creating a new log. Start by selecting the log type with `←`/`→`, then press `Tab` to move to the fields.

**Log Types:**

| Type | Use for |
|---|---|
| General | General-purpose logging with no contest or activation context |
| POTA | Parks on the Air activations; optional park reference field |
| Field Day | ARRL Field Day contest |
| Winter FD | Winter Field Day contest |

**Common Fields (all types):**

- **Station Callsign** (required) — your operating callsign
- **Operator** (optional) — only needed if different from the station callsign

**General / POTA Fields:**

- **Grid Square** (required) — Maidenhead locator (e.g. `FN31` or `FN31pr`); any case accepted, normalised to canonical form on submit

**POTA-only Fields:**

- **Park Ref** (optional) — POTA park reference (e.g. `K-0001`), auto-uppercased as you type

**Field Day / Winter FD Fields:**

- **Tx Count** (required) — number of transmitters (1–255)
- **FD Class** / **WFD Class** (required) — operating class; auto-uppercased (FD: A–F; WFD: H/I/O/M)
- **Section** (required) — ARRL/RAC section (e.g. `EPA`, `DX`); auto-uppercased
- Grid Square is not collected for FD/WFD logs (not required by either contest)

| Key | Action |
|---|---|
| `←` / `→` | Switch log type (when Type Selector is focused) |
| `Tab` / `Shift+Tab` | Move between type selector and form fields |
| `Enter` | Validate and create the log |
| `Esc` | Cancel and return to Log Select |
| `F1` | Show help |

Validation errors are shown inline when you submit. If a log already exists with the same station callsign and type on the same UTC day, creation is blocked with an inline error.

### QSO Entry

The main logging screen. A status bar at the top shows the active log context: park reference (if set), callsign, today's QSO count, and — once you reach 10 QSOs — `ACTIVATED` in green. The header below shows your station info, current band/mode, and detailed activation progress. The most recent QSOs are displayed below the form.

The form uses a two-row layout that adapts to the active log type:

**Row 1:**

| Field | General / POTA | Field Day / Winter FD |
|---|---|---|
| Col 1 | Their Callsign (required) | Their Callsign (required) |
| Col 2 | RST Sent (required) | Their Class (required) |
| Col 3 | RST Rcvd (required) | Their Section (required) |

FD and WFD do not exchange RST. Row 1 Col 2/3 capture the contest class (e.g. `3A`) and section (e.g. `CT`) instead.

**Row 2 (type-specific):**

| Log Type | Left | Right |
|---|---|---|
| General | *(empty)* | Comments |
| POTA | Their Park (optional) | Comments |
| Field Day | Frequency (required, kHz) | Comments |
| Winter FD | Frequency (required, kHz) | Comments |

**Field notes:**

- **Their Park** — POTA park reference (e.g. `K-0001`) for park-to-park contacts; auto-uppercased; optional
- **Their Class** — received contest class including transmitter count; auto-uppercased; required for FD/WFD
  - Field Day: `<count><class>` — e.g. `3A`, `1F` (class: A–F)
  - Winter Field Day: `<count><class>` — e.g. `2H`, `1O` (class: H/I/O/M)
- **Their Section** — received ARRL/RAC section (e.g. `CT`, `EPA`, `DX`); auto-uppercased; required for FD/WFD
- **Frequency** — operating frequency in kHz (e.g. `14225`); required for WFD ADIF export
- **Comments** — free-text; optional

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Move between fields |
| `Enter` | Log the QSO; in edit mode: save changes |
| `Esc` | Back to Log Select; in edit mode: cancel and return to QSO List |
| `Alt+b` | Next band |
| `Shift+Alt+B` | Previous band |
| `Alt+m` | Next mode |
| `Shift+Alt+M` | Previous mode |
| `Alt+e` | View QSO list |
| `Alt+x` | Export log |
| `F1` | Show help |

**Bands** (default 20M): 160M, 80M, 60M, 40M, 30M, 20M, 17M, 15M, 12M, 10M, 6M, 2M, 70CM

**Modes** (default SSB): SSB, CW, FT8, FT4, JS8, PSK31, RTTY, FM, AM, Digi

When you change modes, the RST fields auto-update to the new mode's default — unless you've manually edited them.

If you log a contact with the same callsign, band, and mode as an existing QSO in the current log, a duplicate warning is displayed. The QSO is still saved — the operator may intentionally work the same station on the same band/mode.

### QSO List

A scrollable table of all QSOs in the current log. Columns: Time, Date, Call, Band, Mode, RST S/R, Park, Comments. The status bar at the top shows the active log context (same format as QSO Entry).

| Key | Action |
|---|---|
| `Up` / `Down` | Navigate rows |
| `Home` / `End` | Jump to first / last row |
| `Enter` | Edit the selected QSO |
| `q` / `Esc` | Back to QSO Entry |
| `F1` | Show help |

Pressing `Enter` opens the selected QSO in the entry form for editing. Save with `Enter` or cancel with `Esc`.

### Export

Shows the export destination, QSO count, and station info. The status bar at the top shows the active log context. Press `Enter` to write the ADIF file.

| Key | Action |
|---|---|
| `Enter` | Export the ADIF file |
| `Esc` / `q` | Back to QSO Entry |
| `F1` | Show help |

After export (success or error), press any key to return.

The default export path is `~/duklog-{PARK}-{YYYYMMDD}.adif` (or `~/duklog-{CALLSIGN}-{YYYYMMDD}.adif` if no park reference is set).

### Help

Press `F1` from any screen to open context-sensitive help. The title shows which screen you are on, and only that screen's keybindings are shown. Pressing `q` or `Esc` returns you to the screen you came from.

| Key | Action |
|---|---|
| `Up` / `Down` | Scroll |
| `q` / `Esc` | Return to previous screen |

## Data Storage

- **Log files**: `~/.local/share/duklog/logs/` (one JSONL file per log)
- **ADIF exports**: Default path is `~/duklog-{PARK}-{YYYYMMDD}.adif`
- Logs are auto-saved after every change — no manual save needed

## POTA Activation Workflow

1. Create a new log with your callsign and park reference
2. Enter QSOs as you make contacts
3. The status bar shows your progress toward the 10-QSO activation threshold
4. When done, export your log as an ADIF file
5. Upload the ADIF file to pota.app when you have internet access
