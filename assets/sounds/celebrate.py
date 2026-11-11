"""Placeholder celebration music for the gate: an original 12-second dance loop at 126 BPM (kick,
clap, hi-hats, bass and chord stabs). Writes celebrate.wav (22.05 kHz, 16-bit, mono). Standard
library only.

Your own track goes in assets/sounds/local/celebrate.wav (git-ignored); system/dev-install.sh
installs that one instead when it exists. The gate finds the beat of whatever it plays."""
import math
import random
import struct
import wave

RATE = 22050
BPM = 126
BEAT = 60 / BPM
BARS = 6
LENGTH = BARS * 4 * BEAT

random.seed(1)
out = [0.0] * int(LENGTH * RATE)


def add(start, samples):
    first = int(start * RATE)
    for i, s in enumerate(samples):
        if first + i < len(out):
            out[first + i] += s


def kick():
    n = int(0.25 * RATE)
    phase = 0.0
    for i in range(n):
        t = i / RATE
        phase += 2 * math.pi * (45 + 110 * math.exp(-t * 30)) / RATE  # pitch drops: a thump
        yield 0.9 * math.sin(phase) * math.exp(-t * 12)


def noise_hit(length, decay, loud):
    for i in range(int(length * RATE)):
        yield loud * (random.random() * 2 - 1) * math.exp(-i / RATE * decay)


def tone(freq, length, loud, decay):
    for i in range(int(length * RATE)):
        t = i / RATE
        saw = 2 * ((t * freq) % 1) - 1
        yield loud * (0.6 * saw + 0.4 * math.sin(2 * math.pi * freq * t)) * math.exp(-t * decay)


A2, C3, D3, E3, G3 = 110.0, 130.81, 146.83, 164.81, 196.0
BASS = [A2, A2, C3, D3, A2, A2, G3 / 2, E3]   # one note per beat, two-bar pattern
CHORDS = [(220.0, 261.63, 329.63), (196.0, 246.94, 293.66)]  # Am, G

for bar in range(BARS):
    for beat in range(4):
        t = (bar * 4 + beat) * BEAT
        add(t, kick())
        if beat in (1, 3):
            add(t, noise_hit(0.18, 22, 0.35))           # clap
        add(t + BEAT / 2, noise_hit(0.05, 90, 0.18))     # off-beat hi-hat
        add(t, tone(BASS[(bar % 2) * 4 + beat], BEAT * 0.9, 0.25, 4))
        if bar >= 2 and beat in (1, 3):                  # chord stabs join after two bars
            for f in CHORDS[(bar // 2) % 2]:
                add(t + BEAT / 2, tone(f, 0.2, 0.08, 14))

peak = max(abs(s) for s in out)
with wave.open('celebrate.wav', 'wb') as w:
    w.setnchannels(1)
    w.setsampwidth(2)
    w.setframerate(RATE)
    w.writeframes(b''.join(struct.pack('<h', round(s / peak * 0.89 * 32767)) for s in out))
