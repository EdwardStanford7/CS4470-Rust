# JPL compiler in Typst

This directory contains a pure Typst implementation of the CS 4470 JPL
front-end and a WebAssembly-GC text (`.wat`) backend.

Typst cannot read arbitrary command-line files or write arbitrary output
files.  Consequently the public interface is a pure function:

```typst
#import "compiler.typ": compile, lex, parse, check
#raw(compile(read("program.jpl"), backend: "wasmgc"), lang: "wat")
```

`main.typ` is a ready-to-use driver. Compile it with:

```sh
typst compile --input program=../t.jpl main.typ program.pdf
```

The generated module imports the small host ABI documented in
`runtime.md`.  It uses GC arrays for JPL arrays and GC structs for boxed JPL
values. Compile emitted text with a WebAssembly toolchain supporting the GC
proposal (for example Binaryen or a recent `wasm-tools`).

The compiler exposes every assignment phase independently: tokenization,
parsing, name/type checking, typed AST formatting, and code generation.
`compile` fails with a source-positioned diagnostic on malformed programs.

Run the local Typst and WasmGC validation checks with `make test`. The
historical assignment interface is available as `make run TEST=file.jpl
FLAGS=-l` (or `-p`, `-t`, and `-s`).
