#!/bin/sh
# Ground-truth pass-rate snapshot across all 15 CS4470 grader assignments.
# Read-only: just invokes the existing grader, never touches compiler source.
set -u
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
COMPILER_DIR=$(CDPATH= cd -- "$here/.." && pwd)
GRADER_DIR=$(CDPATH= cd -- "$COMPILER_DIR/../grader" && pwd)

for hw in $(seq 1 15); do
  out=$(cd "$GRADER_DIR" && python3 grader.py test --hw "$hw" --dir "$COMPILER_DIR" --part all 2>&1)
  # grader.py prints one or more "Passed X/Y = Z%" lines (one per part)
  lines=$(printf '%s\n' "$out" | grep -E "^Passed [0-9]+/[0-9]+" )
  if [ -z "$lines" ]; then
    err=$(printf '%s\n' "$out" | tail -3 | tr '\n' ' ')
    printf 'hw%-3s ERROR: %s\n' "$hw" "$err"
  else
    printf 'hw%-3s %s\n' "$hw" "$(printf '%s' "$lines" | tr '\n' ' | ')"
  fi
done
