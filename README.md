# grapnl

[![CI](https://github.com/naosuke884/grapnl/actions/workflows/ci.yml/badge.svg)](https://github.com/naosuke884/grapnl/actions/workflows/ci.yml)

push と PR ごとに、書式・lint・テスト・WASM ビルドを GitHub Actions で確かめています（`.github/workflows/ci.yml`）。

Rust + [macroquad](https://github.com/not-fl3/macroquad) で作る、ブラウザ向け 2D アクションの土台です。

開発中はネイティブで実行します。WASM ビルドは公開するときだけ行います。macroquad の WASM は `wasm-bindgen` ではなく、同梱している専用 JS ローダー（`web/mq_js_bundle.js`）で動かします。

## 遊び方

ワイヤーを天井や浮いたブロックに刺し、振り子で右へ進み続けるエンドレスゲームです。最初は真上の天井にワイヤーが刺さった状態で始まります。

- 左クリック: カーソルの方向へワイヤーを撃つ。刺さっている間にクリックすると手元へ戻す。戻っている間に押したクリックは、戻り切ったときのカーソルの向きへ撃つ
- Esc: 一時停止する。もう一度 Esc かクリックで再開する。タブを離れたときやウィンドウのフォーカスが外れたときも止まる（ゲームオーバーの画面では止まらない）。操作の説明は、一時停止の画面の下にだけ出る
- 一時停止の画面で、BGM と効果音の音量を 5 段階で変えたり、消したりできる。← → でも変えられ、M で消す・出す。設定はブラウザに覚えておく
- ワイヤーを敵に当てると倒せて、その場所へ引き寄せられながら上へ大きく跳ねる
- 敵は 3 種類。揺れながら流れる敵に加え、400 m から上下に大きく往復する敵（上下にひれ）、900 m から画面に入ると光って溜めてから左へ突っ込む敵（左向きのとげ）が出る
- 進むほど地形が難しくなる。天井の穴が増えて広がり、穴の中のブロックは細く、間隔が広くなる（3000 m で最も難しくなる）
- ゲームオーバーの画面に、今日のコースのランキングの上位 5 位と自分の順位が出る。自己ベストを更新したら「NEW BEST!」が出る。「Retry」ボタン（または R・スペース）でやり直す
- 左から赤い領域が迫ってくる。上限なしにだんだん速くなり、約 6 分で自機の最高速を超えるので、終わりのないコースでも実質の制限時間がある。追いつかれるか、下へ落ちるか、敵に触れると終わり
- 背景は遠くの夜の街。遠い層ほどゆっくり流れ、空はなめらかなグラデーションで、色は進んだ距離で夕焼け → 夜 → 夜明けと移る（4000 m で一巡）
- 同じコースを同時に走っている人が、青い影と「名前#ID の頭 4 文字」で見える

初めて開いたときに、ランキングに出す名前の入力欄が一度だけ出ます（roommate-sdk が出す）。「あとで」で閉じても遊べ、一時停止の画面（Esc）の「Change name」でいつでも付けたり変えたりできます。入力欄を開いている間はゲームが一時停止します。

一時停止の画面（Esc）の「Delete my data」で、自分の匿名 ID と、roommate のすべてのゲームのスコアを消せます。確認のダイアログで「Delete」を選ぶと消し、消し終えたらページを開き直して新しい匿名 ID で始めます。ダイアログを開いている間はゲームが一時停止します。

一時停止の画面（Esc）の「← roommate」で、サイトのゲームページ（説明と今日のランキング）へ戻れます。

## 必要なもの

- [rustup](https://rustup.rs/) 経由の Rust stable（この雛形は rustc 1.99.1 でビルドした）
- ネイティブ実行に必要なシステムライブラリ（Linux）

```sh
sudo apt install pkg-config libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev
```

WASM ターゲットはビルドスクリプトが追加します。先に入れておく場合:

```sh
rustup target add wasm32-unknown-unknown
```

## ネイティブで実行する

```sh
cargo run
```

リサイズできるウィンドウが開きます。枠と中央の解像度表示は、ウィンドウサイズに合わせて毎フレーム配置し直します。

## 画面の文字とフォント

画面の文字は英語で、`src/text.rs` の `Text` で描きます。ランキングの名前には日本語も出るので、macroquad の既定のフォントの `draw_text` は使いません。データを消すダイアログも英語です（名前の入力欄は roommate-sdk が出すので日本語のまま）。

フォントはドットフォントを 3 つ使い分けます（どれも SIL OFL 1.1、`assets/fonts/OFL-*.txt`）。見出し・スコア・ボタン・ランキングの数字と英数字の名前は [Press Start 2P](https://fonts.google.com/specimen/Press+Start+2P)（8 の倍数の大きさ。名前を予約しているので絞らずに置く）、説明などの本文は [Pixelify Sans](https://fonts.google.com/specimen/Pixelify+Sans) の Regular（11 の倍数の大きさ）、日本語の名前は [DotGothic16](https://fonts.google.com/specimen/DotGothic16) です。本文と日本語は `src/` の文字列リテラルで使う文字（と「名無し#1a2b」の文字）だけに絞って `assets/fonts/` に置き、wasm に埋め込んでいます。文字列に新しい文字を足したら、絞り直します。

```sh
pip install fonttools
python3 scripts/subset-font.py
```

絞り直し忘れは `cargo test` が見つけます。

## WASM をビルドしてブラウザで確認する

```sh
./scripts/build-wasm.sh
python3 -m http.server 8080 --directory web
```

ブラウザで <http://127.0.0.1:8080> を開きます。`file://` では WASM を読めないので、必ず HTTP で配信します。

`web/` が配信用の一式です。

- `index.html` — 説明と OGP はサイトのゲームページと同じにし、共有の行き先（canonical・og:url）もそこにする。キャンバスで画面全体を埋める（ヘッダーは置かない）。サイトへ戻る・名前を変える・データを消すは一時停止の画面のボタンから page プラグイン（`page_go_to_site` / `roommate_sdk::open_name_dialog` / `page_open_delete_dialog`）を呼ぶ。データを消す確認と結果は `<dialog>` で出し、`window.roommate.client` の `requestDelete()` / `deleteState()` を呼ぶ。消し終えたらページを開き直す。「Change name」は `window.roommate.openNameDialog()` で roommate-sdk の入力欄を開き、開いている間は `isNameDialogOpen()` を見て一時停止する
- `mq_js_bundle.js` — macroquad 0.4.16 同梱の専用ローダー（`wasm-bindgen` ではない）
- `roommate-sdk.js` — roommate-sdk。匿名 ID・名前の入力欄・スコアの送信・自己ベスト・データの削除を受け持つ。`Cargo.toml` の `roommate-sdk` と同じタグの `packages/roommate-sdk/dist/roommate-sdk.js` を置く
- `realtime.js` — 同じコースを同時に走っている人の影を出すため、roommate の API の部屋に WebSocket で繋ぐ。部屋のサーバーが知らせる各人の表示名（「名前#1a2b」）も wasm へ渡し、影の上に出す
- `grapnl.wasm` — `./scripts/build-wasm.sh` が生成する（gitignore 済み）
- `DotGothic16-Regular-names.ttf` — ランキングの名前を描く全文字のフォント（約 1.8 MB）。画面の文字だけに絞った同梱のフォントで描けない名前があったときだけ、ゲームが後から読み込む。`scripts/subset-font.py` が作る
- `bgm.ogg` — BGM。`scripts/gen-bgm.py` で合成する。最初の読み込みを重くしないよう、ゲームが始まってから読み込む

main に push すると、CI が WASM をビルドし、R2 のバケット `roommate-games` の `grapnl/` に上書きで置きます。<https://play.roommate.hayashi-naoki.com/grapnl/> で遊べます。置く前に `scripts/stage-web.sh` が JS と wasm の参照に `?v=<commit>` を付け、更新後に古いファイルと新しいファイルが組み合わさらないようにします。戻すときは前のコミットに戻して push し直します。アップロードには GitHub の Environment `production` の Secret `R2_ACCOUNT_ID` / `R2_ACCESS_KEY_ID` / `R2_SECRET_ACCESS_KEY` を使います。

ほかの場所に置くときは、ビルド後の `web/` ディレクトリをそのまま置きます。itch.io などに上げる場合もこの 7 ファイルです（影は roommate の配信用ドメインからしか繋がりません）。キャンバスは表示領域の 100% なので、ページや埋め込み枠のサイズ変更に追従します。

## 効果音

効果音は `scripts/gen-sfx.py` で sfxr 風に合成した `assets/sfx/*.wav`（22050 Hz・16 bit・モノラル）で、wasm に埋め込みます。音を変えるときはスクリプトを直し、実行して WAV も一緒にコミットします。標準ライブラリだけで動き、何度実行しても同じファイルになります。

```sh
python3 scripts/gen-sfx.py
```

ブラウザは最初のクリックまで音を鳴らせないので、読み込んですぐの最初のワイヤーの音は鳴らないことがあります。

## ドット絵

自機・敵・地形・ワイヤーとフック・照準・破片・迫ってくる領域の縁はドット絵で、`src/sprites.rs` に文字の並びで書いています（文字 1 つがドット 1 つ、`.` は透明、色は絵ごとのパレット）。素材のファイルは使わず、起動時にテクスチャにして最近傍で拡大します。ドット 1 つはワールドで 4 の大きさです。絵を変えるときは文字の並びを直します。

## BGM

BGM は `scripts/gen-bgm.py` でチップチューン風に合成した `web/bgm.ogg`（A マイナー・150 BPM・16 小節で約 26 秒）で、繰り返し流します。買ったり借りたりした曲ではないので、使い方の制限はありません。大きさは効果音の 0.45 倍で、一時停止の画面の音量に従います。曲を変えるときはスクリプトを直し、実行して ogg も一緒にコミットします。sox が要ります。

```sh
python3 scripts/gen-bgm.py
```

macroquad の audio は JS 側の関数を import 先の指定なしで宣言していて、今の rustc ではリンクで落ちます。そのため `.cargo/config.toml` で、WASM のリンクだけ未定義の関数を JS からの import として通しています。

## 方針

- 反復は `cargo run`。デバッガが使え、WASM のビルド待ちを挟まない
- `cargo build --release --target wasm32-unknown-unknown` は公開用。release プロファイルはサイズ優先（LTO、`opt-level = "z"`）
- `web-sys` や `wasm-bindgen` 前提のクレートは、このローダーとはそのまま組み合わせられない
