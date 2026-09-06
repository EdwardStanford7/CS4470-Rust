# Refactor plan: JPL compiler magnum opus

Written against the codebase as of the hw10-15 x86 port landing (`asm.typ`
present, `emit-asm` wired into `compiler.typ`'s `compile()`). Do not start
any of this until all 15 assignments are green — every step below assumes a
passing baseline you can diff against.

## 1. Current-state findings (why this needs a pass)

- `compiler.typ` (274 lines) is lexer + parser-duplicate + typechecker +
  four separate AST pretty-printers, all in one file, written in a
  maximally-terse one-line-per-function style (multiple `if/else if` chains
  crammed onto single lines). It reads more like golfed code than a
  compiler.
- **Dead code**: `legacy-parse` (compiler.typ:111-170, ~60 lines) is a
  complete second hand-rolled parser that nothing calls — `check()` uses
  `parse()` from `parser.typ` exclusively. Delete it; it's a trap for
  future readers who'll assume it's live.
- **Duplication**: `format-expr`/`format-command` (untyped AST printers,
  lines 79-108) and `format-typed-expr`/`format-typed-command` (lines
  222-265) are near-identical tree walks that differ only by whether a
  type annotation is spliced in. Same shape duplicated for `format-lvalue`
  usage, `format-type`. This should be one generic pretty-printer
  parameterized on "annotate node with type or not."
- **Two parsers, one file each**: `parser.typ` (production, index-passing
  style: every function takes/returns `(tokens, pos)` pairs manually) and
  the dead `legacy-parse` closure-based one in `compiler.typ`. Only one
  should exist post-refactor.
- **No shared IR**: `wat.typ`'s `emit-wat` and `asm.typ`'s `emit-asm` each
  walk the *typed surface AST* directly and each re-derive expression
  types via `checked.infer` on every node, redundantly, per backend. Any
  lowering decision (e.g. how array-loop desugars, how struct field
  offsets are computed) is duplicated between two backends with no shared
  vocabulary.
- **Ad hoc errors**: `failure(phase, line, col, message)` is used for
  lex/parse, but every typechecker error is a bare `panic("type error: "
  + ...)` with no source position at all — inconsistent, and a real
  regression vs. lexer/parser diagnostics. `asm.typ`/`wat.typ` presumably
  inherit this (codegen-time panics with no position).
- **Naming**: mix of kebab-case Typst idioms (`format-typed-expr`) and
  abbreviations (`gs` for "gen state" in `asm.typ`, `n()`/`die()`/`at()`/
  `eat()` one-letter-ish helpers in `parser.typ`). Fine locally, inconsistent
  globally — no single naming convention document.
- **CLI plumbing** (`jplc`, `cli-query.typ`, `query.typ`, `main.typ`) is
  four tiny files doing one job (shell mode-flag parsing -> typst query ->
  jq unwrap) split in a way that's hard to trace. Could be one documented
  `cli.typ` + the unavoidable shell shim.

## 2. Target module structure

Typst's import model is just `#import "path.typ": names` — no package
manager needed for a single project, so one file per phase is natural and
cheap:

```
lexer.typ         tokenize(source) -> tokens        (pure, no parser coupling)
ast.typ           node constructors + tags, shared vocabulary + doc comments
parser.typ        tokens -> ast (keep current index-passing style; it's
                   already idiomatic-enough Typst since the language
                   forbids captured mutation)
types.typ         type representations + type-name/same-type/normalized-type
                   + show-type-string (currently duplicated between
                   compiler.typ's format-type and asm.typ's show-type-string)
checker.typ       check(ast) -> typed program (name/type checking only,
                   no formatting mixed in)
diagnostics.typ   single `error(phase, pos, message)` + a pretty renderer
                   (source snippet + caret) reused by every phase
ir.typ            NEW: a small lowered IR that both backends target -
                   see §3
printer.typ       the AST/typed-AST pretty printers (format-ast,
                   format-typed), written ONCE as a generic tree-walk
                   parameterized by "with-types: bool"
backend-wasmgc.typ  (rename of wat.typ) ir.typ -> WAT text
backend-x86.typ     (rename of asm.typ) ir.typ -> NASM text
compiler.typ      thin orchestration only: lex/parse/check/compile,
                   re-exporting the public API described in README.md
cli-query.typ, query.typ, main.typ, jplc   unchanged in spirit, but
                   consolidate query.typ + cli-query.typ into one
                   cli.typ if both are ever needed at once
```

`compiler.typ` should shrink to under ~40 lines: imports + the phase
pipeline + `compile()`. All real logic moves to its owning module.

## 3. Shared IR (the highest-value structural change)

Both backends currently re-walk the typed surface AST and re-infer types
per node. Introduce a `lower(checked) -> ir` pass in `ir.typ` that:

- Resolves every expression's type once (reusing `checked.infer` a single
  time per node, not per backend), producing IR nodes carrying their type
  directly (`(tag: "binop", op: "+", ty: "int", left: .., right: ..)`)
  instead of requiring `infer` to be re-run downstream.
