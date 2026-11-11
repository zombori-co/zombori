#!/usr/bin/env bash
# Boot the target VM: the system you are building, booted the way real hardware would boot it.
#   - display: a window (virtio-gpu, so Linux gets a real DRM/KMS device for the splash)
#   - audio:   Intel HDA, output only, played through macOS CoreAudio (for the boot sound)
#   - mic:     QEMU's CoreAudio backend can't record, so the gate's microphone comes over the
#              network: run vm/mic.sh on the Mac (UDP port 4713 is forwarded; see system/vm-mic/)
#   - serial:  this terminal also shows the kernel console. Quit QEMU with: Ctrl-A then X.
#   - ssh:     ssh -p 2223 <user>@localhost
#   - share:   this repo is exported over 9p with mount tag "zombori"
source "$(dirname "$0")/common.sh"

[[ -f "$TARGET_DISK" ]] || { echo "Missing $TARGET_DISK. Install it with vm/builder.sh first." >&2; exit 1; }

REPO_DIR="$(cd "$VM_DIR/.." && pwd)"

qemu_base_args "$(ensure_vars target)"
exec qemu-system-aarch64 "${BASE_ARGS[@]}" \
  -drive "if=none,id=root,format=qcow2,file=$TARGET_DISK" \
  -device virtio-blk-pci,drive=root,bootindex=0 \
  -device virtio-gpu-pci \
  -display cocoa,show-cursor=on \
  -device qemu-xhci -device usb-kbd -device usb-tablet \
  -audiodev coreaudio,id=snd0 \
  -device intel-hda -device hda-output,audiodev=snd0 \
  -nic user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:2223-:22,hostfwd=udp:127.0.0.1:4713-:4713 \
  -virtfs "local,path=$REPO_DIR,mount_tag=zombori,security_model=mapped-xattr" \
  -serial mon:stdio
