# Contributing

Thanks for helping improve LMU Overlay.

## Development Rules

- Do not work directly on `main`.
- Keep branches short and focused.
- Use Conventional Commits.
- Run the relevant tests before opening a pull request.
- Keep the runtime local and read-only.
- Do not add heavy dependencies without a clear reason.

## Branch Names

Use names like:

```text
feat/lmu-reader
feat/telemetry-widget
feat/delta-engine
fix/shared-memory-layout
perf/ring-buffer
chore/project-setup
```

## Commit Style

Use clear Conventional Commits:

```text
feat: add LMU shared memory reader
fix: correct lap progress interpolation
perf: remove allocations from telemetry loop
test: add ring buffer tests
docs: update usage guide
```

## Pull Request Checklist

Before opening a pull request:

- The code builds.
- Relevant tests pass.
- The change is scoped to one topic.
- User-facing behavior is documented when needed.
- Fair-play rules are preserved.

## Fair Play Boundary

Contributions must not:

- write to LMU memory;
- inject DLLs into the game;
- automate steering, pedals, gear shifts or other inputs;
- access hidden opponent data;
- bypass game protections.

This project is for visualizing and analyzing telemetry that LMU exposes for legitimate local use.
