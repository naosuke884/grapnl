#!/usr/bin/env python3
"""BGM をチップチューン風に合成し、web/bgm.ogg に書き出す。

効果音（scripts/gen-sfx.py）と同じく、曲の作り方をここに残す。買ったり借りたりした曲ではないので、
使い方の制限は無い。乱数の種を固定しているので、何度実行しても同じ曲になる。
曲を変えたらこのスクリプトを直して実行し、ogg も一緒にコミットする。

    python3 scripts/gen-bgm.py

必要なもの: sox（ogg vorbis に書き出す）
曲は最初の読み込みを重くしないよう wasm に埋め込まず、ゲームが始まってから読み込む（src/bgm.rs）。
"""

import math
import random
import struct
import subprocess
import tempfile
import wave
from pathlib import Path

RATE = 22050
OUT = Path(__file__).resolve().parent.parent / "web" / "bgm.ogg"
BPM = 150
# 16 分音符 1 つの長さ（秒）。
STEP = 60.0 / BPM / 4
STEPS_PER_BAR = 16
# A マイナー。1 小節に 1 つの和音。16 小節で 1 周し、最後から最初へつながる。
CHORDS = [
    "Am", "F", "C", "G",
    "Am", "F", "C", "E",
    "F", "G", "Em", "Am",
    "F", "G", "E", "E",
]
# 和音の構成音（A3 = 57 を基準にした MIDI の番号）。
CHORD_NOTES = {
    "Am": [57, 60, 64],
    "F": [53, 57, 60],
    "C": [48, 52, 55],
    "G": [55, 59, 62],
    "E": [52, 56, 59],
    "Em": [52, 55, 59],
}
BARS = len(CHORDS)
LENGTH = BARS * STEPS_PER_BAR * STEP


def freq(note):
    return 440.0 * 2 ** ((note - 69) / 12)


def tone(buffer, start, seconds, note, shape, volume, duty=0.5, release=0.02):
    """`buffer` の `start` 秒から 1 音を足す。端は短くフェードしてプチッといわせない。"""
    f = freq(note)
    begin = int(start * RATE)
    n = int(seconds * RATE)
    attack = int(0.004 * RATE)
    tail = int(release * RATE)
    for i in range(n):
        phase = (i * f / RATE) % 1.0
        if shape == "square":
            v = 1.0 if phase < duty else -1.0
        else:
            # 三角波。ベースに使う。
            v = 4.0 * abs(phase - 0.5) - 1.0
        env = min(1.0, i / attack if attack else 1.0, (n - i) / tail if tail else 1.0)
        buffer[(begin + i) % len(buffer)] += v * env * volume


def noise(buffer, start, seconds, volume, rng, decay_rate, smooth=1.0):
    """ノイズの打楽器。`decay_rate` が大きいほど短く切れる。`smooth` が小さいほどこもる。"""
    begin = int(start * RATE)
    n = int(seconds * RATE)
    filtered = 0.0
    for i in range(n):
        filtered += smooth * (rng.uniform(-1.0, 1.0) - filtered)
        env = math.exp(-decay_rate * i / RATE)
        buffer[(begin + i) % len(buffer)] += filtered * env * volume


def kick(buffer, start, volume):
    """周波数を急に下げる正弦波のキック。"""
    begin = int(start * RATE)
    n = int(0.16 * RATE)
    phase = 0.0
    for i in range(n):
        t = i / RATE
        f = 50 + 110 * math.exp(-t * 35)
        phase += f / RATE
        env = math.exp(-t * 18)
        buffer[(begin + i) % len(buffer)] += math.sin(2 * math.pi * phase) * env * volume


def melody(rng):
    """和音の構成音の上を、跳びすぎないように歩く旋律。8 分音符単位で、休みも混ぜる。"""
    notes = []
    current = 72
    for bar, chord in enumerate(CHORDS):
        # 構成音を 1〜2 オクターブ上に並べた候補から選ぶ。
        pool = sorted({n + 12 * k for n in CHORD_NOTES[chord] for k in (1, 2) if 64 <= n + 12 * k <= 84})
        for eighth in range(8):
            # 小節の頭は必ず鳴らし、後半の小節ほど音を詰める。
            rest_chance = 0.0 if eighth == 0 else (0.45 if bar < 8 else 0.25)
            if rng.random() < rest_chance:
                notes.append(None)
                continue
            near = sorted(pool, key=lambda n: (abs(n - current), n))[:3]
            current = rng.choice(near)
            notes.append(current)
    return notes


def render():
    rng = random.Random(7)
    buffer = [0.0] * int(LENGTH * RATE)
    tune = melody(rng)
    for bar, chord in enumerate(CHORDS):
        bar_start = bar * STEPS_PER_BAR * STEP
        root = CHORD_NOTES[chord][0]
        bass_root = root - 12 if root >= 52 else root
        for step in range(STEPS_PER_BAR):
            t = bar_start + step * STEP
            # ベース: 8 分音符で根音とオクターブ上を行き来する。
            if step % 2 == 0:
                note = bass_root + (12 if step % 4 == 2 else 0)
                tone(buffer, t, STEP * 1.8, note, "triangle", 0.28)
            # アルペジオ: 16 分音符で構成音を上へなぞる。細い矩形波で小さく。
            arp = CHORD_NOTES[chord]
            tone(buffer, t, STEP * 0.9, arp[step % 3] + 12, "square", 0.045, duty=0.125)
            # ドラム: 1・3 拍目にキック、2・4 拍目にスネア、8 分でハイハット。
            if step % 8 == 0:
                kick(buffer, t, 0.55)
            if step % 8 == 4:
                noise(buffer, t, 0.18, 0.22, rng, 22.0, smooth=0.6)
            if step % 2 == 0:
                noise(buffer, t, 0.05, 0.07, rng, 90.0)
        # 旋律: 8 分音符。同じ音が続くときは前の音を伸ばす。
        for eighth in range(8):
            note = tune[bar * 8 + eighth]
            if note is None:
                continue
            length = 1
            while eighth + length < 8 and tune[bar * 8 + eighth + length] == note:
                length += 1
            if eighth > 0 and tune[bar * 8 + eighth - 1] == note:
                continue
            tone(buffer, bar_start + eighth * 2 * STEP, length * 2 * STEP * 0.92, note, "square", 0.09, duty=0.5)
    peak = max(abs(s) for s in buffer)
    # 効果音より控えめな大きさにそろえる。ゲームの中でさらに小さくして鳴らす。
    return [s / peak * 0.8 for s in buffer]


def main():
    samples = render()
    with tempfile.TemporaryDirectory() as tmp:
        wav = Path(tmp) / "bgm.wav"
        with wave.open(str(wav), "wb") as w:
            w.setnchannels(1)
            w.setsampwidth(2)
            w.setframerate(RATE)
            w.writeframes(b"".join(struct.pack("<h", int(s * 32767)) for s in samples))
        OUT.parent.mkdir(parents=True, exist_ok=True)
        # 品質 3 は約 100 kbps 弱。チップチューンなら十分に聞ける。
        subprocess.run(["sox", str(wav), "-C", "3", str(OUT)], check=True)
    print(f"{OUT}: {LENGTH:.1f} 秒, {OUT.stat().st_size} バイト")


if __name__ == "__main__":
    main()
