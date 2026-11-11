# 03 — The gate: a spoken question before login

Prerequisite: `docs/02-dev-loop-and-splash.md` is done (dev loop, splash and boot sound work).

After the splash and the boot sound, the screen shows the logo and a voice plays the question
**«Скажи мені, що таке Зомборі?»**. Only the answer **«музика»** lets the boot go on: a 3D head
with a platinum quiff and Zombori's orange glasses appears in front of laser beams and headbangs to
the music, then the login prompt comes up. A wrong
answer shakes the logo and asks again.

```
splash ─► boot sound ─► gate: question ─► listen ─► recognize ─┬─ «музика» ─► head + music ─► login
                                   ▲                           └─ anything else ─► "no" shake ─┐
                                   └───────────────────────────────────────────────────────────┘
```

The same 3D man is on screen the whole time:

| Stage | On screen |
|---|---|
| Question playing | he fades in, hands clasped under his chin, breathing calmly |
| Listening | he leans in; the lasers glow brighter the louder you speak; a red dot means recording |
| Recognizing | he glances to the side; three dots in the corner |
| Wrong answer | he shakes his head "no", the lasers flash red, then he asks again |
| «музика» | his hands drop away during the spoken "yes", then he headbangs with the music |

Everything runs offline in the VM. Speech is recognized by [Whisper](https://github.com/ggml-org/whisper.cpp)
(the `base` model, 148 MB), which understands Ukrainian well.

## How it is built

| Part | Where | Tested |
|---|---|---|
| Answer check (spellings, stress marks, one misheard letter) | `crates/gate/src/answer.rs` | on the Mac |
| Speech recognition | `crates/gate/src/recognize.rs` | on the Mac: `transcribe` example |
| Finding where speech starts and ends | `crates/audio/src/speech.rs` | on the Mac |
| Beat of the music, so the head bangs in time | `crates/gate/src/beat.rs` | on the Mac |
| The head: a 3D model (distance field) ray-traced on the CPU into the pixel-art screen | `crates/gate/src/head3d.rs` | on the Mac |
| What is on screen, and the headbang motion | `crates/gate/src/scene.rs` | on the Mac: `preview` example |
| Microphone, speaker, screen, and the order of it all | `crates/gate/src/main.rs` | in the VM |

## 1. Microphone in the VM

QEMU on macOS can play sound but not record it (its CoreAudio backend has no input), so the VM
gets the Mac's microphone over the network instead:

```
Mac: vm/mic.sh ── ffmpeg records the mic ──► UDP port 4713 ──► VM: zombori-vm-mic.service
                                                              └─► Loopback sound card ──► gate
```

Once, inside the VM (it installs `socat`, loads the Loopback card, makes it the default
recording device and starts the receiving service):

```sh
/mnt/zombori/system/vm-mic/install.sh
```

Whenever you use the gate, keep this running in its own terminal on the Mac. The first time,
macOS asks whether the terminal app may use the microphone: allow it (or later in System Settings
→ Privacy & Security → Microphone). It picks the built-in microphone by name; another input:
`MIC_DEVICE="iPhone" vm/mic.sh`.

```sh
vm/mic.sh
```

Inside the VM, check that your voice arrives:

```sh
arecord -d 3 -f S16_LE -r 16000 /tmp/test.wav && aplay /tmp/test.wav   # say something
```

The gate itself just records from the default device, so on real hardware with a real
microphone none of this is needed.

## 2. Build dependencies

Whisper is C++ and is compiled as part of the build:

```sh
sudo apt install -y build-essential cmake clang libclang-dev libasound2-dev pkg-config
```

## 3. Your clips

The question and the music are your own clips. They stay out of git (the folder is ignored), since
the repository is public:

```
assets/sounds/local/question.wav     the question, in the voice you made
assets/sounds/local/celebrate.wav    the music for the head (optional)
```

Both must be **16-bit PCM WAV**; any sample rate, mono or stereo. Convert on the Mac:

```sh
mkdir -p assets/sounds/local
afconvert -f WAVE -d LEI16@48000 ~/Downloads/question.mp3 assets/sounds/local/question.wav
afconvert -f WAVE -d LEI16@44100 ~/Downloads/song.mp3     assets/sounds/local/celebrate.wav
```

- Without a question clip the gate still works: it shows the listening bars and waits.
- Without your music, `assets/sounds/celebrate.wav` plays: an original placeholder loop made by
  `assets/sounds/celebrate.py`.
- The gate plays at most the first 25 seconds of the clip, fading out. The head finds the beat by
  itself, so any song with a steady beat works.
- The clip may start with something spoken (a "yes!", say), then a pause, then the music. The head
  appears right away but only starts headbanging when the music comes in, after the longest pause
  in the first 15 seconds, building up over 3 seconds.
- On every beat the head hits the bottom of a forward nod exactly on the beat, rebounds, and drops
  into the next one; the hair follows a moment late and the lasers flare.

Check a clip before installing it: the `beat` example prints what the gate finds, and `preview`
renders the celebration timed to it. With `ffmpeg` you can put the two together and watch it:

```sh
cargo run -p gate --release --example beat -- assets/sounds/local/celebrate.wav
cargo run -p gate --release --example preview -- /tmp/gate assets/sounds/local/celebrate.wav
ffmpeg -framerate 25 -i /tmp/gate/celebration/frame-%04d.png -i assets/sounds/local/celebrate.wav \
  -t 25 -af afade=t=out:st=23:d=2 -c:v libx264 -pix_fmt yuv420p -shortest /tmp/gate.mp4
```

## 4. Install

```sh
/mnt/zombori/system/dev-install.sh
```

On top of the splash and boot sound, this builds and installs `zombori-gate`, downloads the speech
model once (checked against its SHA-256) to `/usr/local/share/zombori/`, installs your clips there,
and enables `zombori-gate.service`.

Try it without rebooting, from the QEMU window (it takes over the screen):

```sh
sudo systemctl start zombori-gate
```

Then for real: `sudo reboot`. What the gate heard each time is in the log:

```sh
journalctl -b -u zombori-gate
```

## If it won't let you in

The gate only holds back the login prompt **on the screen** (tty1). These always work:

- **SSH:** `ssh -p 2223 <user>@localhost`, then `sudo systemctl stop zombori-gate`.
- **The serial console:** the terminal running `vm/target.sh`.
- To turn the gate off: `sudo systemctl disable zombori-gate`.

If the gate can't run at all (no screen, microphone or model), it logs why and exits, and the login
prompt appears as usual.

## Testing on the Mac

```sh
cargo test -p gate -p audio                    # answer check, speech detection, beat, scene
cargo run -p gate --example preview -- /tmp/gate-frames [music.wav]   # every stage as PNG frames
cargo run -p gate --release --example beat -- music.wav                # tempo and music start
cargo run -p gate --release --example transcribe -- ggml-base.bin answer.wav   # needs 16 kHz WAV
```

For `transcribe`, download the model from the URL in `system/dev-install.sh` and record or
synthesize an answer, e.g. `say -v Lesya -o a.aiff "музика"` then
`afconvert -f WAVE -d LEI16@16000 -c 1 a.aiff a.wav`.

## Limits

- Speech recognition gives text, not stress: «мУзика» and «музИка» look the same, so the stress
  is not checked.
- This is a ritual, not security. Anyone who hears the answer can say it, and a login challenge
  never protects the disk: whoever has the disk can mount it elsewhere. Keep your password.
