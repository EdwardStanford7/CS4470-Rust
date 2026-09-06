# hw12 port plan: arrays, loops, indexing, argnum/args

## 1. What's new in hw12

Test dirs (`grader/hw12/{array,fuzzer,if,sum,index,args}`) exercise:
- `array[i:B, j:B2, ...] expr` — array-loop literals (allocate + fill via nested loop)
- `sum[i:B, ...] expr` — sum-loop (reduction into an int/float accumulator)
- `a[i, j, ...]` — array indexing with runtime bounds checks
- array literals `[e1, e2, ...]` (may already partly exist from earlier hw — check)
- `if`/`args` dirs look like regression coverage of existing If and `argnum`/`args` globals in the new array-capable codegen path, not new features per se — `args`/`argnum` are pre-existing implicit globals (cmdline arg count / array) that must keep working once the stack layout changes to accommodate arrays.

`show` on array-typed values also changes: format string becomes `(ArrayType <elem> <rank>)` (see `Self::get_show_format_string`), and the value passed to `_show` is a *pointer* (`lea rsi, [rsp]` over the two-word array representation: `[data_ptr, dim0, dim1, ...]`), not an inline scalar.

## 2. Array runtime representation (inferred from .expected + assembly.rs)

An array value on the JPL abstract/shadow stack occupies `8*rank + 8` bytes, pushed in this order (low address = top of stack, growing down):
```
[data pointer]      <- rsp + 0            (pushed last / lowest offset)
[dim_0]             <- rsp + 8
[dim_1]             <- rsp + 16
...
[dim_{rank-1}]      <- rsp + 8*rank
```
Wait — check the literal example (`hw12/array/001.jpl.expected`): after the loop, stack has (from the alloc site) pointer at `[rsp+8]`, bound at `[rsp+0]`... but that's mid-generation, before "re-contextualizing" collapses bound+pointer into one array shadow item of size `8*rank+8`. The *external* convention (what `_show`/calls/struct fields see) is: **pointer first (lowest address), then dims 0..rank-1 increasing address** — confirmed by `calculate_linear_index`'s `bounds_offset + rank*8` reads the pointer *after* all rank dim words, i.e. pointer is at the highest offset of the block = it was pushed *first* (so it sits below the dims in stack-grows-down terms, but the block layed out as `[dim_{rank-1} .. dim_0, pointer]` reading upward)... Just port the exact instruction sequences below rather than re-deriving the layout abstractly — the existing hw10/hw11 asm.typ already has `type-size`/shadow item helpers; arrays need a new shadow item variant that is *not* single `size` but tagged with rank so `remove-item`/bounds-check code knows how many words to pop and where the pointer word lives relative to dim words. Concretely, from `ArrayLoop` codegen order:
1. `sub rsp, 8` — reserve pointer slot (temporarily typed as Int placeholder)
2. bounds pushed via `check_loop_bounds`, one push per range var, **in `range.iter().rev()`** order → so after this, stack (low→high offset) is: `[pointer-slot(uninit), bound_{n-1}, ..., bound_0]`? Actually `check_loop_bounds` iterates `range.iter().rev()` and each iteration does `generate_expression` which itself pushes (net effect: push bound for last range var first, ending with bound for range var 0 on top). So final stack top→down: `bound_0` (rsp+0), `bound_1` (rsp+8), ..., `bound_{rank-1}` (rsp+8*(rank-1)), then pointer-slot at `rsp+8*rank`.
3. size calc: `rdi = element_size; rdi *= bound_i` for `i in 0..range.len()` reading `[rsp + i*8]` (so bound_0 first, matches above), with `jno` overflow check each multiply.
4. `call _jpl_alloc`, store result at `[rsp + rank*8]` (the pointer slot) — confirms pointer sits at the *highest offset* (allocated first / bottom of the group).
5. `init_indices` pushes rank more words (loop counters, all init to 0), **also in `range.iter().rev()`**, so loop-index words end up above (lower offset than) the bounds words: layout now top→down is `idx_0(rsp+0), idx_1(rsp+8), ..., idx_{rank-1}(rsp+8*(rank-1)), bound_0(rsp+8*rank), ..., bound_{rank-1}(rsp+8*(2*rank-1)), pointer(rsp+8*2*rank)`.
6. loop body generates the element expression on top of that (element occupies `[rsp, rsp+element_size)`), then `calculate_linear_index` is called with `loop_vars_offset = body.resolved_type.usize()` (skip over the just-computed body value) and `bounds_offset = body.resolved_type.usize() + range.len()*8` (skip body + all loop indices) to reach the bounds block, `element_size = body.resolved_type.usize()`.
7. linear index formula (non-optimized branch, `optimization_level == 0` — **use this branch only, skip the `optimized_*` paths entirely**, they're optimizer-only and irrelevant for matching `-O0` expected output... but VERIFY: check whether the grader's `.expected` files were generated at `-O0` or some other level by grepping for `imul rax, [rsp` vs the optimized `push qword` literal-bound pattern in a `.expected` file with a constant bound):
   ```
   rax = 0
   for i in 0..rank: rax = rax*bound[i] + idx[i]      // row-major, bound read from bounds_offset+i*8, idx from loop_vars_offset+i*8
   rax *= element_size
   rax += pointer   // read from [rsp + bounds_offset + rank*8]
   ```
