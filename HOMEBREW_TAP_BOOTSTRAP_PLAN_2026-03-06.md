# Homebrew Tap Bootstrap Plan 2026-03-06

## Scope

- `envfort` release artifacts are produced by [`.github/workflows/release.yml`](./.github/workflows/release.yml).
- Current macOS asset names are `envfort-macos` and `envfort-macos.sha256`.
- Tap updates are still manual and belong in `TT-AIXion/homebrew-tap`.

## Bootstrap Once

If `TT-AIXion/homebrew-tap` does not exist yet:

```bash
brew tap-new TT-AIXion/tap
```

Then:

- push the generated tap skeleton to the GitHub repo `TT-AIXion/homebrew-tap`
- keep `Formula/envfort.rb` in that repo
- keep the branch users tap release-ready at all times

## Required Repositories And Permissions

| Target | Purpose | Minimum access |
| --- | --- | --- |
| `TT-AIXion/envfort` | source repo, tags, GitHub Releases, Actions logs | `Contents: read`, `Actions: read` |
| `TT-AIXion/envfort` | cut tag / rerun release if the same operator owns release | `Contents: write` |
| `TT-AIXion/homebrew-tap` | update `Formula/envfort.rb` on the branch users tap | `Contents: write` |

Local prerequisites:

- `brew`, `gh`, `git`, `shasum` installed
- `gh auth status` is green
- Tap repo name stays `homebrew-tap` so `brew tap tt-aixion/tap` keeps working
- Formula file lives at `Formula/envfort.rb`

## Release Workflow Per Version

1. Release `envfort` first.
   - Merge release commit to `main`.
   - Create tag `vX.Y.Z`.
   - Wait until GitHub Release contains `envfort-macos` and `envfort-macos.sha256`.
2. Sync local tap worktree.
   - Clone once: `gh repo clone TT-AIXion/homebrew-tap /tmp/homebrew-tap-envfort`
   - Later runs: `git -C /tmp/homebrew-tap-envfort pull --ff-only`
3. Download the release artifacts.

```bash
export VERSION=0.1.0
export TAG="v${VERSION}"
export RELEASE_DIR=/tmp/envfort-release

rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"
gh release download "$TAG" -R TT-AIXion/envfort -D "$RELEASE_DIR" -p 'envfort-macos*'
ls -la "$RELEASE_DIR"
```

4. Update `Formula/envfort.rb`.
   - `url`: point at the immutable GitHub Release asset for `vX.Y.Z`
   - `sha256`: replace with the value from `envfort-macos.sha256`
   - `version`: update only if the formula uses an explicit version or Homebrew cannot infer it from the URL
5. Validate locally.
6. Only after validation, open or merge the tap change in `TT-AIXion/homebrew-tap`.

## Checksum Update Procedure

Use the release-generated checksum file as the source of truth, then verify it once locally before copying it into the formula.

```bash
cd "$RELEASE_DIR"
shasum -a 256 envfort-macos
cat envfort-macos.sha256
awk '{print $1}' envfort-macos.sha256
```

Operational rule:

- The first field of `envfort-macos.sha256` must match the local `shasum -a 256 envfort-macos` output.
- Copy that hash into `sha256` in `Formula/envfort.rb`.
- If the values differ, stop. Treat the release artifact as invalid and repair the release first.

## Local Verification

Run verification from the tap repo checkout.

```bash
cd /tmp/homebrew-tap-envfort
export HOMEBREW_NO_INSTALL_FROM_API=1

brew audit --strict --online ./Formula/envfort.rb
brew uninstall --force envfort
brew install --formula ./Formula/envfort.rb
envfort --version
brew test envfort
```

Notes:

- `HOMEBREW_NO_INSTALL_FROM_API=1` forces Homebrew to use the local formula checkout during install or reinstall work.
- If `brew uninstall --force envfort` reports "No such keg", continue.
- `brew test envfort` assumes the formula defines `test do`; keep that block meaningful.

## Failure Recovery

- Missing release asset:
  - `gh run list --workflow release.yml --limit 5`
  - `gh run view <run-id> --log`
  - repair the release workflow, rerun, then re-download artifacts
- Checksum mismatch:
  - do not update the tap
  - if the bad asset is already public, cut a new patch release instead of mutating the published asset in place
- `brew audit` failure:
  - fix formula style or metadata, rerun `brew audit --strict --online ./Formula/envfort.rb`
- `brew install` uses stale tap or API state:
  - confirm `HOMEBREW_NO_INSTALL_FROM_API=1`
  - retry with a fresh tap clone: `brew untap tt-aixion/tap` then `brew tap tt-aixion/tap`
- `brew test` failure:
  - debug locally first with `brew install --debug --verbose ./Formula/envfort.rb` or `brew test --debug envfort`
  - do not publish the tap update until `brew install` and `brew test` are both green
- Bad tap publish:
  - ship an immediate follow-up tap commit with the corrected `url`, `sha256`, or `version`
  - re-run `brew update` and `brew install envfort` as a smoke test

## References

- Homebrew tap guide: https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap
- Homebrew taps overview: https://docs.brew.sh/Taps
- Homebrew Formula Cookbook: https://docs.brew.sh/Formula-Cookbook
- Homebrew manpage: https://docs.brew.sh/Manpage
- Repo handoff doc: [`RELEASE_HANDOFF_CHECKLIST.md`](./RELEASE_HANDOFF_CHECKLIST.md)
