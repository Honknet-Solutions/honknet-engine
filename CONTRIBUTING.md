# Contributing

Space Station 15 is developed by Open Station.

Early development rules:

- Keep core setting-agnostic.
- Do not add space-station-specific mechanics into core crates.
- Prefer explicit data formats over hidden engine behavior.
- Server authority is mandatory for gameplay state.
- Client code may predict and render, but not decide gameplay results.
