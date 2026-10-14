#!/usr/bin/env bash
# Build the engine's disk templates into DEST, as the smolvm release does
# (scripts/build-dist.sh): a 512 MiB ext4 grown sparse to 20 GiB (storage) and
# 10 GiB (overlay), zstd-compressed. The engine finds them beside the SDK's
# agent-rootfs.tar and expands them on first use.
#
# Why the SDKs need them: without a storage template, an image machine's disk is
# a copy of its image seed. Where the filesystem has no reflink (ext4, the usual
# Linux case) that copies the whole pulled image into every new machine, and the
# guest's first flush waits for all of it to reach the disk (15-30 s per
# create). With the template, each machine is a qcow2 overlay on the seed.
#
# Linux only: macOS clones the seed with APFS, which is instant. Needs mkfs.ext4
# and zstd; without them it warns and builds nothing (the SDK still works, just
# without the fast path). SMOLVM_TEMPLATES_DIR copies prebuilt templates instead.
#
# Usage: build-disk-templates.sh DEST
set -euo pipefail
DEST="${1:?usage: build-disk-templates.sh DEST}"
[ "$(uname -s)" = Linux ] || { echo "disk templates: not needed on $(uname -s), skipping"; exit 0; }
mkdir -p "$DEST"

if [ -n "${SMOLVM_TEMPLATES_DIR:-}" ]; then
  for name in storage-template.ext4.zst overlay-template.ext4.zst; do
    cp -f "$SMOLVM_TEMPLATES_DIR/$name" "$DEST/$name"
  done
  echo "disk templates: copied from $SMOLVM_TEMPLATES_DIR"
  exit 0
fi

MKFS="$(command -v mkfs.ext4 || true)"
[ -n "$MKFS" ] || for p in /sbin/mkfs.ext4 /usr/sbin/mkfs.ext4; do [ -x "$p" ] && MKFS="$p" && break; done
if [ -z "$MKFS" ] || ! command -v zstd >/dev/null 2>&1; then
  echo "disk templates: WARNING — mkfs.ext4 or zstd not found; shipping none (image machines copy their seed)" >&2
  exit 0
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
build() { # name label virtual_bytes
  local img="$work/$1"
  truncate -s $((512 * 1024 * 1024)) "$img"
  "$MKFS" -F -q -m 0 -L "$2" "$img"
  truncate -s "$3" "$img"
  zstd -19 -T0 -q -f "$img" -o "$DEST/$1.zst"
  echo "disk templates: $1.zst ($(du -h "$DEST/$1.zst" | cut -f1))"
}
build storage-template.ext4 smolvm $((20 * 1024 * 1024 * 1024))
build overlay-template.ext4 smolvm-overlay $((10 * 1024 * 1024 * 1024))
