#!/bin/sh
# コミットメッセージが Conventional Commits の形かを確かめる。
# 依存なしで動くよう POSIX sh だけで書いている。
#
#   scripts/check-commit-msg.sh <メッセージのファイル>
#
# lefthook の commit-msg フック (lefthook.yml) がコミットのたびに呼び、CI が push と PR の各コミットに呼ぶ。
# 確かめること:
#   - 1 行目が header_pattern に合う (既定は `type(scope)!: 件名`。scope と ! は省略可、
#     type は @commitlint/config-conventional と同じ 11 種)
#   - 1 行目は 100 文字以内 (config-conventional の header-max-length と同じ)
#   - 件名が「.」「。」で終わらない
#   - 2 行目があるなら空行 (件名と本文を分ける)
# マージ・git revert・fixup! / squash! / amend! のコミットは git が作る形なので確かめない。
set -eu

# 1 行目の形 (grep -E の正規表現)。エラーのときに見せる形と例も合わせて変える
header_pattern='^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([^()]+\))?!?: [^ ]'
header_form='type(scope): 件名  (type は feat fix docs style refactor perf test build ci chore revert)'
header_example='feat(board): セクションの並べ替えを追加する'

msg_file=${1:?使い方: scripts/check-commit-msg.sh <メッセージのファイル>}

# git がエディタ用に付けるコメント行 (# で始まる行) は保存されないので除く
body=$(grep -v '^#' "$msg_file" || true)
header=$(printf '%s\n' "$body" | sed -n '1p')
second=$(printf '%s\n' "$body" | sed -n '2p')

case $header in
  "Merge "* | "Revert \""* | "fixup! "* | "squash! "* | "amend! "*) exit 0 ;;
esac

fail() {
  {
    echo "コミットメッセージが規約に合いません: $1"
    echo "  1 行目: $header"
    echo "  形: $header_form"
    echo "  例: $header_example"
  } >&2
  exit 1
}

[ -n "$header" ] || fail "1 行目が空です"

printf '%s\n' "$header" | grep -Eq "$header_pattern" ||
  fail "1 行目が決まった形ではありません (コロンの後は半角スペース 1 つ)"

# 文字数で数える。wc -m はロケール次第でバイト数になるので、UTF-8 の継続バイト (0x80-0xBF) を除いたバイト数を数える
length=$(printf '%s' "$header" | LC_ALL=C tr -d '\200-\277' | wc -c | tr -d ' ')
[ "$length" -le 100 ] || fail "1 行目が ${length} 文字です (100 文字まで)"

case $header in
  *. | *。) fail "件名の終わりに「.」「。」を付けません" ;;
esac

[ -z "$second" ] || fail "2 行目は空行にして、本文はその次の行から書きます"
