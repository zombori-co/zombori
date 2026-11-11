# Shared settings for the VM scripts. Sourced, not executed.
set -euo pipefail

VM_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGES="$VM_DIR/images"
QEMU_SHARE="$(brew --prefix qemu)/share/qemu"

FIRMWARE_CODE="$QEMU_SHARE/edk2-aarch64-code.fd"
FIRMWARE_VARS_TEMPLATE="$QEMU_SHARE/edk2-arm-vars.fd"

BASE_IMAGE="$IMAGES/debian-13-nocloud-arm64.qcow2"
BUILDER_DISK="$IMAGES/builder.qcow2"
TARGET_DISK="$IMAGES/target.qcow2"
TARGET_SIZE="${TARGET_SIZE:-20G}"

MEMORY="${MEMORY:-4G}"
CPUS="${CPUS:-4}"

# Each VM keeps its own UEFI variable store (boot entries etc.).
ensure_vars() {
  local vars="$IMAGES/$1-vars.fd"
  [[ -f "$vars" ]] || cp "$FIRMWARE_VARS_TEMPLATE" "$vars"
  echo "$vars"
}

# The parts every VM shares: an Apple Silicon-accelerated ARM64 machine booting UEFI.
# Fills the BASE_ARGS array.
qemu_base_args() {
  local vars="$1"
  BASE_ARGS=(
    -machine virt -accel hvf -cpu host -smp "$CPUS" -m "$MEMORY"
    -drive "if=pflash,format=raw,readonly=on,file=$FIRMWARE_CODE"
    -drive "if=pflash,format=raw,file=$vars"
  )
}
