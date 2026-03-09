# Dependabot Triage 2026-03-06

前提:
- 初版は 2026-03-07 時点の triage。
- 2026-03-10 status sync: `#3` merged, `#4` merged, `#7` は re-review 後も blocked / manual migration only。
- この status sync は GitHub PR metadata の反映。checked-out repo の workflow 実体変更とは分けて読む。

| PR | Title | Update type | Checks | Status | Risk note | Recommendation |
| --- | --- | --- | --- | --- | --- | --- |
| #5 | `build(deps): bump actions/github-script from 7 to 8` | `github-actions` / semver-major | `CI on: push` success, `CI on: pull_request` success | Merged 2026-03-07 | `.github/workflows/automerge.yml` のみ。Node 24 / runner `>=2.327.1` 要件あり。repo内で直接裏取りできるのは CI 成功までで、automerge workflow 自体の実走はこの資料だけでは断定しない。ただし `ubuntu-latest` 利用のため runner 要件リスクは比較的低い。 | Done |
| #6 | `build(deps): bump actions/checkout from 4 to 6` | `github-actions` / semver-major | `CI on: push` success, `CI on: pull_request` success | Merged 2026-03-07 | `.github/workflows/ci.yml` と `.github/workflows/release.yml`。CI 側は実走済み。release 側は未実走。v6 は Node 24 化、認証情報の保存場所変更あり。ただし現 workflow では大きな刺さり先は薄い。 | Done |
| #4 | `build(deps): bump actions/upload-artifact from 4 to 7` | `github-actions` / semver-major | `CI on: push` success, `CI on: pull_request` success | Merged 2026-03-10 | `.github/workflows/release.yml` のみ。PR CI では release workflow 自体を閉じきれない前提は残るが、re-review 後に merge 済み。 | Done |
| #3 | `build(deps): bump actions/download-artifact from 4 to 8` | `github-actions` / semver-major | `CI on: push` success, `CI on: pull_request` success | Merged 2026-03-10 | `.github/workflows/release.yml` のみ。artifact 集約 + checksum 検証に触る高注意 bump だったが、re-review 後に merge 済み。 | Done |
| #7 | `build(deps): update bincode requirement from 1 to 3` | `cargo` / semver-major | `CI on: push` failure, `CI on: pull_request` failure | Re-reviewed; blocked | 破壊的。現コードが `bincode::serialize` / `deserialize` を直接使用 (`src/crypto.rs`, `src/main.rs`, `tests/cli_test.rs`)。`bincode 3.0.0` は unmaintained かつ crate-level `compile_error!` あり。2026-03-10 re-review 後も as-is merge 不可。 | Manual migration only. Do not revive this PR; use separate small-batch migration to maintained `bincode 2.x` + `legacy()` |

## Current Queue

完了:
- `#5` merged 2026-03-07
- `#6` merged 2026-03-07
- `#4` merged 2026-03-10
- `#3` merged 2026-03-10

保留中:
1. `#7` は re-review 後も blocked。manual migration 別PR

運用メモ:
- merged 済み: `#5`, `#6`, `#4`, `#3`
- GitHub PR queue 上では release workflow 系の open Dependabot PR は解消
- ただし checked-out repo の workflow 実体反映確認、および `#4` / `#3` merge 後の CI / release rehearsal 記録は別管理
- `#7` は blocked のまま。Dependabot PR はそのまま merge しない
- 後続は別 manual migration PR。target は maintained `bincode 2.x` + `legacy()` 固定