- Desugars surface constructs that both backends handle identically:
  array-loop/sum-loop bound evaluation order, dot-access field index
  resolution (struct field name -> integer offset, computed once here
  instead of independently in wat.typ's struct type lookup and asm.typ's
  field-offset math), and lvalue-to-binding resolution.
- Leaves genuinely backend-specific concerns (WasmGC's `(ref $obj)` boxing
  vs. x86's shadow-stack byte layout) to each backend, since those
  *should* differ — the point isn't to unify codegen, it's to unify the
  parts that are accidentally duplicated (type resolution, field offsets,
  loop bound arity checks).

This is the one piece of real design work in the refactor, not just a
file-shuffle. Do it after both backends are green and stable so you have
two working implementations to compare against when designing the IR shape
(the IR should be "what wat.typ and asm.typ both already compute
redundantly," discovered by diffing their logic, not invented from
scratch).

## 4. Idiomatic Typst patterns

Adopt, everywhere:
- **Tagged dictionaries** for every node kind (`(tag: "...", ...)`) —
  already the convention, keep it, but centralize the list of valid tags
  and their required fields in `ast.typ` as documentation (a comment table
  is fine; Typst has no static typing to enforce it, so the comment *is*
  the contract).
- **One constructor per tag**, not a generic `node(tag, ..fields)` splat
  that accepts anything — replace `node("binary", (op: x.op), (left: ..),
  (right: ..))` call sites with `ast.binary(op: .., left: .., right: ..)`
  functions that name their required fields, so a missing field is a
  Typst named-argument error at the call site instead of a silently
  incomplete dictionary discovered later at codegen time.
- **Explicit recursion helpers over closures-capturing-mutable-state**
  where possible — `parser.typ`'s pass-`p`-explicitly style is *correct*
  for this language (Typst forbids mutating captured locals in nested
  functions) and should be the template for any stateful traversal, not
  something to "fix" into a more imperative shape.
- **No one-line if/else-if chains spanning 300+ columns.** Every
  multi-branch dispatch on `.tag` should be one branch per line, each
  branch short enough to read without horizontal scrolling. This is pure
  formatting discipline but matters enormously for "beautiful."

Eliminate:
- The generic untyped `node()` splat constructor (see above).
- Duplicate type-name/formatting logic between `compiler.typ` and
  `asm.typ` (`format-type` vs `show-type-string` compute the same thing
  in different syntax) — one function in `types.typ`, both backends
  import it.
- Dead code (`legacy-parse` and anything else the hw10-15 port fork left
  behind as scaffolding — grep for `legacy` and any commented-out blocks
  before starting).

## 5. Diagnostics design

One function, `diagnostics.typ`:

```
#let error(phase, pos, message, source: none) = { .. }
```

- Always takes a `(line, col)` position (lexer/parser already have this;
  extend the typechecker's `infer`/`check` to thread source positions
  through AST nodes — they already carry `line`/`col` on tokens, so
  propagate them onto AST nodes at parse time if not already done, and
  onto typed-AST nodes at check time).
- Single rendering: `"<phase> error at <line>:<col>: <message>"` for the
  CLI (matches current `failure()` format, keep it — the grader parses
  exactly this shape via `Compilation failed: <message>`), but *also*
  offer a rich renderer (source line + caret) for interactive/PDF use via
  `main.typ`, since Typst can typeset a source snippet with a caret
  trivially (this is one of the "impressive as software" wins — see §7).
- Every phase (lex, parse, check, both backends) raises through this one
  function. No more bare `panic("type error: ...")` calls without
  position.

## 6a. Step 3 status (done, with one scope adjustment)

`diagnostics.typ` now exists (`error(phase, message, pos: none)` +
`type-error(message, pos: none)` convenience wrapper). `compiler.typ`'s
`failure()` and `parser.typ`'s `die()` both route through it, byte-identical
output preserved. All ~64 bare `panic("type error: ...")` call sites in
`compiler.typ`'s `check()` (both `infer-raw` and `annotate` — note these are
two near-duplicate implementations of the same rules, a pre-existing
duplication from the hw15 perf fix, out of scope for this step) now go
through `type-error(...)`.

