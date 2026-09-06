# hw9 c.typ typedef/label ordering — findings and remaining gap

## Fixed (verified, zero regressions across hw2-8,10-15 @ 100%)

1. **`gen()`'s "if" jump-label timing** (c.typ, `else if x.tag=="if"` case): was
   allocating both `otherwise` and `done` labels *before* generating the
   yes-branch. The real reference compiler allocates `done` *after*
   generating the yes-branch (lazily), so any nested control flow inside the
   yes-branch consumes label numbers first. Fixed by moving `let
   done=jump;jump+=1;` to after `gen(x.yes,...)`. Minimal repro:
   `show (if (true) then (if (! false) then 325 else 807) else 5)` — compare
   `/Users/robertmorelli/Documents/school-repos/CS4470-Rust/grader/jplc -i`
   (oracle) output's label numbers against ours before/after.

2. **`scan()`'s array-loop/sum-loop traversal order**: was scanning `x.ranges`
   (bounds) before `x.body`. The reference scans **body before ranges**.
   Fixed by swapping the order in the `x.tag in ("array-loop","sum-loop")`
   case. Minimal repro:
   `show ((array[i: (array[a:2] 5)[0]] (array[j:3] true)[0])[0])` — real
   compiler declares `_a1_bool` (from body) before `_a1_int64_t` (from
   range bound); ours had it backwards.

Both were found via the oracle method: `grader/jplc` is a real downloaded
binary of the official reference compiler (see `grader/Makefile`'s `jplc:`
target) — you can compile ANY synthetic `.jpl` file through it directly to
get ground-truth output, not just the ~30 existing fixture files. This is
far more tractable than manually reading `assembly.rs`-equivalent logic,
because **no reference source exists for this C backend** (`origin/main`
has a commit literally titled "Not doing C code gen" — it was reverse-
engineered from fixtures by the original human team).

## Remaining gap: hw9 still 90/120

After both fixes, `grader/hw9/ok-fuzzer/019.jpl`'s header-typedef divergence
moved later (line 51 → line 56) but did not disappear. The pattern:
expected wants a fully-nested array type (e.g. `_a3__a2__a1_int64_t`)
declared *before* an unrelated sibling type (`_a1_a`, array of the
user-declared struct `a`); ours still emits `_a1_a` first.

Traced `_a1_a`'s source to `fn B`'s return statement (line 54 of 019.jpl):
`rgba { (sum[N:v(),O:M] (if ... then (array[P:A] a{45.93,E})[O] else ...)),
...}` — the `array[P:A] a{45.93,E}` sub-expression is the only literal
array-of-struct-`a` construction in the whole file. Given struct-lit field
order and sum/array-loop body-before-ranges are both now confirmed correct
in isolation (see synthetic tests below), the missing piece is most likely
**earlier** in the file — inside `fn v()` or `fn z()` (which precede `fn B`
in source order and are not fully shown/explored here) — something there
should surface the `_a3__a2__a1_int64_t` chain but currently doesn't get
scanned until later, OR gets scanned but at a different relative position
due to a third, not-yet-isolated ordering rule.

