# crates.io Release Runbook

Scope: `envfort` crate only. Run from the release commit already promoted to `main`. Git tag / GitHub Release / Homebrew are handled elsewhere.

## Setup

```bash
export CRATE=envfort
export VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"
```

## Pre-publish checklist

- [ ] Clean tree: `git diff --quiet && git diff --cached --quiet`
- [ ] Release commit is the intended `main` commit.
- [ ] `Cargo.lock` exists, is tracked, and matches the intended dependency graph: `git ls-files --error-unmatch Cargo.lock`
- [ ] Active toolchain satisfies `rust-version = 1.87`.
- [ ] Manifest metadata is current: `version`, `license`, `description`, `homepage`, `repository`, `readme`, `keywords`, `categories`.
- [ ] README install line still matches crates.io flow: `cargo install envfort --locked`
- [ ] README / release handoff install docs still describe the same manual-binary asset mapping: `envfort-macos` = macOS, `envfort-linux` = Linux, `envfort-windows.exe` = Windows.
- [ ] README / release handoff install docs still show checksum verification plus `--version` smoke steps for manual binaries, with `chmod +x` on macOS/Linux when the executable bit is missing.
- [ ] README manual install docs still show how the verified binary becomes the supported command name: `envfort` on macOS/Linux and `envfort.exe` on Windows.
- [ ] README / release handoff install docs still carry the Linux keyring caveat: OS keyring backend expected by default, `ENVFORT_PASSPHRASE` fallback documented for headless/minimal hosts.
- [ ] crates.io auth is ready: `CARGO_REGISTRY_TOKEN` in the environment or prior `cargo login`
- [ ] If this is not the first release, owners look correct: `cargo owner --list "$CRATE"`
- [ ] `Cargo.toml` keeps the package surface on the explicit include whitelist (`Cargo.toml`, `Cargo.lock`, `README.md`, `LICENSE`, `src/**`, `tests/**`).
- [ ] Do not use `--allow-dirty` or `--no-verify` for a real release.

Quick check:

```bash
git ls-files --error-unmatch Cargo.lock
rg -n '^version = "|^license = "|^description = "|^homepage = "|^repository = "|^readme = "|^keywords = |^categories = |^include = |cargo install envfort --locked|envfort-macos|envfort-linux|envfort-windows.exe|envfort.exe|install -m 0755|chmod \\+x|Get-FileHash|--version|ENVFORT_PASSPHRASE|sha256' Cargo.toml README.md RELEASE_HANDOFF_CHECKLIST.md
```

## Dry-run

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release --locked
cargo package --list --locked
cargo package --locked
ls -lh "target/package/$CRATE-$VERSION.crate"
cargo publish --dry-run --locked
```

## Publish

```bash
cargo publish --locked
```

## Post-publish verification

- [ ] Crate page resolves: `https://crates.io/crates/envfort`
- [ ] Fresh install works:

```bash
cargo install envfort --locked --version "$VERSION" --force
envfort --version
```

- [ ] Install docs still describe the same manual-binary contract called out in `RELEASE_HANDOFF_CHECKLIST.md`.
  - Asset meanings: `envfort-macos`, `envfort-linux`, `envfort-windows.exe`
  - Manual download notes: verify `.sha256`, run the platform binary with `--version`, then place it on `PATH` as `envfort` (macOS/Linux) or `envfort.exe` (Windows)
  - Platform specifics: `chmod +x` on macOS/Linux if needed; Windows compares `Get-FileHash` with `envfort-windows.exe.sha256`
  - Linux caveat: supported OS keyring backend by default, `ENVFORT_PASSPHRASE` fallback documented

- [ ] Owners still look correct:

```bash
cargo owner --list "$CRATE"
```

## Failure handling / retry rules

- Dry-run failed: stop. Fix the issue, then rerun the full dry-run sequence.
- `cargo publish` returned non-zero or timed out: do not retry immediately. First check whether the upload actually landed on crates.io.
- Wait 1-5 minutes, then verify `https://crates.io/crates/envfort` and retry install.
- Retry `cargo publish --locked` only if the new version is still absent and `https://status.crates.io/` is healthy.
- Once a version is published, it cannot be overwritten. If the published version is broken, bump the version and publish a replacement.
- Yank only for exceptional cases, ideally after the replacement version is live:

```bash
cargo yank --version "$VERSION" "$CRATE"
```

- If the crates.io token may have leaked, revoke it immediately. Yanking does not contain credential exposure.
