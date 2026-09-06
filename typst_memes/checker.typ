// Name/type checking: parsed AST -> typed program. No formatting mixed in
// (see printer.typ for that).

#import "ast.typ": array-type
#import "diagnostics.typ": type-error
#import "types.typ": type-name, same-type, normalized-type

#let check(program, parse: none) = {
  let ast = if type(program) == str { parse(program) } else { program }
  let globals = (argnum: "int", args: array-type("int", 1))
  let structs = (
    rgba: (
      (name: "r", typ: "float"),
      (name: "g", typ: "float"),
      (name: "b", typ: "float"),
      (name: "a", typ: "float"),
    ),
  )
  let funcs = (:)
  for name in ("sqrt", "exp", "sin", "cos", "tan", "asin", "acos", "atan", "log") {
    funcs.insert(name, (params: ((typ: "float"),), returns: "float"))
  }
  for name in ("pow", "atan2") {
    funcs.insert(name, (params: ((typ: "float"), (typ: "float")), returns: "float"))
  }
  funcs.insert("to_float", (params: ((typ: "int"),), returns: "float"))
  funcs.insert("to_int", (params: ((typ: "float"),), returns: "int"))

  // Pre-scan every top-level declaration (unwrapping `time` wrappers) so struct/function
  // names are visible everywhere below, regardless of declaration order.
  for c in ast {
    let decl = c
    while decl.tag == "time" { decl = decl.command }
    if decl.tag == "struct" {
      if decl.name in structs or decl.name in globals or decl.name in funcs {
        type-error("duplicate struct " + decl.name)
      }
      let seen = ()
      for f in decl.fields {
        if f.name in seen { type-error("duplicate field " + f.name) }
        seen.push(f.name)
      }
      structs.insert(decl.name, decl.fields)
    } else if decl.tag == "fn" {
      if decl.name in funcs or decl.name in globals or decl.name in structs {
        type-error("duplicate function " + decl.name)
      }
      funcs.insert(decl.name, decl)
    }
  }

  // Struct fields may only reference already-declared struct names, and may not recurse
  // (directly or transitively) back into their own struct.
  let contains-name(t, name) = {
    if type(t) == dictionary and t.tag == "named" { t.name == name }
    else if type(t) == dictionary and t.tag == "array-type" { contains-name(t.element, name) }
    else { false }
  }
  for (sn, fs) in structs {
    if sn != "rgba" {
      for f in fs {
        if contains-name(f.typ, sn) { type-error("recursive struct " + sn) }
        let tn = type-name(f.typ)
        if type(f.typ) == dictionary and f.typ.tag == "named" and tn not in structs {
          type-error("unknown struct " + tn)
        }
      }
    }
  }
  let base-name(t) = {
    if type(t) == dictionary and t.tag == "array-type" { base-name(t.element) }
    else if type(t) == dictionary and t.tag == "named" { t.name }
    else { none }
  }
  let reaches(start, current, seen: ()) = {
    if current == start { return true }
    if current in seen or current not in structs { return false }
    let next-seen = seen + (current,)
    for f in structs.at(current) {
      let dep = base-name(f.typ)
      if dep != none and reaches(start, dep, seen: next-seen) { return true }
    }
    false
  }
  for sn in structs.keys() {
    if sn != "rgba" {
      for f in structs.at(sn) {
        let dep = base-name(f.typ)
        if dep != none and reaches(sn, dep) { type-error("recursive struct " + sn) }
      }
    }
  }
  // infer-raw: the original, unmemoized recursive type-derivation logic. Kept verbatim as a
  // fallback for any node that never went through `annotate` below (e.g. a freshly-constructed
  // synthetic node). Every recursive call here is a full re-descent -- this is what makes
  // repeated external queries on the same subtree (once per backend/formatter) quadratic or
  // worse on deep/wide trees, which is why `annotate` exists to compute each node's type once.
  let infer-raw(x, env) = {
    let t = x.tag
    if t in ("int", "float", "bool", "void") {
      return t
    }
    if t == "var" {
      if x.name not in env { type-error("undefined variable " + x.name) }
      return normalized-type(env.at(x.name))
    }
    if t == "unary" {
      let a = infer-raw(x.value, env)
      if x.op == "!" and a != "bool" { type-error("! expects bool") }
      if x.op == "-" and a not in ("int", "float") { type-error("- expects numeric") }
      return a
    }
    if t == "binary" {
      let a = infer-raw(x.left, env)
      let b = infer-raw(x.right, env)
      if not same-type(a, b) { type-error("binary operands differ") }
      if x.op in ("==", "!=", "<", ">", "<=", ">=", "&&", "||") { return "bool" }
      return a
    }
    if t == "if" {
      if infer-raw(x.cond, env) != "bool" { type-error("if condition is not bool") }
      let a = infer-raw(x.yes, env)
      let b = infer-raw(x.no, env)
      if not same-type(a, b) { type-error("if branches differ") }
      return a
    }
    if t == "array" {
      if x.items.len() == 0 { type-error("empty array") }
      let a = infer-raw(x.items.first(), env)
      for e in x.items {
        if not same-type(a, infer-raw(e, env)) { type-error("heterogeneous array") }
      }
      return array-type(a, 1)
    }
    if t == "array-loop" or t == "sum-loop" {
      if x.ranges.len() == 0 { type-error("loop needs at least one binding") }
      let e = env
      let names = ()
      for r in x.ranges {
        let bt = infer-raw(r.bound, env)
        if bt != "int" {
          type-error("loop bound must be int, got " + type-name(bt) + " from " + r.bound.tag)
        }
        if r.name in names { type-error("duplicate loop variable") }
        names.push(r.name)
      }
      for r in x.ranges { e.insert(r.name, "int") }
      let b = infer-raw(x.body, e)
      if t == "sum-loop" {
        if b not in ("int", "float") { type-error("sum body must be numeric") }
        return b
      }
      return array-type(b, x.ranges.len())
    }
    if t == "call" {
      if x.name not in funcs { type-error("undefined function " + x.name) }
      let f = funcs.at(x.name)
      if x.args.len() != f.params.len() { type-error("arity mismatch") }
      for pair in x.args.zip(f.params) {
        if not same-type(infer-raw(pair.first(), env), pair.last().typ) {
          type-error("argument mismatch")
        }
      }
      return normalized-type(f.returns)
    }
    if t == "struct-lit" {
      if x.name not in structs { type-error("undefined struct " + x.name) }
      let fs = structs.at(x.name)
      if x.items.len() != fs.len() { type-error("struct arity mismatch") }
      for pair in x.items.zip(fs) {
        if not same-type(infer-raw(pair.first(), env), pair.last().typ) {
          type-error("struct field mismatch")
        }
      }
      return x.name
    }
    if t == "dot" {
      let sn = type-name(infer-raw(x.base, env))
      if sn not in structs {
        type-error("field access on non-struct, got " + sn + " from " + x.base.tag)
      }
      let fs = structs.at(sn)
      let hit = fs.filter(f => f.name == x.field)
      if hit.len() != 1 { type-error("unknown field " + x.field) }
      return normalized-type(hit.first().typ)
    }
    if t == "index" {
      let a = infer-raw(x.base, env)
      if type(a) != dictionary or a.tag != "array-type" { type-error("indexing non-array") }
      for i in x.indices {
        if infer-raw(i, env) != "int" { type-error("index must be int") }
      }
      if x.indices.len() != a.rank { type-error("wrong number of indices") }
      return a.element
    }
    type-error("unsupported expression " + t)
  }
  // infer: the public, memoized entry point. Every downstream consumer (this file's own
  // command loop below, format-typed-*, wat.typ, asm.typ) calls this the same way as before --
  // it's just O(1) whenever `x` already carries a `ty` field (i.e. came from `checked.ast`,
  // which `annotate` below decorates once), falling back to the full recursive derivation only
  // for un-annotated nodes.
  let infer(x, env) = {
    if type(x) == dictionary and "ty" in x { x.ty } else { infer-raw(x, env) }
  }
  // annotate: single bottom-up pass mirroring infer-raw's exact rules, but reading each
  // child's type from the `ty` field it just attached (O(1)) instead of re-deriving it via a
  // fresh infer-raw descent. Returns a copy of `x` with the same shape plus `ty`, and with
  // every expression-valued child field replaced by its own annotated copy, so the whole tree
  // ends up decorated after one full traversal.
  let annotate(x, env) = {
    let t = x.tag
    if t in ("int", "float", "bool", "void") {
      let x2 = x; x2.insert("ty", t); x2
    } else if t == "var" {
      if x.name not in env { type-error("undefined variable " + x.name) }
      let x2 = x; x2.insert("ty", normalized-type(env.at(x.name))); x2
    } else if t == "unary" {
      let v = annotate(x.value, env)
      let a = v.ty
      if x.op == "!" and a != "bool" { type-error("! expects bool") }
      if x.op == "-" and a not in ("int", "float") { type-error("- expects numeric") }
      let x2 = x; x2.insert("value", v); x2.insert("ty", a); x2
    } else if t == "binary" {
      let l = annotate(x.left, env)
      let r = annotate(x.right, env)
      if not same-type(l.ty, r.ty) { type-error("binary operands differ") }
      let ty = if x.op in ("==","!=","<",">","<=",">=","&&","||") { "bool" } else { l.ty }
      let x2 = x; x2.insert("left", l); x2.insert("right", r); x2.insert("ty", ty); x2
    } else if t == "if" {
      let c = annotate(x.cond, env)
      if c.ty != "bool" { type-error("if condition is not bool") }
      let y = annotate(x.yes, env)
      let n = annotate(x.no, env)
      if not same-type(y.ty, n.ty) { type-error("if branches differ") }
      let x2 = x; x2.insert("cond", c); x2.insert("yes", y); x2.insert("no", n); x2.insert("ty", y.ty); x2
    } else if t == "array" {
      if x.items.len() == 0 { type-error("empty array") }
      let items2 = x.items.map(e => annotate(e, env))
      let a = items2.first().ty
      for it in items2 { if not same-type(a, it.ty) { type-error("heterogeneous array") } }
      let x2 = x; x2.insert("items", items2); x2.insert("ty", array-type(a, 1)); x2
    } else if t == "array-loop" or t == "sum-loop" {
      if x.ranges.len() == 0 { type-error("loop needs at least one binding") }
      let e = env
      let names = ()
      let ranges2 = ()
      for r in x.ranges {
        let b2 = annotate(r.bound, env)
        if b2.ty != "int" { type-error("loop bound must be int, got " + type-name(b2.ty) + " from " + r.bound.tag) }
        if r.name in names { type-error("duplicate loop variable") }
        names.push(r.name)
        let r2 = r; r2.insert("bound", b2); ranges2.push(r2)
      }
      for r in x.ranges { e.insert(r.name, "int") }
      let body2 = annotate(x.body, e)
      let b = body2.ty
      let ty = if t == "sum-loop" {
        if b not in ("int", "float") { type-error("sum body must be numeric") }
        b
      } else {
        array-type(b, x.ranges.len())
      }
      let x2 = x; x2.insert("ranges", ranges2); x2.insert("body", body2); x2.insert("ty", ty); x2
    } else if t == "call" {
      if x.name not in funcs { type-error("undefined function " + x.name) }
      let f = funcs.at(x.name)
      if x.args.len() != f.params.len() { type-error("arity mismatch") }
      let args2 = ()
      for pair in x.args.zip(f.params) {
        let a2 = annotate(pair.first(), env)
        if not same-type(a2.ty, pair.last().typ) { type-error("argument mismatch") }
        args2.push(a2)
      }
      let x2 = x; x2.insert("args", args2); x2.insert("ty", normalized-type(f.returns)); x2
    } else if t == "struct-lit" {
      if x.name not in structs { type-error("undefined struct " + x.name) }
      let fs = structs.at(x.name)
      if x.items.len() != fs.len() { type-error("struct arity mismatch") }
      let items2 = ()
      for pair in x.items.zip(fs) {
        let i2 = annotate(pair.first(), env)
        if not same-type(i2.ty, pair.last().typ) { type-error("struct field mismatch") }
        items2.push(i2)
      }
      let x2 = x; x2.insert("items", items2); x2.insert("ty", x.name); x2
    } else if t == "dot" {
      let b2 = annotate(x.base, env)
      let sn = type-name(b2.ty)
      if sn not in structs { type-error("field access on non-struct, got " + sn + " from " + x.base.tag) }
      let fs = structs.at(sn)
      let hit = fs.filter(f => f.name == x.field)
      if hit.len() != 1 { type-error("unknown field " + x.field) }
      let x2 = x; x2.insert("base", b2); x2.insert("ty", normalized-type(hit.first().typ)); x2
    } else if t == "index" {
      let b2 = annotate(x.base, env)
      let a = b2.ty
      if type(a) != dictionary or a.tag != "array-type" { type-error("indexing non-array") }
      let idx2 = x.indices.map(i => annotate(i, env))
      for i2 in idx2 { if i2.ty != "int" { type-error("index must be int") } }
      if idx2.len() != a.rank { type-error("wrong number of indices") }
      let x2 = x; x2.insert("base", b2); x2.insert("indices", idx2); x2.insert("ty", a.element); x2
    } else {
      type-error("unsupported expression " + t)
    }
  }
  // Walks back down through any `time` wrapper(s) to graft an annotated innermost command
  // back into the original wrapper chain (time's own codegen is untyped, it just recurses).
  let replace-inner(c, new-inner) = {
    if c.tag == "time" {
      let c2 = c
      c2.insert("command", replace-inner(c.command, new-inner))
      c2
    } else {
      new-inner
    }
  }
  let bind = (lv, t, env) => {
    let e = env
    let nt = normalized-type(t)
    e.insert(lv.name, nt)
    if lv.tag == "array-lvalue" {
      if type(nt) != dictionary or nt.tag != "array-type" or nt.rank != lv.dims.len() {
        type-error("array destructuring rank mismatch")
      }
      for d in lv.dims { e.insert(d, "int") }
    }
    e
  }
  let new-ast = ()
  for c in ast {
    let top = c
    while top.tag == "time" { top = top.command }
    let top2 = top
    if top.tag == "let" {
      let target-in-dims = top.target.tag == "array-lvalue" and top.target.name in top.target.dims
      if top.target.name in globals or target-in-dims { type-error("duplicate binding") }
      let v2 = annotate(top.value, globals)
      globals = bind(top.target, v2.ty, globals)
      top2 = top; top2.insert("value", v2)
    }
    else if top.tag == "read" {
      if top.target.name in globals { type-error("duplicate binding") }
      globals = bind(top.target, array-type("rgba", 2), globals)
    }
    else if top.tag=="assert" {
      let cond2 = annotate(top.cond, globals)
      if cond2.ty != "bool" { type-error("assert expects bool") }
      top2 = top; top2.insert("cond", cond2)
    }
    else if top.tag=="show" {
      let v2 = annotate(top.value, globals)
      top2 = top; top2.insert("value", v2)
    }
    else if top.tag=="write" {
      let v2 = annotate(top.value, globals)
      if not same-type(v2.ty,array-type("rgba", 2)){type-error("write expects rgba[,]")}
      top2 = top; top2.insert("value", v2)
    }
    else if top.tag=="fn" {
      let e = globals
      let local = ()
      for p in top.params {
        let names = (p.target.name,) + if p.target.tag == "array-lvalue" { p.target.dims } else { () }
        for name in names {
          if name in local { type-error("duplicate parameter " + name) }
          local.push(name)
        }
        e = bind(p.target, p.typ, e)
      }
      let returned=false
      let body2 = ()
      for s in top.body {
        if s.tag=="let"{
          let names=(s.target.name,)+if s.target.tag=="array-lvalue"{s.target.dims}else{()}
          for name in names{if name in local{type-error("duplicate local "+name)};local.push(name)}
          let v2 = annotate(s.value, e)
          e=bind(s.target,v2.ty,e)
          let s2 = s; s2.insert("value", v2); body2.push(s2)
        } else if s.tag=="assert"{
          let cond2 = annotate(s.cond, e)
          if cond2.ty != "bool" { type-error("assert expects bool") }
          let s2 = s; s2.insert("cond", cond2); body2.push(s2)
        } else if s.tag=="return"{
          returned=true
          let v2 = annotate(s.value, e)
          if not same-type(v2.ty,top.returns){type-error("bad return in "+top.name)}
          let s2 = s; s2.insert("value", v2); body2.push(s2)
        }
      }
      if not returned and type-name(top.returns)!="void"{type-error("missing return in "+top.name)}
      top2 = top; top2.insert("body", body2)
    }
    new-ast.push(replace-inner(c, top2))
  }
  (ast:new-ast, globals:globals, structs:structs, funcs:funcs, infer:infer)
}
