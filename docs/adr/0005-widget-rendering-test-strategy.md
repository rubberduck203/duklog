# ADR-0005: Widget Rendering Test Strategy

**Status:** Under Review — revisit with owner before Phase 5.6 ships
**Phase:** 5.6

> **⚠️ Decision Pending:** Two approaches are in use experimentally as of Phase 5.6.
> Before merging Phase 5.6, the owner and Claude should review what was learned and
> settle on a single preferred pattern (or a principled rule for when to use each).
> See the "Revisit" section at the bottom.

## Context

duklog TUI screens and widgets are tested by rendering to a `TestBackend` and inspecting
the output. Prior to Phase 5.6 the only pattern in use was:

```rust
fn buffer_to_string(buf: &Buffer) -> String { /* collect symbols row by row */ }
let output = render_qso_entry(&state, Some(&log), 80, 30);
assert!(output.contains("K-5678"));
```

This tests **presence** of text but not **position**. Three bugs shipped in Phase 5.3
(`draw_recent_qsos`) that presence-only tests could not catch:

- **#37** — POTA with no Their Park showed frequency in the park column. A test existed
  for POTA + park present, but not for POTA + park absent. Even if it had existed, a
  `.contains()` assertion could not have distinguished "frequency appears in the park
  column" from "frequency appears correctly elsewhere."
- **#39** — Frequency visibility in the panel (would have been caught by a missing test
  case, but not by test style per se).
- **#40** — Row count hard-coded to 3 even on tall terminals. No test varied terminal
  height or counted rendered rows.

Ratatui's own documentation recommends snapshot testing via the `insta` crate:

```rust
// insta approach
assert_snapshot!(terminal.backend()); // captures full rendered layout to a .snap file
```

The `.snap` file is human-readable text — a literal picture of the terminal — making
layout bugs immediately visible when reviewing snapshot diffs.

## Decision

Phase 5.6 **experiments with both approaches** side by side:

1. **`insta` snapshots** for `draw_recent_qsos` — one snapshot per log type + key edge
   case (e.g., POTA with park, POTA without park, varying terminal heights). The snapshot
   files make column layout directly visible and will catch position-level regressions.

2. **Targeted presence tests** (`.contains()`) are retained where they are the clearest
   way to assert semantic intent (e.g., "POTA log does NOT render 'Their Park' header
   when park is absent").

Both are used together to learn which feels more natural to write, read, and maintain
over time.

## Consequences

- `insta` is added as a dev-dependency (`cargo add insta --dev`).
- Snapshot files live in `src/tui/screens/snapshots/` (adjacent to the source file being tested) and are committed.
- `insta.yaml` at the workspace root sets `update: unseen` — new snapshots auto-accept; changed snapshots require `INSTA_UPDATE=always cargo test`.
- The `buffer_to_string` helper has been consolidated into `src/tui/test_utils.rs`; all screens now import it from there.

## Revisit

Before or during Phase 5.7, owner and Claude should discuss:

1. Did snapshot tests catch things `.contains()` would have missed?
2. Was `cargo insta review` friction acceptable during active development?
3. Should snapshot tests replace, supplement, or be reserved for stabilised components?
4. Is there a cleaner rule: e.g., "snapshots for layout components, `.contains()` for
   semantic/logic tests"?

Update this ADR's **Status** to `Accepted` once a decision is made, and update
`docs/reference/ratatui-notes.md` with the settled guidance.
