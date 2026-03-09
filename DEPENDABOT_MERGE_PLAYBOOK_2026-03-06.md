# Dependabot Merge Playbook 2026-03-06

前提:
- 根拠: `DEPENDABOT_TRIAGE_2026-03-06.md` と `.github/workflows/{automerge,ci,release}.yml`。
- 事前に `gh auth status` を通す。
- `#3` と `#4` は同時 merge しない。`release.yml` の故障点切り分け優先。
- 2026-03-10 status sync: `#4` → `#3` の順で re-review / merge 完了。open PR queue は `#7` manual migration のみ。
- ただし `#4` / `#3` merge 後の CI 確認 / release rehearsal は、この playbook の post-merge verification として別残件。

## Current Status

- merged 2026-03-07: `#5`, `#6`
- merged 2026-03-10 after re-review: `#4`, `#3`
- merge しない / manual migration only: `#7`

## Remaining Queue

1. `#7` `build(deps): update bincode requirement from 1 to 3`
   - 対応: merge しない。manual migration PR へ切替。
   - 状態: 2026-03-10 re-review 後も blocked。as-is merge 不可。

## Historical Checks / Remaining Check

### `#4`
- `gh pr checks 4`
- `gh pr diff 4 --name-only`
- 実施結果: re-review 後に 2026-03-10 merge 済み

### `#3`
- `gh pr checks 3`
- `gh pr diff 3 --name-only`
- 実施結果: `#4` 後に re-review し、2026-03-10 merge 済み

### `#7`
- `gh pr checks 7`
- `gh pr diff 7 --name-only`
- 確認点: CI failure 継続なら merge 禁止 / `rg -n "bincode::serialize|bincode::deserialize|bincode = \"1\"" Cargo.toml src tests`
- 現況: re-review 後も merge 禁止。manual migration 別PRのみ。

## Command Notes (`gh` CLI)

`#4` / `#3`:

```bash
# 2026-03-10 merge 済み
gh pr view 4 --json number,title,mergeable,reviewDecision,mergedAt
gh pr view 3 --json number,title,mergeable,reviewDecision,mergedAt
```

保留 `#7`:

```bash
gh pr view 7 --json number,title,mergeable,reviewDecision
# merge しない
```

## Post-merge Verification Checklist

- この batch の `#4` / `#3` は 2026-03-10 merge 済み。
- 各 merge 後:
  - `gh run list --branch develop --limit 5`
  - 直近 `CI` が success
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
