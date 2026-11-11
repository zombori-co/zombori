#!/usr/bin/env bash
# Stream the Mac's microphone into the target VM, for the gate. QEMU's macOS audio backend can
# play sound but not record it, so the microphone travels over the network instead: ffmpeg
# records here and sends raw 16 kHz mono audio over UDP to port 4713, which vm/target.sh forwards
# into the VM, where system/vm-mic/ feeds it into a virtual sound card the gate records from.
#
# Leave it running in its own terminal while you use the VM; it keeps working across VM
# reboots. Ctrl-C stops it. The first time, macOS asks to allow microphone access for this
# terminal app.
#   vm/mic.sh                         use the built-in microphone
#   MIC_DEVICE="iPhone" vm/mic.sh     use the first input whose name contains "iPhone"
set -euo pipefail

PORT="${MIC_PORT:-4713}"
command -v ffmpeg >/dev/null || { echo "ffmpeg is needed: brew install ffmpeg" >&2; exit 1; }

# Pick the input by name, not number: virtual devices (Teams, Zoom...) often come first.
# (Listing devices always ends with an ffmpeg error, hence the `|| true`.)
inputs="$( { ffmpeg -hide_banner -f avfoundation -list_devices true -i "" 2>&1 || true; } |
  sed -n '/audio devices/,$p' | sed -nE 's/.*\] \[[0-9]+\] (.*)$/\1/p')"
want="${MIC_DEVICE:-}"
if [[ -n "$want" ]]; then
  device="$(grep -iF -- "$want" <<<"$inputs" | head -1 || true)"
else
  device="$(grep -iE 'built-in|macbook' <<<"$inputs" | grep -i microphone | head -1 || true)"
fi
[[ -n "$device" ]] || { echo "No matching microphone. Inputs:" >&2; echo "$inputs" >&2; exit 1; }

echo "Streaming \"$device\" to the VM on port $PORT. Ctrl-C to stop."
# UDP rather than TCP: there is no connection to go stale when the VM reboots; packets simply
# arrive whenever the VM is listening. 640-byte packets are 20 ms of audio each.
while true; do
  ffmpeg -hide_banner -loglevel error -f avfoundation -i ":$device" \
    -ac 1 -ar 16000 -f s16le "udp://127.0.0.1:$PORT?pkt_size=640" 2>/dev/null || true
  sleep 1 # ffmpeg stopped (VM not running, device busy): start again
done
