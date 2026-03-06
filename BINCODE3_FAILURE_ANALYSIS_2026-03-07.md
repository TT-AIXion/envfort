# BINCODE3 Failure Analysis — 2026-03-07

対象: Dependabot PR `#7` (`bincode` `1` -> `3`)

前提:
- 今回は分析のみ。repo 本体コード変更なし。
- ローカル確認は `rg` 中心。
- `cargo check --locked --offline` は依存キャッシュ不足で失敗。現環境ではフル compile 再現未了。
  - 実測: `no matching package named 'argon2' found`
- ただし PR #7 の主失敗面は、envfort 側コードより前段の upstream crate 側で確定可能。

## 1. exact failure surface / why PR #7 fails

結論:
- PR #7 は `bincode 3.0.0` を引く時点で失敗。
- 理由: `bincode 3.0.0` 自体が unmaintained で、crate root に `compile_error!` を置いているため。

根拠:
- 現行依存は [Cargo.toml](/Users/tt/Projects/envfort/Cargo.toml#L18) の `bincode = "1"`。
- official docs.rs の `bincode 3.0.0` は package page 上で build failure を示し、source でも `"The bincode package version 3.0.0 is unmaintained"` と明示。
- 同 page は maintained line として `bincode 2.0.1` / `bincode-next 2.0.1` を案内。

つまり:
- PR #7 は envfort の callsite 以前、依存 compile 段で落ちる。
- そのため「PR #7 をそのまま直して merge」は不可。

二次 failure surface:
- 仮に maintained 系へ切り直しても、envfort 側はまだ `bincode 1.x` API 前提。
- 直呼び 4 箇所が `serialize` / `deserialize` 依存なので、次は source migration が必要。

## 2. concrete code hotspots/files that must change

直接変更必須:
- [Cargo.toml](/Users/tt/Projects/envfort/Cargo.toml#L18)
  - `bincode = "3"` は不可。manual migration で maintained target を選び直す必要あり。
- [src/crypto.rs](/Users/tt/Projects/envfort/src/crypto.rs#L81)
  - `AadData::to_aad_bytes()`
  - 現在 `bincode::serialize(self)`。
  - ここは AAD byte 列の心臓部。互換性破壊時、既存 secret decrypt / unwrap が壊れる。
- [src/main.rs](/Users/tt/Projects/envfort/src/main.rs#L828)
  - `EncryptedBackupFile` export path。
  - 現在 [src/main.rs](/Users/tt/Projects/envfort/src/main.rs#L841) で `bincode::serialize(&payload)`。
- [src/main.rs](/Users/tt/Projects/envfort/src/main.rs#L869)
  - encrypted backup import path。
  - 現在 [src/main.rs](/Users/tt/Projects/envfort/src/main.rs#L870) で `bincode::deserialize(&encoded)`。
- [tests/cli_test.rs](/Users/tt/Projects/envfort/tests/cli_test.rs#L157)
  - seeded AAD bytes 生成。
  - 現在 [tests/cli_test.rs](/Users/tt/Projects/envfort/tests/cli_test.rs#L164) で `bincode::serialize(&aad)`。

波及確認必須:
- [src/main.rs](/Users/tt/Projects/envfort/src/main.rs#L161)
  - set path の AAD 作成。
- [src/main.rs](/Users/tt/Projects/envfort/src/main.rs#L310)
  - read/decrypt path の AAD 作成。
- [src/main.rs](/Users/tt/Projects/envfort/src/main.rs#L817)
  - export path の AAD 作成。
- [src/main.rs](/Users/tt/Projects/envfort/src/main.rs#L884)
  - import path の AAD 再構築。
- [src/storage.rs](/Users/tt/Projects/envfort/src/storage.rs#L341)
  - KEK rotate 時の old/new AAD。
- [src/storage.rs](/Users/tt/Projects/envfort/src/storage.rs#L688)
  - rotate 回帰 test。
- [src/ui.rs](/Users/tt/Projects/envfort/src/ui.rs#L623)
  - UI set path。`AadData::to_aad_bytes()` 変更の実害がここにも乗る。

重要点:
- direct bincode call は 4 箇所だけ。
- ただし `AadData` byte 列互換は runtime 全体へ波及。小さく見えて高リスク。

## 3. migration strategy in small safe batches

Batch 0: PR #7 は merge しない
- as-is では upstream compile failure。
- Dependabot PR は close/defer 扱いが安全。

Batch 1: target version 再決定
- `bincode 3` ではなく、maintained line を採用。
- 第一候補: `bincode 2.x` + `serde` feature。
- 判断根拠は official migration guide に固定。

Batch 2: AAD path だけ先に移行
- 対象:
  - [src/crypto.rs](/Users/tt/Projects/envfort/src/crypto.rs#L81)
  - [tests/cli_test.rs](/Users/tt/Projects/envfort/tests/cli_test.rs#L157)
- 方針:
  - `bincode::serde::encode_to_vec(..., bincode::config::legacy())`
- 理由:
  - `legacy()` で byte 互換を守る。
  - ここを先に固めると decrypt/rotate 回帰が見やすい。

Batch 3: backup export/import 移行
- 対象:
  - [src/main.rs](/Users/tt/Projects/envfort/src/main.rs#L828)
  - [src/main.rs](/Users/tt/Projects/envfort/src/main.rs#L869)
- 方針:
  - export: `encode_to_vec(..., legacy())`
  - import: `decode_from_slice(..., legacy())`
- 追加判断:
  - `bytes_read == encoded.len()` を確認し、trailing bytes を reject するか。
- 推奨:
  - import は strict 寄りでよい。外部入力なので余剰 bytes は弾く。

Batch 4: compatibility test 追加
- AAD byte fixture test
- old backup import test
- export -> import roundtrip test
- trailing bytes reject test

Batch 5: full validation
- `cargo check`
- `cargo clippy`
- `cargo test`
- release 系は今回 scope 外。触らない。

## 4. test / validation plan

最低限:
```bash
rg -n "bincode::serialize|bincode::deserialize|bincode = \"1\"|bincode = \"3\"" Cargo.toml src tests
cargo check --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

回帰重点:
```bash
cargo test rotate_profile_kek_rewraps_deks --locked
cargo test --test cli_test full_workflow_init_set_list_run --locked
```

追加推奨:
```bash
# tests/bincode_compat_test.rs を追加した後、その file の concrete test 名で個別実行
cargo test --test bincode_compat_test --locked
```

観点:
- 既存 DB record を decrypt できること
- KEK rotate 後も unwrap/decrypt できること
- CLI seeded path が壊れないこと
- old encrypted backup を import できること
- new export が self-roundtrip すること
- `bincode::serialize` / `deserialize` 残存ゼロ

今回の制約:
- 現環境では crates.io index / cache 不足で cargo 実行を完遂できていない。
- よって compile/test は plan 化まで。failure root cause 自体は upstream docs で確定済み。

## 5. recommendation: merge now vs defer

推奨: defer

理由:
- PR #7 as-is は upstream compile failure。merge 不可。
- 仮に version だけ直しても、AAD / backup byte 互換を意識した source migration が未実施。
- ここを雑に進めると既存 vault decrypt と encrypted backup import を壊すリスクが高い。

merge 可能条件:
- PR #7 とは別に manual migration PR を切る
- target は maintained version のみ
- `legacy()` 採用を明示
- compatibility tests 追加
- `cargo check` / `clippy` / `test` green

実務判断:
- 「今すぐ merge」ではなく「Dependabot PR #7 は defer、別 PR で安全移行」が妥当。

## References

official:
- docs.rs: `https://docs.rs/crate/bincode/3.0.0`
- docs.rs source: `https://docs.rs/crate/bincode/3.0.0/source/src/lib.rs`
- migration guide: `https://docs.rs/bincode/2.0.1/bincode/migration_guide/index.html`

repo-local:
- [BINCODE3_MIGRATION_PLAN.md](/Users/tt/Projects/envfort/BINCODE3_MIGRATION_PLAN.md)
- [BINCODE3_IMPLEMENTATION_TASKLIST_2026-03-06.md](/Users/tt/Projects/envfort/BINCODE3_IMPLEMENTATION_TASKLIST_2026-03-06.md)
