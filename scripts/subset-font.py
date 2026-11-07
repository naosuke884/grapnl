#!/usr/bin/env python3
"""画面に出す日本語のフォントを、src/ の文字列で使う文字だけに絞って assets/fonts/ に置く。

src/*.rs の文字列リテラルに新しい文字を足したら、これを実行し直す。
絞ったフォントに無い文字を使うと、`cargo test` の text::tests が落ちる。

使い方: python3 scripts/subset-font.py
必要なもの: fonttools（pip install fonttools）
"""

import hashlib
import pathlib
import subprocess
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
# 元のフォントは大きい（約 2 MB）ので、リポジトリには入れず target/ に落としておく。
SOURCE_URL = (
    "https://raw.githubusercontent.com/google/fonts/"
    "d5ef175583bb5f7a3b01bc6c4603dd4a1f445f34/ofl/dotgothic16/DotGothic16-Regular.ttf"
)
SOURCE_SHA256 = "3ad9af88726d42b40f7f365f0dcac785af73cf20ea6f1d5b44e57cc21150b8f1"
SOURCE = ROOT / "target" / "fonts" / "DotGothic16-Regular.ttf"
OUTPUT = ROOT / "assets" / "fonts" / "DotGothic16-Regular-subset.ttf"
# ランキングの名前は誰でも付けられるので、全文字を入れたものを別に置く。
# ゲームは描けない名前があったときだけ後から読み込む（src/ranking.rs）。
NAMES_OUTPUT = ROOT / "web" / "DotGothic16-Regular-names.ttf"


def string_literal_chars(source: str) -> set[str]:
    """Rust のソースから、コメントを除いた文字列リテラルの中の文字を集める。

    src/text.rs のテストも同じ規則で文字を集めるので、変えるときは両方を揃える。
    """
    chars: set[str] = set()
    i = 0
    n = len(source)
    while i < n:
        c = source[i]
        if source.startswith("//", i):
            while i < n and source[i] != "\n":
                i += 1
        elif source.startswith("/*", i):
            end = source.find("*/", i + 2)
            i = n if end < 0 else end + 2
        elif c == "'":
            # 文字リテラル（'"' など）を読み飛ばす。ライフタイム（'a）はそのまま進む。
            if source.startswith("'\\", i):
                end = source.find("'", i + 2)
                i = n if end < 0 else end + 1
            elif i + 2 < n and source[i + 2] == "'":
                i += 3
            else:
                i += 1
        elif c == '"':
            i += 1
            while i < n and source[i] != '"':
                if source[i] == "\\":
                    i += 2
                    continue
                chars.add(source[i])
                i += 1
            i += 1
        else:
            i += 1
    return chars


def main() -> int:
    if not SOURCE.exists():
        SOURCE.parent.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen(SOURCE_URL) as response:
            SOURCE.write_bytes(response.read())
    digest = hashlib.sha256(SOURCE.read_bytes()).hexdigest()
    if digest != SOURCE_SHA256:
        print(f"{SOURCE} のハッシュが違う: {digest}", file=sys.stderr)
        return 1

    chars: set[str] = set()
    for path in sorted((ROOT / "src").glob("*.rs")):
        source = path.read_text(encoding="utf-8")
        # テストの失敗メッセージは画面に出ないので、テストのモジュールより前だけを見る。
        source = source.split("\n#[cfg(test)]\nmod tests", 1)[0]
        chars |= string_literal_chars(source)
    # ASCII の表示できる文字は、数字や記号を組み立てて出すのでいつも入れる。
    chars |= {chr(code) for code in range(0x20, 0x7F)}
    text = "".join(sorted(c for c in chars if c.isprintable()))

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            "pyftsubset",
            str(SOURCE),
            f"--text={text}",
            f"--output-file={OUTPUT}",
            "--layout-features=",
            "--no-hinting",
            "--desubroutinize",
        ],
        check=True,
    )
    print(f"{OUTPUT.relative_to(ROOT)}: {len(text)} 文字, {OUTPUT.stat().st_size} バイト")

    # 文字は絞らず、ヒントと組版の機能だけ落として小さくする。
    subprocess.run(
        [
            "pyftsubset",
            str(SOURCE),
            "--unicodes=*",
            f"--output-file={NAMES_OUTPUT}",
            "--layout-features=",
            "--no-hinting",
            "--desubroutinize",
        ],
        check=True,
    )
    print(f"{NAMES_OUTPUT.relative_to(ROOT)}: {NAMES_OUTPUT.stat().st_size} バイト")
    return 0


if __name__ == "__main__":
    sys.exit(main())