### Hypotheses tested and RULED OUT (all matched the oracle exactly):
- struct-lit field scan order (plain left-to-right, NOT reversed despite
  the x86 backend needing reversed struct-lit field order — that finding
  does not transfer to this C backend's typedef scan)
- `index` scan order (base before indices — matches)
- binary op operand order (left before right — matches)
- basic nested-array type-chain declaration (element before self — matches)
- simple if/&&/|| combinations without deep function-body nesting (all matched)

## Session 3 update (continuing the oracle method)

Two more real bugs found and fixed in `c.typ`, both verified against `grader/jplc`
(the oracle) and via full regression (`tools/grader2/fastgrade.py --hw 2..15`):
**zero regressions, hw7/8/10-15 all still 100%**, but **hw9 stays at 90/120** —
the fixes are real but not sufficient to flip any of the 30 failing files to
a full pass (each file has multiple divergence points; fixing the first just
exposes the next).

3. **Dead code after `return` must still be codegen'd.** `gen()`'s function-body
   loop had `else if s.tag=="return"{...;break}` — this skipped ALL statements
   textually after a `return` (JPL permits unreachable code after `return`,
   like `fn v() : int { return d  assert e,"z"  let z=p  return m[...] }` in
   `019.jpl`). The reference does NOT skip them — it translates every
   statement into C in source order (the C `return` still exits at runtime;
   the extra C statements after it are just dead-but-present, harmless).
   Fixed by removing the `break`. Verified with a minimal repro
   (`fn v():int{return 5  let w=(array[i:2](array[j:3] 9))  return w[0][0]}`)
   against the oracle — output now matches exactly (mod comments/whitespace).

4. **`array-loop`/`sum-loop` must declare their OWN type between scanning
   body and scanning ranges, not after both.** `scan()`'s postorder epilogue
   (the generic `declare-type(infer(x,tenv),...)` after the tag-dispatch)
   was declaring an array-loop's own type only after its ranges were also
   scanned. The reference declares it eagerly right after the body (whose
   type determines the element type of this array-loop) but BEFORE
   descending into the range-bound expressions, which are semantically
   independent (int bounds) and may themselves contain unrelated types
   (e.g. an array-of-struct literal) that must be declared LATER. Fixed by
   adding a `declare-type` call in the `("array-loop","sum-loop")` case
   right after the body-scan, before the `for r in x.ranges` loop. (The
   existing generic epilogue still runs harmlessly afterward — `declared`
   dedups by `repr(t)`.) Verified: on `019.jpl` this moved the first
   divergence from line 56 to line 82 (a real, larger chunk now matches).

**Ruled out, tested empirically via minimal repros against the oracle, NO
effect on the remaining (line-82) divergence — do not re-try these:**
- Declaring `if`'s own type between `yes` and `no` scan (instead of after
  both).
