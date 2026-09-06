# hw14 port plan

**Correction to assumed scope:** hw14 is NOT about images. `grader/grader.py` line 498-499
registers hw14 part 1 as `NullPart(OptSpec, "hw14/", "-O3")` — it's the **`-O3` optimization**
assignment. The 5 test files (`col.jpl`, `crs.jpl`, `dns.jpl`, `mat.jpl`, `sft.jpl`) are dense
matrix/tensor-style JPL programs (array/sum comprehensions over 2D arrays) chosen to exercise
specific peephole/loop optimizations, not image I/O. Image read/write (`_read_image`/
`_write_image` externs, `rt/pngstuff.c`) is unrelated plumbing already needed since hw10 and is
out of scope here.

## 1. What OptSpec actually checks (grader.py:217-243)

For each `hw14/*.jpl`, it runs the compiler twice and diffs both, whitespace/comment-normalized
via `normalize_asm.ppasm`:
- `-s` output must exactly match `*.jpl.expected` (the **baseline, unoptimized** assembly —
  i.e. hw10-13 codegen must already be exactly correct for these 5 new programs; no new
  behavior needed here beyond what hw10-13 already implements).
- `-s -O3` output must exactly match `*.jpl.expected.opt` (the optimized assembly).

So hw14 = get baseline codegen right on these programs (should already follow from hw10-13
work) + implement `-O3` to reproduce specific transformations byte-for-byte.

## 2. Exact optimizations in the Rust reference (`git show origin/main:src/assembly.rs`)

All gated behind `self.optimization_level > 0` (i.e. any `-O1`/`-O2`/`-O3`) except tensor
contraction which needs `> 2` (`-O3` only). `AssemblyGenerator::new(optimization_level: u8)`
stores the level; `generate_assembly()` (line 2142) takes it in.

1. **Small-int push shortcut** (`insert_int_constant`, ~2037): if `is_32_bit(number)` (fits in
   i32), emit `push qword N` directly instead of allocating a `constK: dq N` data entry and
   `mov rax, [rel constK] / push rax`. `is_32_bit` = fits in an `i32` (line ~2075:
   `num_traits::ToPrimitive::to_i32(value).is_some()`).

2. **Multiplication strength optimization** (Binop `*`, ~755-780): when the left operand is an
   int literal that is either 32-bit or a power of two and nonzero, generate the right operand
   first; if the right operand is ALSO such a literal (and neither is exactly 1, and left is not
   a power of two), emit `pop rax / pop r10 / imul rax, r10 / push rax` instead of the generic
   path (there's more of this function below the shown excerpt — read lines ~755-850 in full
   before implementing, the snippet cuts off mid-branch).

3. **Boolean-to-int ternary shortcut** (`if`/`then`/`else`, ~1002-1013): if both branches are
   int literals `1` and `0` respectively, skip generating the branch/jump code entirely — the
   already-pushed boolean condition value IS the int result (bool and int share representation
   on the stack), just leave it and return.

4. **Array index optimization** (`ArrayIndex`, ~1035, impl at `optimized_array_index` ~1606):
   when indexing a bare variable (not `from_main` in statement position), use the already-known
   stack `offset` of that variable to compute the linear index directly against `[rsp + ...]`
   slots instead of re-evaluating/copying the whole array value first. This avoids the generic
   "copy array to stack then index" path. Read `calculate_linear_index` (~ line after 1660,
   truncated in excerpt) fully — it also special-cases int-literal bounds that are 32-bit/pow2
   to fold the multiply.

5. **Tensor contraction optimization** (`ArrayLoop` containing a `SumLoop` body, ~1108-1366,
   `-O3` only, requires `tensor_optimizable_expressions(sum_body)` — sum_body must be built only
   from `Binop`, `Int`, `Float`, `Variable`, `ArrayIndex` nodes, no other expression kinds):
   rewrites `array[...] sum[...] <product-of-indexed-arrays>` into a single fused loop nest
   instead of materializing the sum's inner array/temp per outer iteration. Uses
   `build_traversal_graph` + `petgraph::algo::toposort` to order loop variables by data
   dependency (which index vars must be bound before which array reads are legal), then emits
   one combined nested loop allocating the result array once and accumulating into it. This is
   the most involved piece — the 5 hw14 tests are specifically dense matmul/sum-of-products
   patterns (`mat.jpl` is literal matrix multiply, `crs`/`dns`/`sft`/`col` are similar
   contractions) designed to hit exactly this path at `-O3`.

6. **Post-process regex pass** (`string_replacement_optimization`, ~2103-2130, applied once over
   the whole generated string when `optimization_level > 0`):
   - `imul rax, N` where N is a power of two → `shl rax, log2(N)`.
   - `imul rax, 1` → deleted entirely (empty replacement, leaves a blank line normalize_asm
     strips).
   Order matters: this runs on the *entire* generated assembly text as a final pass, after all
   the structural optimizations above.

## 3. Typst-side plan

The concurrent backend-port agent should already have a `generate_expression`-equivalent driven
by the typed AST. Add:

- An `opt_level` parameter threaded through the codegen entry point and down through whatever
  Typst plays the role of `AssemblyGenerator` (likely a dictionary/state object passed
  explicitly, since Typst has no mutable `&mut self` — probably threaded as
  `(asm_state, code)` pairs or accumulated via a mutable-ish pattern already established by the
  concurrent backend work; follow whatever convention that code already uses for `asm_function`
  state instead of inventing a second one).
- Port items 1-4 above as straightforward conditional branches keyed on `opt_level > 0`,
  mirroring the Rust structure line-for-line where possible so the exact instruction sequences
  match.
