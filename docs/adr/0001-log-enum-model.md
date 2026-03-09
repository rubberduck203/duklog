# ADR-0001: Log Enum Over LogConfig-on-Struct

**Status:** Accepted
**Phase:** 4.0

## Context

The domain model needed to support multiple log types (General, POTA, Field Day, Winter Field Day), each with shared fields (callsign, operator, grid square, timestamps, QSOs) and type-specific fields (park ref for POTA; tx_count/class/section/power for Field Day; etc.).

## Decision

`Log` is an enum where each variant wraps a concrete struct that embeds a shared `LogHeader`:

```
LogHeader    — station_callsign, operator, grid_square, qsos, created_at, log_id
GeneralLog   — header: LogHeader
PotaLog      — header: LogHeader, park_ref: String
FieldDayLog  — header: LogHeader, tx_count: u8, class: FdClass, section: String, power: FdPowerCategory
WfdLog       — header: LogHeader, tx_count: u8, class: WfdClass, section: String
Log enum     — General(GeneralLog) | Pota(PotaLog) | FieldDay(FieldDayLog) | WinterFieldDay(WfdLog)
```

Shared fields are accessed via `log.header()` / `log.header_mut()`. Type-specific fields via pattern match: `if let Log::Pota(p) = log`.

`Log` does **not** derive `Serialize`/`Deserialize`. Storage uses ADIF with `APP_DUKLOG_LOG_TYPE` as the discriminant. See ADR-0004 for storage format.

## Rejected Alternative

A single `Log` struct with a `LogConfig` enum field carrying type-specific data.

## Rationale

- Type-specific fields are direct struct fields — no inner pattern match into a config enum
- Type-specific methods live on the concrete type (`PotaLog::is_activated()`); `GeneralLog` simply has no such method
- The compiler enforces exhaustiveness at every dispatch point; adding a log type surfaces all required updates at compile time
- Each concrete type is unit-testable without constructing the `Log` wrapper

## Tradeoffs Accepted

`Log` methods delegating to `LogHeader` (`header()`, `header_mut()`, `add_qso()`) require one match arm per variant. Boilerplate grows linearly with log types.

## Design Note: POTA park_ref

`PotaLog.park_ref` is `String`, **not** `Option<String>`. POTA always has a park; "POTA without park" is not a valid use case. Any code path creating a POTA log without a park ref is a bug.