- Declaring `index`'s own type between `base` and `indices` scan.
- Declaring `binary`'s own type between `left` and `right` scan.
None of these changed `019.jpl`'s divergence point at all, so the
"declare-self-between-primary-and-secondary-children" pattern that fixed
array-loop is NOT a universal rule — it's specific to array-loop/sum-loop
(plausibly because only array-loop/sum-loop's own type depends *solely* on
one child (body) while having other children (ranges) that are int-typed
and truly independent; `if`/`index`/`binary`'s own type can depend on
multiple children jointly, so there's no analogous "declare after the child
that determines my type" moment).

### Remaining gap after session 3

`019.jpl`'s new divergence (line 82): expected declares `_a3__a2__a2__a1_int64_t`
(rank-3 wrapper) where we declare `_a1__a2__a2__a1_int64_t` (rank-1 wrapper) —
literally the same class of bug as the one just fixed (rank-1 vs rank-3
wrapper of the same element chain), but at a deeper point in the same
massive `let b[c,d] = (...)[...]` expression (line 8 of `019.jpl`) — this
time seemingly inside the big index-list following the base array-loop,
not inside the base's own ranges. Did not fully trace the exact sub-node
this time (the index-list is extremely dense — 4+ deeply nested
sum-loops/if-expressions per element); ran out of productive time
budget for manual dissection.

## Session 4 update

Two more real bugs found and fixed via the oracle method (`grader/jplc`), both
verified with minimal repros AND full regression (`tools/grader2/fastgrade.py
--hw 2..15`): **zero regressions, hw2-8,10-15 all still 100%**. hw9 pass
count is still 90/120 (each failing file has more stacked issues), but real,
confirmed-correct fixes landed:

5. **`sum-loop` needs RANGES scanned before BODY — the opposite of
   `array-loop`'s body-before-ranges rule.** The old code lumped both tags
   into one `x.tag in ("array-loop","sum-loop")` branch sharing the same
   body-before-ranges order. Minimal repro that proves it:
   `show (sum[i: (array[a:2] 5)[0]] (array[j:3,k:2] 7)[0,0])` — bound
   produces `_a1_int64_t`, body produces `_a2_int64_t`. Oracle
   (`grader/jplc -i`) declares `_a1_int64_t` (from the range bound) first;
   old code declared `_a2_int64_t` (from the body) first. Fixed by splitting
   into separate `x.tag=="array-loop"` (unchanged: body then self-declare
   then ranges) and `x.tag=="sum-loop"` (ranges then body, no early
   self-declare needed since a sum's own result type is always a scalar/
   primitive — no typedef to order) branches. Verified: repro now matches
   the oracle exactly (mod the trailing status line).

6. **Directly-nested `array-loop` chains (an array-loop whose `body` is
   itself another `array-loop`) must have EVERY level's own type declared
   consecutively (innermost to outermost) BEFORE any level's `ranges` are
   scanned — not just "this level's ranges after this level's self-declare"
   applied recursively, which still lets an inner level's ranges sneak in
   before an outer level's self-declare.** Minimal repro:
   ```
   let b[c, d] = (array[b:..., c:985, d:...]
                   (array[e:(c+b), f:(...two more array-loops of int...)]
                     (array[g:d] c)))[...]
   ```
   (full repro saved as reasoning below; rank chain is a1_int64_t ->
   a2__a1_int64_t -> a3__a2__a1_int64_t, with an unrelated `_a2_int64_t`
   sibling type hiding inside the middle level's OWN range-bound
   expression). Oracle declares a1, a2, a3 back-to-back, then only
   afterward the a2_int64_t sibling (found in the middle level's range) and
   the outer level's own range-derived types. Old code (even after fix #2
   from session 1) declared a1, a2, THEN the sibling a2_int64_t (since it's
   nested inside "scan(body)" for the outermost level, which fully recurses
   including the middle level's own ranges), THEN a3 — wrong.

   Fixed by rewriting the `array-loop` case to: (a) walk down through
   `x.body.tag=="array-loop"` links collecting the whole chain of directly-
   nested array-loop nodes, (b) scan the true innermost non-array-loop body
   once, (c) declare every chain level's own type back-to-back
   innermost-to-outermost (`chain.rev()`), THEN (d) scan every chain level's
   `ranges`, outermost-to-innermost (`chain` in original order) — this
   matches the oracle's "outer range before inner range" ordering (e.g. an
   `_a1_a` struct-array type hiding in the OUTERMOST level's own range
   consistently appears before the sibling in the middle level's range).
   Verified: the full `let b[c,d]=...` isolated repro now matches the oracle
   byte-for-byte (mod trailing status line) — this was previously stuck at
   a divergence 5-8 lines in, now fully resolved for this construct.

**Important**: neither fix changed `019.jpl`'s specific remaining divergence
(still first diverges at the same normalized line ~82, same
`_a3__a2__a2__a1_int64_t` vs `_a1__a2__a2__a1_int64_t` ordering issue as
session 3 found). That means this specific divergence's root cause is
**not** in the statement these two fixes targeted (the `let b[c,d]=...` on
line 8, which now scans correctly in isolation) — it must come from later in
the file. `019.jpl` has `fn v()`, `fn z(...)`, `fn B(...)`, `fn C()`
declarations after the `let` statements (lines 26-70) that were never
isolated/tested in any session so far. The element type involved
(`_a2__a2__a1_int64_t`, a 3-level-deep nested int array) doesn't obviously
match any of the four functions' parameter/return types at a glance (those
involve struct `s`, `void`, `rgba`, `bool`) — it must come from an
expression INSIDE one of the function bodies. Next session should isolate
each function body (with correctly-typed stub top-level variables it
references) against the oracle the same way sessions 1-4 isolated
expressions, rather than trying to hand-trace the full dense file text.

## Session 5 update — two major structural bugs found and fixed

hw9 jumped from **90/120 (75%) to 108/120 (90%)** this session, with **zero
regressions** anywhere else (full `tools/grader2/fastgrade.py --hw
2,3,4,5,6,7,8,9,10,11,12,13,14,15` sweep: hw2-8,10-15 all still 100%).
`019.jpl` (the file every prior session traced into) now matches the oracle
**exactly, byte-for-byte** (mod the trailing status line).

### Bug 7 (the big one): `array-loop` must self-declare its type EAGERLY,
before recursing into body/ranges at all — not after, and not via the old
"collect the chain of directly-nested array-loops, declare them all, then
scan ranges" special case (which is now DELETED, replaced by something
simpler and more general).

