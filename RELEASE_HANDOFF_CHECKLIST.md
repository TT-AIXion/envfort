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
- [ ] Confirm the GitHub Release binary runs.
  ```bash
  chmod +x /tmp/envfort-release/envfort-macos
  /tmp/envfort-release/envfort-macos --version
  ```
- [ ] Confirm the Homebrew install path works end-to-end.
  ```bash
  brew update
  brew tap tt-aixion/tap
  brew install envfort
  envfort --version
  ```
