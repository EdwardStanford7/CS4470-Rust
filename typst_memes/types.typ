// Shared type-representation utilities used by the checker (compiler.typ)
// and the C intermediate backend (c.typ). A JPL type is either a bare
// string ("int"/"float"/"bool"/"void"/a struct name) or a tagged
// dictionary: (tag: "named", name: ..) for an unresolved struct reference,
// or (tag: "array-type", element: .., rank: ..) for an array type.
//
// Leaf module: no imports, so compiler.typ/asm.typ/c.typ/wat.typ can all
// depend on it without creating an import cycle.

// Canonical display name for a type, e.g. "int", "rgba", "float[,]" for a
// rank-2 float array.
#let type-name(t) = if type(t) == str {
  t
} else if t.tag == "array-type" {
  type-name(t.element) + "[" + "," * (t.rank - 1) + "]"
} else if t.tag == "named" {
  t.name
} else {
  t.tag
}

#let same-type(a, b) = type-name(a) == type-name(b)

// Collapses a `(tag: "named", name: ..)` reference (and any array-type
// wrapping one) down to the bare struct-name string, and a scalar-kind
// dictionary (tag: "int"/"float"/"bool"/"void") down to its tag string.
// Idempotent on already-normalized types.
#let normalized-type(t) = if type(t) == dictionary and t.tag == "named" {
  t.name
} else if type(t) == dictionary and t.tag in ("int", "float", "bool", "void") {
  t.tag
} else if type(t) == dictionary and t.tag == "array-type" {
  (tag: "array-type", element: normalized-type(t.element), rank: t.rank)
} else {
  t
}
