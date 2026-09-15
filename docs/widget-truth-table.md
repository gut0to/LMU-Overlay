# Widget truth table

HashOverlay renders a widget only when its source data is available. The
renderer does not invent values for fields that LMU does not publish.

| Surface | Source | Fallback |
| --- | --- | --- |
| Fuel | Official fuel and lap transitions | `--` until two valid laps exist |
| Relative | Official scoring field and track distance | Safe field order without precise gaps |
| Brakes | Official brake temperatures and bias | `--` for unavailable fields |
| Tyres | Official wheel temperatures and pressures | `--` per missing wheel value |
| Electronics | Official vehicle-system flags/settings | `--` per unavailable state |
| Energy | Official hybrid/ERS fields | `--` when absent |
| Damage | Official damage and impact fields | `--` when absent |
| Weather | Official session/weather fields | `--` when absent |
| Performance | Runtime acquisition/rendering counters | `--` before the first sample |

The canonical surface identifiers are maintained in
`crates/overlay-renderer/src/widgets.rs` and validated by unit tests.
