# Development Guide

## Prerequisites

- Rust (latest stable)
- `cargo-llvm-cov` for code coverage
- `cargo-mutants` for mutation testing

### Installing Dev Tools

```bash
rustup component add llvm-tools
cargo install cargo-llvm-cov
cargo install cargo-mutants
```

## Building

```bash
make build
```

## Testing

### Run Tests

```bash
make test
```

### Code Coverage

Generate an HTML coverage report (fails if line coverage < 90%):

```bash
make coverage
```

Open the report in a browser:

```bash
make coverage-report
```

### Mutation Testing

Run mutation testing across the entire codebase:

```bash
make mutants
```

Run for a specific module:

```bash
make mutants-module MOD=src/model/
```

Preview mutants without running:

```bash
make mutants-list
```

## CI Check

Run all checks (formatting, linting, tests, coverage) before committing:

```bash
make ci
```

## Code Review

Before creating a PR, run the code review agent:

```
/code-review
```

This performs an adversarial review against project standards, runs CI and mutation testing, and reports blockers, suggestions, and nits.

## Project Standards

See [CLAUDE.md](../CLAUDE.md) for the full coding standards reference, including:

- Coding style (functional approach, match over if-let, no unwrap in lib code)
- Testing requirements (specific assertions, boundary values, quickcheck)
- Documentation requirements (rustdoc on all pub items)
- Error handling patterns (thiserror per module)

## Git Workflow

1. Create a feature branch off `main`
2. Implement the feature with tests
3. Run `make ci` — all checks must pass
4. Run `make mutants` for changed modules — no surviving mutants
5. Run `/code-review` — fix all blockers
6. Create a PR to `main`
