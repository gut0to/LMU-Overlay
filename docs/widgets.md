# Widget Catalog

| Widget | Data source | Description |
| --- | --- | --- |
| Gear and RPM, Speed, Pedals, Steering | Fast LMU telemetry | Driving HUD values and controls. |
| Input trace, Delta, Lap timing, Mini sectors, Coaching | Local LapEngine history/reference | Bounded reference-lap analysis. |
| Sectors | Official scoring when mapped | S1, S2 and S3 values; unavailable values remain `--`. |
| Position, Relative, Standings, Flags | Official scoring when mapped | Race state and nearby cars. |
| Fuel | Official vehicle data when mapped | Quantity and future consumption estimate. |
| Tyres, Brakes | Official wheel data when mapped | Per-wheel pressure, temperature, wear and brake state. |
| Electronics, Energy, Engine, Damage | Official vehicle data when mapped | Car-state indicators. |
| Weather | Official session data when mapped | Ambient and track conditions. |
| Performance monitor | Overlay runtime | Telemetry/render rate, costs and skipped work. |

Widgets can be enabled from **Settings > Widgets**. Each widget instance has an independent position, size, scale, opacity, lock state and z-order. The Widget Browser identifies whether it needs fast telemetry, scoring or another official source.
