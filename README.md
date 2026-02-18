# duklog

duklog is a simple _offline_ logging tool for ham radio.

I was unhappy with existing solutions for logging in the field on linux.
Existing loggers have too many features and too much reliance on the internet.
I just wanted the ability to create a log, enter a few pieces of info, and start logging QSOs in a simple, lightweight, offline environment.

duklog provides this via a lightweight TUI that runs in the terminal and allows exporting an ADIF file for uploading to my main logs, the POTA site, etc.

## Features

- Create and manage multiple logs with station callsign, operator, park reference, and grid square
- Log QSOs with callsign, band, mode, RST, park-to-park contacts, and comments
- Edit previously logged QSOs from a scrollable list
- Track POTA activation progress (10 QSO threshold)
- Export logs as ADIF v3.1.6 files for upload to POTA and other services
- Auto-save after every change — no data loss
- Fully offline — no network access, ever

## Contributing

duklog is written in Rust. Use `make` targets for development:

```bash
make ci        # fmt + lint + test + coverage (run before every commit)
make test      # run test suite
make lint      # clippy with -D warnings
make coverage  # HTML coverage report, fails if < 90% line coverage
make mutants   # run mutation testing
```

See [docs/development.md](docs/development.md) for the full development guide.
