#!/usr/bin/env bash
# Build from trusted local source/toolchain only. No dependency or image downloads.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $# -gt 1 || $(uname -sm) != 'Linux x86_64' ]]; then
  echo 'Usage: scripts/build-cli-image.sh [local-image-tag] (Linux x86_64 only)' >&2
  exit 2
fi
image=${1:-triage-mail-cli:local}
export LIBONNXRUNTIME_NO_PKG_CONFIG="${LIBONNXRUNTIME_NO_PKG_CONFIG:-1}"
cargo build --offline --locked --release --target x86_64-unknown-linux-gnu -p triage-mail-cli
# Cargo metadata honors CARGO_TARGET_DIR and local Cargo configuration.
target_dir=$(cargo metadata --offline --locked --no-deps --format-version 1 |
  node -e 'let s="";process.stdin.on("data",d=>s+=d);process.stdin.on("end",()=>process.stdout.write(JSON.parse(s).target_directory));')
binary=$target_dir/x86_64-unknown-linux-gnu/release/triage-mail-cli
context=$(mktemp -d)
trap 'rm -rf -- "$context"' EXIT
install -Dm755 "$binary" "$context/rootfs/usr/local/bin/triage-mail-cli"
# INVARIANT: ldd is run only on our just-built trusted executable. Copy its exact
# loader/library closure, not a host directory or an unrelated cached app image.
linkage=$(ldd "$binary")
if [[ $linkage == *'not found'* ]]; then
  echo 'CLI runtime library unavailable' >&2
  exit 1
fi
while read -r first second third rest; do
  library=
  if [[ $second == '=>' && $third == /* ]]; then library=$third;
  elif [[ $first == /* ]]; then library=$first; fi
  [[ -n $library ]] || continue
  case $(basename "$library") in
    libc.so.6|libm.so.6|libgcc_s.so.1|ld-linux-x86-64.so.2) ;;
    *) echo 'Unexpected CLI runtime library; review packaging allowlist' >&2; exit 1 ;;
  esac
  install -Dm755 "$(readlink -f "$library")" "$context/rootfs$library"
done <<< "$linkage"
install -Dm644 LICENSE "$context/rootfs/usr/share/licenses/triage-mail/LICENSE"
# Retain distribution copyright/license notices for the copied glibc/GCC runtime.
for package in libc6 libgcc-s1; do
  install -Dm644 "/usr/share/doc/$package/copyright" \
    "$context/rootfs/usr/share/licenses/$package/copyright"
done
cp packaging/cli/Dockerfile packaging/cli/.dockerignore "$context/"
# scratch has no external base; --network none also prevents build-step networking.
docker build --pull=false --network none --tag "$image" "$context"
