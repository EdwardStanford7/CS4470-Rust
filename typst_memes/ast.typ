// AST/type node constructors. Nodes are plain tagged dictionaries; every
// valid tag has exactly one named constructor here instead of a generic
// `node(tag, ..fields)` splat, so a missing/misnamed field is a call-site
// error (Typst raises on an unknown/missing named argument) instead of a
// silently incomplete dictionary discovered later at codegen time.
//
// Every constructor accepts `line:`/`col:` (both default to `none`) and
// stores them as `line`/`col` fields on the node. That shape is exactly
// what `diagnostics.typ`'s `error(..., pos: ...)` expects: a node can be
// passed directly as `pos:` since it already carries `line`/`col`.
//
// Tag contract (tag: required fields, beyond line/col):
//   Types:
//     named           name
//     int/float/bool/void (as a TYPE, not a literal)   name
//     array-type      element, rank
//   Lvalues:
//     var-lvalue      name
//     array-lvalue    name, dims
//   Expressions:
//     int/float/bool  value      (as a LITERAL — same tag string as the
//                                 primitive-type nodes above, distinguished
//                                 by field shape/parse context, not tag)
//     void            (no fields, as a literal)
//     array           items
//     unary           op, value
//     if              cond, yes, no
//     array-loop      ranges, body
//     sum-loop        ranges, body
//     struct-lit      name, items
//     call            name, args
//     var             name
//     dot             base, field
//     index           base, indices
//     binary          op, left, right
//   Statements/commands:
//     let             target, value
//     assert          cond, message
//     return          value
//     print           message
//     show            value
//     read            source, target
//     write           value, dest
//     time            command
//     struct          name, fields   (declaration)
//     fn              name, params, returns, body

#let type-named(name, line: none, col: none) = (
  tag: "named", name: name, line: line, col: col,
)
#let type-primitive(kind, name, line: none, col: none) = (
  tag: kind, name: name, line: line, col: col,
)
#let array-type(element, rank, line: none, col: none) = (
  tag: "array-type", element: element, rank: rank, line: line, col: col,
)

#let var-lvalue(name, line: none, col: none) = (
  tag: "var-lvalue", name: name, line: line, col: col,
)
#let array-lvalue(name, dims, line: none, col: none) = (
  tag: "array-lvalue", name: name, dims: dims, line: line, col: col,
)

#let lit-int(value, line: none, col: none) = (
  tag: "int", value: value, line: line, col: col,
)
#let lit-float(value, line: none, col: none) = (
  tag: "float", value: value, line: line, col: col,
)
#let lit-bool(value, line: none, col: none) = (
  tag: "bool", value: value, line: line, col: col,
)
#let lit-void(line: none, col: none) = (
  tag: "void", line: line, col: col,
)
#let array-lit(items, line: none, col: none) = (
  tag: "array", items: items, line: line, col: col,
)
#let unary(op, value, line: none, col: none) = (
  tag: "unary", op: op, value: value, line: line, col: col,
)
#let if-expr(cond, yes, no, line: none, col: none) = (
  tag: "if", cond: cond, yes: yes, no: no, line: line, col: col,
)
#let array-loop(ranges, body, line: none, col: none) = (
  tag: "array-loop", ranges: ranges, body: body, line: line, col: col,
)
#let sum-loop(ranges, body, line: none, col: none) = (
  tag: "sum-loop", ranges: ranges, body: body, line: line, col: col,
)
#let struct-lit(name, items, line: none, col: none) = (
  tag: "struct-lit", name: name, items: items, line: line, col: col,
)
#let call(name, args, line: none, col: none) = (
  tag: "call", name: name, args: args, line: line, col: col,
)
#let var(name, line: none, col: none) = (
  tag: "var", name: name, line: line, col: col,
)
#let dot(base, field, line: none, col: none) = (
  tag: "dot", base: base, field: field, line: line, col: col,
)
#let index(base, indices, line: none, col: none) = (
  tag: "index", base: base, indices: indices, line: line, col: col,
)
#let binary(op, left, right, line: none, col: none) = (
  tag: "binary", op: op, left: left, right: right, line: line, col: col,
)

#let let-stmt(target, value, line: none, col: none) = (
  tag: "let", target: target, value: value, line: line, col: col,
)
#let assert(cond, message, line: none, col: none) = (
  tag: "assert", cond: cond, message: message, line: line, col: col,
)
#let return-stmt(value, line: none, col: none) = (
  tag: "return", value: value, line: line, col: col,
)
#let print(message, line: none, col: none) = (
  tag: "print", message: message, line: line, col: col,
)
// Named `show-cmd`, not `show` — `show` is a reserved keyword in Typst.
#let show-cmd(value, line: none, col: none) = (
  tag: "show", value: value, line: line, col: col,
)
#let read(source, target, line: none, col: none) = (
  tag: "read", source: source, target: target, line: line, col: col,
)
#let write(value, dest, line: none, col: none) = (
  tag: "write", value: value, dest: dest, line: line, col: col,
)
#let time(command, line: none, col: none) = (
  tag: "time", command: command, line: line, col: col,
)
#let struct-decl(name, fields, line: none, col: none) = (
  tag: "struct", name: name, fields: fields, line: line, col: col,
)
#let fn-decl(name, params, returns, body, line: none, col: none) = (
  tag: "fn", name: name, params: params, returns: returns, body: body,
  line: line, col: col,
)
