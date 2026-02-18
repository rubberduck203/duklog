# User Guide

## Overview

duklog is an offline logging tool for ham radio activations. It runs entirely in your terminal with no internet connection required — perfect for field use.

## Installation

With Rust installed:

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
# Binary at target/release/duklog
```

## Quick Start

1. Launch duklog from your terminal: `duklog`
2. Press `n` to create a new log
3. Enter your station callsign, optionally your operator callsign and park reference, and your grid square
4. Press `Enter` to create the log
5. Enter your first contact's callsign, adjust RST if needed, and press `Enter` to log the QSO
6. Use `Alt+b` / `Alt+m` to change band and mode as needed
7. When done, press `Alt+x` to export your log as an ADIF file

## Screens

### Log Select

The home screen. Shows all saved logs in a table with columns for Park, Callsign, Date, and QSO count.

| Key | Action |
|---|---|
| `Up` / `Down` | Navigate the log list |
| `Enter` | Open the selected log |
| `n` | Create a new log |
| `q` / `Esc` | Quit duklog |
| `?` | Show help |

### Log Create

A form for creating a new log.

**Fields:**

- **Station Callsign** (required) — your operating callsign
- **Operator** (optional) — only needed if different from the station callsign
- **Park Ref** (optional) — POTA park reference (e.g. `K-0001`)
- **Grid Square** (required) — Maidenhead locator (e.g. `FN31` or `FN31pr`)

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Move between fields |
| `Enter` | Validate and create the log |
| `Esc` | Cancel and return to Log Select |

Validation errors are shown inline when you submit.

### QSO Entry

The main logging screen. The header shows your station info, current band/mode, and POTA activation progress. The most recent QSOs are displayed below the form.

**Fields:**

- **Their Callsign** (required) — auto-uppercased as you type
- **RST Sent** (required) — pre-filled with the mode default
- **RST Rcvd** (required) — pre-filled with the mode default
- **Their Park** (optional) — for park-to-park contacts, auto-uppercased
- **Comments** (optional)

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

**Bands** (default 20M): 160M, 80M, 60M, 40M, 30M, 20M, 17M, 15M, 12M, 10M, 6M, 2M, 70CM

**Modes** (default SSB): SSB, CW, FT8, FT4, JS8, PSK31, RTTY, FM, AM, Digi

When you change modes, the RST fields auto-update to the new mode's default — unless you've manually edited them.

### QSO List

A scrollable table of all QSOs in the current log. Columns: Time, Date, Call, Band, Mode, RST S/R, Park, Comments.

| Key | Action |
|---|---|
| `Up` / `Down` | Navigate rows |
| `Home` / `End` | Jump to first / last row |
| `Enter` | Edit the selected QSO |
| `q` / `Esc` | Back to QSO Entry |
| `?` | Show help |

Pressing `Enter` opens the selected QSO in the entry form for editing. Save with `Enter` or cancel with `Esc`.

### Export

Shows the export destination, QSO count, and station info. Press `Enter` to write the ADIF file.

| Key | Action |
|---|---|
| `Enter` | Export the ADIF file |
| `Esc` / `q` | Back to QSO Entry |
| `?` | Show help |

After export (success or error), press any key to return.

The default export path is `~/duklog-{PARK}-{YYYYMMDD}.adif` (or `~/duklog-{CALLSIGN}-{YYYYMMDD}.adif` if no park reference is set).

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
