# IR design: `ir.typ` + `layout.typ`

Concretizes refactor-plan.md §3. Written against the codebase as of: `compiler.typ`
(274 lines: lexer/parser-facade/checker/4 printers), `wat.typ` (61 lines, WasmGC),
`asm.typ` (340 lines, x86 shadow-stack model, hw10-only so far). Both backends
currently tree-walk `checked.ast` directly and call `checked.infer(node, env)`
redundantly at every node to recover a type already known at check time.

Do not implement this until all 15 assignments are green on both backends —
this document describes the *target* shape to migrate to, using the two
existing (once-complete) backends as the source of truth for "what do they both
already compute that should be shared."

## 1. Why now vs. later

The x86 backend's current hw10 fuzzer bug (shadow-stack alignment drifting on
nested expressions) is exactly the class of bug this design prevents in the
long run: `asm.typ` hand-tracks stack effects per-expression with `add-item`/
`remove-item`/`pad-shadow` calls interleaved into codegen. If the *IR* itself
carried precomputed stack-effect metadata (see §5), the backend would consume
declared effects instead of re-deriving them ad hoc at every call site — but
retrofitting that now, mid-bugfix, would multiply the risk. Fix hw10-15 first
on the current architecture; build the IR once both backends are complete and
you have two finished implementations to diff against.

## 2. Node shapes (`ir.typ`)

Every IR node is a tagged dictionary, same convention as the surface AST, but
with `ty` (a *normalized* type value, see `types.typ`) and `size` (byte size
per `type-size`, see §3) attached directly — no `infer` call needed downstream.

```
// expressions — all carry `ty` and `size`
(tag: "const-int",    ty: "int",   size: 8, value: <int>)
(tag: "const-float",  ty: "float", size: 8, value: <float>)
(tag: "const-bool",   ty: "bool",  size: 8, value: <bool>)
(tag: "local",        ty: .., size: .., slot: <slot-id>)          // was "var"
(tag: "unop",         ty: .., size: 8, op: "-"|"!", operand-ty: .., value: <ir>)
(tag: "binop",        ty: .., size: .., op: <str>, left-ty: .., left: <ir>, right: <ir>)
(tag: "short-circuit",ty: "bool", size: 8, op: "&&"|"||", left: <ir>, right: <ir>)
(tag: "if",           ty: .., size: .., cond: <ir>, yes: <ir>, no: <ir>)
(tag: "call",         ty: .., size: .., name: <str>, args: (<ir>,))
(tag: "array-lit",    ty: .., size: 16, elem-ty: .., elem-size: <int>, items: (<ir>,))
(tag: "array-index",  ty: .., size: .., base: <ir>, base-rank: <int>, indices: (<ir>,))
(tag: "array-loop",   ty: .., size: 16, elem-ty: .., ranges: (loop-range,), body: <ir>)
(tag: "sum-loop",     ty: .., size: .., ranges: (loop-range,), body: <ir>)
(tag: "struct-lit",   ty: .., size: .., struct-name: <str>, fields: (field-init,))
(tag: "dot",          ty: .., size: .., base: <ir>, base-size: <int>,
                       field-offset: <int>, field-size: <int>)   // offset/size PRECOMPUTED

// loop-range: (name: <str>, slot: <slot-id>, bound: <ir>)
// field-init: (name: <str>, offset: <int>, value: <ir>)

// statements/commands — no `ty`, but carry resolved slot/offset info
(tag: "let",     slot: <slot-id>, dim-slots: (<slot-id>,), value: <ir>)
(tag: "read",    slot: <slot-id>, dim-slots: (<slot-id>,), source: <str>)
(tag: "write",   value: <ir>, dest: <str>)
(tag: "print",   message: <str>)
(tag: "show",    value: <ir>)
(tag: "assert",  cond: <ir>, message: <str>)
(tag: "return",  value: <ir>)
(tag: "time",    inner: <ir-command>)
(tag: "fn-def",  name: <str>, params: (param,), returns: .., ret-size: <int>,
                  body: (<ir-command>,), frame: <frame-layout>)   // see §5
(tag: "struct-def", name: <str>, fields: (field-layout,), size: <int>)  // layout only; no codegen

// param: (name: <str>, slot: <slot-id>, ty: .., size: <int>)
// field-layout: (name: <str>, ty: .., offset: <int>, size: <int>)
```

