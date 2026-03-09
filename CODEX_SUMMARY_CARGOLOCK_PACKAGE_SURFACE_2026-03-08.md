# CODEX SUMMARY — Cargo.lock / Package Surface (2026-03-08)

## Changed files

- `.gitignore`
- `Cargo.toml`
- `README.md`
- `CONTRIBUTING.md`
- `CRATES_IO_RELEASE_RUNBOOK_2026-03-06.md`
- `RELEASE_HANDOFF_CHECKLIST.md`
- `Cargo.lock` (now no longer ignored and ready to be tracked in git)

## What was fixed

- Removed the repo-level ignore rule for `Cargo.lock` so the binary-crate `--locked` policy can be committed and enforced.
- Added an explicit package `include` whitelist in `Cargo.toml` to keep the published crate surface to `Cargo.toml`, `Cargo.lock`, `README.md`, `LICENSE`, `src/**`, and `tests/**`.
- Updated contributor and release docs to treat `Cargo.lock` as tracked source of truth for CI / release / `cargo install envfort --locked`.
- Added `cargo package --list --locked` to the release handoff gate so package surface review is explicit.

## Validation

- Ran: `git diff --check`
- Ran: `git check-ignore Cargo.lock` → not ignored
- Ran: `git ls-files -- 'Cargo.toml' 'README.md' 'LICENSE' 'src/**' 'tests/**'` to sanity-check the include surface
- Ran: `git status --short` to confirm `Cargo.lock` is now visible for tracking
- Not run: `cargo fmt`, `cargo clippy`, `cargo test`, `cargo build --release --locked`, `cargo package --list --locked`, `cargo package --locked`, `cargo publish --dry-run --locked`
- Reason not run: `cargo` / `rustc` not available in this environment (`command not found`; `~/.cargo/bin` symlinks are broken)

## Residual risks

- `Cargo.lock` is still untracked in git until the next commit adds it.
- Package surface was validated statically, not with `cargo package`.
- Release/main promotion, Homebrew, signing, provenance, and SBOM remain out of scope for this change.
