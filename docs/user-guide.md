# User Guide

## Overview

duklog is an offline logging tool for POTA (Parks on the Air) activations. It runs entirely in your terminal with no internet connection required.

## Installation

<!-- TODO: Add installation instructions -->

## Quick Start

<!-- TODO: Add quick start workflow -->

## Screens

### Log Select
<!-- TODO: Describe log selection screen -->

### Log Create
<!-- TODO: Describe log creation form -->

### QSO Entry
<!-- TODO: Describe QSO entry workflow -->

### QSO List
<!-- TODO: Describe QSO list view -->

### Export
<!-- TODO: Describe ADIF export -->

### Help
<!-- TODO: Describe help screen -->

## Keybindings

<!-- TODO: Add keybinding reference table -->

## Data Storage

- **Log files**: `~/.local/share/duklog/logs/` (one JSON file per log)
- **ADIF exports**: Default path is `~/duklog-{PARK}-{YYYYMMDD}.adif`
- Logs are auto-saved after every change — no manual save needed

## POTA Activation Workflow

1. Create a new log with your callsign and park reference
2. Enter QSOs as you make contacts
3. The status bar shows your progress toward the 10-QSO activation threshold
4. When done, export your log as an ADIF file
5. Upload the ADIF file to pota.app when you have internet access
