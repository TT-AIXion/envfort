# CODEX Summary: Release Notes Draft

Date: 2026-03-08

## Scope

- Created release-notes draft only.
- No code changes.
- No commit.
- No push.

## Files created

- `RELEASE_NOTES_DRAFT_v0.1.0_2026-03-08.md`
- `CODEX_SUMMARY_RELEASE_NOTES_DRAFT_2026-03-08.md`

## Source files used

- `README.md`
- `Cargo.toml`
- `SECURITY.md`
- `.github/workflows/release.yml`
- `RELEASE_HANDOFF_CHECKLIST.md`
- `CRATES_IO_RELEASE_RUNBOOK_2026-03-06.md`
- `HOMEBREW_FORMULA_INPUTS_2026-03-08.md`

## Drafting rules applied

- Kept unreleased premise intact.
- Marked Homebrew as planned.
- Used repo source-of-truth files only.
- Avoided claims that crates.io publish or GitHub Release already exist.
- Left release-time placeholders and caveats explicit.

## Main content choices

- Framed `v0.1.0` as a public-release draft based on the current package version and explicit pre-release wording.
- Highlights limited to features already documented in repo: secret workflow, injection modes, allowlist/`--ci`/`--llm-safe`, audit/profile/rotation/backup flows, localhost-only Web UI.
- Security/privacy notes limited to currently documented design and threat-model caveats.
