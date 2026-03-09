# envfort v0.1.0 Release Notes Draft

Status: pre-release draft as of 2026-03-08. Repo-fact only. Release-time placeholders remain intentionally unfilled.

## Summary

`envfort` v0.1.0 is drafted for a public release of a write-only secret vault for environment-variable injection. The current repo describes a Rust CLI with encrypted-at-rest storage, keychain-backed key handling, short-lived secret injection for command execution, and a localhost-only Web UI for local operations.

## Highlights

- Write-only workflow for secret operations: `init`, `set`, `list`, `run`, and `rm`.
- Encrypted secret storage in SQLite with envelope-encryption design and keychain-backed KEK handling.
- Multiple injection modes for `run`: `env`, `stdin`, `fd`, `socket`, and `tmpfile`.
- Command allowlist controls for safer execution paths, including `--ci` and `--llm-safe`.
- Local operations for audit tailing, profile isolation, KEK rotation, and encrypted backup export/import.
- Localhost-only Web UI with token auth and DNS rebinding protections.

## Security / privacy notes

- Repo docs describe AEAD encryption with `AES-256-GCM-SIV` and Argon2id support, including `kdf calibrate`.
- Secrets are documented as encrypted at rest, stored in SQLite, and decrypted only for short-lived command execution paths.
- The current design stores KEKs in the OS keychain by default, with a documented passphrase-derived fallback for supported cases.
- Backup and restore flows are documented as encrypted-only: `export --encrypted` and `import --encrypted`.
- `run` supports an allowlist policy, and `--llm-safe` is documented to enforce stricter injection behavior.
- Important caveat from the repo threat-model notes: "write-only" is a workflow/UI constraint, not a cryptographic guarantee.
- Additional repo caveats: same-user attackers are out of scope, LLM/AI isolation requires an OS boundary, and browser-mediated attacks matter when the Web UI is active.

## Install options

### crates.io

Planned release-time install command:

```bash
cargo install envfort --locked
```

Current repo docs describe `Cargo.lock` as tracked for the published binary crate and treat `--locked` as the supported install path.

### GitHub Releases

Planned release-time manual install path:

- Release assets documented in repo:
  - `envfort-macos`
  - `envfort-linux`
  - `envfort-windows.exe`
- Matching `.sha256` checksum files are expected alongside those assets.
- The current release workflow is tag-based (`v*`) and is configured to build Linux, macOS, and Windows binaries, verify checksums, and publish them to GitHub Releases.

### Homebrew tap planned

Planned install path after tap update:

```bash
brew tap tt-aixion/tap
brew install envfort
```

Current repo caveat: Homebrew support is planned, but this repo explicitly does not contain `Formula/envfort.rb`, and tap automation/update work remains separate from this repository.

## Release-time fill-ins / caveats

- Confirm publish status for `envfort` on crates.io before removing pre-release wording.
- Fill in final tag, release URL, asset URLs, and checksum references after the `v0.1.0` release exists.
- Keep Homebrew marked as planned until the separate tap repo is updated and validated.
- Verify public-facing notes stay aligned with the documented asset naming contract:
  - `envfort-macos` -> installed as `envfort`
  - `envfort-linux` -> installed as `envfort`
  - `envfort-windows.exe` -> installed as `envfort.exe`
- Preserve the Linux keyring caveat in install docs: headless/minimal hosts may need a supported keyring backend or the documented passphrase fallback.
- If the Web UI is mentioned in final release copy, keep the scope narrow: localhost-only, token-authenticated, local operations only.