`slot` is an opaque id (integer counter), not a name string — `local.typ`'s
`(name: vn, typ: t)` env-entry pattern in `wat.typ` and `asm.typ`'s reliance
on shadow-stack *position* both become "look up this slot in whatever
storage model the backend uses" (a WASM local index, an rbp-relative offset,
or a global). The lowering pass assigns slots once; each backend maps
`slot -> concrete storage` independently, since that mapping is genuinely
backend-specific (WasmGC locals vs. x86 stack/rbp offsets).

## 3. `layout.typ` (extracted from `type-size`/`show-type-string` in `asm.typ`
   and the type-name logic in `compiler.typ`)

```
#let type-size(t, structs) -> int          // moved verbatim from asm.typ
#let struct-field-layout(struct-name, structs) -> (field-layout,)
  // computes (name, ty, offset, size) for every field ONCE;
  // both backends currently redo the "sum sizes of preceding fields"
  // arithmetic per dot-access (asm.typ's hw15-plan §Dot) and per struct
  // codegen — this makes it a single cached lookup.
#let show-type-string(t) -> str            // moved verbatim from asm.typ,
  // also replaces compiler.typ's format-type (same computation, different
  // surface syntax — normalize on ONE function, parameterize the two
  // rendering styles if genuinely both still needed by CLI printers).
```

Both `struct-def` IR nodes and `dot`/`struct-lit` IR nodes get their
`offset`/`size` fields from `struct-field-layout`, computed once during
lowering — not recomputed by each backend at codegen time.

## 4. Lowering pass (`lower(checked) -> ir-program`)

`ir.typ` exports `#let lower(checked) = { .. }`, walking `checked.ast` with
`checked.infer`/`checked.globals` exactly once per node (today: `asm.typ` and
`wat.typ` each independently call `infer` on the same node — twice the calls,
twice the chance to introduce a mismatch between backends). Concretely:

