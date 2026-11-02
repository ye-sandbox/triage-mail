#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
fixture=apps/desktop/src/routes/__performance
if [[ -e "$fixture" ]]; then
  echo 'Refusing to overwrite an existing performance route' >&2
  exit 1
fi
cleanup() { rm -f "$fixture/+page.svelte" "$fixture/performance.fixtures.ts" "$fixture/visual-themes.fixtures.ts"; rmdir "$fixture"; }
mkdir "$fixture"
trap cleanup EXIT
cp tools/performance/page.svelte "$fixture/+page.svelte"
cp tools/performance/performance.fixtures.ts "$fixture/performance.fixtures.ts"
node tools/performance/prepare-visuals.mjs
pnpm --filter desktop check
pnpm --filter desktop build
mkdir -p target/performance-ui
cp -a apps/desktop/build/. target/performance-ui/
