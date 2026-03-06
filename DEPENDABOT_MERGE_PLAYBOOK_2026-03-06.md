# Dependabot Merge Playbook 2026-03-06

前提:
- 根拠: `DEPENDABOT_TRIAGE_2026-03-06.md` と `.github/workflows/{automerge,ci,release}.yml`。
- 事前に `gh auth status` を通す。
- `#3` と `#4` は同時 merge しない。`release.yml` の故障点切り分け優先。

## Recommended Order

1. `#5` `build(deps): bump actions/github-script from 7 to 8`
   - 対象: `.github/workflows/automerge.yml` のみ。
   - 理由: 低リスク。automerge 基盤を先に更新。
2. `#6` `build(deps): bump actions/checkout from 4 to 6`
   - 対象: `.github/workflows/ci.yml`, `.github/workflows/release.yml`。
   - 理由: CI 実走済み。release 影響はあるが変更面は狭い。
3. `#4` `build(deps): bump actions/upload-artifact from 4 to 7`
   - 対象: `release.yml` build 側 `Upload artifact`。
   - 理由: release 専用。`#3` より先に入れて fault isolation。
4. `#3` `build(deps): bump actions/download-artifact from 4 to 8`
   - 対象: `release.yml` publish 側 `Download artifacts` / `Verify checksums`。
   - 理由: 最も release 破壊しやすい。最後。
5. `#7` `build(deps): update bincode requirement from 1 to 3`
   - 対応: merge しない。manual migration PR へ切替。

## Pre-merge Checks Per PR

### `#5`
- `gh pr checks 5`
- `gh pr diff 5 --name-only`
- 確認点: `.github/workflows/automerge.yml` だけ変更 / required checks green

### `#6`
- `gh pr checks 6`
- `gh pr diff 6 --name-only`
- 確認点: `ci.yml` と `release.yml` の checkout 更新だけ / develop 直近 CI green

### `#4`
- `gh pr checks 4`
- `gh pr diff 4 --name-only`
- 確認点: `release.yml` の `actions/upload-artifact` 更新だけ / `#6` merge 後の develop CI green / rollback 担当者を決める

### `#3`
- `gh pr checks 3`
- `gh pr diff 3 --name-only`
- 確認点: `release.yml` の `actions/download-artifact` 更新だけ / `#4` merge 後の develop CI green / disposable tag rehearsal の段取りを先に持つ

### `#7`
- `gh pr checks 7`
- `gh pr diff 7 --name-only`
- 確認点: CI failure 継続なら merge 禁止 / `rg -n "bincode::serialize|bincode::deserialize|bincode = \"1\"" Cargo.toml src tests`

## Merge Commands (`gh` CLI)

低リスク枠 `#5` `#6`:

```bash
gh pr view 5 --json number,title,mergeable,reviewDecision
gh pr merge 5 --squash --delete-branch

gh pr view 6 --json number,title,mergeable,reviewDecision
gh pr merge 6 --squash --delete-branch
```

release 枠 `#4` `#3`:

```bash
gh pr view 4 --json number,title,mergeable,reviewDecision
gh pr merge 4 --squash --delete-branch

gh pr view 3 --json number,title,mergeable,reviewDecision
gh pr merge 3 --squash --delete-branch
```

保留 `#7`:

```bash
gh pr view 7 --json number,title,mergeable,reviewDecision
# merge しない
```

## Post-merge Verification Checklist

- 各 merge 後:
  - `gh run list --branch develop --limit 5`
  - 直近 `CI` が success
- `#5` 後:
  - 次の Dependabot PR で automerge ラベル運用が継続できることを確認
- `#6` 後:
  - `.github/workflows/release.yml` の checkout step が v6 に揃っていることを確認
- `#4` 後:
  - release rehearsal を推奨。失敗箇所が `Upload artifact` なら `#4` を疑う
- `#3` 後:
  - release rehearsal を必須。失敗箇所が `Download artifacts` または `Verify checksums` なら `#3` を最優先で疑う

rehearsal メモ:
- `workflow_dispatch` だけでは不足。現 `release.yml` は tag checkout 前提。
- 必要なら disposable tag で実施し、成功後に release/tag を掃除する。

## Rollback Guidance If Release Breaks After `#3` / `#4`

1. まず failing step を見る
   - `Upload artifact` 失敗: `#4` rollback
   - `Download artifacts` / `Verify checksums` 失敗: `#3` rollback
2. rollback は 1 PR ずつ
   - `#4` merge 直後に壊れた: `#4` だけ戻す
   - `#4` は通り、`#3` 後に壊れた: `#3` だけ戻す
3. revert 後、develop CI を green に戻してから再度 rehearsal
4. 失敗 tag は再利用しない
   - 新しい patch tag を切る。既存 tag の付け替えは避ける
5. `#7` は別件
   - release rollback と混ぜない。`bincode` 移行 PR で切り離す
