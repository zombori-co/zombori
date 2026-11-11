#!/usr/bin/env bash
# Run INSIDE the target VM, once: give the VM a microphone fed from the Mac (vm/mic.sh).
# QEMU on macOS can't pass the microphone through, so it arrives over the network and is played
# into the Loopback virtual sound card, which becomes the default recording device.
#   /mnt/zombori/system/vm-mic/install.sh
set -euo pipefail

cd "$(dirname "$0")"
sudo apt install -y socat alsa-utils

# The Loopback card, always loaded, always card 5 so its number never shifts the real card.
echo snd-aloop | sudo tee /etc/modules-load.d/zombori-vm-mic.conf >/dev/null
echo "options snd-aloop index=5" | sudo tee /etc/modprobe.d/zombori-vm-mic.conf >/dev/null
sudo modprobe snd-aloop index=5

sudo install -m 644 asound.conf /etc/asound.conf
sudo install -m 644 zombori-vm-mic.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable zombori-vm-mic.service
sudo systemctl restart zombori-vm-mic.service  # also picks up a changed unit on re-runs

echo "Done. On the Mac, run vm/mic.sh; then test here:"
echo "  arecord -d 3 -f S16_LE -r 16000 /tmp/t.wav && aplay /tmp/t.wav"
