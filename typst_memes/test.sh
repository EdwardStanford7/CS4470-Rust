#!/bin/sh
set -eu
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
tmp=${TMPDIR:-/tmp}/typst-jpl-tests-$$
trap 'rm -f "$tmp.wat" "$tmp.wasm" "$tmp-smoke.pdf"' EXIT HUP INT TERM

typst compile "$here/smoke.typ" "$tmp-smoke.pdf"
for source in sample.jpl sample-wat.jpl sample-image.jpl; do
  typst query --root / --input program="$here/$source" "$here/query.typ" '<wat-output>' --field value --one \
    | jq -r . > "$tmp.wat"
  wasm-tools parse "$tmp.wat" -o "$tmp.wasm"
  wasm-tools validate --features gc "$tmp.wasm"
done
printf '%s\n' 'Typst and WasmGC tests passed.'