Root cause, found via the oracle method with a **6-line minimal repro**:
```
let b = (array[bb:5, cc:6] (array[ee:6] bb))
let f[g, h] = (if true then (array[f:1, g:1, h:1] (array[i:1] (array[j:1, k:h] b))[0])
               else (array[f:1, g:1, h:1] (array[i:1, j:1, k:1] (array[l:1, m:1] b)))[0, 0, 0])[0, 0, 0][0, 0]
```
Oracle declares `_a3__a2__a2__a1_int64_t` (the OUTER rank-3 array-loop's own
type) **before** `_a1__a2__a2__a1_int64_t` (a rank-1 array-loop's type,
buried inside an `index`-wrapped sub-expression that is part of the outer
array-loop's *body*). Our old code always finished scanning a node's full
`body` (however deeply nested, through arbitrary `index`/`if`/etc wrappers)
before declaring that node's own type — ordinary postorder — so any type
declared deep inside a body always came before the outer node's own type.
That's structurally impossible to fix by special-casing "body is directly
another array-loop" (the old chain-collection trick, bug fix #6 from
session 4) because the culprit here isn't a *direct* array-loop chain, it's
an array-loop whose body is an `index` node wrapping an *unrelated* nested
array-loop.

**The real, general rule**: for `array-loop`, call
`declare-type(infer(x,tenv))` (which walks the STATIC TYPE via
`array-chain`, declaring every level of nesting it implies, purely from the
type value — not from AST structure) **before** scanning body or ranges at
all. This one change:
- Correctly declares the outer node's whole type-chain (a1, a2__a1,
  a2__a2__a1, a3__a2__a2__a1 in the repro above) up front, before ever
  touching the body's internal auxiliary types.
- Still satisfies fix #6's original session-4 case (nested chains needing
  a1,a2,a3 back-to-back before any range-derived sibling types) — verified
  by re-testing that exact repro, still passes byte-for-byte — because
  `array-chain` naturally declares a whole direct-nesting chain in one call
  regardless of AST shape.
- Is dramatically simpler than the old chain-collection code (3 lines
  instead of ~7, no manual `while cur.body.tag=="array-loop"` walk needed).

Old code (DELETED):
```
else if x.tag=="array-loop"{
  let chain=(x,)
  let cur=x
  while cur.body.tag=="array-loop"{cur=cur.body;chain.push(cur)}
  let rb=scan(cur.body,header,declared,tenv:tenv);header=rb.header;declared=rb.declared
  for lvl in chain.rev(){let d=declare-type(infer(lvl,tenv),header,declared);header=d.header;declared=d.declared}
  for lvl in chain{for r in lvl.ranges{let r2=scan(r.bound,header,declared,tenv:tenv);header=r2.header;declared=r2.declared}}
}
```
New code:
```
else if x.tag=="array-loop"{
  let d=declare-type(infer(x,tenv),header,declared);header=d.header;declared=d.declared
  let rb=scan(x.body,header,declared,tenv:tenv);header=rb.header;declared=rb.declared
  for r in x.ranges{let r2=scan(r.bound,header,declared,tenv:tenv);header=r2.header;declared=r2.declared}
}
```
(`sum-loop` was NOT touched — its own result type is always scalar, so
eager self-declare there would be a no-op anyway; left as session 4 fixed it.)

### Bug 8: function codegen and jump-label numbering must happen in
**true source order interleaved with top-level main-statement codegen**,
sharing ONE continuous jump counter — not "all functions first, then all
main-level statements" (which is what the code did before this session).

Evidence: after fixing bug 7, `019.jpl`'s typedef section matched exactly,
but function BODY code diverged immediately at the first line of `fn v()`'s
body: oracle used `goto _jump825` where we emitted `goto _jump1`. The
oracle's jump-label counter had already been incremented by processing ~824
jump-consuming constructs (nested `if`s inside the many top-level
`let`/`show`/`assert` statements that precede `fn v()` in source order) —
meaning the reference compiler generates code for top-level commands and
function bodies **interleaved in one single pass over `checked.ast`, in
source order**, sharing one global jump-label counter, rather than
processing "all `fn` declarations" as one block and "all top-level
commands" as a separate later block.