- **Evaluation order is baked in at lowering, not left to each backend.**
  The hw10/11/12/15 plans document JPL's real evaluation-order quirks
  discovered empirically from `.expected` fixtures:
  - binops: right-hand side generated before left (`binary` lowers to an IR
    node whose `left`/`right` sub-IR is already in final emission order, OR
    the IR keeps them semantically named and a doc-comment states "codegen
    backends must emit `right` before `left`" — prefer the former: rename
    the lowered fields `first`/`second` matching actual emission order, so a
    future backend author can't get it backwards).
  - struct literals: fields lowered in **reverse declaration order** (so a
    stack-pushing backend ends up with forward order in memory) — same
    "name IR fields by emission order" fix: `struct-lit.fields` should
    already be reversed by `lower`, not re-reversed independently by
    `asm.typ` today (`gen-expr`'s array case reverses inline; move that
    reversal into `lower` for arrays/structs both).
  - array literals: elements lowered in reverse, same treatment.
  - call arguments: array/struct-typed args before scalar args, each group
    internally reversed (hw11-plan §Gotchas item 5) — `lower` should split
    `call.args` into this exact final order once, so `asm.typ` just emits
    `args` front-to-back with no grouping logic of its own.
  This is the single highest-value thing the IR buys: three independent
  "get the order backwards" bug classes collapse into one lowering rule,
  tested once.//
- **Loop desugaring**: `array-loop`/`sum-loop` bound expressions are
  evaluated before the loop body per JPL semantics; `lower` resolves each
  `range.name` to a fresh `slot` immediately (today `wat.typ` and `asm.typ`
  would each need their own fresh-slot/fresh-local counters — keep counters
  IR-local, assigned during lowering, so backends never invent their own
  numbering scheme).
- **Struct/array field & offset resolution**: `dot` and `struct-lit` nodes
  get `field-offset`/`field-size` (or an ordered `fields` list with offsets
  already attached) from `layout.typ` at lowering time, not at codegen time.
- **Return convention marking**: `fn-def.frame` (see §5) records whether the
  function returns via a hidden pointer arg (array/struct returns, per
  hw11-plan) so backends don't each re-derive "is this return type
  hidden-pointer-shaped" from `ty` at every `return` site.

## 5. Stack-effect / shadow-stack abstraction

This is the part directly motivated by the current hw10 bug. Two options,
ranked:

**Option A (recommended): keep the shadow stack OUT of the IR, but extract it
into its own shared module, `shadow-stack.typ`**, used only by `backend-x86.typ`
(WasmGC has no analogous concept — its "stack" is the WASM value stack managed
by the VM, not something the backend tracks by hand). Rationale: the shadow
stack's job is purely operational bookkeeping for one backend's calling
convention (rsp-relative addressing, 16-byte alignment before `call`), not a
property of program *meaning* — it doesn't belong in a backend-neutral IR.
What DOES belong in the IR is `size` on every node (already precomputed per
§2), which is 90% of what `asm.typ` needs `type-size` for today; the shadow
stack module then only has to track *position*, not *re-derive size*, cutting
the surface area of the exact class of bug that's currently blocking hw10:

```
// shadow-stack.typ — lift verbatim from asm.typ's new-gs/add-item/remove-item/
// pad-shadow/unpad-shadow/pad-shadow-with-size, unchanged in logic, just
// promoted to its own file so it's independently testable (write direct
// unit tests: push a sequence of known sizes/pads, assert final gs.stack
// and gs.shadow match hand-computed expectations, WITHOUT going through
// full codegen — this is the missing piece that made the current fuzzer
// bug hard to bisect; a bisection tool over real .jpl files is necessary
// but a unit-level test of the shadow-stack primitives themselves, in
// isolation, is what actually prevents regressions here).
```

**Option B (do not do this):** encode push/pop effects as IR node metadata
(e.g. `stack-delta: 8`) and have the x86 backend fold that into shadow-stack
updates automatically. Rejected: `size` already gives the backend everything
it needs to compute deltas; adding a second derived field (`stack-delta`)
that must stay consistent with `size` reintroduces exactly the "two things
that should agree can drift apart" bug class this whole design is trying to
eliminate. One source of truth (`size`) is safer than two.

`fn-def.frame` (§2) is a small `layout.typ`-computed record independent of
the shadow stack:
```
(frame: (
  hidden-return-ptr: <bool>,     // true if returns array/struct
  param-slots: (slot-id: <int>, ty: .., size: <int>),
  local-slot-count: <int>,       // upper bound, for frame-size backends that need one
))
```
x86 uses this for stack-frame layout; WasmGC ignores `hidden-return-ptr`
entirely (structs/arrays are GC refs, always single-value returns) — proof
this field is genuinely backend-agnostic *data*, even though only one backend
acts on it.

## 6. What each backend does with the IR

`backend-x86.typ`'s `gen-expr(gs, node)` becomes a pure dispatch on
`node.tag` with NO `infer`/`env`/`structs` parameters — every fact it needs
(`ty`, `size`, field offsets, evaluation order) is already on the node. The
function signature shrinks from `gen-expr(gs, x, infer, env, structs)` to
`gen-expr(gs, node)`, and `gs` becomes purely the shadow-stack-module state
imported from `shadow-stack.typ`. Same for `backend-wasmgc.typ`'s `gen`:
drops the `(checked.infer)(x, types(env))` calls scattered through
`wat.typ`'s binary/if/array/dot/index cases, replaced by reading `node.ty`/
`node.size` directly.

Both backends keep their real, legitimately-different codegen: WasmGC's
box/unbox scheme for GC-tagged locals (`wat.typ`'s `box`/`unbox` helpers) has
no x86 equivalent and shouldn't be unified into the IR — the IR's job is only
to remove the *redundant, easy-to-desync* computations (type inference,
evaluation order, field offsets), not to unify codegen strategies that are
legitimately different per target.

## 6a. Step 6 status: layout.typ extracted, full shared IR NOT attempted

Two premises this document rests on turned out not to hold, checked against
the real, complete (post-hw15) `wat.typ`/`asm.typ`:

1. **"Field offsets computed independently in wat.typ and asm.typ" is false.**
   `wat.typ`'s `dot` case resolves a field by declaration-order *index*
   (`fs.position(f => f.name == x.field)`) and hands it to WasmGC's
   `struct.get $struct_X <idx>` — the wasm engine owns byte layout entirely.
   Only `asm.typ` computes byte offsets, and only at one call site. There is
   no cross-backend duplication of offset arithmetic to eliminate — `size`/
   `show-type-string`/offset computation were extracted into `layout.typ`
   anyway (§3), because factoring ~30 lines out of a 1000+-line `asm.typ`
   into their own well-named, importable module is worthwhile for
   readability on its own merits, not because it deduplicates anything
   between backends.

2. **"Evaluation order should be baked in once, shared by both backends" is
   wrong, not just unimplemented.** `asm.typ`'s right-before-left binop
   order, reversed struct/array-literal field order, etc. (§4 above) are not
   a JPL language requirement — they exist *solely* to byte-match the
   historical reference x86 compiler's specific (arbitrary) codegen choices,
   because the grader diffs `-s` output literally against fixtures generated
   by that reference. `wat.typ` has no such fixture to match (WasmGC output
   is graded by execution/validation, never text-diffed against a reference)
   and correctly uses plain left-then-right evaluation. A shared `lower()`
   that baked in one canonical evaluation order would necessarily be *wrong*
   for one backend or the other — this isn't "the same rule discovered
   independently by two backends," it's two backends with genuinely
   different, backend-specific ordering constraints for unrelated reasons
   (grading-fixture parity vs. nothing at all).

**Conclusion**: the "shared IR to eliminate accidentally-duplicated
lowering logic" premise doesn't survive contact with the actual, complete
implementation — what looked like duplication from the outside (before
either backend was finished) turns out to be either backend-specific-by-
necessity (evaluation order) or not actually shared in the first place
(field layout). Building `ir.typ`/`lower()` on top of this design as
written would be speculative restructuring with no real bug class to
eliminate, plus real risk to a 100%-passing 3310-test baseline — precisely
what refactor-plan.md's own §7 closing note warns against conflating with
"the goal."  **Recommendation: do not build the full shared IR as designed
here.** If a future need for shared lowering logic emerges (e.g. a third
backend), revisit with THAT concrete duplication as the driver, per
refactor-plan.md §3's own instruction to derive the IR shape empirically
from what backends "already compute redundantly" — not from an assumption
made before either backend existed in complete form.

`layout.typ` (type-size, show-type-string, struct-field-layout, field-layout)
is real, done, and verified: 3310/3310 after `asm.typ` migrated to import it
instead of defining these locally. `shadow-stack.typ` extraction (§5) remains
a reasonable, separate, lower-risk follow-up (it's real self-contained
duplication-of-concerns within `asm.typ` alone, independent of whether a
cross-backend IR ever gets built) — not attempted in this pass, left for a
future one if desired.

## 7. Migration path (no big-bang rewrite)

Do this only after all 15 assignments are green on the current
direct-AST-walking backends, as separate, independently-testable commits,
running the full `make -C grader test-hwN` suite (N=1..15) after each:

1. **Add `layout.typ`** (pure extraction: `type-size`, `struct-field-layout`,
   `show-type-string` moved out of `asm.typ`/`compiler.typ` verbatim, both
   call sites updated to import instead of defining locally). Zero behavior
   change — this step can't regress anything if the extracted functions are
   copied byte-for-byte before any cleanup.
2. **Add `shadow-stack.typ`** (pure extraction of `asm.typ`'s gs-manipulation
   functions, unchanged). Write the isolated unit tests described in §5
   *before* moving on — these are your regression net for the exact bug
   class currently live in hw10's fuzzer cases.
3. **Add `ir.typ` with `lower()`, but don't switch any backend to consume it
   yet.** Write `format-ir` (an IR pretty-printer, mirroring
   `format-typed`) and manually diff `lower(checked)` against hand-derived
   expectations for a handful of programs spanning every node tag — this is
   where you validate the evaluation-order/offset claims in §4 independently
   of either backend, using the two existing backends' *outputs* as an oracle
   (if `lower` + a trivial re-walk reproduces the same instruction order
   `asm.typ` currently emits for a given file, the IR's evaluation-order
   encoding is correct).
4. **Migrate `backend-wasmgc.typ` (renamed from `wat.typ`) to consume the IR
   first** — lower risk than x86 because WasmGC has no shadow-stack
   correctness hazard, so a lowering bug shows up as a validation/semantic
   failure caught immediately by `wasm-tools validate` and the WasmGC
   execution test suite, not a subtle alignment drift. Do it one node tag at
   a time (start with the leaf literals, then binops, then structs/arrays,
   then loops), running the WasmGC test suite after each tag migrates.
5. **Migrate `backend-x86.typ` (renamed from `asm.typ`) last**, same
   per-tag-at-a-time discipline, running `make -C grader test-hwN` for every
   N whose fixtures exercise that tag after each migration step. Keep the
   fuzzer corpora (`ok-fuzzer*`, `fail-fuzzer*` under every `grader/hw*`) in
   the loop throughout — they're what caught the current alignment bug and
   are the best available regression net for this exact failure mode.
6. Only after both backends consume `ir.typ` exclusively, delete the
   now-dead direct-AST-walking code paths (the `infer`/`env`/`structs`
   parameters on `gen-expr`/`gen`, and any leftover inline evaluation-order
   reversals now redundant with `lower`).
