# AGENTS.md

## 作業の進め方

- issue は Linear で管理する（チーム: Hayashi Naoki `HAY`、プロジェクト: grapnl）。GitHub Issues は使わない

## 構成

- Rust + macroquad のブラウザ向け 2D アクション。ゲームのコードは `src/`、配信用の一式は `web/`
- WASM は `wasm-bindgen` ではなく、同梱の macroquad の JS ローダー（`web/mq_js_bundle.js`）で動かす。`web-sys` や `wasm-bindgen` 前提のクレートはそのままでは使えない
- `roommate-sdk` は crate と `web/roommate-sdk.js` を同じタグにそろえる（`Cargo.toml` のタグを上げたら JS も差し替える）。Dependabot の対象から外しているので手で上げる

## 開発

- ネイティブで動かす: `cargo run`（Linux では README の「必要なもの」のシステムライブラリが要る）
- push 前に CI と同じものを通す: `cargo fmt --check`・`cargo clippy --all-targets -- -D warnings`・`cargo test`・`cargo build --release --target wasm32-unknown-unknown`
- ブラウザで確かめる: `./scripts/build-wasm.sh` の後に `python3 -m http.server 8080 --directory web`
- 画面の文字列に新しい文字を足したら `python3 scripts/subset-font.py` でフォントを絞り直す（忘れると `cargo test` が落ちる）
- 効果音・BGM はスクリプト（`scripts/gen-sfx.py`・`scripts/gen-bgm.py`）を直して実行し、生成したファイルも一緒にコミットする
- main に push すると CI が R2 にデプロイする

## コミット

- Conventional Commits（`type(scope): 件名`、1 行目は 100 文字まで、件名の終わりに「。」「.」を付けない）。本文は空行を挟んで、なぜ変えたかを書く
- `scripts/check-commit-msg.sh` が確かめる。各クローンで一度 `lefthook install` すると commit-msg フックが入る