- Port item 5 (tensor contraction) as a dedicated function gated on `opt_level > 2`. Needs: (a)
  a `tensor_optimizable_expressions` predicate matching the Rust one exactly, (b) a dependency
  graph + topological sort over loop variables — Typst has no petgraph, so implement a small
  manual topo-sort (build adjacency from "index variable X's bound expression references
  variable Y" edges, Kahn's algorithm is ~15 lines and sufficient; must produce the *same* order
  as petgraph's toposort when the graph is a DAG with a unique valid order, which it will be for
  these test cases — if the reference's toposort has ties, may need to match iteration order,
  i.e. insertion order of nodes, since petgraph's toposort visits in node-index order for ties).
- Port item 6 as a final string-level regex/replace pass over the fully generated NASM text,
  applied only when `opt_level > 0`, in the exact order given (power-of-two `imul`→`shl` before
  the `imul rax, 1` deletion, matching the Rust `replacements` vec order — verify this doesn't
  matter here since the two patterns don't overlap, but preserve the order anyway for fidelity).
- Wire `-O3` (and generically `-ON`) CLI flag parsing in `jplc`/`cli-query.typ` through to this
  `opt_level` if not already plumbed (check whether the concurrent backend work already added
  `-O*` flag handling — `jplc` currently has `-O*) ;;` as a no-op catch-all per the driver script
  seen earlier in this session, so this needs to change from ignored to captured-and-passed).

## 4. No runtime/build-process work needed

Unlike a real image assignment, hw14 needs no linking, no libpng, no binary execution — it's a
pure text-diff on `-s` output, same harness as hw10-13. `rt/pngstuff.c` / `libpng` are irrelevant
here.

## 5. Ordered checklist

1. Confirm hw10-13 baseline (`-s`, no `-O`) already produces exact matches for all 5 hw14
   `.jpl` files against `*.jpl.expected` (this is really a hw10-13 completeness check, not new
   work — if it fails, the gap is in earlier-assignment array/sum-loop codegen, not here).
2. Plumb `-O<N>` flag through `jplc` → compiler entry → codegen state as an integer opt level
   (currently silently discarded).
3. Implement optimizations 1-4 (small-int push, mul strength reduction, bool-ternary shortcut,
   array-index-by-offset) gated on `opt_level > 0`. Test against `*.jpl.expected.opt` for
   whichever hw10-13 test files also carry `.expected.opt` (check — `hw10`/`hw11`/etc. may not
   use OptSpec at all; only re-verify hw14 files here to start).
4. Implement optimization 6 (regex post-pass) — cheap, do it right after 1-4 since it interacts
   with the `imul` instructions those emit.
5. Re-run hw14 OptSpec on all 5 files at this point; likely still failing only on `mat.jpl`,
   `crs.jpl`, `dns.jpl`, `sft.jpl`, `col.jpl` insofar as they hit the tensor contraction path —
   check the diff output to confirm which lines diverge before assuming which optimization is
   still missing.
6. Implement optimization 5 (tensor contraction fusion) last — hardest, needs the manual
   toposort and the fused-loop emission logic from `optimized_array_loop`/
   `build_traversal_graph`/`init_indices`/`check_loop_bounds` (read the full, uncut versions of
   these functions in `origin/main:src/assembly.rs`, this plan only saw partial excerpts around
   lines 1108-1420 — read the whole function bodies before implementing).
7. Iterate against `python3 grader.py test --hw 14 --dir <typst_memes path> --part all` until
   both O0 and O3 diffs are empty for all 5 files.

## Status update (2026-09-05, from a standalone-optimizer investigation)

Was asked to build a standalone `-O1`/`-O3` peephole optimizer module. Found that **`-O1`
(hw13) is now fully done and passing 100% (65/65)** directly inside `asm.typ`
(`insert-int-constant`, the `*` binop arms around line 279/294, the `if`-as-cond shortcut at
line ~584, and `apply-peephole` for the `imul`->`shl` regex pass) — so no standalone module was
built for that; it would only have duplicated working code. Did not touch `asm.typ`.

**hw14 (`-O3`) is still 0/5** (confirmed live via `tools/grader2/fastgrade.py --hw 14`), and
there's a real plumbing bug blocking any `-O3`-specific logic from ever running even if
written: `jplc`'s flag parser has `-O0) opt=0 ;; -O*) opt=1 ;;` — **every** non-`-O0` flag,
including `-O3`, collapses to `opt=1`. `cli-query.typ` and `compile()`/`emit-asm()` then thread
that single int straight through, and `asm.typ`'s optimizations all gate on `gs.opt > 0`, with
no `gs.opt >= 3` branch anywhere. Concretely, before any of the loop-fusion/index-offset work
in this plan can be observed to do anything, someone needs to:
1. Fix `jplc`: `-O1) opt=1 ;; -O2) opt=2 ;; -O3) opt=3 ;;` (or at minimum `-O3) opt=3 ;;`)
   instead of the catch-all `-O*) opt=1 ;;`.
2. Add `gs.opt >= 3` (not just `> 0`) gates in `asm.typ` for the `-O3`-only additions (array-
   indexing offset optimization, tensor-contraction loop fusion) so `-O1` behavior doesn't
   silently change and `-O3` isn't just a no-op alias for `-O1`.

Did not make this fix myself since it touches `jplc`/`asm.typ` directly, which another agent
may be actively editing — flagging here instead. The toposort/Kahn's-algorithm loop-fusion
logic itself (section 6 above) is still unimplemented as of this note; that's the real
remaining work for hw14, on top of the flag-plumbing fix.