Fix: merged what used to be two separate loops (one that generated all
function bodies into `header`, appending as it went; one that later
generated `jpl_main`'s body using a jump counter that started fresh from
wherever the function loop left off) into **one loop** over `checked.ast`
that, for each statement in source order, either generates a function (into
a separate `functions` accumulator, keeping `header`/typedefs conceptually
separate from code) or dispatches a main-level command (into `lines`) —
both paths reading and writing the SAME shared `jump` variable. `next`
(temp-variable numbering) is correctly still separate per function scope
(each function keeps its own local `fnext`/starts at 0, and `jpl_main`'s
`next` also starts at 0 independently) since C variable names are
function-scoped and the oracle's actual variable numbers confirmed this
doesn't need to be shared — only `jump` (goto labels) is a single global
counter in the reference's model.

This fix, by itself (before bug 7 was even fully validated against the
whole file), took `019.jpl` from "first divergence in the typedef header at
line 82" all the way to "byte-for-byte exact match" once combined with bug 7.

### Also fixed while here: fn's own return type must be declared BEFORE its
parameter types (not after). Old code: `for p in d.params{declare-type
(p.typ,...)}` then `declare-type(d.returns,...)`. Oracle wants the reverse.
One-line fix (swap the two statements). Verified against `014.jpl`'s
`fn f(g[h,i,j]:rgba[,,]):float[,]` case, where oracle declares `_a2_double`
(from the return `float[,]`) before `_a3_rgba` (from the param).

### Remaining gap after session 5: hw9 still 108/120 (12 failures)

