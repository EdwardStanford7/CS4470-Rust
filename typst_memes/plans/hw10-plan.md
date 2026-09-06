# hw10 port plan: x86-64 NASM backend, scalar/array literal subset

## 1. What hw10 actually exercises
All 136 tests (`ok` 56, `ok-fuzzer1` 30, `ok-fuzzer2` 10, `ok-fuzzer12` 40) are
JPL programs consisting *only* of one or more top-level `show <expr>;`
commands. No `let`, `fn`, `if`, `array comprehension`, structs, or calls other
than the implicit `_show`. Expression forms present:
- int/float/bool literals, unary `-`/`!`
- binops: `+ - * / %` (int and float), comparisons `== != < > <= >=` on int/
  float/bool, `&&`/`||` do NOT appear in hw10 samples but the shared codegen
  path handles them — port them anyway since hw11+ will need them and it's
  the same function.
- array literals, including nested arrays (`[[...]]`), of scalars.
- multiple `show` statements per file (each is an independent stack frame
  segment inside `jpl_main`, sharing constants).

This is a pure "evaluate expression into rsp-top scratch stack, hand a
pointer + type-tag string to `_show`" backend. No functions, no variables, no
control flow, no heap arrays except via `_jpl_alloc` for literal arrays.

## 2. assembly.rs pieces to port (line numbers from `git show origin/main:src/assembly.rs`)
- `AsmFunction` struct + impl (~L33-215): `body` string, `stack_size`,
  `shadow_stack: VecDeque<Shadow>`, `assert_stack`. Methods: `push_instruction`
  /`push_instructions`/`push_label`/`push_comment`, `add_shadow_type`/
  `add_shadow_expr`, `remove_shadow` (pops the **last Item from the back**,
  skipping Padding entries — it's a `rev().find()`, not a plain pop), `pad_shadow`
  /`unpad_shadow` (16-byte alignment tracking via `stack_size % 16`),
  `push_assert`/`pop_assert`/`pop_assert_opt` (debug-only invariant checks on
  shadow stack depth — safe to port as no-ops or keep for self-checking during
  development, they never affect emitted text since they're `assert!`, not
  emission).
- `Shadow` enum (~L17-30): `Padding(bool)` / `Item(Type)`, `Display` impl only
  matters for the `; shadow stack #N#: ...` **comments**, which the grader
  strips via `ppasm` (`if ";" in line: line = line.split(";",1)[0]`). So the
  shadow-stack comment text content is irrelevant to grading — only its
  *side effects* (stack_size bookkeeping) matter. Do not waste time matching
  comment text exactly.
- `AssemblyValue` enum + `Display` (~L217-232): `Number(String)` → `dq {v}`,
  `String(String)` → `` db `{v}`, 0 ``.
- Constant pool: `data_section: Vec<AssemblyValue>`, `constants: HashMap<AssemblyValue,String>`
  (~L225-227, L2051-2060 `insert_asm_constant`). **Critical**: constants are
  interned lazily in emission order (first occurrence during codegen), not
  pre-collected. Dedup by value+kind (a `db` string constant with the same
  text always reuses its `constN`; likewise identical numeric bit patterns,
  though float `-0.0` vs `0.0` must NOT collide — Rust uses `format!("{:?}", f64)`
  which distinguishes them).
- `insert_int_constant`/`insert_float_constant`/`insert_string_constant`
  (~L2031-2049): emit `mov rax, [rel constN]` + `push rax` for numbers (both
  int and float use the same integer push! floats are pushed as raw bits via
  `mov rax`, not `movsd`, until an operation needs them in an xmm register —
  confirm against a float-arithmetic `.expected` file before assuming, but the
  int10 samples like show 1.0 support this: literal float push looks
  identical in structure to int push, just the interpretation differs at
  use-time), and `lea rdi, [rel constN]` for strings (used for show's type-tag
  arg).
- `get_show_format_string(&Type)` — produces the exact tag strings seen in
  `.expected` files: `` `(IntType)` ``, `` `(FloatType)` ``, `` `(BoolType)` ``,
  and (for hw10's array cases) presumably an array/element format descriptor —
  **must locate and read this function in full** (grep `fn get_show_format_string`)
  before implementing `show`, since array show-strings encode rank/element
  type and are not simply `(ArrayType ...)` — check an actual array `.expected`
  (e.g. `ok/050.jpl.expected` for `[1,2]`) to get the literal string.
- `generate_command` `CommandType::Show` (~L366-385): sequence is
  `pad_shadow_with(expr)` (reserves alignment slot sized to the expr's type,
  pushed/popped around) → `generate_expression` → `insert_string_constant` for
  the tag → `lea rsi,[rsp]` → `call _show` → `add rsp, <expr size>` →
  `unpad_shadow`. Port this exactly; it's the outermost driver for every hw10
  test.
- `generate_expression` dispatch (~L640-900+): `Int`, `Float`, `Bool` literal
  cases (~L654-676, look these up precisely — includes the `Bool` true/false
  as int constants 1/0 per L668/676), `Unop` (~L685-703: int→`neg`, float→
  xmm subtract-from-zero via `movsd`/`pxor`/`subsd` with explicit
  `sub rsp,8`/`add rsp,8` shuffling because the value lives on the integer
  stack but needs an xmm round-trip, bool→`xor rax,1`), `Binop` (~L707-834):
  - short-circuit `&&`/`||` via a `.jumpN` label and `self.jump_counter`
    (global, monotonically increasing across the whole program — must match
    numbering exactly if any hw10 fuzzer file uses `&&`/`||`; grep for it,
    likely none in hw10 but will matter for hw11+).
  - `*` int constant-folding "optimization" gated on `self.optimization_level
    > 0` (i.e. `-O1`). **At default `-O0`, hw10's base `.expected` files skip
    this branch entirely** — confirm `optimization_level` defaults to 0 in the
    Typst port and only implement the optimized path when/if `-O1` tests show
    up (hw10 likely has no `.expected.opt` files — verify with
    `find grader/hw10 -name '*.opt'`; if none, skip the optimization branch
    for now, note as future work for whichever hw introduces `-O1` grading).
  - generic path: **evaluate `right` before `left`** (this is why `show 6*5`
    emits const0=5 (right) before const1=6 (left) — verified against
    `ok/017.jpl.expected`), then dispatch on `expression.resolved_type`:
    `generate_int_op`/`generate_float_op`/`generate_bool_op` (the latter keyed
    also on `left.resolved_type` to know int-vs-float-vs-bool comparison
    codegen).
  - `generate_int_op` (~L1675-1740): `+`/`-`/`*` are `pop rax; pop r10; <op>
    rax,r10; push rax` (note operand order: rax is the *left* value since it
    was pushed last → popped first; r10 is *right*). `/`/`%` include a
    divide-by-zero assert (`self.assert(asm_function,"jne","divide by zero"/
    "mod by zero")` — must port `assert` helper, ~L340-364, which calls
    `_fail_assertion` with a message constant) before `cqo`/`idiv r10`,
    picking `rax`(quotient)/`rdx`(remainder).
  - `generate_float_op` (~L1742+) and `generate_bool_op` (~L1780+) — **read
    both in full**, not yet excerpted here; float ops almost certainly load
    both operands into `xmm0`/`xmm1` via `movsd` from the integer-stack
    slots, `addsd`/`subsd`/`mulsd`/`divsd`, `fmod` case likely calls the
    extern `_fmod` (note `pad_shadow()` is called specially for `%` on floats
    at L712-714 — that padding call happens *before* evaluating operands, so
    order matters when porting).
  - `ArrayLiteral` (~L836-861): elements generated **in reverse** (`.rev()`),
    then `mov rdi, <total bytes>`, `pad_shadow()`/`call _jpl_alloc`/
    `unpad_shadow()`, then a reverse-order copy loop from stack into the
    allocated block (`mov r10,[rsp+i*8]` / `mov [rax+i*8],r10`), `add rsp,
    <bytes>`, then push the resulting `(pointer, length)` pair — pointer
    first via `push rax` then length via a fresh `mov rax,<len>; push rax`
    (i.e. on the shadow/value stack, **length ends up on top of pointer**;
    confirm this matches how `show` and later indexing expect array layout —
    cross-check against `runtime.md` in typst_memes if it documents array
    ABI).
- `generate_assembly` top-level driver (~L261-303): fixed header block
  (`global`/`extern` lines — copy verbatim, it's identical across all
  `.expected` files including hw10's), `jpl_main`/`_jpl_main` labels, prelude
  (`push rbp; mov rbp,rsp; push r12; mov r12,rbp` + shadow slot for r12),
  per-command loop, then postlude stack cleanup (`if stack_size > 8: add
  rsp, stack_size-8` — this is the "very sussy" catch-all for any leftover
  shadow entries, needed because `Show` should already balance to 0 but the
  code defensively cleans up), `pop r12; pop rbp; ret`. Then final
  `format!(...)` interleaves the fixed extern header, `section .data` (one
  line per constant via `Display` on `AssemblyValue`, prefixed
  `\nconst{i}: `), `section .text`, and `main_function.body`.

## 3. Proposed Typst module structure
New file `typst_memes/asm.typ`, mirroring how `wat.typ` is wired into
`compiler.typ`'s `compile()` (see `compiler.typ:270 #let compile(source,
backend: "wasmgc")`). Since Typst has no mutable structs/objects, model
`AsmFunction` as a plain dictionary threaded through pure functions returning
updated copies (or, cheaper given Typst content-array patterns already used
elsewhere in this repo — check how `wat.typ` accumulates instructions — likely
via array concatenation), e.g.:

```
#let new-asm-function() = (body: "", stack-size: 0, shadow-stack: (), assert-stack: ())
#let push-instruction(f, s) = (..f, body: f.body + "\n\t" + s)
#let push-instructions(f, ss) = ss.fold(f, push-instruction)
#let push-label(f, s) = (..f, body: f.body + "\n" + s + ":")
#let push-comment(f, s) = (..f, body: f.body + "\n\t; " + s)
#let add-shadow-type(f, t) = (..f, shadow-stack: f.shadow-stack + (("item", t),), stack-size: f.stack-size + type-size(t))
#let remove-shadow(f) = { find last "item" from the back, splice it out, subtract size }
#let pad-shadow(f) = ...
```

Constant pool: since Typst functions are pure, thread a `(gen, f)` pair (or a
top-level mutable-via-state pattern already used elsewhere in compiler.typ —
check if `check()`/typechecker already uses a similar threaded-state
approach, e.g. `checked.globals`/`checked.infer`, and mirror that style for
consistency) holding `data-section: array` + `constants: dict from
value-key to const-name`. Value key must distinguish Number vs String and,
for floats, must format via Typst's float-to-string in a way that matches
Rust's `format!("{:?}", f64)` — **this is a real risk area**: Rust's `{:?}`
debug format for f64 prints the shortest round-trippable decimal (e.g. `1.0`
stays `1.0`, `999999999.999` stays as-is, `-0.0` prints as `-0.0`). Typst's
`str(float)` formatting must be checked against every hw10 float literal
sample (`ok/004.jpl` through `ok/010.jpl`, `023`, `028`, etc.) to confirm it
doesn't need extra massaging (trailing `.0`, exponent form thresholds, etc.)
— note the grader's `normalize_asm.py` `redo_float` already re-parses both
sides as Python `float()` for `const*: dq` lines and reformats via Python's
`str(float(...))`, which *may* absorb minor formatting differences between
Rust's and Typst's float-to-string — verify empirically rather than
hand-matching Rust's exact algorithm.

Top-level entry point: `#let generate-asm(checked) = { ... }` taking the
`check()` output (typed AST + env, same shape `format-typed` already
consumes at compiler.typ:266) and returning the final assembly string, wired
into `compile()` as `backend: "asm"` (or reuse `"wat"`/`-s` flag's dispatch —
check `cli-query.typ`/`jplc` mode mapping: `-s`/`--assembly` currently maps to
`mode=wat`; repoint it to the new backend once ready, or add a transitional
flag so both backends coexist during bring-up).

## 4. Key gotchas (ranked by how likely to cause silent mismatches)
1. **Right-before-left evaluation order** for all binops (confirmed above) —
   easy to get backwards since it's counter-intuitive.
2. **Constant interning is lazy + order-sensitive + deduped**, not a
   pre-pass. A naive Typst port that pre-collects all literals in AST
   pre-order will produce wrong `constN` numbering/reuse. Must intern exactly
   at the point the reference code calls `insert_*_constant`.
3. **`remove_shadow` pops the last `Item` from the back, not literally
   `pop_back`** — if any `Padding` entries are interleaved (from `pad_shadow`
   calls for alignment before `_show`/`_jpl_alloc`/`_fail_assertion` calls),
   a naive stack pop will desync the shadow bookkeeping. Since shadow-stack
   comments are stripped by the grader, this bookkeeping *only* matters if it
   feeds back into emitted instructions (e.g. `stack_size` driving `add rsp,
   N` amounts) — audit every place `stack_size` (not just the comment) is
   read to know how much this actually matters for hw10 specifically (the
   postlude `add rsp, stack_size - 8` in `generate_assembly` is one such
   place, and `pad_shadow`'s 16-byte alignment decision is another, directly
   changing whether a `sub rsp, 8; padding` line appears in output — this
   *does* affect grading).
4. **16-byte stack alignment padding before calls** (`_show`, `_jpl_alloc`,
   `_fail_assertion`) is data-dependent (`stack_size % 16 == 8`) — every
   `call` site's padding must be derived from the live running stack_size at
   that exact point, meaning the Typst port cannot hardcode alignment and
   must carry stack_size through in lock-step with the reference.
5. **Array element copy/push order** — elements pushed in reverse for
   `_jpl_alloc` copy, but need to double check whether the *first* codegen
   pass (`for element in elements.iter().rev()`) means element 0 ends up
   deepest or shallowest on the value stack before the copy loop; get this
   from a concrete nested-array `.expected` (`ok/056.jpl.expected`) rather
   than reasoning about it abstractly.
6. **`optimization_level`** — confirm default is 0 and hw10 has no `.opt`
   fixtures (`find grader/hw10 -name '*.expected.opt'`) before spending time
   on the `*`-folding optimization branch.
7. **Show format-string exact text** for arrays/nested arrays — must be
   pulled verbatim from `get_show_format_string`'s source, not guessed.
8. **Bool literal true/false as int constants 1/0** sharing the *same*
   constant pool as actual int literals — a `show true` and a `show 1`
   appearing in the same file will/won't collide depending on whether the
   Rust code truly funnels both through `insert_int_constant` with the same
   `AssemblyValue::Number` key (per L668/676 it does: `insert_int_constant(...,
   1)` / `insert_int_constant(..., 0)`) — this means `show true` and a
   separately occurring `show 1` in the same program **reuse** const N, per
   the interning rule in gotcha #2. Verify against a mixed fixture if one
   exists in `ok-fuzzer*`.

## 5. Ordered implementation checklist
1. Read `get_show_format_string`, `generate_float_op`, `generate_bool_op`,
   `assert()` helper, and the string-escaping rules for `db \`...\`, 0`
   (backtick-delimited — confirm no escaping needed for the fixed tag
   strings) in full from `/tmp/assembly.rs` (or re-fetch via `git show
   origin/main:src/assembly.rs`) — this plan excerpted but did not fully
   transcribe those bodies.
2. Stand up `asm.typ` skeleton: shadow-stack/AsmFunction dict helpers,
   constant-pool threading, the fixed header/prelude/postlude string
   templates, wired to accept `checked` from `compiler.typ:check()`.
3. Implement literal-only expressions first (Int/Float/Bool + Unop) and get
   `ok/001.jpl` through `ok/012.jpl` byte-matching via
   `python3 grader/grader.py test --hw 10 --dir <typst_memes> --part ok --test 001` equivalent (check exact CLI for single-test selection, or just run
   the small handful manually with `make run` + diff against `.expected`).
4. Add int/float/bool binops (`generate_int_op`/`float_op`/`bool_op`),
   validate against `013`-`047`.
5. Add array literals (`048`-`056`), including nested arrays.
6. Run full hw10 (`ok`, `ok-fuzzer1`, `ok-fuzzer2`, `ok-fuzzer12`) to 100%.
7. Regression-check hw1-9 still pass (this backend swap must not affect
   earlier phases; if `-s`/`wat` mode is being repointed, make sure no
   earlier assignment's test suite depends on the old WasmGC `-s` output).

## Open items for the next engineer
- This plan does not yet capture `generate_float_op`, `generate_bool_op`,
  `assert()`, or `get_show_format_string` bodies verbatim — read them before
  coding, they're short (~30-60 lines combined) but load-bearing for exact
  text match.
- Confirm whether `compiler.typ`'s existing typed-AST node shapes
  (`format-typed-expr` etc., compiler.typ:222-266) already carry
  `resolved_type` per node in a form directly usable by codegen, or whether
  a separate annotation pass is needed first.
