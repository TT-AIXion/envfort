# Release Handoff Checklist

Use this after the approved release commit is merged to `main`. Current repo state: `.github/workflows/release.yml` already builds Linux/macOS/Windows artifacts plus `.sha256` files from `v*` tags, but the Homebrew tap update is still separate/manual.

```bash
export VERSION=0.1.0
export TAG="v${VERSION}"
```

## crates.io

- [ ] Confirm the release commit is on `main`.
  ```bash
  git fetch origin --tags
  git log --oneline origin/main -n 5
  ```
- [ ] Confirm `Cargo.lock` is tracked for the binary-crate `--locked` path.
  ```bash
  git ls-files --error-unmatch Cargo.lock
  ```
- [ ] Confirm release metadata is aligned across the manifest and docs.
  ```bash
  rg -n '^version = "|^include = |cargo install envfort|brew install envfort|Release Flow|Release Process|Cargo.lock' Cargo.toml README.md CONTRIBUTING.md
  ```
- [ ] Run the release gates from the release commit.
  ```bash
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo test
  cargo build --release --locked
  cargo package --list --locked
  cargo package --locked
  cargo publish --dry-run --locked
  ```
- [ ] Publish the crate.
  ```bash
  cargo publish --locked
  ```

## GitHub Release

- [ ] Confirm the release workflow is still tag-triggered.
  ```bash
  sed -n '1,220p' .github/workflows/release.yml
  ```
- [ ] Create and push the release tag from the merged `main` commit.
  ```bash
  git tag -a "$TAG" -m "$TAG"
  git push origin "$TAG"
  ```
- [ ] Watch the release workflow and inspect logs if anything looks off.
  ```bash
  gh run list --workflow release.yml --limit 5
  gh run view <run-id> --log
  ```
- [ ] Verify the GitHub Release exists and includes the expected artifacts.
  ```bash
  gh release view "$TAG"
  gh release download "$TAG" -D /tmp/envfort-release
  ls -la /tmp/envfort-release
  cat /tmp/envfort-release/*.sha256
  ```
- [ ] Confirm the asset names still match install docs.
  - `envfort-macos` = macOS CLI binary
  - `envfort-linux` = Linux CLI binary
  - `envfort-windows.exe` = Windows CLI binary
- [ ] Add or polish release notes if the auto-created release body is too thin.
  ```bash
  gh release edit "$TAG" --notes-file /tmp/envfort-release-notes.md
  ```

## Homebrew tap

- [ ] Acknowledge current repo gap: this repo does not contain `Formula/envfort.rb` or tap automation.
- [ ] Clone or update the separate tap repository.
  ```bash
  gh repo clone TT-AIXion/homebrew-tap /tmp/homebrew-tap-envfort
  cd /tmp/homebrew-tap-envfort
  ```
- [ ] Pull the macOS release artifact and checksum.
  ```bash
  gh release download "$TAG" -R TT-AIXion/envfort -D /tmp/envfort-release
  cat /tmp/envfort-release/envfort-macos.sha256
  ```
- [ ] Update `Formula/envfort.rb` with the new version, URL, and `sha256`.
- [ ] Audit and install the formula locally before pushing.
  ```bash
  brew audit --strict ./Formula/envfort.rb
  brew install ./Formula/envfort.rb
  envfort --version
  ```
- [ ] If the formula defines `test do`, run the Homebrew test too.
  ```bash
  brew test envfort
  ```
- [ ] Commit and push the tap repo update after validation.

## post-release verification

- [ ] Confirm the crate is installable from crates.io.
  ```bash
  cargo install envfort --locked --version "$VERSION"
  envfort --version
  ```
- [ ] Confirm the macOS GitHub Release binary verifies and runs.
  ```bash
  cd /tmp/envfort-release \
    && shasum -a 256 -c envfort-macos.sha256 \
    && chmod +x envfort-macos \
    && ./envfort-macos --version
  ```
- [ ] Confirm the Linux GitHub Release binary verifies and runs on a Linux host.
  ```bash
  cd /tmp/envfort-release \
    && sha256sum -c envfort-linux.sha256 \
    && chmod +x envfort-linux \
    && ./envfort-linux --version
  ```
- [ ] If Linux post-release smoke goes beyond `--version`, use a supported OS keyring backend or document the passphrase fallback before sign-off.
  ```bash
  export ENVFORT_PASSPHRASE='<release-smoke-passphrase>'
  /tmp/envfort-release/envfort-linux init --profile release-smoke
  ```
- [ ] Confirm the Windows GitHub Release binary checksum, smoke test, and docs remain aligned.
  ```powershell
  $tag = if ($env:TAG) { $env:TAG } else { throw 'Set $env:TAG first, e.g. v0.1.0' }
  $releaseDir = Join-Path $env:TEMP 'envfort-release'
  New-Item -ItemType Directory -Force $releaseDir | Out-Null
  gh release download $tag -R TT-AIXion/envfort -D $releaseDir
  Set-Location $releaseDir
  $expected = ((Get-Content .\envfort-windows.exe.sha256 -Raw).Trim() -split '\s+')[0].ToLower()
  $actual = (Get-FileHash .\envfort-windows.exe -Algorithm SHA256).Hash.ToLower()
  if ($expected -ne $actual) { throw "SHA256 mismatch: expected $expected actual $actual" }
  .\envfort-windows.exe --version
  ```
- [ ] Confirm install docs still mention checksum verify, final command naming, and platform smoke steps.
  ```bash
  rg -n 'envfort-macos|envfort-linux|envfort-windows.exe|envfort.exe|install -m 0755|chmod \\+x|Get-FileHash|--version|sha256|ENVFORT_PASSPHRASE' README.md RELEASE_HANDOFF_CHECKLIST.md CRATES_IO_RELEASE_RUNBOOK_2026-03-06.md
  ```
- [ ] Confirm the Homebrew install path works end-to-end.
  ```bash
  brew update
  brew tap tt-aixion/tap
  brew install envfort
  envfort --version
  ```
