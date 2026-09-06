#!/bin/sh
set -u
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
tmp=${TMPDIR:-/tmp}/typst-wasmgc-tests-$$
trap 'rm -f "$tmp.wat" "$tmp.wasm"' EXIT HUP INT TERM

pass=0
fail=0

compile_wasmgc() {
  # $1 = .jpl path -> writes $tmp.wat, returns 0/1
  if out=$(typst query --root / --input program="$1" "$here/query.typ" '<wat-output>' --field value --one 2>"$tmp.err"); then
    printf '%s' "$out" | jq -r . > "$tmp.wat"
    return 0
  else
    return 1
  fi
}

echo "== ok/: compile, validate, execute, diff stdout =="
for jpl in "$here"/tests/wasmgc/ok/*.jpl; do
  name=$(basename "$jpl")
  exp="$jpl.expected"
  if ! compile_wasmgc "$jpl"; then
    echo "FAIL $name: compilation failed"; cat "$tmp.err"; fail=$((fail+1)); continue
  fi
  if ! wasm-tools parse "$tmp.wat" -o "$tmp.wasm" 2>"$tmp.err"; then
    echo "FAIL $name: wat parse failed"; cat "$tmp.err"; fail=$((fail+1)); continue
  fi
  if ! wasm-tools validate --features gc "$tmp.wasm" 2>"$tmp.err"; then
    echo "FAIL $name: gc validation failed"; cat "$tmp.err"; fail=$((fail+1)); continue
  fi
  actual=$(node "$here/tests/wasmgc/run-wasm.mjs" "$tmp.wasm" 2>"$tmp.err")
  if [ $? -ne 0 ]; then
    echo "FAIL $name: execution trapped"; cat "$tmp.err"; fail=$((fail+1)); continue
  fi
  expected=$(cat "$exp")
  if [ "$actual" = "$expected" ]; then
    pass=$((pass+1))
  else
    echo "FAIL $name: output mismatch"
    echo "  expected: $(printf '%s' "$expected" | tr '\n' '|')"
    echo "  actual:   $(printf '%s' "$actual" | tr '\n' '|')"
    fail=$((fail+1))
  fi
done

echo "== structural-only/: compile + validate only =="
for jpl in "$here"/tests/wasmgc/structural-only/*.jpl; do
  name=$(basename "$jpl")
  if ! compile_wasmgc "$jpl"; then
    echo "FAIL $name: compilation failed"; cat "$tmp.err"; fail=$((fail+1)); continue
  fi
  if wasm-tools parse "$tmp.wat" -o "$tmp.wasm" 2>"$tmp.err" && wasm-tools validate --features gc "$tmp.wasm" 2>"$tmp.err"; then
    pass=$((pass+1))
  else
    echo "FAIL $name: parse/validate failed"; cat "$tmp.err"; fail=$((fail+1))
  fi
done

echo "== reject/: must fail compilation cleanly (via jplc -t) =="
for jpl in "$here"/tests/wasmgc/reject/*.jpl; do
  name=$(basename "$jpl")
  out=$("$here/jplc" -t "$jpl" 2>&1)
  last=$(printf '%s' "$out" | tail -1)
  case "$last" in
    "Compilation failed"*) pass=$((pass+1)) ;;
    *) echo "FAIL $name: expected clean rejection, got: $last"; fail=$((fail+1)) ;;
  esac
done

echo "== known-gaps/: expected to currently fail codegen (tracked, not counted) =="
for jpl in "$here"/tests/wasmgc/known-gaps/*.jpl; do
  name=$(basename "$jpl")
  if compile_wasmgc "$jpl"; then
    echo "NOTE $name: now compiles! move it out of known-gaps/ into ok/ with an .expected file"
  else
    echo "gap  $name: still not implemented (expected)"
  fi
done

echo
echo "wasmgc suite: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
