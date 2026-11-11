#!/usr/bin/env bash
# Manage snapshots of the target disk. The VM must be shut down.
#   vm/snapshot.sh save <name>      vm/snapshot.sh restore <name>
#   vm/snapshot.sh list             vm/snapshot.sh delete <name>
source "$(dirname "$0")/common.sh"

case "${1:-list}" in
  save)    qemu-img snapshot -c "${2:?name required}" "$TARGET_DISK" ;;
  restore) qemu-img snapshot -a "${2:?name required}" "$TARGET_DISK" ;;
  delete)  qemu-img snapshot -d "${2:?name required}" "$TARGET_DISK" ;;
  list)    qemu-img snapshot -l "$TARGET_DISK" ;;
  *) echo "usage: $0 save|restore|delete|list [name]" >&2; exit 1 ;;
esac
