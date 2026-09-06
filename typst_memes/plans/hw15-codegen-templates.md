# hw15 missing-codegen templates

Concrete, transcription-ready templates for the six panics currently blocking
hw15 in `asm.typ`:

- `gen-expr`, line ~629: `panic("x86 backend: expression not representable yet: " + x.tag)`
  → needs `x.tag == "struct-lit"` and `x.tag == "dot"` cases, inserted as
  `else if` branches right before that final `else { panic(...) }`.
- `emit-asm`, line ~818 (top-level command dispatch inside the `for c in checked.ast`
  loop): `panic("x86 backend: command not representable yet: " + c.tag)`
  → needs `c.tag == "read"`, `"print"`, `"time"`, `"struct"` cases, inserted as
  `else if` branches before that final `else { panic(...) }`.

All templates below use the exact helper names already in `asm.typ`
(`pad-shadow`, `unpad-shadow`, `pad-shadow-with-size`, `insert-string-constant`,
`insert-int-constant`, `add-item`, `remove-item`, `gen-assert`, `type-size`,
`gen-expr`) — no new helpers needed for print/read/time/struct-decl. struct-lit
and dot are pure `gen-expr` arms using only `gen-expr` recursion + stack math.

AST shapes (confirmed from `compiler.typ`/`parser.typ`):
- struct-lit: `{tag: "struct-lit", name: <str>, items: (<expr>, ...)}` — `items`
  in **source order**.
- dot: `{tag: "dot", base: <expr>, field: <str>}`.
- `structs` param (already threaded everywhere): `structs.at(name)` →
  `((name: <str>, typ: <type>), ...)` in **declaration order**, matching
  `type-size`'s summation order.
- `read`/`print`/`time`/`struct` command nodes: need to check their exact field
  names in `compiler.typ`'s AST builder for these commands (I did not locate
  them in this pass — search `compiler.typ` for `"read"`/`"print"`/`"time"`/
  `"struct"` node constructors; likely `{tag:"read", source: <str-literal-expr-or-str>, target: <lvalue>}`,
  `{tag:"print", message: <str>}`, `{tag:"time", body: <command>}`,
  `{tag:"struct", name: <str>, fields: (...)}` by analogy with the Rust AST
  field names shown below — **verify field names against compiler.typ before
  coding**, this doc gives the codegen logic, not the exact Typst dict keys).

---

## 1. `struct-lit` (expression)

