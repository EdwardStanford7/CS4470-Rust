#import "compiler.typ": format-lex, format-ast, format-typed, check, compile
#import "c.typ": emit-c
#let path=sys.inputs.at("program")
#let mode=sys.inputs.at("mode",default:"wat")
#let opt=int(sys.inputs.at("opt",default:"0"))
#let source=read(path)
#let output = if mode == "lex" {
  format-lex(source)
} else if mode == "wat" {
  compile(source, backend: "x86", opt: opt)
} else if mode == "parse" {
  format-ast(source)
} else if mode == "typecheck" {
  format-typed(source)
} else if mode == "c" {
  emit-c(check(source))
} else {
  panic("unknown mode")
}
#metadata(output) <compiler-output>
