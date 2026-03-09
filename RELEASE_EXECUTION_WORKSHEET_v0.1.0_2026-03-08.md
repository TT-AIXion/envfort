# Release Execution Worksheet — v0.1.0

Status: pre-release worksheet as of 2026-03-08. Fill at execution time only. Blank values are intentional until the approved release commit is on `main` and live release artifacts exist.

Basis in repo:

- `RELEASE_HANDOFF_CHECKLIST.md`
- `RELEASE_NOTES_DRAFT_v0.1.0_2026-03-08.md`
- `HOMEBREW_FORMULA_INPUTS_2026-03-08.md`

Guardrails:

- Do not replace blanks with pre-release examples.
- Keep pre-release wording until publish/release visibility is confirmed.
- Homebrew work remains in the separate tap repo; this repo does not contain `Formula/envfort.rb`.
- Treat release-generated checksum files as the source of truth, then record one local verification result before sign-off.

## 0. Session

| Field | Value |
| --- | --- |
| Operator | |
| Release window | |
| Timezone used for notes | |
| Working branch / ref used for checks | |
| Notes / incident channel | |

## 1. Release IDs And URLs

| Item | Value | Notes |
| --- | --- | --- |
| Intended version on release commit | | `Cargo.toml` value at execution time |
| Release tag | | Expected form: `v<version>` |
| Release commit SHA | | Must be on `main` |
| Release commit merged to `main` at | | Timestamp |
| Release workflow run ID | | GitHub Actions |
| Additional run IDs | | Optional: retries / follow-up runs |
| GitHub Release URL | | Leave blank until live |
| crates.io package URL | | Leave blank until live |
| Homebrew tap PR URL | | Separate repo |
| Homebrew tap commit SHA | | Separate repo |

## 2. Preflight On Release Commit

| Check | Done | Recorded value / evidence |
| --- | --- | --- |
| Approved release commit confirmed on `main` | [ ] | |
| `Cargo.lock` tracked on release commit | [ ] | |
| Manifest / docs / install contract rechecked | [ ] | |
| Pre-release notes draft kept in pre-release state before publish | [ ] | |

Preflight notes / blockers:

- 

## 3. crates.io Stage

| Check | Done | Recorded value / evidence |
| --- | --- | --- |
| Release gates run from release commit | [ ] | Note command set / terminal log location |
| `cargo publish --locked` executed | [ ] | Timestamp + operator |
| crates.io page visible for target version | [ ] | Record final URL |
| `cargo install envfort --locked --version <version>` smoke passed | [ ] | |
| `envfort --version` matched expected version | [ ] | |

### crates.io Run Log

| Item | Value |
| --- | --- |
| Gate execution summary | |
| Publish timestamp | |
| crates.io URL | |
| Smoke-test host / shell | |
| `envfort --version` output | |
| Notes / blockers | |

## 4. GitHub Releases Stage

| Check | Done | Recorded value / evidence |
| --- | --- | --- |
| `release.yml` still tag-triggered at execution time | [ ] | |
| Release tag created from the merged release commit | [ ] | |
| Release tag pushed successfully | [ ] | |
| Release workflow completed successfully | [ ] | Record run ID / rerun ID if any |
| GitHub Release page exists | [ ] | Record final URL |
| Release notes body reviewed / patched if needed | [ ] | Record source file used |
| Asset names still match repo docs contract | [ ] | `envfort-macos`, `envfort-linux`, `envfort-windows.exe` |

### GitHub Release Tracking

| Item | Value |
| --- | --- |
| Triggering tag | |
| Release workflow run ID | |
| Release workflow URL | |
| Release URL | |
| Release notes source | |
| Download directory used for verification | |

### Asset And Checksum Ledger

| Asset name | Asset present | Checksum file present | Recorded sha256 | Local checksum verify result | Smoke-test result | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `envfort-macos` | [ ] | [ ] | | | | |
| `envfort-linux` | [ ] | [ ] | | | | |
| `envfort-windows.exe` | [ ] | [ ] | | | | |

### Asset Naming Contract

| Contract | Done | Notes |
| --- | --- | --- |
| `envfort-macos` installs/runs as `envfort` | [ ] | |
| `envfort-linux` installs/runs as `envfort` | [ ] | |
| `envfort-windows.exe` installs/runs as `envfort.exe` | [ ] | |

## 5. Homebrew Stage

Boundary note: fill this section only after the GitHub Release exists. The formula update is outside this repo and belongs in `TT-AIXion/homebrew-tap`.

| Check | Done | Recorded value / evidence |
| --- | --- | --- |
| Tap repo cloned / updated | [ ] | Record tap repo path / branch; run all Homebrew commands from that repo root |
| Formula values copied from live release assets only | [ ] | |
| Formula `url` updated to final macOS asset URL | [ ] | |
| Formula `sha256` updated from `envfort-macos.sha256` first field | [ ] | |
| Formula install mapping keeps final command name `envfort` | [ ] | |
| `brew audit --strict --online ./Formula/envfort.rb` passed | [ ] | Run from the tap repo root recorded above |
| `brew install --formula ./Formula/envfort.rb` passed | [ ] | Run from the tap repo root recorded above |
| `envfort --version` passed from formula install | [ ] | |
| `brew test envfort` passed or marked N/A | [ ] | |
| Tap PR opened | [ ] | Record final PR URL |
| End-to-end `brew tap` + `brew install envfort` smoke passed | [ ] | |

### Homebrew Formula Input Ledger

| Field | Value entered in tap repo |
| --- | --- |
| `version` | |
| `url` | |
| `sha256` | |
| `bin.install` mapping | `"envfort-macos" => "envfort"` |
| Tap commit SHA | |
| Tap PR URL | |

### Homebrew Smoke Notes

Run the following from the tap repo root recorded above unless a different path is explicitly noted.

| Step | Result | Notes |
| --- | --- | --- |
| `brew audit --strict --online ./Formula/envfort.rb` | | |
| `brew install --formula ./Formula/envfort.rb` | | |
| `envfort --version` | | |
| `brew test envfort` | | |
| `brew tap tt-aixion/tap && brew install envfort` | | |

## 6. Release Notes And Caveat Retention

Use this only for final copy review. Keep within current repo source of truth.

| Item | Done | Notes |
| --- | --- | --- |
| crates.io publish confirmed before removing pre-release wording | [ ] | |
| GitHub Release URL filled only after live release exists | [ ] | |
| Homebrew still marked planned until tap validation is complete | [ ] | |
| Linux keyring / passphrase-fallback caveat preserved if relevant | [ ] | |
| Web UI wording kept narrow: localhost-only / token-authenticated / local operations only | [ ] | |
| “write-only” caveat not overstated beyond repo docs | [ ] | |

## 7. Final Smoke-Test Summary

| Channel | Checksum verified | Smoke-test passed | Observed version / output | Notes |
| --- | --- | --- | --- | --- |
| crates.io install path | [ ] | [ ] | | |
| GitHub Release macOS asset | [ ] | [ ] | | |
| GitHub Release Linux asset | [ ] | [ ] | | |
| GitHub Release Windows asset | [ ] | [ ] | | |
| Homebrew install path | [ ] | [ ] | | |

## 8. Final Sign-Off

| Item | Value |
| --- | --- |
| Remaining blockers | |
| Operator sign-off | |
| Reviewer / witness | |
| Final decision | `GO` / `HOLD` |
| Sign-off timestamp | |
| Follow-up links | |
