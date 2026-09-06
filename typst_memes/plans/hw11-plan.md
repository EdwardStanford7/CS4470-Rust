# hw11 port plan: globals (`let`), functions, calls, return

## 1. What's new vs hw10

hw10 = `show <expr>` only, over full scalar/array-literal expressions (arith,
cmp, bool, negation, nested arrays). No bindings, no functions.

hw11 adds (confirmed via grader/hw11/ok{1,2,3}/*.jpl):
- top-level `let x = expr` (ok1) — global variables usable by later top-level
  commands and by function bodies (ok3/002-003: `let x = 1; fn f(d:int) {
  return x + d }`).
- `fn name(params) : rettype { statements... }` declarations (ok2/ok3),
  including zero-arg, int/bool/float/array return types.
- `return expr` inside function bodies, including array-typed returns
  (ok3/001: `fn f(): int[] { return [1,2,3] }` — "padding" comment in that
  test name refers to the hidden pointer-return-slot arg, see §3).
- calling functions from expressions (`show f()`, `show f(13)`, `show f(x*x)`).
- destructured array lvalues in `let`/params (`let a[b] = arr`, i.e. bound
  index names alongside the data), per `LValueType::Array { indices }`.

Not yet in scope for hw11: control flow (`if`/`sum`/`array` comprehensions —
that's hw12), structs (hw13+), images (hw14).

## 2. Rust reference: exact functions to port (`origin/main:src/assembly.rs`)

- `generate_assembly` (top, ~L260-306): builds `jpl_main` prologue —
  `push rbp; mov rbp, rsp; push r12; mov r12, rbp` and epilogue
  `add rsp, stack_size-8; pop r12; pop rbp; ret`. **r12 is a second frame
  pointer that anchors jpl_main's globals** so functions can be compiled
  without knowing jpl_main's current stack depth. Also pre-seeds
  `self.offsets["args"] = (-16, false)` and `"argnum"` (CLI args support,
  not exercised until later hw but worth carrying the slots now).
- `generate_command` match arm `CommandType::Function` (~L391-462): builds a
  **separate `AsmFunction`** per function (own shadow stack / stack_size),
  pushes `name` and `_name` labels (both — NASM needs the leading-underscore
  alias for the C-ish extern convention used elsewhere), `push rbp; mov rbp,
  rsp`, then walks `parameters`:
  - hidden return-pointer arg pushed FIRST when `return_type` is
    `Array`/`Struct`, consuming `INT_REGS[0]` (rdi) before any real params.
  - scalar (`Int|Bool|Void`) params: `push <int_reg>`; `Float`: `sub rsp,8`
    + `movsd [rsp], <xmm_reg>`. Each records
    `self.offsets[param.name] = (stack_size_at_push, false)` — `false`
    marks it as **not** a jpl_main global, so `handle_variable` will address
    it via `rbp`.
  - `Array`/`Struct` params are NOT pushed onto the stack at all — they're
    passed as a pointer already on the *caller's* stack; callee just
    records a *negative* offset (`-arr_param_size - 16`) into `self.offsets`
    pointing at the caller-owned memory, and running index-name aliases for
    `LValueType::Array{indices}` at `offset - 8*i`.
  - if function has no explicit return in all control paths (`has_return`
    false, computed by typechecker), synthesize `return 1` (push int
    constant 1, `pop rax; add rsp, stack_size; pop rbp; ret`).
  - finished function objects are pushed to `self.functions: Vec<AsmFunction>`
    and text-joined once at the very end (functions after `jpl_main` in
    output — check hw11 `.expected` ordering to confirm placement, likely
    globals'/jpl_main's data section first then each function body in
    declaration order).
- `generate_statement` match arm `Return` (~L311-342): pops the value via
  `generate_expression(.., in_statement: true)`; for scalar just
  `pop rax`/`movsd xmm0,[rsp]; add rsp,8`; for **Array/Struct** it does NOT
  return via rax as a value — instead `mov rax, [rbp - 8]` (loads the hidden
  return-pointer, which handle_let/param logic stashed at the very first
  stack slot pushed in the prologue) and copies `rank+1` (array: rank fields
  + pointer) or `isize/8` (struct) words from the expression result at
  `[rsp+i*8]` into `[rax+i*8]`. Then `add rsp, stack_size; pop rbp; ret` —
  caller reads the struct/array back out of the pointer it originally
  passed in rdi.
- `handle_let` (~L612-644, shared by both top-level `Let` command and
  in-function `Let` statement via `in_statement` flag): generates the rvalue
  expression, then records `self.offsets[name] = (stack_size, !in_statement)`
  — so top-level lets get `from_main = true`, function-local lets get
  `false`. Handles `LValueType::Array{indices}` the same alias-offset way as
  params.
- `handle_variable` (~L1997-2027): the read side. Looks up
  `(offset, from_main)`; **`var_offset_reg = if from_main && in_statement {
  "r12" } else { "rbp" }`** — i.e. reading a global while generating
  top-level (jpl_main) code uses r12; reading a global from *inside* a
  function, or reading any local/param, uses rbp. This is the single most
  important subtlety to replicate exactly — a naive port will use rbp
  everywhere and only fail once a `let` appears in jpl_main before a later
  command that also touches the stack.
- `ExpressionType::Call` (~L877-975): argument-passing convention —
  1. if return type is scalar: no extra rsp reservation; if it's
     array/struct: `sub rsp, ret_type.usize()` first (space for the callee
     to fill) *and* that reservation eats `INT_REGS[0]`.
  2. evaluate array/struct-typed args first (in reverse order), then
     scalar args (also reverse order) — so stack layout is
     `[..non-scalar args (in reverse)..][..scalar args (in reverse)..]`.
  3. pop scalar args into `INT_REGS`/`FLO_REGS` in **forward** order now.
  4. if return is array/struct, compute `lea rdi, [rsp + arr_arg_size]`
     (pointing just past the leftover array/struct args still sitting on
     the stack) as the hidden return-pointer arg.
  5. `call _<function>` (underscore-prefixed).
  6. pop/deallocate the array/struct arg space that's still sitting there.
  7. push result: scalar -> `push rax` / xmm0 movsd; array/struct -> nothing
     (value already written through the pointer into the pre-reserved slot
     from step 1).
- `insert_int_constant`/`insert_float_constant`/`insert_string_constant`/
  `insert_asm_constant` (~L2031-2059): dedup constant pool keyed by
  `AssemblyValue` (interned), named `const{data_section_len}` in
  first-seen-order — **must** match this exact numbering scheme since
  `.expected` files hardcode `const0`, `const1`, ... in the literal order
  constants are first referenced during codegen (not sorted, not
  declaration order — codegen-emission order).
- `AsmFunction::push_assert`/`pop_assert`/shadow-stack helpers (~L41-165):
  these are **debug-only invariants** (`assert!` on stack-depth bookkeeping)
  that don't emit any instructions themselves (`push_comment` for
  `print_shadow_stack` is presumably feature-gated / no-op in release —
  confirm by checking if `print_shadow_stack` bodies ever emit real
  asm vs. just internal comments starting with `;`, since the normalizer
  strips `;`-comments anyway). Port these as sanity-check-only helpers (or
  skip if Typst's assertion story is awkward) — they exist to catch bugs in
  the *port*, not to produce required output.

## 3. Proposed Typst approach

Assume hw10's `asm.typ` already provides: `emit-instr`, `emit-label`,
a constant pool dict (`name -> const label`, insertion-ordered) via
`insert-constant(state, value) -> label`, and `generate-expression(state,
expr, in-statement) -> state` pushing/popping a stack-depth counter plus
instruction text accumulator.

Extend `asm.typ` with:
- A mutable-by-convention `state` dict (Typst has no mutation, so thread an
  explicit `state` value through every function and return `(state,
  extra-info)` tuples, or use a single dictionary passed/returned
  everywhere — whatever hw10's asm.typ already established, stay consistent
  with it) carrying: `offsets: (name: (offset, from-main))`,
  `constants: (key: label)`, `data-section: array of (label, kind, value)`
  in insertion order, `functions: array of function-bodies` (rendered
  separately from jpl_main, concatenated at the end).
- `compile-function(state, fn-cmd) -> state`: mirrors the Rust match arm —
  own local `fn-state` with its own `stack-size`/`offsets` overlay (function
  params shadow-but-don't-clobber the global `offsets` map: easiest is to
  snapshot global `offsets`, layer param offsets on top typed as `from-main:
  false`, generate body, then restore global offsets afterward since
  Typst state is immutable/scoped naturally by function call return value
  — no need to "restore", just don't propagate the function-local offsets
  dict back into the state that continues into the next top-level command).
- `compile-return(state, ret-expr, is-array-or-struct) -> state`: implement
  the `rax`/`[rbp-8]` pointer-copy pattern verbatim.
- `compile-call(state, call-expr, in-statement) -> state`: implement the
  7-step argument convention above exactly, including the reverse-order
  evaluation split between array/struct args and scalar args.
- `handle-let(state, lvalue, rvalue-expr, in-statement) -> state` and
  `handle-variable(state, name, resolved-type, in-statement) -> state`,
  with the `r12`-vs-`rbp` branch reproduced exactly:
  `reg = if from-main and in-statement { "r12" } else { "rbp" }`.
- jpl_main prologue/epilogue: literally the 4-instruction prologue/3-
  instruction epilogue shown above, with the `stack_size > 8` trim-to-8
  special case at the end (comment in source literally says "Very sussy" —
  keep it, it's load-bearing for matching output, not a code smell to
  "fix").
- Final assembly assembly order: header externs/globals line (already
  presumably present from hw10 for the extern list — verify hw10's version
  didn't need `_fail_assertion`/etc since hw10 has no asserts/calls; may
  need to always emit the full extern list even if unused, since NASM
  tolerates unused externs and the `.expected` files always include the
  full list per the sample shown in hw10/ok-fuzzer2/007.jpl.expected) +
  data section (`section .data` + one line per constant in insertion
  order) + jpl_main body + each function body in declaration order.

## 4. Gotchas / diff-matching traps

1. **r12 vs rbp** for global reads — the single highest-value thing to get
   right; will silently produce *correct-looking-but-wrong* assembly
   (valid semantically under some circumstances, fails others) if skipped.
2. **Both `name` and `_name` labels** on every function (and jpl_main) —
   easy to drop the underscore-alias and get a "close but one line off"
   diff on every function boilerplate line.
3. **Constant numbering is emission-order, not sorted/dedup-by-sort** — if
   the Typst port collects constants into a dict/array in a different
   traversal order than the Rust reference (e.g. two-pass instead of
   single-pass), `const7` in yours may be `const12` in theirs even though
   both are "correct". Must literally replicate single-pass emit-as-you-go
   ordering.
4. **Array/struct return convention has no visible "return value"** in the
   normal push/pop sense — it's a silent pointer-write. A Typst port that
   tries to model "every expression pushes N words then let the parent pop"
   uniformly will need a special case here, same as Rust's does.
5. **Argument evaluation order for calls**: array/struct args before scalar
   args (both groups internally reversed) — NOT simply "all args reversed"
   or "all args in order". Get a call with mixed arg types wrong otherwise
   (hw11 tests are all single-arg though — ok3 only tests `f(13)`/`f(x*x)`,
   both int — so this specific trap may not be *tested* until hw12/13 with
   multi-param functions or array params, but implement it correctly now
   since assembly.rs already assumes it and hw11 params can be arrays per
   the match arm above even if the sample .jpl files don't exercise it).
6. **`has_return` synthesized `return 1`** — functions whose body doesn't
   explicitly return on every path (typechecker already proved this is only
   legal when unreachable in practice, or JPL allows implicit trailing
   value?) must still emit a return-1 stub matching the reference exactly;
   check the typechecker's `has_return` flag equivalent already exists in
   compiler.typ's typed AST (search for how hw9's type-checker records
   return-completeness) — if not present, hw9/porting work already needs
   it and this is likely already solved upstream in compiler.typ.
7. **Padding/shadow-stack alignment (`pad_shadow_with*`)** before `call`
   instructions — x86-64 SysV requires 16-byte stack alignment at `call`.
   The Rust code's `pad_shadow`/`pad_shadow_with` insert an extra 8-byte
   push when the current shadow-stack parity is odd. This affects `show`
   (already needed since hw10 calls `_show`) and now also affects function
   `call`s. Verify hw10's asm.typ already got padding-parity right (it must
   have, to pass hw10 which calls `_show`) and reuse the *same* parity
   counter/logic for call-site padding rather than reimplementing.
8. **Function body ordering relative to jpl_main** — verify against an
   actual hw11 `.expected` file (not yet inspected byte-for-byte here)
   whether functions are emitted in first-declared-order and whether any
   inter-function forward references matter (JPL likely disallows forward
   calls or resolves all names before codegen, so probably a non-issue, but
   confirm order matches before assuming).

## 5. Ordered implementation checklist

1. Read one full hw11 `.expected` file end-to-end (e.g. `ok3/002.jpl.expected`)
   to lock in exact label/ordering conventions before writing code.
2. Add `offsets` (name -> (offset, from-main)) threading through
   asm-generation state; wire top-level `let` (`handle-let`) and read-side
   `handle-variable` with the r12/rbp branch.
3. Implement `jpl_main` prologue/epilogue with r12 setup/teardown and the
   stack_size-trim-to-8 special case.
4. Implement function compilation: label pair, prologue, hidden-return-ptr
   param handling, scalar param handling (push to stack, record offset),
   array/struct param handling (alias into caller stack, no push),
   body statement codegen, has_return-false synthetic `return 1` stub.
5. Implement `return` statement codegen for scalar and array/struct cases.
6. Implement function-call expression codegen (7-step convention above),
   reusing existing call/pad-shadow parity logic from hw10's `_show` calls.
7. Run `python3 grader.py test --hw 11 --dir <typst_memes> --part all`
   against `ok1` first (plain `let`/`show`, no functions) to validate
   globals+r12 in isolation, then `ok2` (zero-arg functions, all scalar
   return types), then `ok3` (params + array return + global access from
   inside function body), then the fuzzer suites.
8. Re-run hw10 to confirm no regression (jpl_main prologue changed shape,
   must not break constant/show codegen).
