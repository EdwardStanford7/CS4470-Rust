# WasmGC backend test suite

Run with `../../test-wasmgc.sh` from this directory's parent, or via `make test`.

- `ok/*.jpl` + matching `*.jpl.expected`: valid programs. Each is compiled to
  WasmGC text via `query.typ` (`backend: "wasmgc"`, independent of the `-s`
  CLI flag which now targets the x86 backend), parsed and validated with
  `wasm-tools --features gc`, then **actually executed** with Node.js 22+
  (native WasmGC support) via `run-wasm.mjs`, which implements the host ABI
  from `../../runtime.md` in JS. Real stdout is diffed against `.expected`.
- `structural-only/*.jpl`: valid programs that only get parse+validate
  checking, not execution — currently just image I/O programs, since
  `read_image`/`write_image` are stubbed as no-ops/null in the JS harness
  (no real PNG codec), so running them traps on the null dereference.
- `reject/*.jpl`: programs that must fail compilation (parse or type errors).
  Checked via `../../jplc -t <file>`, which exercises the shared front end
  regardless of backend. Verifies the compiler fails cleanly (a diagnostic,
  not an uncaught crash).
- `known-gaps/*.jpl`: valid JPL the front end accepts but the WasmGC backend
  cannot yet codegen (currently: `array-loop`/`sum-loop` comprehensions —
  `wat.typ`'s `gen` has no case for them and panics with "expression not
  representable yet: array-loop"). Tracked separately so the main suite
  stays green; move a file to `ok/` once the backend grows support.