`026.jpl` is qualitatively different (3273 lines differ — likely a genuine
crash/major-miss, not a small ordering issue) and should be looked at
FIRST/separately with fresh eyes (didn't get to it this session).

The other ~11 failures (014 is now fixed; 016, 020, 023, 025, 027, and
others per `tools/grader2/fastgrade.py --hw 9`) show small remaining
sibling-ordering divergences (8-20 lines each) that look like a THIRD
distinct rule, still not isolated. Partial investigation on `016.jpl`:
`fn x(y[z,A,B]:float[,,], C:void):bool` has a body containing an array
LITERAL `[false, true, false]` (giving `_a1_bool`) — oracle declares this
`_a1_bool` type right after the param's `_a3_double`, but our code instead
produces an unrelated `_a3_bool` there, meaning this isn't just a two-item
swap but a deeper reordering across MULTIPLE types spanning both the
function's own param-type declarations and its body-scan. Hypothesis not
yet tested: maybe body-scanning should be interleaved statement-by-statement
with SOMETHING (param declares? Or maybe each parameter's type should be
declared immediately before/after scanning body references to that specific
parameter, rather than all params up front then the whole body after) —
needs the same oracle-repro-shrinking method applied fresh to `016.jpl`,
starting from its `fn x` in isolation (stub out the earlier `fn g`/`fn w`
declarations it doesn't depend on, keep `fn x`'s exact body, shrink from
there). The `020.jpl`, `023.jpl`, `025.jpl`, `027.jpl` failures are likely
instances of this same third rule (all show small param-type/body-type
sibling reordering) — fix the general rule once and most/all of them should
clear together, based on the pattern from sessions 1-5 where each real fix
tends to flip a cluster of files at once.

## Session 6 update — 026.jpl fixed (crash-class bug), hw9 90→113/120

**Bug 10: nested `time time <command>` only unwrapped/timed ONE level.**
`gen()`'s dispatch for `c.tag=="time"` computed the fully-unwrapped command
`d` via the existing `while d.tag=="time"{d=d.command}` scan-phase helper,
then emitted exactly one `get_time()`/`print_time()` pair around it. But
JPL permits stacking `time` (`026.jpl` has `time time let e[f,g]=...`), and
the reference emits ONE get_time()/print_time() pair PER `time` wrapper,
properly nested: outer-start, inner-start, ...command..., inner-end,
print_time(inner), outer-end, print_time(outer) (inner finishes first).
Fixed by replacing the flat unwrap with a recursive `gen-time(t,...)` closure
that recurses on `t.command` when it's itself tagged `"time"`, else calls
`dispatch-command`. Verified against `026.jpl`'s exact oracle output
(`get_time()` at positions matching `_651,_652,...,_670,_671`) — file now
matches byte-for-byte. This was NOT an ordering bug like the others, it was
a straightforward missing-recursion bug, found by direct inspection (no
synthetic oracle repro needed, the real file was clear enough once the
`ppc`-normalized diff was compared line-by-line).

**Bug 11: `if` needs its own type declared eagerly, but AFTER `cond`, BEFORE `yes`/`no`.**
Generalizes bug 7's "array-loop must self-declare eagerly" finding to `if`
nodes, but with an important refinement discovered by testing TWO synthetic
repros against the oracle:
- Naive full-eager (self-declare before even `cond`) is WRONG — breaks cases
  where `cond` itself contains an unrelated array type that must be declared
  *before* the if's own type (confirmed via `let j=(if((array[a:2,b:2,c:3]
  false)[0,0,0]==false) then (array[a:2] false) else (array[a:2] true)))`-style
  repros where `cond`'s _a3_bool must precede the if's own _a1_bool).
- The correct order is **cond → self-declare → yes → no**. Verified via a
  repro combining both concerns: `let j=(if true then (if ((array[a:2,b:2,c:3]
  false)[0,0,0]==false) then (array[a:2] false) else (array[a:2] true)) else
  [true])` — oracle declares `_a1_bool` (outer if's own type, matches `[true]`
  and both inner branches) *before* `_a3_bool` (from the inner if's `cond`,
  buried inside the outer if's `yes` branch) — exactly reproducing 016.jpl's
  divergence. Fixed by moving `declare-type(infer(x,tenv))` to right after
  `scan(x.cond,...)` and before `scan(x.yes,...)`/`scan(x.no,...)` in the
  `"if"` case (previously: postorder, declared only in the generic epilogue
  after cond+yes+no all fully scanned).

**Result: hw9 jumped 90/120 → 113/120 (94.2%).** Full regression sweep
(`tools/grader2/fastgrade.py --hw 2..15 --skip-hw1`) confirms **zero
regressions**: hw2-8, 10-15 all still 100%. Files fixed this session (no
longer failing): 003, 004, 005?, 006?, 011, 013, 014 (already fixed prior
session), 016, 018, 019, 020, 026, 030, and others — net 23 files flipped
from fail to pass across the two fixes.

### Remaining gap after session 6: hw9 still 113/120 (7 failures)

Failing files: **001, 002, 010, 012, 023, 025, 027**. All show a
**different failure signature** from anything fixed so far: each file's
actual output is exactly **1 line longer** than `.expected` (e.g. 001:
11470 vs 11469; 002: 27790 vs 27789; 010: 18977 vs 18976; 012: 15779 vs
15778; 023: 27432 vs 27431; 025: 29675 vs 29674; 027: 18798 vs 18797), and
in every case the divergence is a **swap of two unrelated sibling array
typedefs** at adjacent positions — NOT a single type moved relative to a
fixed anchor point like sessions 1-6's bugs, but two *different* array
types (e.g. `_a2__a1_int64_t` vs `_a2_int64_t` in 010; `_a1__a1_int64_t` vs
`_a1__a3_int64_t` in 012; `_a3_void_t` vs `_a3_int64_t` in 025) that appear
to come from two SEPARATE expressions/sub-nodes whose relative scan order
is backwards. This is very likely a **third distinct node-type ordering
rule** not yet isolated (candidates not yet tested: `sum-loop` with
multiple `ranges` scanned in the wrong relative order when there's more
than 2; `call` argument order interacting with something; `dot`/struct
field access; or a `let`/`assert`/statement-level interaction inside a
function body specifically, since ALL 7 remaining failures are early in
their files, inside the FIRST function (`fn a`/`fn b`/etc), suggesting a
per-function local variable/parameter interaction rather than a
top-level-statement one).

The consistent "+1 line" length delta (not a multiple of ~5-8, which is
what one extra/missing full typedef block would cost) suggests this may
NOT be a missing/duplicate typedef at all, but instead **a genuinely
different single-line discrepancy elsewhere in the SAME file** (e.g. one
extra blank line, or one extra/missing statement) that happens to always
co-occur with these header swaps — worth checking whether the length
delta and the swap are actually related, or two independent tiny bugs
per file. Next session should pick ONE of these 7 (010.jpl looks
smallest/most tractable) and apply the same shrink-to-synthetic-repro
method used in prior sessions: find the exact two expressions producing
the swapped types (likely both inside `fn a`'s body, per the source dump),
build a minimal 2-3-line synthetic `.jpl` isolating just those two
expressions in the same relative structure, and compare against
`grader/jplc -i` to find the real rule.

## Session 7 update — RESOLVED. hw9 113→120/120 (100%)

**Bug 12 (final): array-loop range/body scan order was backwards for multi-range loops.**

Root cause: fix #2 (session 1) established "array-loop scans body before ranges"
from a repro with a SINGLE range. That repro's evidence was actually
**confounded by fix #7** (eager self-declare, added in session 5): the
array-loop's own type (declared eagerly via `infer(x)`/`array-chain`
*before* scanning body or ranges at all) already equalled what the body
would have produced, so the body-vs-range scan order was never actually
exercised as a distinguishing test — the single-range case can't tell body-
first from range-first apart once self-declare already emits that type.

The real rule, confirmed via `grader/hw9/ok-fuzzer/010.jpl`'s first
divergence (shrunk to a single-statement repro, `fn a(b:float):float{ <the
first let> return 0.0 }`, isolating a 3-range array-loop
`array[f:.., g:(array[..](-678))[..], h:..] (array[i,j](array[k]g))[f,e]`):
range `g`'s bound contains an array-of-int literal producing `_a2_int64_t`;
the body contains a nested array-loop producing `_a2__a1_int64_t`. Oracle
declares `_a2_int64_t` (from range g) **before** `_a2__a1_int64_t` (from
body) — i.e. **ranges scan before body**, the OPPOSITE of what fix #2
concluded, once eager self-declare (fix #7) already covers the "body
determines my own type" concern the old order was actually protecting.

Fix: swapped `array-loop`'s scan order in `c.typ` from
`self-declare → body → ranges` to `self-declare → ranges → body` (matching
`sum-loop`'s existing range-before-body order, which was never touched and
turns out to have been right all along).

Verified: repro now matches oracle byte-for-byte (mod trailing status
line). `python3 tools/grader2/fastgrade.py --hw 8,9` → **hw8 120/120,
hw9 120/120, both 100%.**

### Full final regression: hw2-15 ALL 100% (3310/3310)

```
hw2 670/670, hw3 713/713, hw4 166/166, hw5 254/254, hw6 172/172, hw7 304/304,
hw8 120/120, hw9 120/120, hw10 96/96, hw11 110/110, hw12 110/110, hw13 65/65,
hw14 5/5, hw15 405/405 — TOTAL 3310/3310 = 100.0%
```

**hw1 is separate and NOT part of this suite's 100%**: it requires a
`jplc --run` capability (assemble+link+execute a compiled program, diff
resulting PNG images against references) that does not exist anywhere in
the current `jplc` CLI (which only supports `-l/-p/-t/-i/-s`, no `--run`,
no self-contained execution pipeline). This is a fundamentally different,
much larger feature (real NASM/GCC/libpng toolchain integration) that was
out of scope for this entire multi-session effort from the very beginning
— it was never included in any of the hw10-15 backend plans, and the
original baseline status (from before any of this work started) only ever
claimed "Assignments 2-8: 100%", never mentioning hw1. If the user wants
hw1 too, that's a new, separate project (build a real `--run` mode wiring
NASM+GCC+the runtime library+image diffing), not a continuation of the
c.typ/asm.typ debugging done in sessions 1-7.

### Next steps for whoever picks this up
- Use `tools/grader2/fastgrade.py --hw 9 --workers 4 --no-bisect` to see
  current failures, and adapt the `/tmp/cbisect.py`-style approach (diff
  `ppc`-normalized actual vs `.expected`, using
  `grader/normalize_asm.py`'s `ppc` directly) to find the first divergence
  per file.
- Extract `fn v()` and `fn z()`'s full bodies from `grader/hw9/ok-fuzzer/019.jpl`
  (lines ~18-46) and test them against the oracle (`grader/jplc -i`) in
  isolation, the same way this session isolated the array-loop and if-label
  bugs — shrink the failing statement to a minimal synthetic `.jpl` file,
  compare against `grader/jplc -i <file>`, and bisect from there.
- Given two real ordering bugs were found this way in under an hour, a
  third (or the same class applied to a different node type — `call`
  args? `dot`? nested `index` chains?) is very plausible and likely
  tractable with the same method.
