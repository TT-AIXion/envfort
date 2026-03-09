# CODEX REVIEW - Cargo.lock / Package Surface (2026-03-08)

## Scope reviewed

- 対象差分のみレビュー:
  - `.gitignore`
  - `Cargo.toml`
  - `README.md`
  - `CONTRIBUTING.md`
  - `CRATES_IO_RELEASE_RUNBOOK_2026-03-06.md`
  - `RELEASE_HANDOFF_CHECKLIST.md`
  - `Cargo.lock`
  - `CODEX_SUMMARY_CARGOLOCK_PACKAGE_SURFACE_2026-03-08.md`
- 実施:
  - `git diff --stat` / `git diff`
  - `git diff --check` + `git diff --no-index --check` for new files
  - `git check-ignore Cargo.lock`
  - `git ls-files` sanity checks for package surface
  - static audit of `src/**` / `tests/**` coverage and runtime asset paths
  - static compare of `Cargo.toml` direct deps vs `Cargo.lock` root package deps
- 制約:
  - `cargo` / `rustc` unavailable in this environment
  - `cargo package --list --locked`, `cargo package --locked`, `cargo publish --dry-run --locked` は未実行

## Findings

- Blocking findings: none.
- Manifest / docs / release flow alignment: OK.
  - `.gitignore` no longer ignores `Cargo.lock`.
  - `Cargo.toml` include whitelist is explicit and narrow: `Cargo.toml`, `Cargo.lock`, `README.md`, `LICENSE`, `src/**`, `tests/**`.
  - `README.md`, `CONTRIBUTING.md`, runbook, handoff checklist all describe the same tracked-`Cargo.lock` + `--locked` policy.
- Package surface: static reviewでは妥当.
  - Runtime assets live under `src/ui/*`, so `src/**` covers shipped Web UI assets.
  - No `build.rs`, examples, benches, or out-of-tree runtime assets were found that would be accidentally excluded by the new whitelist.
  - `tests/**` contains only `tests/cli_test.rs`; no obvious secret/fixture spill into the published crate.
- Lockfile shape: static reviewでは妥当.
  - `Cargo.lock` root package deps match `Cargo.toml` direct deps + dev-deps exactly.
  - No `git+` / `path+` / other non-registry package sources found.
  - Registry packages carry checksums; no obvious lockfile corruption signs.
- Whitespace / diff hygiene: OK.
  - `git diff --check` produced no findings for tracked edits.
  - `git diff --no-index --check` produced no findings for new `Cargo.lock` and summary doc.
- Non-blocking note:
  - `CODEX_SUMMARY_CARGOLOCK_PACKAGE_SURFACE_2026-03-08.md` says `Cargo.lock` is "content unchanged". In current repo state, that claim is not verifiable from Git alone because `Cargo.lock` is currently untracked. Safer wording would be neutral unless external comparison evidence exists.

## Final verdict

safe to commit: Yes

Basis:

- No packaging, manifest-alignment, or release-flow blocker found in the reviewed diff.
- Verdict is static-review-based only.
- Assumption: the actual commit includes `Cargo.lock` itself, not just the surrounding doc changes.

## If No, exact fixes required

- N/A
