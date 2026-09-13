# Widget Catalog

| Widget | Data source | Status |
| --- | --- | --- |
| Gear and RPM, Speed, Pedals, Steering | Fast LMU telemetry | Real |
| Input trace, Delta, Lap timing, Mini sectors, Coaching | Local LapEngine history/reference | Computed from official data |
| Sectors, Position, Flags | Official scoring | Real |
| Relative, Standings | Official scoring field | Real |
| Fuel | Official vehicle data | Real |
| Tyres, Brakes | Official wheel data | Real |
| Electronics, Energy, Engine, Damage | Official vehicle data | Real |
| Weather | Official session data | Real |
| Performance monitor | Overlay runtime | Real |

Widgets can be enabled from **Settings > Widgets**. Each widget instance has an independent position, size, scale, opacity, lock state and z-order. The Widget Browser identifies both the required source and the truth status. Widgets marked unavailable remain disabled until the official LMU header is available and the corresponding layout is verified; they are not treated as live data.