**Deferred**: full `(line, col)` position-threading onto AST nodes. AST
nodes currently carry no position fields at all (only tokens do), and every
node is built via `n(tag, ..fields)` one-liners in `parser.typ` (~40 call
sites, each a dense single-line expression). Adding positions now means
touching every one of those call sites; step 5 (named constructors) touches
the exact same call sites again. Doing position-threading twice is wasted
risk — fold it into step 5 instead: when replacing `n("binary", (op:x.op),
(left:..), (right:..))` with `ast.binary(op:.., left:.., right:..)`, have
each named constructor take/attach a position in the same pass. Confirmed
this is *not* grading-risk-sensitive either way: the grader only checks the
accept/reject boolean (via jplc's "Compilation failed"/"succeeded" framing),
never diagnostic message text, so there's no rush driven by test correctness
— only by the "beautiful" goal in §7.1.

Verified: 3310/3310 (100%) after this step.

## 6. Refactor ordering (never break green)

Work in small, independently-testable steps, running `make -C grader
test-hwN DIR=<repo> PART=all` for N=1..15 after *every* step:

1. Delete dead code (`legacy-parse`) — pure deletion, zero behavior change,
   run full suite once to confirm nothing secretly depended on it.
2. Extract `types.typ` (type-name/same-type/normalized-type/show-type
   unification) — mechanical move + one shared formatter, update both
   `compiler.typ` and `asm.typ`/`wat.typ` call sites. Full suite.
3. Extract `diagnostics.typ`, thread positions through typechecker errors
   (this is the riskiest step — touches every `panic` site in `check()`).
   Do it function-by-function (infer's branches, then bind, then the
   top-level command loop), running the hw2-9 typecheck-focused parts
   after each chunk since those are cheapest to iterate on.
4. Split `compiler.typ` into `lexer.typ` / `ast.typ` / `checker.typ` /
   `printer.typ`, keeping `compiler.typ` as the re-exporting facade the
   CLI and README already document. Full suite after the split compiles
   at all, then again after confirming output bytes are unchanged (diff
   compiler output on a few sample files before/after, not just grader
   pass/fail, since a passing-but-differently-worded error could slip by
   grader normalization).
5. Replace the generic `node()` splat with named constructors in
   `ast.typ`, updating every call site in `parser.typ`. Mechanical but
   large diff — do it with a script/find-replace per tag, verify via full
   suite.
6. **DONE, with a scope correction — see ir-design.md §6a for full detail.**
   `layout.typ` was extracted (type-size/show-type-string/struct-field-layout,
   ~30 lines out of asm.typ, real readability win). The larger `ir.typ`/
   `lower()` shared-IR was investigated and deliberately NOT built: its two
   founding premises don't hold against the actual finished backends —
   field-offset computation was never duplicated (wat.typ resolves fields by
   index, not byte offset; only asm.typ needs byte layout at all), and the
   "shared evaluation order" idea is actively wrong, not just unimplemented
   (asm.typ's ordering quirks exist only to byte-match a historical
   reference compiler's fixtures for grading; wat.typ has no such
   constraint and correctly does plain left-to-right evaluation — a shared
   canonical order would misimplement one backend or the other). Verified
   3310/3310 after the layout.typ extraction.
7. Formatting/naming pass last, once structure is settled — reflowing the
   dense one-liners into readable multi-line branches, standardizing
   kebab-case naming, adding the doc-comment table in `ast.typ`.

## 7. What "magnum opus" concretely means here

Pick 3-5, don't try to do all of them if time is short — ranked by
impact-to-effort:

1. **Source-positioned diagnostics with a caret-annotated snippet.**
   Since this already renders through Typst, `main.typ`'s PDF driver can
   show a genuinely nice compiler-error page (line, gutter, caret, message)
   — something no CLI compiler bothers with unless it's clang/rustc-tier.
   This is unique to a Typst-hosted compiler and worth highlighting.
2. **A clean IR pretty-printer** (`ir.typ` should have a `format-ir`
   alongside `lower`, mirroring how the typed-AST already has
   `format-typed`) so `-i`/`--intermediate` can show the real shared
   lowering, not just a stub.
3. **One playground `.typ` file** that runs the whole pipeline over a
   folder of `.jpl` files and renders a typeset report (pass/fail, timing,
   generated-assembly diff against `.expected` when available) as a PDF —
   turns the grader's fuzzer corpora into a visual regression dashboard,
   which is a genuinely novel thing to do with a course compiler.
4. **Exhaustive fuzz-corpus reuse**: wire `make test` to also replay every
   `grader/hw*/ok-fuzzer*` and `fail-fuzzer*` input through all phases
   (not just the WasmGC smoke test it does today) as a local regression
   suite independent of having the grader repo path handy — currently
   `test.sh` only checks 3 hand-picked samples.
5. **Symmetry between the two backends' documentation**: `runtime.md`
   currently documents only the WasmGC host ABI; add the x86 runtime ABI
   (the `extern _jpl_alloc`/`_read_image`/etc. contract) alongside it so
   the doc reflects both real backends, not just the original one.

Do not consider the refactor "done" just because grader tests stay green
— green tests are the *constraint* the refactor must preserve, not the
*goal* the refactor is aiming at. The goal is: a reader unfamiliar with
the codebase can open `compiler.typ`, see a ~40-line pipeline, follow one
import into `checker.typ` or `backend-x86.typ`, and understand that phase
completely without needing the other phases' context.