**Reference** (`assembly.rs` `generate_expression`, `ExpressionType::StructLiteral`):
fields are evaluated **in reverse order** (last field first) so they end up on
the stack in forward field order (lowest address = first field, matching
`type-size`'s summation order). No padding/alignment around it — it's pure
stack arithmetic, evaluated in whatever alignment context it's already in.

```
push_assert()
for field in fields.iter().rev() { generate_expression(field) }  // reverse!
for _ in fields { remove_shadow() }
add_shadow_type(resolved_struct_type)
pop_assert(1)
```

**Real fixture** (`grader/hw15/ok1/040.jpl`):
```
struct iiii { a: int  b: int  c: int  d: int }
show array[i:10,j:10,k:10,l:10] iiii{i, j, k, l}
```
No isolated single-shot literal fixture exists in the sample set inspected —
this one is inside an array comprehension, so its raw asm is interleaved with
loop codegen. The struct-lit itself, once loop variables i/j/k/l are already
on the stack, is JUST: push d, push c, push b, push a (each already-computed
value re-pushed via whatever `gen-expr` emits for a plain variable read) —
i.e. **no extra instructions beyond evaluating each field expression**, emitted
back-to-front.

**Typst template** (append to `gen-expr`'s if-chain, before the final `else`):
```typst
} else if x.tag == "struct-lit" {
  let lines = ()
  for item in x.items.rev() {
    let (gs2, code) = gen-expr(gs, item, infer, env, structs, in-statement)
    gs = gs2
    lines = lines + (code,)
  }
  for _ in x.items { gs = remove-item(gs) }
  let t = infer(x, env)  // or just x.name if infer(struct-lit) returns the struct name
  gs = add-item(gs, type-size(t, structs))
  (gs, lines.join("\n"))
```

**Gotchas:**
- Evaluate `items.rev()` — reverse order is load-bearing (confirmed both by
  the Rust reference and by the general "right-before-left" evaluation-order
  convention already noted in `plans/hw10-plan.md`/`hw11-plan.md` for binops —
  this is the same family of quirk).
- `remove-item` must be called once per field (each field's `gen-expr` call
  already did `add-item` for its own type; you're collapsing N shadow entries
  into 1 struct entry) — mirrors the Rust `for _ in fields { remove_shadow() }`
  then a single `add_shadow_type`.
- No stack space is actually moved/copied here — the fields are already
  contiguous on the stack in the right order purely because they were pushed
  back-to-front. This is the cheapest of the six cases.

---

## 2. `dot` (field access expression)

**Reference** (`assembly.rs` `ExpressionType::Dot`):
```
generate_expression(struct_variable)     // struct now on stack, full size
offset_into_struct = sum of sizes of fields before `field` (declaration order)
field_type = type of `field`
for i in (0..field_type.size()).step_by(8).rev() {
    mov r10, [rsp + offset_into_struct + i]
    mov [rsp + (struct_size - field_size + i)], r10
}
add rsp, (struct_size - field_size)
remove_shadow(); add_shadow_type(field_type)
```
This is an **in-place compaction**: it copies the field's bytes down to the
*top* of the stack region (highest address end, i.e. what becomes the new
top-of-stack after `add rsp`), overwriting from the far end backward so the
copy never clobbers source bytes it hasn't read yet (safe because it walks
high-to-low when field is destined for higher addresses than source, or simply
because struct fields are always aligned this way in this compiler's layout).

**Real fixture** (`grader/hw15/ok1/095.jpl`, `m1[i,k].r`): confirms
`.field` on an array-index result — the dot logic applies uniformly whether
`struct_variable` is a plain variable, an index result, or anything else,
since it just operates on "whatever's on top of the stack after evaluating
struct_variable".

**Typst template:**
```typst
} else if x.tag == "dot" {
  let (gs, base-code) = gen-expr(gs, x.base, infer, env, structs, in-statement)
  let base-t = infer(x.base, env)
  let fs = structs.at(base-t)  // assumes base-t is the bare struct-name string
  let offset = 0
  let field-type = none
  for f in fs {
    if f.name == x.field { field-type = f.typ; break }  // Typst has no `break` in for; use a flag/find instead
    offset += type-size(f.typ, structs)
  }
  // Typst idiom (no early-break in #for): use .position()/.find() on fs instead:
  // let idx = fs.position(f => f.name == x.field)
  // let offset = fs.slice(0, idx).map(f => type-size(f.typ, structs)).sum(default: 0)
  // let field-type = fs.at(idx).typ
  let field-size = type-size(field-type, structs)
  let struct-size = type-size(base-t, structs)
  let lines = (base-code,)
  for i in range(int(field-size / 8)).rev() {
    let off = i * 8
    lines = lines + (
      "mov r10, [rsp + " + str(offset + off) + "]",
      "mov [rsp + " + str(struct-size - field-size + off) + "], r10",
    )
  }
  lines = lines + ("add rsp, " + str(struct-size - field-size),)
  gs = remove-item(gs)
  gs = add-item(gs, field-size)
  (gs, lines.join("\n"))
```

**Gotchas:**
- Typst has no `break`/early-exit inside `for` the way Rust does — use
  `fs.position(f => f.name == x.field)` to find the index, then slice+sum for
  the offset, as sketched in the comment above. Don't hand-roll a mutable
  loop-with-break; it doesn't exist in Typst's `#for`.
- `base-t` from `infer(x.base, env)` — confirm it returns the bare struct name
  string (matches how `type-size` and `structs.at(...)` are used elsewhere in
  this file, e.g. `type-size(t, structs)`'s struct branch does exactly
  `structs.at(t)` where `t` is a plain string).
- The copy loop direction (`.rev()`) matters when field-size spans multiple
  8-byte words and `offset` could overlap the destination range if walked
  forward — copy high-to-low exactly like the Rust reference to avoid
  corrupting not-yet-read source bytes on overlapping in-place shifts.

---

## 3. `print` (command)

**Reference** (`assembly.rs` `CommandType::Print`):
```
push_assert()
insert_string_constant(message)
pad_shadow()
call _print
unpad_shadow()
pop_assert(0)
```
Simplest of the six — a pure side-effecting call, no value pushed/left on
the stack, message is a compile-time string constant (not a JPL expression).

**Real fixture** (`grader/hw15/ok2/pixel-sort.jpl`):
```
lea rdi, [rel const22] ; 'Time taken to generate sorted sample image:'
call _print
```
(No visible `sub`/`add rsp` around it in this excerpt — stack was already
16-byte aligned at that point in the file, i.e. `pad-shadow` correctly
no-ops when already aligned. Don't assume alignment instructions are always
emitted — trust `pad-shadow`'s own alignment tracking.)

**Typst template** (append to `emit-asm`'s command dispatch, before the
final `else`):
```typst
} else if c.tag == "print" {
  let (gs, lea) = insert-string-constant(gs, c.message)
  let (gs, prepad) = pad-shadow(gs)
  let lines = (lea,)
  if prepad != "" { lines = lines + (prepad,) }
  lines = lines + ("call _print",)
  let (gs, unpad) = unpad-shadow(gs)
  if unpad != "" { lines = lines + (unpad,) }
  body.push(lines.join("\n"))
```
**Gotcha:** `insert-string-constant` must run *before* `pad-shadow` here to
match the reference's constant-numbering order (constants are numbered in
emission order — see `hw10-plan.md`'s "lazy/deduped constant interning"
gotcha; getting this order backwards will misnumber `constN` labels and fail
the diff even though the logic is "equivalent"). Verify against
`insert-string-constant`'s actual signature — in `gen-show` above it's called
*before* `pad-shadow-with-size`, consistent with this ordering.

---

## 4. `read` (command: `read image <str> to <lvalue>`)

**Reference** (`assembly.rs` `CommandType::Read`): allocates a fixed 24-byte
shadow slot (array header: 2×8-byte dims + 8-byte data pointer, rank 2, RGBA
struct element), calls `_read_image(rdi=&slot, rsi=&filename_const)`, then
registers the destination variable (and its dimension-binding names, if
`to arr[H,W]` form) as offsets into that slot — **no value is pushed by a
generate_expression call; this command directly manipulates the shadow stack
and offsets table**, closer to how `handle_let`/array-param binding works
than to a normal expression.

```
push_assert()
sub rsp, 24                      // array-of-struct{r,g,b,a} header: dims+ptr
add_shadow_type(array_type)      // rank 2
lea rdi, [rsp]
pad_shadow()
lea rsi, [rel <filename-const>]
call _read_image
unpad_shadow()
offsets[destination.name] = current stack_size   // "from-main"-style bit = whatever pad/unpad leaves it as
if destination has bound dim names (e.g. `to m1[H, W]`):
    offsets[dim_name_i] = stack_size - 8*i   for each dim name, in order
pop_assert(1)
```

**Real fixture** (`grader/hw15/ok1/013.jpl`, `read image "a.png" to a`, at
top level / jpl_main):
```
sub rsp, 24
lea rdi, [rsp]
lea rsi, [rel const0] ; 'a.png'
call _read_image
```
No `pad_shadow`/`unpad_shadow` instructions visibly emitted here — stack was
already aligned (24 bytes pushed keeps 16-byte alignment relative to
whatever came before, since the prelude already established alignment) —
again, trust the helper's own no-op-when-aligned behavior rather than assuming
fixed instructions always appear.

**Typst template:**
```typst
} else if c.tag == "read" {
  gs = add-item(gs, 24)  // array-of-struct{4×int} header, rank 2, matches type-size's array formula (8 + 8*rank) generalized... verify: reference always uses a FIXED 24-byte slot (2 dims + 1 ptr) regardless of declared rank, since JPL images are always rank-2 arrays of an rgba-like struct. Confirm this against the JPL grammar (image type is always `float[,]` of RGBA per hw15-plan.md's OWN read, or `<struct>[,]`) before hardcoding "24".
  let lines = ("sub rsp, 24", "lea rdi, [rsp]")
  let (gs, prepad) = pad-shadow(gs)
  if prepad != "" { lines = lines + (prepad,) }
  let (gs, lea) = insert-string-constant(gs, c.filename)  // verify field name
  lines = lines + (lea, "call _read_image")
  let (gs, unpad) = unpad-shadow(gs)
  if unpad != "" { lines = lines + (unpad,) }
  gs.offsets.insert(c.target.name, (off: gs.stack, from-main: <match gen-let's convention: not in-statement>))
  if c.target.tag == "array-lvalue" {
    for (i, d) in c.target.dims.enumerate() {
      gs.offsets.insert(d, (off: gs.stack - 8 * i, from-main: <same>))
    }
  }
  body.push(lines.join("\n"))
```
**Gotchas:**
- The 24-byte fixed size assumes JPL's image read target is always a rank-2
  float/rgba-struct array — cross-check `compiler.typ`'s typechecking of
  `read image` (search for how it types the `read` command) to confirm this
  isn't parametric over rank before hardcoding 24. hw15-plan.md may already
  cover this — check there first.
- `from-main` bit: mirror exactly what `gen-let`/array-param code does for
  top-level vs in-function bindings (`c.tag == "read"` only ever appears as
  a top-level *command*, per the grammar shown in `emit-asm`'s dispatch loop,
  which only handles top-level `checked.ast` entries — so this is analogous
  to the `c.tag == "let"` branch's `from-main: not in-statement` with
  `in-statement` always `false` here).
- Filename is a compile-time string literal in the fixtures seen — confirm
  `compiler.typ`'s AST node field name for it (guessed `c.filename` above;
  could be `c.source`/`c.path`).

---

## 5. `time` (command wrapping another command)

**Reference** (`assembly.rs` `CommandType::Time`): calls `_get_time` (returns
float in xmm0) before and after recursively generating the inner command,
then computes and prints the delta via `_print_time`. **Fully recursive** —
`time time assert true, "b"` (real fixture, `grader/hw15/ok-fuzzer2/080.jpl`)
nests two `_get_time`/`_print_time` pairs around the innermost command.

```
push_assert()
pad_shadow(); call _get_time; unpad_shadow()
sub rsp, 8; movsd [rsp], xmm0            // stash start time on stack
add_shadow_type(Float)
stack_size_before = current stack size
generate_command(inner_command)          // RECURSE into generate_command, not generate_expression
pad_shadow(); call _get_time; unpad_shadow()
sub rsp, 8; movsd [rsp], xmm0
movsd xmm0, [rsp]; add rsp, 8
movsd xmm1, [rsp + (stack_size_now - stack_size_before)]   // reach back to the stashed start time
subsd xmm0, xmm1
pad_shadow(); call _print_time; unpad_shadow()
pop_assert(0)
```
Note: the start-time slot is **never popped** by this command itself (matches
the fixture: two `sub rsp,8/movsd` stashes accumulate, and only the final
`add rsp, 32 ; Local variables` epilogue at the very end of `jpl_main` cleans
everything up) — `time` leaves its timestamp on the stack permanently, it's
not scoped/freed inline. This matters for shadow-stack accounting: `add-item`
for the Float, but no matching `remove-item` inside this command's own
codegen.

**Typst template:**
```typst
} else if c.tag == "time" {
  let (gs, prepad1) = pad-shadow(gs)
  let (gs, unpad1) = unpad-shadow(gs)  // NOTE: order is pad, call, unpad -- see below, don't call unpad before the `call` line
  let lines = ()
  if prepad1 != "" { lines = lines + (prepad1,) }
  lines = lines + ("call _get_time",)
  if unpad1 != "" { lines = lines + (unpad1,) }
  lines = lines + ("sub rsp, 8", "movsd [rsp], xmm0")
  gs = add-item(gs, 8)
  let stack-before = gs.stack
  // recurse into whatever top-level command dispatch this `else if` chain lives in --
  // if `emit-asm`'s dispatch isn't already factored into a reusable gen-command(gs, c) helper,
  // this is the strongest argument for extracting one now: `time` needs to call the SAME
  // dispatch logic recursively for its inner command.
  let (gs, inner-code) = gen-command(gs, c.body, ...)   // c.body / c.inner -- verify field name; requires a gen-command(gs, c, ...) helper to exist (may need factoring emit-asm's if-chain into a standalone function first)
  lines = lines + (inner-code,)
  let (gs, prepad2) = pad-shadow(gs)
  let lines2 = ()
  if prepad2 != "" { lines2 = (prepad2,) }
  lines2 = lines2 + ("call _get_time",)
  let (gs, unpad2) = unpad-shadow(gs)
  if unpad2 != "" { lines2 = lines2 + (unpad2,) }
  lines2 = lines2 + (
    "sub rsp, 8", "movsd [rsp], xmm0",
    "movsd xmm0, [rsp]", "add rsp, 8",
    "movsd xmm1, [rsp + " + str(gs.stack - stack-before) + "]",
    "subsd xmm0, xmm1",
  )
  let (gs, prepad3) = pad-shadow(gs)
  if prepad3 != "" { lines2 = lines2 + (prepad3,) }
  lines2 = lines2 + ("call _print_time",)
  let (gs, unpad3) = unpad-shadow(gs)
  if unpad3 != "" { lines2 = lines2 + (unpad3,) }
  body.push((lines + lines2).join("\n"))
```
**Gotchas — this is the trickiest of the six:**
- **Requires factoring `emit-asm`'s command if-chain into a standalone
  `gen-command(gs, c, ...)` function** so `time` can recurse into it (currently
  the dispatch is inline in `emit-asm`'s `for c in checked.ast` loop, not a
  callable function — `time`'s inner command needs the SAME dispatch, since
  the fixture shows `time time assert ...` nesting arbitrarily). This is a
  small refactor but a *required* one, not optional — flag it to whoever
  implements this so they don't try to inline-duplicate the dispatch logic.
- The stashed start-time is **deliberately leaked** (no `remove-item`/`add rsp`
  for it inside `time`'s own codegen) — don't "fix" this by adding cleanup,
  it would break the fixture match. It's cleaned up by the normal end-of-block
  stack teardown (`add rsp, <total>` at function/program end).
- `_get_time`/`_print_time` calling convention: `_get_time` takes no args,
  returns a double in `xmm0`. `_print_time` takes the delta in `xmm0`, no
  return value used.

---

## 6. `struct` (declaration command)

**Reference:** `CommandType::Struct { name: _, elements: _ } => {}` — literally
a no-op in codegen. All the real work (registering the struct's field layout)
already happened during typechecking, which per `plans/hw15-plan.md` and the
existing `structs` parameter already threaded through every `gen-*` function
in `asm.typ`, is **already done in this codebase's front end** (`compiler.typ`
already builds the `structs` dict passed everywhere).

**Typst template:**
```typst
} else if c.tag == "struct" {
  (gs, "")  // or simply: no-op, emit nothing, don't push anything to `body`
```
Or, more simply, just skip pushing anything to `body` for this tag — a
one-line addition to the dispatch that does nothing. **This is the free win
of the six** — implement it first, it can't possibly be wrong.

**Gotcha:** none. Just don't forget it needs SOME arm (even a no-op one) or
the `panic(...)` in the final `else` still fires for every struct declaration.

---

## Suggested implementation order

1. **`struct` decl** — trivial no-op, zero risk, do first.
2. **`print`** — simplest real case, validates the const-ordering convention.
3. **`struct-lit`** — pure `gen-expr` stack arithmetic, no new helpers.
4. **`dot`** — needs the `fs.position(...)` idiom instead of Rust-style
   early-break; moderate risk on the copy-loop math, test against
   `grader/hw15/ok1/095.jpl` and `115.jpl`/`123.jpl` specifically.
5. **`read`** — needs the fixed-24-byte-slot assumption verified against
   `compiler.typ`'s typechecking of image reads before hardcoding.
6. **`time`** — do last: requires factoring `emit-asm`'s inline dispatch into
   a reusable `gen-command` helper first (a real, if small, structural change),
   and has the trickiest "leaked stack slot" semantics.

After all six: rerun `tools/grader2/fastgrade.py --hw 15` and use
`tools/asm_bisect.py` on any remaining hw15 failures (there were still some
pure instruction-count/ordering mismatches unrelated to these six panics,
per the ground-truth sweep — e.g. `hw15 part 6 :: 176.jpl: 6 lines differ`,
not a panic).
