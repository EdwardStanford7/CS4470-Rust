#let q(s) = "\"" + s.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n") + "\""
#let safe(n) = n.replace(regex("[^A-Za-z0-9_.$]"), "_")
#let wasm-type(t) = {let n=if type(t)==str{t}else{t.tag};if n=="int"{"i64"}else if n=="float"{"f64"}else if n in ("bool","void"){"i32"}else if n=="array-type"{"(ref null $obj)"}else if n=="named"{"(ref null $struct_"+safe(t.name)+")"}else if type(t)==str{"(ref null $struct_"+safe(t)+")"}else{"(ref null $obj)"}}
#let types(env) = { let d=(:); for (k,v) in env { d.insert(k,v.typ) }; d }

#let emit-wat(checked) = {
  let out="(module\n"
  let ln(s)="  "+s+"\n"
  out += ln("(type $string (array (mut i8)))")
  out += ln("(type $dims (array (mut i64)))")
  out += ln("(type $data (array (mut (ref null eq))))")
  out += ln("(type $obj (struct (field (mut (ref null $data))) (field (mut (ref null $dims)))))")
  out += ln("(type $i64box (struct (field i64)))")
  out += ln("(type $f64box (struct (field f64)))")
  out += ln("(type $i32box (struct (field i32)))")
  out += ln("(import \"jpl\" \"print\" (func $print (param (ref $string))))")
  out += ln("(import \"jpl\" \"fail\" (func $fail (param (ref $string))))")
  out += ln("(import \"jpl\" \"show_i64\" (func $show_i64 (param i64)))")
  out += ln("(import \"jpl\" \"show_f64\" (func $show_f64 (param f64)))")
  out += ln("(import \"jpl\" \"show_bool\" (func $show_bool (param i32)))")
  out += ln("(import \"jpl\" \"show_ref\" (func $show_ref (param (ref null eq))))")
  out += ln("(import \"jpl\" \"time\" (func $clock (result f64)))")
  out += ln("(import \"jpl\" \"read_image\" (func $read_image (param (ref $string)) (result (ref null $obj))))")
  out += ln("(import \"jpl\" \"write_image\" (func $write_image (param (ref null $obj)) (param (ref $string))))")
  for name in ("sqrt","exp","sin","cos","tan","asin","acos","atan","log"){out += ln("(import \"jpl\" \""+name+"\" (func $fn_"+name+" (param f64) (result f64)))")}
  for name in ("pow","atan2"){out += ln("(import \"jpl\" \""+name+"\" (func $fn_"+name+" (param f64) (param f64) (result f64)))")}
  out += ln("(import \"jpl\" \"to_float\" (func $fn_to_float (param i64) (result f64)))")
  out += ln("(import \"jpl\" \"to_int\" (func $fn_to_int (param f64) (result i64)))")
  for kv in checked.structs.pairs() { out += ln("(type $struct_"+safe(kv.first())+" (struct "+kv.last().map(f=>"(field (mut "+wasm-type(f.typ)+"))").join(" ")+"))") }
  let string-ref(s)={let cs=s.clusters();"(array.new_fixed $string "+str(cs.len())+" "+cs.map(c=>"(i32.const "+str(c.to-unicode())+")").join(" ")+")"}
  let box(code,t)={let n=if type(t)==str{t}else{t.tag};if n=="int"{"(struct.new $i64box "+code+")"}else if n=="float"{"(struct.new $f64box "+code+")"}else if n in ("bool","void"){"(struct.new $i32box "+code+")"}else{code}}
  let unbox(code,t)={let n=if type(t)==str{t}else{t.tag};if n=="int"{"(struct.get $i64box 0 (ref.cast (ref $i64box) "+code+"))"}else if n=="float"{"(struct.get $f64box 0 (ref.cast (ref $f64box) "+code+"))"}else if n in ("bool","void"){"(struct.get $i32box 0 (ref.cast (ref $i32box) "+code+"))"}else{"(ref.cast "+wasm-type(t)+" "+code+")"}}
  let gen(x, env) = {
    if x.tag == "int" {
      "(i64.const " + str(x.value) + ")"
    } else if x.tag == "float" {
      "(f64.const " + str(x.value) + ")"
    } else if x.tag == "bool" {
      "(i32.const " + if x.value { "1" } else { "0" } + ")"
    } else if x.tag == "void" {
      "(i32.const 0)"
    } else if x.tag == "var" {
      "(local.get " + env.at(x.name).name + ")"
    } else if x.tag == "unary" {
      let a = gen(x.value, env)
      if x.op == "!" {
        "(i32.eqz " + a + ")"
      } else if (checked.infer)(x, types(env)) == "float" {
        "(f64.neg " + a + ")"
      } else {
        "(i64.sub (i64.const 0) " + a + ")"
      }
    } else if x.tag == "binary" {
      let a = gen(x.left, env)
      let b = gen(x.right, env)
      let t = (checked.infer)(x.left, types(env))
      let pre = if t == "float" { "f64" } else if t == "bool" { "i32" } else { "i64" }
      let ops = (
        "+": "add", "-": "sub", "*": "mul",
        "/": if pre == "f64" { "div" } else { "div_s" },
        "%": if pre == "f64" { "rem" } else { "rem_s" },
        "==": "eq", "!=": "ne",
        "<": if pre == "f64" { "lt" } else { "lt_s" },
        ">": if pre == "f64" { "gt" } else { "gt_s" },
        "<=": if pre == "f64" { "le" } else { "le_s" },
        ">=": if pre == "f64" { "ge" } else { "ge_s" },
        "&&": "and", "||": "or",
      )
      "(" + pre + "." + ops.at(x.op) + " " + a + " " + b + ")"
    } else if x.tag == "if" {
      let ty = wasm-type((checked.infer)(x, types(env)))
      "(if (result " + ty + ") " + gen(x.cond, env) + " (then " + gen(x.yes, env) + ") (else " + gen(x.no, env) + "))"
    } else if x.tag == "call" {
      let args = if x.args.len() > 0 { " " + x.args.map(e => gen(e, env)).join(" ") } else { "" }
      "(call $fn_" + safe(x.name) + args + ")"
    } else if x.tag == "struct-lit" {
      "(struct.new $struct_" + safe(x.name) + " " + x.items.map(e => gen(e, env)).join(" ") + ")"
    } else if x.tag == "array" {
      let et = (checked.infer)(x.items.first(), types(env))
      let items = x.items.map(e => box(gen(e, env), et)).join(" ")
      "(struct.new $obj (array.new_fixed $data " + str(x.items.len()) + " " + items + ") (array.new_fixed $dims 1 (i64.const " + str(x.items.len()) + ")))"
    } else if x.tag == "dot" {
      let bt = (checked.infer)(x.base, types(env))
      let sn = if type(bt) == str { bt } else { bt.name }
      let fs = checked.structs.at(sn)
      let idx = fs.position(f => f.name == x.field)
      "(struct.get $struct_" + safe(sn) + " " + str(idx) + " " + gen(x.base, env) + ")"
    } else if x.tag == "index" {
      let base = gen(x.base, env)
      let bt = (checked.infer)(x.base, types(env))
      let idx = gen(x.indices.first(), env)
      for pair in x.indices.enumerate().slice(1) {
        let dim = "(array.get $dims (struct.get $obj 1 " + base + ") (i32.const " + str(pair.first()) + "))"
        idx = "(i64.add (i64.mul " + idx + " " + dim + ") " + gen(pair.last(), env) + ")"
      }
      let raw = "(array.get $data (struct.get $obj 0 " + base + ") (i32.wrap_i64 " + idx + "))"
      unbox(raw, (checked.infer)(x, types(env)))
    } else {
      panic("WAT backend: expression not representable yet: " + x.tag)
    }
  }
  // Declares a fresh local for `target` (and, for array-lvalues, one local per
  // destructured dimension), recording each in `env` and appending the `(local ...)`
  // decl plus a `(local.set ...)` init to `locals`/`body`.
  let declare-target(target, value-name, value-typ, env, locals, body) = {
    env.insert(target.name, (name: value-name, typ: value-typ))
    if target.tag == "array-lvalue" {
      for pair in target.dims.enumerate() {
        let dn = "$d_" + safe(pair.last()) + "_" + str(locals.len())
        env.insert(pair.last(), (name: dn, typ: "int"))
        locals.push("(local " + dn + " i64)")
        let dim = "(array.get $dims (struct.get $obj 1 (local.get " + value-name + ")) (i32.const " + str(pair.first()) + "))"
        body.push("(local.set " + dn + " " + dim + ")")
      }
    }
    (env: env, locals: locals, body: body)
  }
  let emit-body = (commands, initial-env: (:), initial-locals: (), is-main: false) => {
    let env = initial-env
    let locals = initial-locals
    let fresh = locals.len()
    let body = ()
    for c in commands {
      if c.tag == "let" {
        let t = (checked.infer)(c.value, types(env))
        let vn = "$v_" + safe(c.target.name) + "_" + str(fresh)
        fresh += 1
        let value = gen(c.value, env)
        locals.push("(local " + vn + " " + wasm-type(t) + ")")
        body.push("(local.set " + vn + " " + value + ")")
        let r = declare-target(c.target, vn, t, env, locals, body)
        env = r.env; locals = r.locals; body = r.body
      } else if c.tag == "read" {
        let t = (tag: "array-type", element: "rgba", rank: 2)
        let vn = "$v_" + safe(c.target.name) + "_" + str(fresh)
        fresh += 1
        locals.push("(local " + vn + " (ref null $obj))")
        body.push("(local.set " + vn + " (call $read_image " + string-ref(c.source) + "))")
        let r = declare-target(c.target, vn, t, env, locals, body)
        env = r.env; locals = r.locals; body = r.body
      } else if c.tag == "write" {
        body.push("(call $write_image " + gen(c.value, env) + " " + string-ref(c.dest) + ")")
      } else if c.tag == "print" {
        body.push("(call $print " + string-ref(c.message) + ")")
      } else if c.tag == "assert" {
        body.push("(if (i32.eqz " + gen(c.cond, env) + ") (then (call $fail " + string-ref(c.message) + ") unreachable))")
      } else if c.tag == "show" {
        let t = (checked.infer)(c.value, types(env))
        let suffix = if t == "int" { "i64" } else if t == "float" { "f64" } else if t == "bool" { "bool" } else { "ref" }
        body.push("(call $show_" + suffix + " " + gen(c.value, env) + ")")
      } else if c.tag == "return" {
        body.push("(return " + gen(c.value, env) + ")")
      } else if c.tag == "time" {
        body.push("(drop (call $clock))")
      }
    }
    (locals: locals, body: body)
  }
  for c in checked.ast.filter(x => x.tag == "fn") {
    let fenv = (:)
    let params = ()
    for p in c.params {
      let vn = "$p_" + safe(p.target.name)
      fenv.insert(p.target.name, (name: vn, typ: p.typ))
      params.push("(param " + vn + " " + wasm-type(p.typ) + ")")
    }
    let b = emit-body(c.body, initial-env: fenv)
    let returns-void = (if type(c.returns) == str { c.returns } else { c.returns.tag }) == "void"
    let tail = if returns-void { " (i32.const 0)" } else { "" }
    let sig = "(func $fn_" + safe(c.name) + " " + params.join(" ") + " (result " + wasm-type(c.returns) + ") "
    out += ln(sig + b.locals.join(" ") + " " + b.body.join(" ") + tail + ")")
  }
  let main = emit-body(checked.ast.filter(x => x.tag not in ("fn", "struct")), is-main: true)
  out += ln("(func (export \"jpl_main\") " + main.locals.join(" ") + " " + main.body.join(" ") + ")")
  out + ")\n"
}
