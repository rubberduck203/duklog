# ADIF Band Frequency Ranges

Source: ADIF v3.1.4 band enumeration
URL: https://adif.org/314/ADIF_314.htm#Band_Enumeration

## Bands supported by duklog

The `Band` enum covers the 13 amateur bands most common in HF/VHF portable operation.
Frequency ranges are in MHz from the ADIF spec; kHz equivalents are used internally
(stored as `u32`, passed to `Band::from_frequency_khz`).

| Band | ADIF value | Lower (MHz) | Upper (MHz) | Lower (kHz) | Upper (kHz) |
|------|-----------|-------------|-------------|-------------|-------------|
| 160m | `160M`  | 1.8         | 2.0         | 1800        | 2000        |
| 80m  | `80M`   | 3.5         | 4.0         | 3500        | 4000        |
| 60m  | `60M`   | 5.06        | 5.45        | 5060        | 5450        |
| 40m  | `40M`   | 7.0         | 7.3         | 7000        | 7300        |
| 30m  | `30M`   | 10.1        | 10.15       | 10100       | 10150       |
| 20m  | `20M`   | 14.0        | 14.35       | 14000       | 14350       |
| 17m  | `17M`   | 18.068      | 18.168      | 18068       | 18168       |
| 15m  | `15M`   | 21.0        | 21.45       | 21000       | 21450       |
| 12m  | `12M`   | 24.890      | 24.99       | 24890       | 24990       |
| 10m  | `10M`   | 28.0        | 29.7        | 28000       | 29700       |
| 6m   | `6M`    | 50.0        | 54.0        | 50000       | 54000       |
| 2m   | `2M`    | 144.0       | 148.0       | 144000      | 148000      |
| 70cm | `70CM`  | 420.0       | 450.0       | 420000      | 450000      |

## ADIF-defined bands not in duklog's `Band` enum

These exist in the ADIF spec but are omitted because they are not used in POTA,
Field Day, or Winter Field Day contexts:

| Band    | Lower (MHz) | Upper (MHz) |
|---------|-------------|-------------|
| 2190m   | 0.1357      | 0.1378      |
| 630m    | 0.472       | 0.479       |
| 560m    | 0.501       | 0.504       |
| 8m      | 40          | 45          |
| 5m      | 54.000001   | 69.9        |
| 4m      | 70          | 71          |
| 1.25m   | 222         | 225         |
| 33cm    | 902         | 928         |
| 23cm    | 1240        | 1300        |
| 13cm    | 2300        | 2450        |
| 9cm     | 3300        | 3500        |
| 6cm     | 5650        | 5925        |
| 3cm     | 10000       | 10500       |
| 1.25cm  | 24000       | 24250       |
| 6mm     | 47000       | 47200       |
| 4mm     | 75500       | 81000       |
| 2.5mm   | 119980      | 123000      |
| 2mm     | 134000      | 149000      |
| 1mm     | 241000      | 250000      |
| submm   | 300000      | 7500000     |

## Notes

- Frequency is stored internally in **kHz** as `u32`.
- ADIF `FREQ` field is **MHz** (floating point); export converts with `freq_khz / 1000.0`.
- `Band::from_frequency_khz` uses inclusive range matches (`lower..=upper` in kHz).
  Frequencies in gaps between bands (e.g., 2001–3499 kHz) return `None`.
- The 60m band (5.06–5.45 MHz) covers the ITU/IARU channelized allocation;
  some regions may have narrower allocations but the ADIF spec uses these edges.
