#!/usr/bin/env python3
"""効果音を sfxr 風に合成し、assets/sfx/ に WAV で書き出す。

GUI の合成ツールの代わりに、音の作り方をここに残す。標準ライブラリだけで動き、
乱数の種を固定しているので、何度実行しても同じバイト列になる。
音を変えたらこのスクリプトを直して実行し、WAV も一緒にコミットする。

    python3 scripts/gen-sfx.py
"""

import math
import random
import struct
import wave
from pathlib import Path

RATE = 22050
OUT = Path(__file__).resolve().parent.parent / "assets" / "sfx"
# src/game.rs の MAX_MULTIPLIER と同じ。連鎖の倍率ごとに撃破音を 1 つずつ作る。
MAX_MULTIPLIER = 5


def synth(
    seconds,
    wave_shape,
    freq,
    freq_end=None,
    duty=0.5,
    attack=0.005,
    decay=None,
    volume=0.35,
    arp=None,
    vibrato=(0.0, 0.0),
    lowpass=None,
    seed=1,
):
    """sfxr と同じ考え方の 1 音。波形・周波数の滑り・音量の包絡・アルペジオ・ビブラートを持つ。

    - wave_shape: "square" / "saw" / "noise"
    - freq → freq_end: 周波数を指数的に滑らせる
    - arp: (時刻の比, 倍率)。その時刻から周波数に倍率を掛ける
    - vibrato: (深さの比, 速さ Hz)
    - lowpass: (始めの係数, 終わりの係数)。1 に近いほど高い音を通す
    """
    rng = random.Random(seed)
    n = int(seconds * RATE)
    freq_end = freq if freq_end is None else freq_end
    decay = seconds - attack if decay is None else decay
    phase = 0.0
    noise = rng.uniform(-1.0, 1.0)
    filtered = 0.0
    samples = []
    for i in range(n):
        t = i / RATE
        p = i / n
        f = freq * (freq_end / freq) ** p
        if arp is not None and p >= arp[0]:
            f *= arp[1]
        depth, speed = vibrato
        f *= 1.0 + depth * math.sin(2.0 * math.pi * speed * t)
        prev = phase
        phase = (phase + f / RATE) % 1.0
        if wave_shape == "square":
            v = 1.0 if phase < duty else -1.0
        elif wave_shape == "saw":
            v = 2.0 * phase - 1.0
        else:
            # sfxr のノイズと同じく、1 周期ごとに新しい乱数へ切り替える。周波数が音の粗さになる。
            if phase < prev:
                noise = rng.uniform(-1.0, 1.0)
            v = noise
        if lowpass is not None:
            k = lowpass[0] + (lowpass[1] - lowpass[0]) * p
            filtered += k * (v - filtered)
            v = filtered
        if t < attack:
            env = t / attack
        else:
            env = max(0.0, 1.0 - (t - attack) / decay)
        samples.append(v * env * volume)
    return samples


def mix(*layers):
    n = max(len(layer) for layer in layers)
    return [sum(layer[i] for layer in layers if i < len(layer)) for i in range(n)]


def write(name, samples):
    OUT.mkdir(parents=True, exist_ok=True)
    frames = b"".join(
        struct.pack("<h", int(max(-1.0, min(1.0, s)) * 32767)) for s in samples
    )
    with wave.open(str(OUT / f"{name}.wav"), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(frames)


def main():
    # ワイヤーを撃つ: 高いところから下がる短い「ピュッ」。
    write("shoot", synth(0.09, "square", 1100, 420, duty=0.25, volume=0.22))
    # 刺さる: ノイズの「カッ」と、低く沈む矩形波を重ねる。
    write(
        "attach",
        mix(
            synth(0.05, "noise", 3000, 800, volume=0.3, seed=2),
            synth(0.08, "square", 260, 120, duty=0.5, volume=0.2),
        ),
    )
    # 戻す: 低いところから上がる「シュッ」。撃つ音より控えめにする。
    write("retract", synth(0.1, "saw", 250, 900, volume=0.18))
    # 敵を倒す: ノイズの砕ける音と、上へ跳ねる 2 音。倍率が上がるほど半音 2 つずつ高くする。
    for m in range(1, MAX_MULTIPLIER + 1):
        base = 520 * 2 ** (2 * (m - 1) / 12)
        write(
            f"kill_{m}",
            mix(
                synth(0.08, "noise", 1800, 300, volume=0.28, seed=3),
                synth(0.18, "square", base, base * 1.1, duty=0.5, arp=(0.35, 1.5), volume=0.22),
            ),
        )
    # ゲームオーバー: 揺れながら下がっていく長めの音。
    write(
        "game_over",
        synth(0.6, "square", 440, 90, duty=0.5, vibrato=(0.04, 9.0), attack=0.01, volume=0.25),
    )


if __name__ == "__main__":
    main()
