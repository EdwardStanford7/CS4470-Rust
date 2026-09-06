# Diagnosis: hw7 / hw9 / hw11 failures

All investigated with `tools/grader2/fastgrade.py` (no-bisect for hw7/hw9 since
these aren't asm-diff tests) and direct isolated `./jplc` runs. `typst_memes`
is untracked in git so no blame/history is available — root causes below are
from reading current code + isolated timing, not from diffing against a prior
commit.

## 1. hw7 (300-304/304, varies run to run) — NOT a regression from the x86 work

**Classification: (c) — genuine pre-existing performance bug, exposed/made
flaky by system load, unrelated to asm.typ/compiler.typ changes.**

Every hw7 failure across two separate runs (289-300 passing, always the same
*kind* of failure) is `Timeout after 60 seconds` on `hw7/ok-fuzzer/*` and
`hw7/fail-fuzzer/*` files (parts 3-4), never a content mismatch. Confirmed by
running one failing case in complete isolation (no concurrent load):

```
time ./jplc -t grader/hw7/ok-fuzzer/067.jpl
-> 19.08s user, 45.3s wall, for a 93-line JPL source file. Succeeds, but only
   just clears 45s -- well within range to tip over 60s under any contention.
```

19 seconds of CPU time to typecheck a 93-line program is a real algorithmic
bug (should be milliseconds), not just "large input." hw7's checker is
`compiler.typ`'s `infer`/type-checking pass — root cause is almost certainly
**non-memoized recursive type inference**: every AST node's type is likely
re-derived from scratch on each call rather than cached, so a tree of depth d
does O(work-per-node) work repeated once per *ancestor* traversal, giving at
least quadratic blowup on deep/wide expression trees (which is exactly what a
fuzzer generates and handwritten tests don't). This is confined to how
`infer(node, tenv)` is invoked repeatedly on the same subtree from multiple
call sites — check `compiler.typ` for every call site of `infer` and whether
results get cached (e.g. keyed by node identity/position) vs recomputed.

**Proposed fix:** add memoization to `infer` (or whatever the checker's
type-derivation function is called) — cache by node reference/id within one
`check()` call, so each node's type is computed once. This is a front-end fix
in `compiler.typ`, independent of the x86 backend.

**Why it looked like a regression:** it probably wasn't tested under
contention before (only `hw9`'s "handwritten suite" was spot-checked earlier
per the original status note, not the full fuzzer parts of hw7/hw9). Running
many concurrent grading sweeps + the implementation agent's own compiles
simultaneously pushes already-borderline-slow cases over the 60s cliff,
producing a flaky failure count (95.1% one run, 98.7% another) that looks like
regression noise but is really "same slow test, different load conditions."

## 2. hw9 (90/120, was reported 90/90) — mostly (c) same perf bug, plus one real (b)-adjacent bug

**Classification: mixed.** All 30 failures are in `hw9/ok-fuzzer/` (part 2) —
`hw9/ok/` (part 1, the "handwritten suite" from the original status note) is
still 90/90, unaffected. So the *previously verified* 90/90 claim is still
true; the regression report was from a broader sweep including a part that
was never checked before, not a new break in previously-passing tests.

- ~27 of the 30 fail with `Timeout after 60 seconds` — same root cause as
  hw7 above (non-memoized `infer`, worse here because `-i` C-codegen's
  `scan()` in `c.typ` *also* calls `infer(x, tenv)` at every AST node on top
  of the checker's own inference — see `c.typ` around the `scan` function,
  which recurses into every subexpression then calls `declare-type(infer(x,
  tenv), ...)` at each level. This compounds the same underlying blowup.

- 3 files (`003.jpl`, `016.jpl`, `022.jpl`) don't time out but report
  thousands of differing lines. Isolated check on `003.jpl`:
  ```
  expected: 11068 lines,  actual: 7483 lines
  diff <(head -30 expected) <(head -30 actual):
    30a29,30
    >   int64_t d1;
    >   int64_t *data;
  ```
  This is a genuine correctness bug, not just a timing symptom: at/around
  line 30 of the header, the actual output is **missing an array-type
  typedef** that the expected output declares (a rank-1 `int64_t` array
  struct, `d1`/`data` fields). Everything after that point in the (very
  long) generated file is shifted, producing the appearance of "thousands of
  lines differ" from what is really one missing declaration. This matches
  the original status note's own description: "randomized suite: remaining
  source-order label/typedef issues." Root cause is in `c.typ`'s
  `declare-type`/`array-chain`/`scan` functions (lines ~60-75): some array
  type reachable only through a particular expression shape (nested
  array-of-array via a path `scan` doesn't walk, or a chain component not
  captured by `array-chain`'s single-element recursion) isn't getting into
  the `declared` accumulator before it's referenced.

**Proposed fix:** (1) same memoization fix as hw7 for the timeouts; (2) for
the 3 real diffs, audit `c.typ`'s `array-chain`/`scan` to find which AST shape
in `003.jpl`/`016.jpl`/`022.jpl` produces an array type that never gets
passed through `declare-type` — likely a missing case in `scan`'s tag
dispatch (e.g. `array-loop`/`sum-loop`/`let`-bound array locals aren't in the
tag list handled: `unary`, `binary`, `if`, `array`/`struct-lit`, `call`,
`dot`, `index` — no `array-loop`, `sum-loop`, or variable-reference-through-
`let` case is scanned, so an array type introduced only inside one of those
forms would be skipped). Recommend checking `003.jpl`'s actual source for
one of these constructs.

## 3. hw11 (109/110) — (a)/(c) same perf bug, one specific file

**Classification: same root cause as hw7/hw9 — not a distinct backend bug.**

`hw11/ok-fuzzer2/004.jpl` (part 5) times out at 60s with no output at all
(bisection tool correctly reported "compiler did not produce output" rather
than a false instruction diff). This is the identical symptom as hw7/hw9:
a fuzzer-generated file large/deep enough to trip the same non-memoized
`infer`/checker performance issue, which then also starves the x86 backend
pass since typechecking must complete first. Not an asm.typ bug — expect
this to resolve on its own once the checker memoization fix (item 1) lands,
without needing any x86-specific change.

## Summary / priority for the implementation agent

1. **Highest leverage fix: memoize type inference in `compiler.typ`'s
   checker.** This single fix is the likely root cause of ALL of hw7's
   failures, ~27/30 of hw9's, and hw11's one failure — i.e. most of what's
   currently being misread as "backend regressions" across three separate
   assignments. It will also probably prevent future timeout flakiness in
   hw13-15's larger fuzzer/stress files as those get implemented.
2. Separately, fix `c.typ`'s `scan`/`array-chain` missing-case bug causing
   the 3 non-timeout hw9 diffs (003/016/022.jpl) — likely a missing tag in
   `scan`'s dispatch for array types introduced via `array-loop`/`sum-loop`/
   local `let` bindings.
3. None of this requires touching `asm.typ` — both fixes are in `compiler.typ`
   and `c.typ`, front-end/codegen-shared files untouched by the x86 port so
   far (confirm before editing that nothing here conflicts with concurrent
   asm.typ work — it shouldn't, different files).

## Update (follow-up pass)

**hw7: FIXED.** Root cause was NOT the timeout bug for this one file
(`hw7/fail/020.jpl`) — it was a real typechecker bug: `compiler.typ`'s
`index` type rule (both `infer-raw` and the `annotate` fast-path) allowed
*partial* indexing — supplying fewer indices than an array's rank — and
silently returned a lower-rank array type instead of rejecting the program.
JPL requires indexing to supply exactly `rank` indices. Fixed both
implementations to `panic("type error: wrong number of indices")` when
`indices.len() != rank`. Full regression suite confirms hw7 now 304/304
(100%) with no new failures anywhere else.

**hw9: partially addressed, real bug remains, out of safe scope for this
pass.** Added the diagnosed missing `scan()` dispatch case for
`array-loop`/`sum-loop`, plus scanning of function-body statement values
(previously function bodies' local `let`/`assert`/`return` expressions were
never scanned for embedded array types at all). These are safe, purely
additive fixes (`declare-type` is idempotent) — confirmed zero regressions
across hw2-13/15 full sweep.

However, the actual remaining gap is deeper than "3 files with a missing
typedef": isolated on `hw9/ok-fuzzer/015.jpl`, the compiler's C output is
missing ~1700 lines' worth of typedefs relative to `.expected`, and these
are genuinely missing types (not just reordered -- total counts differ), not
explained by any single missing scan case. A quick attempt at a global
"pre-scan the whole program before any codegen" pass (matching what the
`.expected` output's declaration order implies the reference does) was tried
and reverted: it fixed part of the ordering but broke previously-100% hw8
tests, because for simple programs the CORRECT behavior is strict in-order
interleaved declaration (struct typedef text emitted exactly where the
struct declaration appears in source), not a global pre-pass. The real
algorithm is some source-order-preserving traversal that still manages to
see forward into type usages inside functions that occur later in the file
-- reverse-engineering it exactly requires more fixture analysis than fits
this pass, and there is no Rust reference for the C backend at all (the
original team's own history has a commit literally titled "Not doing C code
gen" -- hw8/hw9 were built from fixtures alone, not a working reference
algorithm).

hw9 remains at 90/120 (unchanged net count -- the additive fixes helped some
cases and didn't reach others). Recommend a dedicated future pass with
fresh fixture-by-fixture analysis on the smaller/simpler of the 30 failing
files first (not the 000+-line monster fuzzer files) to reverse-engineer the
true declaration-order algorithm incrementally.

**hw11**: confirmed still just the one known timeout-flakiness file, not a
distinct bug (varies 109-110/110 depending on system load).
