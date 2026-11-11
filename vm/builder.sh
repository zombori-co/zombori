#!/usr/bin/env bash
# Boot the builder VM: a stock Debian 13 used as a workshop for building the target system.
# The target disk is attached as a second drive (/dev/vdb inside the builder).
# Console is this terminal (serial). Log in as root (no password). Quit with: Ctrl-A then X.
source "$(dirname "$0")/common.sh"

[[ -f "$BASE_IMAGE" ]] || { echo "Missing $BASE_IMAGE. Run vm/fetch-base.sh first." >&2; exit 1; }

# Copy-on-write overlay so the downloaded image stays pristine.
if [[ ! -f "$BUILDER_DISK" ]]; then
  qemu-img create -q -f qcow2 -F qcow2 -b "$BASE_IMAGE" "$BUILDER_DISK" 10G
fi
[[ -f "$TARGET_DISK" ]] || qemu-img create -q -f qcow2 "$TARGET_DISK" "$TARGET_SIZE"

qemu_base_args "$(ensure_vars builder)"
exec qemu-system-aarch64 "${BASE_ARGS[@]}" \
  -nographic \
  -drive "if=none,id=root,format=qcow2,file=$BUILDER_DISK" \
  -device virtio-blk-pci,drive=root,bootindex=0 \
  -drive "if=virtio,format=qcow2,file=$TARGET_DISK" \
  -nic user,model=virtio-net-pci