8. copy body bytes from `[rsp+i]` to `[rax+i]` for `i in 0..element_size step 8` (reverse order iteration but same addresses — order doesn't matter for correctness, but MUST match for literal diff, so iterate `.rev()` exactly as Rust does).
9. free body: `add rsp, body_size`
10. `increment_loop_index`: nested odometer increment — for `i` from `rank-1` down to `1`: `[rsp+i*8] += 1; if [rsp+i*8] < [rsp + 8*(i+rank)] goto continue; [rsp+i*8] = 0` (rolls over), then unconditionally increment `[rsp+0]` and compare/jump for the outermost. **Order/carry logic must match exactly** — this is classic odometer/multi-index increment, least-significant = index 0? Actually re-read: loop iterates `(0..rank).skip(1).rev()` i.e. indices `rank-1, rank-2, ..., 1`, and index 0 is handled unconditionally last with no rollover (since it's the outermost/slowest-varying and loop naturally exits via the final `jl` failing). So **index 0 is the slowest-varying (outer) dimension, index rank-1 fastest**, consistent with row-major with dim0 as outer.
11. after loop: free indices (`add rsp, 8*rank`) and bounds (`add rsp, 8*rank`), remove those shadow entries, then push one shadow entry for the final array type covering the (pointer+dims) block that's left (pointer + one dim = original bound expr values, but note: the *bound* values pushed during `check_loop_bounds` are consumed/freed above — the array's dims stored in the *runtime* array value shown to `_show` must come from somewhere else). **Re-check**: in the `.expected` for `array/001.jpl`, after the loop, only `add rsp, 8` happens (removing 1 word) before the comment `; array left on stack` and the `_show` call with `lea rsi, [rsp]` — so what's "left on stack" for `_show` is `[pointer, dim]`, i.e. **2 words for rank 1**, and the earlier `add rsp, 8` freed just the loop-index word (rank=1 case: only 1 index freed, bound word is NOT freed — it becomes the dim word of the resulting array value!). Generalize: for rank R, free `8*R` bytes (indices only), and the R bound words directly below become the array's dim words in the final representation — no separate re-store needed. This matches `assembly.rs`'s comment "Free all loop variables" followed by `; array left on stack` in the literal expected file — the "re-contextualizing" step in Rust is purely a *bookkeeping* shadow-stack type change, not an instruction emission, which the literal-diff based Typst codegen can also do as zero-cost (just change what the `gs.shadow` list says is there, emit no instructions).
12. Final on-stack layout for a rank-R array value = `[pointer (rsp+0)? or dims first?]` — from the expected snippet: after `add rsp, 8` (free 1 index word), remaining top-of-stack (rsp+0) is what *was* the bound word (`bound_0`, was at old rsp+8, now at rsp+0 after popping 1 word) and below it (rsp+8) is the pointer (was at old rsp+16). So **final layout: dims at low offsets (rsp+0 = dim0, rsp+8 = dim1, ... rsp+8*(R-1) = dim_{R-1}), pointer at rsp+8*R (highest offset)**. This is the opposite of what a naive "pointer-first" assumption would give — MUST implement with dims-low/pointer-high ordering to match.
13. `_show` call: `lea rdi, [rel constN]` (type-string constant), `lea rsi, [rsp]` (pointer to the whole block, base = dims start), `call _show`, then `add rsp, (8*R + 8)` to pop the whole array value, then continue with existing alignment/postlude pop sequence already in asm.typ.

## 3. ArrayIndex (`a[i,j,...]`)

From `ArrayIndex` non-optimized path (`optimization_level == 0`, ignore `optimized_array_index`):
1. push_assert (bookkeeping)
2. generate the array sub-expression `array` → pushes its array-value block (dims..., pointer) per §2.12 layout
3. generate each index expression **in `indices.iter().rev()`** order → pushes rank int words, index for last dim first (so after this loop, `[rsp+0]` = index for dim 0, consistent with "iterate rev so first-declared index ends up on top")

   Wait — re ordering: verify against `index/001.jpl` (`a[0]`, rank 1, trivial) — need a rank≥2 example from `grader/hw12/array` or `hw11` to nail multi-index order precisely. **Action item: grep grader/hw12/* for a `.jpl` with 2+ indices and diff the exact `.expected` before implementing**, don't trust this summary blindly for rank ≥ 2.
4. bounds check per index `k`: `mov rax, [rsp + 8*k]; cmp rax, 0; assert jge "negative array index"; cmp rax, [rsp + (k+rank)*8]; assert jl "index too large"` — the comparison bound at `(k+rank)*8` refers to the dims block sitting right below the just-pushed indices (consistent with §2.12: indices pushed on top of the array block whose dims start immediately after all `rank` index words).
5. `calculate_linear_index` with `bounds = None` (so always the plain non-constant-folded loop), `rank = indices.len()`, `loop_vars_offset = 0` (indices are right on top), `bounds_offset = indices.len()*8`, `element_size = result element size`.
6. pop the rank index words (`add rsp, 8` × rank) and the array's dim+pointer block (`add rsp, array.usize()`), i.e. discard the whole base array value now that `rax` holds the target address.
7. `sub rsp, element_size`, copy `element_size` bytes from `[rax+i]` to `[rsp+i]` (reverse-order iteration, step 8).

## 4. SumLoop (`sum[i:B,...] expr`)

Simpler than ArrayLoop — no heap alloc, just an accumulator:
1. `sub rsp, result_size` reserve accumulator, shadow-type it as the result type
2. `check_loop_bounds` (same as ArrayLoop, pushes rank bound words, `range.iter().rev()`, asserting `jg "non-positive loop bound"` each)
3. init accumulator to 0 (`mov rax,0; mov [rsp+8*rank], rax` for int — **check float case uses same store or `movsd`/xor**; look at an actual float sum-loop `.expected` if present under hw12/sum, else defer exact float-zero-init instruction to whatever the grader shows)
4. `init_indices` (rank words, init 0, same rev order as ArrayLoop)
5. loop label, generate body expression (pushes body value on top)
6. accumulate: for Int, `pop rax; add [rsp + 2*8*rank], rax` (note the `2*8*rank` — accumulator sits *below* both the loop-index block and the bounds block since it was allocated before either); for Float, `movsd xmm0,[rsp]; add rsp,8; addsd xmm0,[rsp+2*8*rank]; movsd [rsp+2*8*rank],xmm0`
7. remove body shadow, `increment_loop_index` (identical odometer logic to §2.10)
8. free indices (`add rsp, 8*rank`), free bounds (`add rsp, 8*rank`) — two separate `add rsp,` + shadow-remove blocks per the Rust source (looks redundant/doubled in the source — replicate literally: it frees indices then in a *second* pass frees "the same" 8*rank again per two back-to-back blocks in the excerpt; **re-verify against an actual multi-dim sum `.expected` before assuming — the doubled block in the pasted source may be indices-then-bounds, i.e. correct, not a literal bug — read it again as: first `add rsp, 8*rank` for indices+shadow removal, second `add rsp, 8*rank` for bounds+shadow removal — this is correct, not a bug**)
9. accumulator (`result_size` bytes) remains on top of stack as the SumLoop's value; shadow already tracks it from step 1, no re-push needed.

## 5. `_jpl_alloc` calling convention

`mov rdi, <byte_count>` then `call _jpl_alloc`, result pointer in `rax`. Must `pad_shadow`/`unpad_shadow` around the call (16-byte stack alignment requirement — asm.typ already has `pad-shadow`/`unpad-shadow` helpers from hw10, reuse them exactly as `gen-assert` does around `_fail_assertion`).

Array literal `[e1,...,en]` (from `ExpressionType::ArrayLiteral`, may already be needed pre-hw12 — check if asm.typ has it; grep showed nothing) also uses `_jpl_alloc`: push all elements (rev order), `mov rdi, n*element_size`, alloc, copy stack→heap (reverse byte-order like everywhere else), free the element stack space, then `push rax` (pointer) then `push n` (dim) — **note this order is pointer-then-dim on the *push* instructions, meaning pointer ends up at lower rsp offset than dim, i.e. `[rsp+0]=pointer, [rsp+8]=dim`** — this CONTRADICTS §2.12's "dims low, pointer high" conclusion drawn from the array-loop case! Re-examine: `array literal` pushes `push rax` (pointer) first, THEN `push rax=len` (dim) second — in x86, `push` decrements rsp then stores, so the *second* push ends up at the *lower* address. So after `push rax(ptr); push rax(len)`: `[rsp+0] = len (dim)`, `[rsp+8] = pointer`. **This actually agrees with §2.12** (dims low, pointer high) — good, no contradiction, just make sure the Typst implementation double-checks push-order-vs-final-offset arithmetic like this every time (easy off-by-one-push source of bugs).

## 6. Bounds-check / assert plumbing

Already exists in asm.typ as `gen-assert(gs, cmp-mode, message)` (hw10-era, used for divide-by-zero etc.) — reuse verbatim for:
- `"jg", "non-positive loop bound"`
- `"jno", "overflow computing array size"`
- `"jge", "negative array index"`
- `"jl", "index too large"`

The Rust `push_assert`/`pop_assert` stack-depth bookkeeping is a **debug-only sanity check** (`assert!` inside the Rust compiler itself, not emitted assembly) — it can be skipped entirely in Typst, or replicated as a lightweight internal invariant check during development to catch shadow-stack arithmetic bugs, but it emits zero instructions either way, so it's optional tooling, not a codegen requirement.

## 7. `show` format string for arrays/structs

Port `get_show_format_string`: `(ArrayType <elem-format> <rank>)` recursively, `(TupleType <f1> <f2> ...)` for structs, else `typ.to_string()` (existing scalar format already in asm.typ's show handling from hw10 — check `gen-show` at asm.typ:284, likely needs a rank/array branch added).

## 8. Implementation checklist (ordered)

1. [ ] Grep an actual `hw12/array` or `hw11` test with rank ≥ 2 and diff its `.expected` to lock down multi-index push order (§3 step 3) and multi-dim linear-index formula (§2.7) before writing code — do not trust the single-rank example alone.
2. [ ] Add array-literal codegen (`ExpressionType::ArrayLiteral` equivalent, likely `x.tag == "array"` in compiler.typ's typed AST) to `gen-expr` in asm.typ, per §5.
3. [ ] Add a shadow-stack representation for array-typed items that carries `rank` (or just `size = 8*rank+8` is enough if bounds-checking code re-derives rank from the *type*, not the shadow entry — check how hw10/11 asm.typ's shadow items store type info; may already be generic enough via `type-size(t, structs)` if array types are sized correctly there).
4. [ ] Implement `check-loop-bounds(gs, ranges, infer, env, structs)` per §2.2/§4.2.
5. [ ] Implement `init-indices(gs, ranges)` per §2.5.
6. [ ] Implement `calculate-linear-index(gs, rank, loop-vars-offset, bounds-offset, element-size)` per §2.7/§3.5.
7. [ ] Implement `increment-loop-index(gs, rank, continue-label)` per §2.10.
8. [ ] Implement ArrayLoop codegen wiring 2-7 together per §2 steps 1-13.
9. [ ] Implement SumLoop codegen per §4.
10. [ ] Implement ArrayIndex codegen per §3.
11. [ ] Extend `show` format-string generation for array/struct types per §7.
12. [ ] Run `python3 grader.py test --hw 12 --dir <typst_memes> --part all`, iterate on diffs — expect the first real bugs to be push-order/offset arithmetic (off-by-8s), not logic errors.
13. [ ] Re-run hw10/hw11 to confirm no regression (array-literal/index paths may be shared with earlier array support if any existed).

## Key risk

This entire plan is reconstructed from static reading of assembly.rs plus ONE literal `.expected` file per construct; several offset/ordering claims (especially rank ≥ 2 index order, and the "doubled add rsp" in SumLoop cleanup) are marked **unverified** above and must be checked against real multi-dimensional `.expected` fixtures before coding, not assumed correct from this document alone.

**UPDATE 2026-09-05 (consistency-check pass):** the rank ≥ 2 concern is now
empirically resolved — as of this check, `asm.typ` (implemented by a
separate concurrent agent) passes ALL of `hw12/array` (28/28), `hw12/index`
(10/10), and `hw12/args` (25/25, including `args/017.jpl`:
`show array[i:3, j:argnum, k:9] [i,j,k]`, a genuine rank-3 case) via
`tools/asm_bisect.py --summary`, confirming whatever multi-index push order
was actually implemented is correct against real fixtures, regardless of
whether it matches this plan's guess verbatim. Still failing at check time:
`hw12/if` (part 1) and `hw12/fuzzer` (part 6, 2/20) — these are the two
areas still needing work, not the array/index/args layout this plan focused
on. Grader pass rates are moving fast (checked twice ~10 min apart, several
parts flipped from 0% to 100% in between) — always re-run
`tools/asm_bisect.py <dir> --summary` for current ground truth rather than
trusting any pass-rate number in this repo's plan docs, including this one.
