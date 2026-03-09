# CODEX SUMMARY — Homebrew Inputs (2026-03-08)

## Changed files

- `HOMEBREW_FORMULA_INPUTS_2026-03-08.md`
- `CODEX_SUMMARY_HOMEBREW_INPUTS_2026-03-08.md`

## What was prepared

- Added a repo-root handoff doc that centralizes the current Homebrew formula input constants from repo source of truth only.
- Captured the current naming/install contract so tap work keeps `envfort` / `envfort.exe` and does not leak platform-suffixed asset names into the installed command.
- Documented the exact macOS asset URL pattern, checksum source, variable fields that must be filled after release, and the shortest tap-side execution path.
- Kept the formula itself out of this repo, per current repo boundaries.

## Residual gaps

- Actual release-time values are still variable until the release exists: final `VERSION`, tag, immutable asset URL, and `sha256`.
- The tap repo remains separate/manual; no tap-side validation (`brew audit`, `brew install`, `brew test`) was run in this task.
- Current Homebrew flow in repo docs is still macOS-asset based; Linux and Windows remain GitHub Release/manual install contracts, not Homebrew inputs for this repo.
