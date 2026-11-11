#!/usr/bin/env bash
# Run INSIDE the target VM: build Zombori's components from the shared repo and install them.
#   /mnt/zombori/system/dev-install.sh
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
# Building on the 9p share is slow, so build output stays on the VM's own disk.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/.cache/zombori-target}"

cd "$REPO"
cargo build --release

sudo install -m 755 "$CARGO_TARGET_DIR/release/splash" /usr/local/bin/zombori-splash
sudo install -m 644 system/splash/zombori-splash.service /etc/systemd/system/
sudo install -m 755 "$CARGO_TARGET_DIR/release/bootsound" /usr/local/bin/zombori-bootsound
sudo install -m 644 system/bootsound/zombori-bootsound.service /etc/systemd/system/

sudo systemctl daemon-reload
sudo systemctl enable zombori-splash.service zombori-bootsound.service

echo "Installed. Reboot to see and hear it, or run: sudo systemctl start zombori-splash zombori-bootsound"
