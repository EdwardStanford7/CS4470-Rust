// Type layout: byte sizes and struct field offsets on the JPL evaluation
// stack. Used by the x86 backend (asm.typ), which represents every value as
// a fixed-size run of stack slots. The WasmGC backend (wat.typ) has no
// analogous need: GC structs/arrays are sized and offset by the wasm engine
// itself, and field access there is by declaration-order INDEX (see
// wat.typ's `dot` case, `fs.position(...)`), not by byte offset — so there
// is no cross-backend duplication to eliminate here, just organization: this
// was previously private to asm.typ, factored out for its own readability
// and so a future backend needing byte layout doesn't reinvent it.

// s-expression rendering of a type, e.g. `(ArrayType (IntType) 1)` — used in
// assert-failure message strings emitted by the x86 backend.
#let show-type-string(t, structs) = {
  if type(t) == str {
    if t == "int" { "(IntType)" } else if t == "float" { "(FloatType)" } else if t == "bool" { "(BoolType)" } else if t == "void" { "(VoidType)" } else {
      "(TupleType " + structs.at(t).map(f => show-type-string(f.typ, structs)).join(" ") + ")"
    }
  } else if t.tag == "array-type" {
    "(ArrayType " + show-type-string(t.element, structs) + " " + str(t.rank) + ")"
  } else if t.tag == "named" {
    show-type-string(t.name, structs)
  } else {
    show-type-string(t.tag, structs)
  }
}

// size in bytes of a value of type t as represented on the JPL evaluation stack
#let type-size(t, structs) = {
  if type(t) == str {
    if t in ("int", "float", "bool", "void") { 8 } else {
      let fs = structs.at(t)
      fs.map(f => type-size(f.typ, structs)).sum(default: 0)
    }
  } else if t.tag == "array-type" {
    8 + 8 * t.rank
  } else if t.tag == "named" {
    type-size(t.name, structs)
  } else {
    8
  }
}

// (name, ty, offset, size) for every field of a struct, computed once.
// offset is the byte offset of the field within the struct's stack
// representation (fields before it in declaration order, summed).
#let struct-field-layout(struct-name, structs) = {
  let fs = structs.at(struct-name)
  let offset = 0
  let out = ()
  for f in fs {
    let size = type-size(f.typ, structs)
    out.push((name: f.name, ty: f.typ, offset: offset, size: size))
    offset += size
  }
  out
}

// layout of one named field, by name, within a struct.
#let field-layout(struct-name, field-name, structs) = {
  let fields = struct-field-layout(struct-name, structs)
  fields.at(fields.position(f => f.name == field-name))
}
