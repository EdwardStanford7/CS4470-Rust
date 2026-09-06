# Cross-plan consistency check (2026-09-05)

Six hw10-15 plan docs were written independently by separate agents that
couldn't see each other's work. This pass cross-checked them against each
other, against the real `origin/main:src/assembly.rs` reference, and
against actual current grader ground truth (using the new
`tools/asm_bisect.py`). One correction was made; one flagged risk was
resolved as verified-correct; no cross-plan register/layout contradictions
were found.

## Corrections made

1. **hw13-plan.md** — the "Gotcha: origin/main uses `imul`, official
   grader expects `shl`" section was factually wrong. It claimed
   origin/main's own reference compiler would fail the `.expected.opt`
   diff because it hardcodes `imul` where fixtures want `shl`. In reality,
   `assembly.rs` has a whole-text regex post-pass
   (`string_replacement_optimization`, ~line 2103) that runs once at the
   end of `generate_assembly` whenever `optimization_level > 0`: it
   rewrites every `imul rax, N` where N is a power of two into
   `shl rax, {log2(N)}`, and deletes `imul rax, 1` outright. Codegen proper
   emits `imul` uniformly everywhere (binops, linear-index calc, bounds
   math) with zero per-site branching; the shift conversion is a single
   uniform textual pass, not scattered special-casing. Verified against
   `grader/hw13/ok3/003.jpl.expected` vs `.expected.opt`
   (`imul rax, 16` → `shl rax, 4`, confirming the pattern). Rewrote the
   section with the correct mechanism and revised implementation guidance:
   implement one post-pass, not N per-callsite branches.

## Flagged risks resolved as correct (no fix needed, updated status only)

2. **hw12-plan.md** — flagged the multi-index (rank ≥ 2) array push order
   as "inferred from a single rank-1 fixture, unverified." Checked directly
   against `grader/hw12/args/017.jpl` (`show array[i:3, j:argnum, k:9]
   [i,j,k]`, genuinely rank-3) via `tools/asm_bisect.py`: passes exactly.
   All of hw12/array (28/28), hw12/index (10/10), hw12/args (25/25)
   currently pass. Whatever order the implementer actually used is
   empirically correct; added an update note to the plan rather than
   guessing why.

## Not contradictions, just noted

- No adjacent-plan disagreements found on shared conventions (INT_REGS/
  FLO_REGS order, `r12`-for-globals-in-jpl_main-context, constant
  emission-order interning, 16-byte alignment padding, `name`/`_name`
  dual labels) — all six plans that touch these describe them identically,
  consistent with them all having read the same `assembly.rs` sections.
- hw14-plan.md already correctly described the `imul`→`shl` regex
  post-pass (it's the -O3 assignment, so it independently rediscovered the
  same mechanism hw13 needs) — hw13-plan.md was the outlier, now fixed to
  match.

## Ground truth is moving fast — don't trust snapshots

The implementation agent is actively closing gaps in real time. Two grader
sweeps of hw12 taken ~10 minutes apart during this check showed several
parts flip from 0% to 100% in between. **Any pass-rate number written into
a plan doc (including this file) should be treated as stale the moment
it's read.** Always re-run `tools/asm_bisect.py <dir> --summary` (fast,
reuses the grader's own normalizer) or the full grader for current truth
before acting on a plan's "this part is broken" claim.

## Current ground truth at time of this check

- hw10: 100% (all 3 parts, 96/96)
- hw11: 100% (all 5 parts)
- hw12: array/index/args all 100%; `if` (part 1) and `fuzzer` (part 6,
  2/20) still failing as of last check — likely still moving
- hw13: part 1 partial (33%) at first snapshot; not re-checked after the
  imul/shl correction above, which may directly fix ok3/ok4 once
  implemented
- hw14: 0/5 (not started — this is the `-O3` tensor-fusion assignment, not
  images, per the earlier hw14-plan finding)
- hw15: part 1 (structs/commands) 50.4% (62/123); parts 2-3 at 0%; part 4
  (fail-fuzzer, should-reject cases) not fully re-run within this check's
  time budget

## Remaining risk areas for the implementer to watch

- hw13's `if`-constant-folding and hw14's toposort-based loop fusion are
  the two most structurally novel pieces left (everything else so far has
  been incremental extensions of the same shadow-stack model) — budget
  more time/bisection there.
- hw15 part 3 (`ok3`, `-O3` on structs) and hw14 both need the tensor/loop
  fusion optimization — worth implementing once, shared, rather than twice.
- No image-runtime-linking work is needed anywhere in hw10-15 per this
  and the earlier hw14 finding — don't build that unless a future
  assignment surfaces a real image test needing actual execution.
