# Homebrew Formula Inputs 2026-03-08

Purpose: keep the `envfort` repo source of truth in one place so the later `TT-AIXion/homebrew-tap` update is copy-paste ready.

Scope limits:

- This repo does not contain `Formula/envfort.rb`.
- Do not add the Homebrew formula to this repo.
- Current tap target remains `TT-AIXion/homebrew-tap` with formula path `Formula/envfort.rb`.

## Source Of Truth Used

- `Cargo.toml`
- `README.md`
- `.github/workflows/release.yml`
- `HOMEBREW_TAP_BOOTSTRAP_PLAN_2026-03-06.md`
- `RELEASE_HANDOFF_CHECKLIST.md`
- `CRATES_IO_RELEASE_RUNBOOK_2026-03-06.md`

## Formula Constants

| Key | Current repo source of truth | Notes for tap repo |
| --- | --- | --- |
| `name` | `envfort` | Formula file name: `Formula/envfort.rb` |
| `desc` | `Write-only environment variable secret vault with keychain-backed envelope encryption` | Keep aligned with `Cargo.toml` `description` unless intentionally changed later |
| `homepage` | `https://github.com/TT-AIXion/envfort` | Same as current manifest `homepage` |
| `license` | `MIT` | Same as current manifest/license file |
| `version` | `0.1.0` | Use the value from `Cargo.toml` on the release commit; current repo value shown here |
| `asset URL pattern` | `https://github.com/TT-AIXion/envfort/releases/download/v${VERSION}/envfort-macos` | Homebrew tap currently consumes the macOS release asset |
| `sha256 source` | First field of `envfort-macos.sha256` | Verify it matches local `shasum -a 256 envfort-macos` before pasting |
| `binary install name` | `envfort` | Final installed executable name must stay `envfort` |

## Copy-Paste Ready Formula Inputs

Replace only the marked variable fields after the GitHub Release exists. In particular, `url` and `sha256` are release-time substitutions and must not be copied from a pre-release example.

```rb
desc "Write-only environment variable secret vault with keychain-backed envelope encryption"
homepage "https://github.com/TT-AIXion/envfort"
license "MIT"
url "<replace-with-https://github.com/TT-AIXion/envfort/releases/download/v<VERSION>/envfort-macos>"
sha256 "<replace-with-first-field-from-envfort-macos.sha256>"
# version "0.1.0" # keep only if the formula needs an explicit version

bin.install "envfort-macos" => "envfort"
```

Pattern form:

```text
url = https://github.com/TT-AIXion/envfort/releases/download/v${VERSION}/envfort-macos
sha256 = first field of envfort-macos.sha256
bin.install = "envfort-macos" => "envfort"
```

## Artifact Naming vs Installed Command Contract

| Platform | GitHub Release artifact | Manual install final command name | Formula-side expected executable name |
| --- | --- | --- | --- |
| macOS | `envfort-macos` | `envfort` | `envfort` |
| Linux | `envfort-linux` | `envfort` | Not used by the current Homebrew tap flow |
| Windows | `envfort-windows.exe` | `envfort.exe` | Not used by the current Homebrew tap flow |

Notes:

- The release workflow builds `target/release/envfort`, `target/release/envfort`, and `target/release/envfort.exe`, then republishes them as `envfort-macos`, `envfort-linux`, and `envfort-windows.exe`.
- The install contract in repo docs is explicit: GitHub Release assets may be platform-suffixed, but the supported installed command name remains `envfort` on macOS/Linux and `envfort.exe` on Windows.
- Do not publish a formula that leaves the executable name as `envfort-macos`.

## Variable Fields To Fill After Release

These values are intentionally left for the release operator or later tap work:

- `VERSION` from `Cargo.toml` on the release commit
- `TAG`, always `v${VERSION}`
- Final immutable macOS asset URL using that tag
- `sha256` copied from the first field of `envfort-macos.sha256`
- Optional explicit `version` line in the formula, only if Homebrew cannot infer it from `url`
- Tap commit/PR metadata in `TT-AIXion/homebrew-tap`

Operational note:

- Treat the release-generated checksum file as the source of truth, but only after one local verification pass confirms it matches `shasum -a 256 envfort-macos`.

## Shortest Handoff To Tap Repo

Tap checkout assumption:

```bash
gh repo clone TT-AIXion/homebrew-tap /tmp/homebrew-tap-envfort
# or, if it already exists:
git -C /tmp/homebrew-tap-envfort pull --ff-only
```

1. Read `VERSION` from `Cargo.toml` on the merged release commit, then set `TAG="v${VERSION}"`.
2. Download the macOS release files:

   ```bash
   export VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"
   export TAG="v${VERSION}"
   export RELEASE_DIR=/tmp/envfort-release
   rm -rf "$RELEASE_DIR"
   mkdir -p "$RELEASE_DIR"
   gh release download "$TAG" -R TT-AIXion/envfort -D "$RELEASE_DIR" -p 'envfort-macos*'
   ```

3. Verify checksum source, then extract the formula hash:

   ```bash
   cd "$RELEASE_DIR"
   shasum -a 256 envfort-macos
   cat envfort-macos.sha256
   awk '{print $1}' envfort-macos.sha256
   ```

4. In `TT-AIXion/homebrew-tap/Formula/envfort.rb`, update:
   - `desc`
   - `homepage`
   - `license`
   - `url`
   - `sha256`
   - optional `version`
   - install mapping so the final executable name is `envfort`
5. Validate in the tap repo before any push:

   ```bash
   cd /tmp/homebrew-tap-envfort
   export HOMEBREW_NO_INSTALL_FROM_API=1
   brew audit --strict --online ./Formula/envfort.rb
   brew uninstall --force envfort
   brew install --formula ./Formula/envfort.rb
   envfort --version
   brew test envfort
   ```

## Non-Goals For This Repo

- No tap repo creation/update here
- No `Formula/envfort.rb` added here
- No release/tag creation here
