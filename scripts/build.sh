#!/usr/bin/env bash

set -euo pipefail

TARGETS=(
  "x86_64-unknown-linux-gnu"
  "aarch64-unknown-linux-gnu"
)

for target in "${TARGETS[@]}"; do
  rustup target add "${target}" >/dev/null

  echo "==> Building ${target}"
  uv build --wheel -C build-args="--target ${target} --zig --compatibility pypi"
done

echo
echo "Wheels in dist/:"
ls -1 dist/*.whl
