#!/usr/bin/env bash
# Download the official Debian 13 (trixie) ARM64 image used for the builder VM, and verify it.
source "$(dirname "$0")/common.sh"

URL=https://cloud.debian.org/images/cloud/trixie/latest
NAME="$(basename "$BASE_IMAGE")"

mkdir -p "$IMAGES"
cd "$IMAGES"
curl -fsSLO "$URL/SHA512SUMS"
[[ -f "$NAME" ]] || curl -fL -o "$NAME" "$URL/$NAME"
grep " $NAME\$" SHA512SUMS | shasum -a 512 -c
