# ADR-0004: Hand-Written ADIF Reader Over a Serde ADIF Format

**Status:** Accepted
**Phase:** 5.5

## Context

ADIF is the single storage format. Reading logs back requires deserializing ADIF records into `Qso` structs and log metadata. The question was whether to implement this via a generic serde format or write explicit field-extraction code.

## Decision

ADIF reading uses an explicit hand-written async function (`adif::reader::read_log`) that extracts fields from `difa::Record` objects by name. Domain enums (`Mode`, `Band`, `FdClass`, `WfdClass`, `FdPowerCategory`) expose `adif_str()` / `from_adif_str()` methods as the explicit ADIF ↔ Rust conversion layer.

`Log` and `Qso` do **not** implement `Serialize`/`Deserialize` for ADIF.

## Rejected Alternative

Implement a `serde::Serializer` + `serde::Deserializer` backed by `difa`'s `TagEncoder`/`RecordStream`, making `Qso` and the log types `#[derive(Serialize, Deserialize)]` with ADIF as the wire format.

## Rationale

The `Qso` ↔ ADIF record mapping has structural mismatches that resist a generic serde format:

- `QSO_DATE` + `TIME_ON` (two ADIF fields) → `DateTime<Utc>` (one Rust field)
- `FREQ` is stored in MHz in ADIF, kHz internally — requires a unit conversion
- Optional fields (`SIG_INFO`, `SRX_STRING`) are absent from the record when `None`
- Each mismatch would require `#[serde(deserialize_with = "...")]` — producing noisier code than the hand-written version

The hand-written reader is ~200 lines and straightforward. The `difa` crate has no serde integration; a `serde_adif` crate would be a standalone project.

## When to Revisit

If a `serde_adif` crate becomes available in the ecosystem, or if the structural mismatches are resolved by schema changes.
