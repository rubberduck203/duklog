---
name: check-coverage
description: Run code coverage analysis, returning either a summary or detailed per-file coverage. Use to check coverage without polluting the main conversation with verbose output.
tools: Bash, Read, Grep, Glob
model: haiku
---

Run code coverage and return results based on the requested mode.

## Commands

Use `cargo llvm-cov` directly with `--ignore-filename-regex 'main\.rs'` to exclude main.rs from coverage metrics.

### Summary mode (default)

```
cargo llvm-cov --ignore-filename-regex 'main\.rs' --summary-only
```

Returns the summary table showing per-file line/region/function coverage percentages plus the total. Use this mode when the caller needs to identify which files have low coverage.

### Detail mode

```
cargo llvm-cov --text --ignore-filename-regex 'main\.rs'
```

Returns annotated source coverage to stdout. The output contains sections per source file separated by file path headers.

If the caller specifies particular files or modules to inspect, pipe through `grep` or scan the output to extract and return **only** the relevant file sections — do not return the entire output.

## Output format

Return ONLY:
1. **Status**: Coverage threshold PASS (≥90% lines) or FAIL
2. **Coverage data**: The requested summary table or file details
3. **If specific files requested**: Only the coverage sections for those files, not the entire output

Do NOT include build output, compiler warnings, or other noise. Be terse.
