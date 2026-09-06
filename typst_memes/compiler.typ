// Thin orchestration: wires lexer.typ -> parser.typ -> checker.typ -> a
// backend, and re-exports the public API documented in README.md
// (lex, parse, check, compile, format-lex, format-ast, format-typed).
// All real logic lives in the modules it imports.

#import "lexer.typ": lex, format-lex
// The production parser is purely functional because Typst intentionally
// forbids mutation of variables captured by nested functions.
#import "parser.typ": parse as parse-tokens
#import "checker.typ": check as check-nodes
#import "printer.typ": format-ast-nodes, format-typed-nodes
#import "wat.typ": emit-wat
#import "asm.typ": emit-asm

#let parse(source) = parse-tokens(if type(source) == str { lex(source) } else { source })
#let format-ast(source) = format-ast-nodes(parse(source))

#let check(program) = check-nodes(program, parse: parse)
#let format-typed(source) = format-typed-nodes(check(source))

#let compile(source, backend: "wasmgc", opt: 0) = {
  if backend == "wasmgc" { emit-wat(check(source)) }
  else if backend == "x86" { emit-asm(check(source), opt: opt) }
  else { panic("unknown backend " + backend) }
}
