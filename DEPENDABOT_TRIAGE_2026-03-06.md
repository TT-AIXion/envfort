# Dependabot Triage 2026-03-06

前提:
- 取得源: GitHub public UI + ローカル repo。
- `gh auth status` は 2026-03-06 時点で token invalid。exact `mergeable_state` は未取得。
- なので mergeability は「conflict 表示なし / checks 実態ベース」の運用判断。

| PR | Title | Update type | Checks | Mergeability | Risk note | Recommendation |
| --- | --- | --- | --- | --- | --- | --- |
| #5 | `build(deps): bump actions/github-script from 7 to 8` | `github-actions` / semver-major | `CI on: push` success, `CI on: pull_request` success | API未確認。conflict 表示なし | `.github/workflows/automerge.yml` のみ。Node 24 / runner `>=2.327.1` 要件あり。ただし `ubuntu-latest` 利用で、更新対象 workflow 自身も PR 上で動いており低リスク。 | Merge first |
| #6 | `build(deps): bump actions/checkout from 4 to 6` | `github-actions` / semver-major | `CI on: push` success, `CI on: pull_request` success | API未確認。conflict 表示なし | `.github/workflows/ci.yml` と `.github/workflows/release.yml`。CI 側は実走済み。release 側は未実走。v6 は Node 24 化、認証情報の保存場所変更あり。ただし現 workflow では大きな刺さり先は薄い。 | Merge after `#5` |
| #4 | `build(deps): bump actions/upload-artifact from 4 to 7` | `github-actions` / semver-major | `CI on: push` success, `CI on: pull_request` success | API未確認。conflict 表示なし | `.github/workflows/release.yml` のみ。PR CI では release workflow 自体を検証していない。v7 は Node 24 化、ESM 化、direct upload 追加。現利用は素直な `path: dist/*` で中リスク。 | Merge after `#6`; ideally with release dry-run nearby |
| #3 | `build(deps): bump actions/download-artifact from 4 to 8` | `github-actions` / semver-major | `CI on: push` success, `CI on: pull_request` success | API未確認。conflict 表示なし | `.github/workflows/release.yml` のみ。PR CI では未検証。v8 は「非 zip を自動 unzip しない」「digest mismatch を既定で error」に変更。artifact 集約 + checksum 検証に触るので `#4` より注意。 | Merge after `#4`; prefer release rehearsal |
| #7 | `build(deps): update bincode requirement from 1 to 3` | `cargo` / semver-major | `CI on: push` failure, `CI on: pull_request` failure | API未確認。required checks failure で実質 hold | 破壊的。現コードが `bincode::serialize` / `deserialize` を直接使用 (`src/crypto.rs`, `src/main.rs`, `tests/cli_test.rs`)。v3 ではそのまま通らない可能性高く、CI failure と整合。 | Do not merge. Replace with manual migration PR |

## Recommended Merge Order

1. `#5`
2. `#6`
3. `#4`
4. `#3`
5. `#7` は保留

運用メモ:
- すぐ merge 候補: `#5`, `#6`
- release 系はまとめて注意: `#4`, `#3`
- 最小運用なら `#4`/`#3` の前後で release workflow の手動 rehearsal を 1 回
- `#7` は Dependabot PR をそのまま merge せず、`bincode` v3 移行を含む手修正 PR に切替推奨
