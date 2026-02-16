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

## Claude Code Integration

### Skills

| Skill | Invocation | Purpose |
|---|---|---|
| `/commit` | Manual | Run CI, stage files, commit with trailer |
| `/create-pr` | Manual | Run code review, then create PR |
| `/learn-from-feedback` | Manual or auto | Process PR/user feedback into project knowledge |
| `coding-standards` | Auto (Claude) | Background knowledge for reviews and implementation |

### Subagents

| Agent | Model | Purpose |
|---|---|---|
| `code-review` | Sonnet | Adversarial pre-PR code review |
| `run-ci` | Haiku | Run CI/mutants, return concise pass/fail summary |

### Rules (conditional context)

| Rule | Loads when | Content |
|---|---|---|
| `testing.md` | Working on `src/**/*.rs` | Testing requirements, coverage exclusions |
| `domain.md` | Working on `src/model/`, `src/adif/`, `src/storage/` | Data model, ADIF format, POTA rules |

### Hooks

PostToolUse hooks run `cargo check` and `cargo clippy` after every `.rs` file edit.

## Project Standards

See [CLAUDE.md](../CLAUDE.md) for always-loaded standards (coding style, error handling, git workflow). Additional standards are loaded conditionally from `.claude/rules/` and `.claude/skills/` when relevant.

## Git Workflow

1. Create a feature branch off `main`
2. Implement the feature with tests
3. `/commit` — runs CI checks and commits
4. `/create-pr` — runs code review subagent, then creates PR
5. After PR feedback: `/learn-from-feedback` to update project knowledge
