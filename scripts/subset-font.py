#!/usr/bin/env python3
"""画面の文字のフォントを、src/ の文字列で使う文字だけに絞って assets/fonts/ に置く。

- 見出し: Press Start 2P（スコア・GAME OVER・ボタンなど目立たせる所）
- 本文: Pixelify Sans の Regular（説明・ランキングなど読ませる所）
- 日本語: DotGothic16（名前のうち「名無し」など、上の 2 つで描けない文字）
- 名前用: DotGothic16 の全文字。描けない名前があったときだけゲームが後から読む（web/ に置く）

どれも SIL OFL 1.1（assets/fonts/OFL-*.txt）。Press Start 2P は名前を予約しているので絞らずに置く。
src/*.rs の文字列リテラルに新しい文字を足したら、これを実行し直す。
どのフォントにも無い文字を使うと、`cargo test` の text::tests が落ちる。

使い方: python3 scripts/subset-font.py
必要なもの: fonttools（pip install fonttools）
"""

import hashlib
import pathlib
import subprocess
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
FONTS = ROOT / "target" / "fonts"
OUT = ROOT / "assets" / "fonts"
GOOGLE_FONTS = "https://raw.githubusercontent.com/google/fonts/"
# 元のフォントはリポジトリには入れず、版を固定して target/ に落としておく。(URL, sha256, 置き場所)
HEADING = (
    GOOGLE_FONTS + "dbd72d6ef9ebea750d33c02c7189633e972d6a40/ofl/pressstart2p/PressStart2P-Regular.ttf",
    "034c77f1f05ec89421e4a63f0e3a4ca1ecf852cc6d2bf611f126f275728e017d",
    FONTS / "PressStart2P-Regular.ttf",
)
BODY = (
    GOOGLE_FONTS + "9ce5017522020232f525003b39971ddb67e33243/ofl/pixelifysans/PixelifySans%5Bwght%5D.ttf",
    "9ba86cd010a4de309d263ceff8e8044092c9db7efda869620cb9ff1c4389e8a5",
    FONTS / "PixelifySans[wght].ttf",
)
JAPANESE = (
    GOOGLE_FONTS + "d5ef175583bb5f7a3b01bc6c4603dd4a1f445f34/ofl/dotgothic16/DotGothic16-Regular.ttf",
    "3ad9af88726d42b40f7f365f0dcac785af73cf20ea6f1d5b44e57cc21150b8f1",
    FONTS / "DotGothic16-Regular.ttf",
)
# 本文は太さを変えられるフォントなので、Regular（400）に固めてから絞る。macroquad は太さを変えられない。
BODY_WEIGHT = 400
# 名前の無い人の「名無し#1a2b」は、名前用のフォントを読まずに描けるよう日本語のフォントに入れる。
NAMELESS = "名無し#0123456789abcdef"
NAMES_OUTPUT = ROOT / "web" / "DotGothic16-Regular-names.ttf"
SUBSET_FLAGS = ["--layout-features=", "--no-hinting", "--desubroutinize"]


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


def fetch(font) -> pathlib.Path:
    url, sha256, path = font
    if not path.exists():
        path.parent.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen(url) as response:
            path.write_bytes(response.read())
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != sha256:
        raise SystemExit(f"{path} のハッシュが違う: {digest}")
    return path


def covered(path: pathlib.Path) -> set[str]:
    from fontTools.ttLib import TTFont

    return {chr(code) for code in TTFont(path).getBestCmap()}


def subset(source: pathlib.Path, text: str, output: pathlib.Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["pyftsubset", str(source), f"--text={text}", f"--output-file={output}", *SUBSET_FLAGS],
        check=True,
    )
    print(f"{output.relative_to(ROOT)}: {len(text)} 文字, {output.stat().st_size} バイト")


def main() -> int:
    heading = fetch(HEADING)
    body_variable = fetch(BODY)
    japanese = fetch(JAPANESE)

    # 本文のフォントを Regular に固める。
    body = FONTS / f"PixelifySans-{BODY_WEIGHT}.ttf"
    subprocess.run(
        [
            sys.executable,
            "-m",
            "fontTools.varLib.instancer",
            str(body_variable),
            f"wght={BODY_WEIGHT}",
            "--static",
            "-o",
            str(body),
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )

    chars: set[str] = set()
    for path in sorted((ROOT / "src").glob("*.rs")):
        source = path.read_text(encoding="utf-8")
        # テストの失敗メッセージは画面に出ないので、テストのモジュールより前だけを見る。
        source = source.split("\n#[cfg(test)]\nmod tests", 1)[0]
        chars |= string_literal_chars(source)
    # ASCII の表示できる文字は、数字や記号を組み立てて出すのでいつも入れる。
    ascii_chars = {chr(code) for code in range(0x20, 0x7F)}
    chars = {c for c in chars | ascii_chars if c.isprintable()}

    heading_chars = chars & covered(heading)
    body_chars = chars & covered(body)
    # 日本語のフォントには、ほかの 2 つで描けない文字と、名前の無い人の名前を入れる。
    rest = chars - heading_chars - body_chars
    japanese_chars = (rest | set(NAMELESS) | ascii_chars) & covered(japanese)
    # Press Start 2P は名前を予約しているので（Reserved Font Name）、絞ると名前を変えなければならない。
    # 絞らずにそのまま置く（約 120 KB）。
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "PressStart2P-Regular.ttf").write_bytes(heading.read_bytes())
    print(f"assets/fonts/PressStart2P-Regular.ttf: 絞らずにそのまま, {heading.stat().st_size} バイト")
    subset(body, "".join(sorted(body_chars)), OUT / "PixelifySans-subset.ttf")
    subset(japanese, "".join(sorted(japanese_chars)), OUT / "DotGothic16-Regular-subset.ttf")

    # 名前用は文字を絞らず、ヒントと組版の機能だけ落として小さくする。
    subprocess.run(
        ["pyftsubset", str(japanese), "--unicodes=*", f"--output-file={NAMES_OUTPUT}", *SUBSET_FLAGS],
        check=True,
    )
    print(f"{NAMES_OUTPUT.relative_to(ROOT)}: {NAMES_OUTPUT.stat().st_size} バイト")
    return 0


if __name__ == "__main__":
    sys.exit(main())
