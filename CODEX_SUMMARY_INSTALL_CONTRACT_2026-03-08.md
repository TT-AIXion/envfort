# CODEX Summary Install Contract (2026-03-08)

Scope: docs-only update. No code changes. No commit/push.

## Updated files

- `README.md`
- `RELEASE_HANDOFF_CHECKLIST.md`
- `CRATES_IO_RELEASE_RUNBOOK_2026-03-06.md`
- `CODEX_SUMMARY_INSTALL_CONTRACT_2026-03-08.md`

## Manual install / verify contract now explicit

- GitHub Releases asset mapping is explicit:
  - `envfort-macos` = macOS CLI binary
  - `envfort-linux` = Linux CLI binary
  - `envfort-windows.exe` = Windows CLI binary
- README manual install now ends with the supported command names: `envfort` on macOS/Linux and `envfort.exe` on Windows.
- macOS/Linux handoff verification now runs from the artifact directory before checksum verify and `--version` smoke.
- Windows handoff verification now uses a native temp path, compares `Get-FileHash` output with `envfort-windows.exe.sha256`, and runs `.\envfort-windows.exe --version`.
- Manual macOS/Linux downloads may require `chmod +x` when the executable bit is not preserved by the download path.
- Linux manual install docs mention the keyring backend caveat and the `ENVFORT_PASSPHRASE` fallback for headless/minimal hosts.
- `CRATES_IO_RELEASE_RUNBOOK_2026-03-06.md` now checks those concrete README / handoff contract points instead of a generic "aligned" assertion.
