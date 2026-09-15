# Live LMU smoke test

This checklist is intentionally not a claim that live validation has been
performed. Run it on a Windows machine with Le Mans Ultimate installed and
record the date, build, and observed results before calling the release
candidate live-validated.

## Lifecycle

- [ ] Open Settings before LMU; start the overlay and confirm `WAITING FOR LMU`.
- [ ] Start LMU and confirm the overlay connects without restarting.
- [ ] Enable Main, Race, and Coach; confirm one `hashoverlay.exe` host and three independent windows.
- [ ] Move or resize each surface independently; disable one and confirm only that window disappears.
- [ ] Close LMU; confirm stale data is rejected and the overlay returns to `WAITING FOR LMU`.
- [ ] Reopen LMU; confirm automatic reconnect without restarting HashOverlay.
- [ ] Stop and reload from Settings; confirm both use the running host state.

## Telemetry and persistence

- [ ] Compare lap timing, sectors, delta, prediction, mini-sectors, invalidation, session best, and PB against LMU.
- [ ] After valid laps, compare fuel usage, rolling average, remaining laps, and pit/refuel exclusion.
- [ ] Compare relative ordering across start/finish, lapped traffic, missing cars, and same-class filtering.
- [ ] Compare tyres, brakes/bias, electronics, energy, engine, damage, weather, flags, and performance values.
- [ ] Verify Track Map only uses observed world positions and shows `TRACK DATA --` when unavailable.
- [ ] Keep Radar disabled unless a trustworthy official heading/orientation is available.

## Stability observation

- [ ] During an approximately one-hour session, observe RAM, CPU, GPU, handles, GDI objects, threads, and frame/telemetry latency.
- [ ] Confirm no monotonic growth, reconnect loop, thread leak, or increasing frame latency.
- [ ] Record limitations and mismatches here rather than presenting synthetic tests as live results.
