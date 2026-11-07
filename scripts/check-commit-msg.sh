#!/bin/sh
# コミットメッセージがこのリポジトリの書き方に合うかを確かめる。
# lefthook の commit-msg フック（lefthook.yml）がコミットのたびに呼び、CI が push と PR の各コミットに呼ぶ。
# 使い方: scripts/check-commit-msg.sh <メッセージのファイル>
# 確かめること:
#   - 1 行目（件名）は日本語の 1 文。`feat:` のような接頭辞は付けない
#   - 件名は 72 文字まで。終わりに「。」「.」を付けない
#   - 2 行目があるなら空行（件名と本文を分ける）
# マージ・git revert・fixup! / squash! / amend! のコミットは git が作る形なので確かめない。
set -eu

msg_file=${1:?使い方: scripts/check-commit-msg.sh <メッセージのファイル>}

# git がエディタ用に付けるコメント行（# で始まる行）は保存されないので除く
body=$(grep -v '^#' "$msg_file" || true)
header=$(printf '%s\n' "$body" | sed -n '1p')
second=$(printf '%s\n' "$body" | sed -n '2p')

case $header in
  "Merge "* | "Revert \""* | "fixup! "* | "squash! "* | "amend! "*) exit 0 ;;
esac

fail() {
  {
    echo "コミットメッセージが書き方に合いません: $1"
    echo "  1 行目: $header"
    echo "  例: 一時停止とゲームオーバーの画面を main.rs から overlay.rs へ移す"
  } >&2
  exit 1
}

[ -n "$header" ] || fail "1 行目が空です"

case $header in
  " "* | "　"*) fail "1 行目を空白で始めません" ;;
esac

printf '%s\n' "$header" | grep -Eq '^[a-z]+(\([^()]*\))?!?: ' &&
  fail "「feat:」のような接頭辞は付けません"

# ASCII だけの件名は日本語で書いていない
printf '%s' "$header" | LC_ALL=C grep -q '[^ -~]' ||
  fail "件名は日本語で書きます"

# 文字数で数える。wc -m はロケール次第でバイト数になるので、UTF-8 の継続バイト（0x80-0xBF）を除いて数える
length=$(printf '%s' "$header" | LC_ALL=C tr -d '\200-\277' | wc -c | tr -d ' ')
[ "$length" -le 72 ] || fail "1 行目が ${length} 文字です（72 文字まで）"

case $header in
  *. | *。) fail "件名の終わりに「。」「.」を付けません" ;;
esac

[ -z "$second" ] || fail "2 行目は空行にして、本文はその次の行から書きます"
