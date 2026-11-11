#!/usr/bin/env bash
# Run INSIDE the target VM: build Zombori's components from the shared repo and install them.
#   /mnt/zombori/system/dev-install.sh
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
# Building on the 9p share is slow, so build output stays on the VM's own disk.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/.cache/zombori-target}"
SHARE=/usr/local/share/zombori

# The gate's speech recognizer (Whisper, base model): downloaded once and checked.
MODEL_URL=https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin
MODEL_SHA256=60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe

cd "$REPO"
for tool in cmake clang pkg-config; do
  command -v "$tool" >/dev/null || {
    echo "Missing $tool. Install the build dependencies first (docs/03-gate.md):" >&2
    echo "  sudo apt install -y build-essential cmake clang libclang-dev libasound2-dev pkg-config" >&2
    exit 1
  }
done
cargo build --release

sudo install -m 755 "$CARGO_TARGET_DIR/release/splash" /usr/local/bin/zombori-splash
sudo install -m 644 system/splash/zombori-splash.service /etc/systemd/system/
sudo install -m 755 "$CARGO_TARGET_DIR/release/bootsound" /usr/local/bin/zombori-bootsound
sudo install -m 644 system/bootsound/zombori-bootsound.service /etc/systemd/system/
sudo install -m 755 "$CARGO_TARGET_DIR/release/gate" /usr/local/bin/zombori-gate
sudo install -m 644 system/gate/zombori-gate.service /etc/systemd/system/
sudo install -m 755 "$CARGO_TARGET_DIR/release/tavern" /usr/local/bin/zombori-tavern
sudo install -m 755 system/session/zombori-session /usr/local/bin/zombori-session
# The tavern's rooms: only the first time, so your own changes in /etc/zombori/rooms.toml stay.
if [[ ! -f /etc/zombori/rooms.toml ]]; then
  sudo install -D -m 644 system/session/rooms.toml /etc/zombori/rooms.toml
fi

sudo install -d "$SHARE"
if ! echo "$MODEL_SHA256  $SHARE/ggml-base.bin" | sha256sum -c --status 2>/dev/null; then
  echo "Downloading the speech model (148 MB)..."
  curl -fL --progress-bar -o /tmp/ggml-base.bin "$MODEL_URL"
  echo "$MODEL_SHA256  /tmp/ggml-base.bin" | sha256sum -c --quiet
  sudo install -m 644 /tmp/ggml-base.bin "$SHARE/ggml-base.bin"
  rm /tmp/ggml-base.bin
fi

# The clips: your own from assets/sounds/local/ (git-ignored) when present, otherwise the
# placeholders in the repo. There are no placeholder questions: without one, the gate (question)
# and the tavern (room-question) just listen.
for clip in question celebrate room-question; do
  for src in "assets/sounds/local/$clip.wav" "assets/sounds/$clip.wav"; do
    if [[ -f "$src" ]]; then
      sudo install -m 644 "$src" "$SHARE/$clip.wav"
      echo "Installed $src as $SHARE/$clip.wav"
      break
    fi
  done
done

sudo systemctl daemon-reload
sudo systemctl enable zombori-splash.service zombori-bootsound.service zombori-gate.service

# The session after the gate: greetd logs you in without a password and starts the tavern, which
# opens programs in cage. Set up only once those are installed (docs/04-tavern.md).
# (greetd lives in /usr/sbin, which is not in a normal user's PATH.)
if PATH="$PATH:/usr/sbin" command -v greetd >/dev/null && command -v cage >/dev/null; then
  sed "s/@USER@/$USER/" system/session/greetd.toml | sudo tee /etc/greetd/config.toml >/dev/null
  sudo install -D -m 644 system/session/greetd-zombori.conf \
    /etc/systemd/system/greetd.service.d/zombori.conf
  sudo systemctl daemon-reload
  sudo systemctl disable getty@tty1.service 2>/dev/null || true
  sudo systemctl enable greetd.service
  echo "Session: after the gate, $USER is logged in automatically into the tavern."
else
  echo "Session not set up (no greetd/cage): after the gate comes the usual login prompt."
  echo "  For the tavern: sudo apt install -y greetd cage firefox-esr, then run this again."
fi

echo "Installed. Reboot to see and hear it, or run: sudo systemctl start zombori-gate"
