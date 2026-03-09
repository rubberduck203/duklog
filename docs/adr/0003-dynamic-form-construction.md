# ADR-0003: Dynamic Form Construction for QSO Entry

**Status:** Accepted
**Phase:** 4.3

## Context

The QSO entry screen needs different field sets depending on the active log type: General (4 fields), POTA (5 fields), Field Day (4 fields), Winter Field Day (5 fields). The layout is a two-row horizontal grid, not a vertical list.

## Decision

`QsoEntryState` holds a `form_type: QsoFormType` field and rebuilds the `Form` from scratch (via `build_form_for_type`) whenever the log type changes. Field indices are numeric constants:

- `THEIR_CALL=0`, `RST_SENT=1`, `RST_RCVD=2` — shared across General/POTA
- Type-specific fields at index 3 (and 4 for WFD)
- Contest logs (`THEIR_CALL=0`, `CONTEST_THEIR_CLASS=1`, `CONTEST_THEIR_SECTION=2`)

A private `draw_qso_entry_form` (`#[mutants::skip]`) renders the form in a two-row layout using `draw_form_field` for per-cell `Rect` placement.

## Rejected Alternative

Separate layout paths in `draw_qso_entry` branching on log type, with a fixed `Form` field set and conditional rendering of fields.

## Rationale

- Mirrors the pattern in `log_create.rs` — both form-owning screens are consistent
- `Form` is the canonical state container (values, focus, errors); rebuilding it when the type changes keeps the field set self-consistent rather than conditionally hiding/showing fixed fields
- `draw_form_field` (added in this phase) places individual fields in arbitrary `Rect`s without re-implementing border/error rendering
- Type-specific rendering is isolated in `draw_qso_entry_form`, which is small and tested via `TestBackend` renders

## Tradeoffs Accepted

Rebuilding the form on type change clears any typed values. In practice, the form type is determined when the screen is entered (from the active log), not mid-session, so this is a non-issue.
