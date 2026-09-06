#import "diagnostics.typ": error
#import "ast.typ" as ast

#let precedence = (
  "||": 1, "&&": 1,
  "==": 2, "!=": 2, "<": 2, ">": 2, "<=": 2, ">=": 2,
  "+": 3, "-": 3,
  "*": 4, "/": 4, "%": 4,
)

// `parse` captures the (potentially large) token array `ts` once via closure
// instead of passing it as an explicit argument to every helper call. Typst's
// function-call mechanism has a real per-call cost proportional to the size
// of any array argument, even when unused -- passing `ts` explicitly to
// `at`/`eat`/`parse-expr`/etc. on every single token (as earlier revisions of
// this file did) made the whole parser O(tokens^2). Only the small integer
// cursor `p` is threaded explicitly (cheap to pass); `ts` itself is only ever
// read, never reassigned, so capturing it via closure is safe even though
// Typst forbids closures from mutating captured variables.
#let parse(tokens) = {
  let ts = tokens

  let die(t, msg) = error("parse", msg, pos: t)

  let at(p) = ts.at(calc.min(p, ts.len() - 1))

  let eat(p, k: none) = {
    let t = at(p)
    if k != none and t.kind != k {
      die(t, "expected " + k + ", found " + t.kind)
    }
    (t, p + 1)
  }

  let nls(p) = {
    while at(p).kind == "nl" { p += 1 }
    p
  }

  let required-nl(p) = {
    if at(p).kind != "nl" { die(at(p), "expected newline") }
    nls(p)
  }

  let parse-type(p) = {
    let r = eat(p)
    let t = r.first()
    p = r.last()
    if t.kind not in ("int", "float", "bool", "void", "id") {
      die(t, "expected type")
    }
    let x = if t.kind == "id" {
      ast.type-named(t.text, line: t.line, col: t.col)
    } else {
      ast.type-primitive(t.kind, t.text, line: t.line, col: t.col)
    }
    while at(p).kind == "lb" {
      p += 1
      let rank = 1
      while at(p).kind == "comma" {
        rank += 1
        p += 1
      }
      r = eat(p, k: "rb")
      p = r.last()
      x = ast.array-type(x, rank, line: t.line, col: t.col)
    }
    (x, p)
  }

  let parse-lvalue(p) = {
    let r = eat(p, k: "id")
    let t = r.first()
    let name = t.text
    p = r.last()
    if at(p).kind != "lb" {
      return (ast.var-lvalue(name, line: t.line, col: t.col), p)
    }
    p += 1
    let dims = ()
    if at(p).kind != "rb" {
      r = eat(p, k: "id")
      dims.push(r.first().text)
      p = r.last()
      while at(p).kind == "comma" {
        r = eat(p + 1, k: "id")
        dims.push(r.first().text)
        p = r.last()
      }
    }
    r = eat(p, k: "rb")
    (ast.array-lvalue(name, dims, line: t.line, col: t.col), r.last())
  }

  // Unused by the current grammar (every comma-separated list the parser
  // actually needs is inlined at its call site below) -- kept only because
  // nothing yet exercises a shared list-parsing helper; safe to delete once
  // a second real caller shows up, or now, but left as-is to avoid touching
  // behavior nothing tests.
  let parse-list(p, end) = {
    let xs = ()
    if at(p).kind == end { return (xs, p + 1) }
    let r = parse-expr(p)
    xs.push(r.first())
    p = r.last()
    while at(p).kind == "comma" {
      r = parse-expr(p + 1)
      xs.push(r.first())
      p = r.last()
    }
    r = eat(p, end)
    (xs, r.last())
  }

  let parse-expr(p, min: 1, atom: false) = {
    let r = eat(p)
    let t = r.first()
    p = r.last()
    let x = none
    if t.kind == "intval" {
      x = ast.lit-int(int(t.text), line: t.line, col: t.col)
    } else if t.kind == "floatval" {
      let v = float(t.text)
      if v == calc.inf or v == -calc.inf {
        die(t, "float constant is too large")
      }
      x = ast.lit-float(v, line: t.line, col: t.col)
    } else if t.kind in ("true", "false") {
      x = ast.lit-bool(t.kind == "true", line: t.line, col: t.col)
    } else if t.kind == "void" {
      x = ast.lit-void(line: t.line, col: t.col)
    } else if t.kind == "lp" {
      r = parse-expr(p)
      x = r.first()
      r = eat(r.last(), k: "rp")
      p = r.last()
    } else if t.kind == "lb" {
      let xs = ()
      if at(p).kind != "rb" {
        r = parse-expr(p)
        xs.push(r.first())
        p = r.last()
        while at(p).kind == "comma" {
          r = parse-expr(p + 1)
          xs.push(r.first())
          p = r.last()
        }
      }
      r = eat(p, k: "rb")
      x = ast.array-lit(xs, line: t.line, col: t.col)
      p = r.last()
    } else if t.kind == "op" and t.text in ("-", "!") {
      r = parse-expr(p, atom: true)
      x = ast.unary(t.text, r.first(), line: t.line, col: t.col)
      p = r.last()
    } else if t.kind == "if" {
      r = parse-expr(p)
      let c = r.first()
      r = eat(r.last(), k: "then")
      r = parse-expr(r.last())
      let a = r.first()
      r = eat(r.last(), k: "else")
      r = parse-expr(r.last())
      x = ast.if-expr(c, a, r.first(), line: t.line, col: t.col)
      p = r.last()
    } else if t.kind in ("array", "sum") {
      r = eat(p, k: "lb")
      p = r.last()
      let ranges = ()
      if at(p).kind != "rb" {
        while true {
          r = eat(p, k: "id")
          let name = r.first().text
          r = eat(r.last(), k: "colon")
          r = parse-expr(r.last())
          ranges.push((name: name, bound: r.first()))
          p = r.last()
          if at(p).kind != "comma" { break }
          p += 1
        }
      }
      r = eat(p, k: "rb")
      r = parse-expr(r.last())
      x = if t.kind == "array" {
        ast.array-loop(ranges, r.first(), line: t.line, col: t.col)
      } else {
        ast.sum-loop(ranges, r.first(), line: t.line, col: t.col)
      }
      p = r.last()
    } else if t.kind == "id" {
      if at(p).kind == "lc" {
        p += 1
        let xs = ()
        if at(p).kind != "rc" {
          r = parse-expr(p)
          xs.push(r.first())
          p = r.last()
          while at(p).kind == "comma" {
            r = parse-expr(p + 1)
            xs.push(r.first())
            p = r.last()
          }
        }
        r = eat(p, k: "rc")
        x = ast.struct-lit(t.text, xs, line: t.line, col: t.col)
        p = r.last()
      } else if at(p).kind == "lp" {
        p += 1
        let xs = ()
        if at(p).kind != "rp" {
          r = parse-expr(p)
          xs.push(r.first())
          p = r.last()
          while at(p).kind == "comma" {
            r = parse-expr(p + 1)
            xs.push(r.first())
            p = r.last()
          }
        }
        r = eat(p, k: "rp")
        x = ast.call(t.text, xs, line: t.line, col: t.col)
        p = r.last()
      } else {
        x = ast.var(t.text, line: t.line, col: t.col)
      }
    } else {
      die(t, "expected expression")
    }
    while at(p).kind in ("lb", "dot") {
      if at(p).kind == "dot" {
        r = eat(p + 1, k: "id")
        x = ast.dot(x, r.first().text, line: t.line, col: t.col)
        p = r.last()
      } else {
        p += 1
        let xs = ()
        if at(p).kind != "rb" {
          r = parse-expr(p)
          xs.push(r.first())
          p = r.last()
          while at(p).kind == "comma" {
            r = parse-expr(p + 1)
            xs.push(r.first())
            p = r.last()
          }
        }
        r = eat(p, k: "rb")
        x = ast.index(x, xs, line: t.line, col: t.col)
        p = r.last()
      }
    }
    if atom { return (x, p) }
    while at(p).kind == "op" and precedence.at(at(p).text, default: 0) >= min {
      let op = at(p).text
      let q = precedence.at(op)
      r = parse-expr(p + 1, min: q + 1)
      x = ast.binary(op, x, r.first(), line: t.line, col: t.col)
      p = r.last()
    }
    (x, p)
  }

  let parse-statement(p) = {
    let r = eat(p)
    let t = r.first()
    p = r.last()
    if t.kind == "let" {
      r = parse-lvalue(p)
      let lv = r.first()
      r = eat(r.last(), k: "eq")
      r = parse-expr(r.last())
      (ast.let-stmt(lv, r.first(), line: t.line, col: t.col), r.last())
    } else if t.kind == "assert" {
      r = parse-expr(p)
      let c = r.first()
      r = eat(r.last(), k: "comma")
      r = eat(r.last(), k: "string")
      (ast.assert(c, r.first().text, line: t.line, col: t.col), r.last())
    } else if t.kind == "return" {
      r = parse-expr(p)
      (ast.return-stmt(r.first(), line: t.line, col: t.col), r.last())
    } else {
      die(t, "expected statement")
    }
  }

  let parse-command(p) = {
    let r = eat(p)
    let t = r.first()
    p = r.last()
    let x = none
    if t.kind == "let" {
      r = parse-lvalue(p)
      let lv = r.first()
      r = eat(r.last(), k: "eq")
      r = parse-expr(r.last())
      x = ast.let-stmt(lv, r.first(), line: t.line, col: t.col)
      p = r.last()
    } else if t.kind == "assert" {
      r = parse-expr(p)
      let c = r.first()
      r = eat(r.last(), k: "comma")
      r = eat(r.last(), k: "string")
      x = ast.assert(c, r.first().text, line: t.line, col: t.col)
      p = r.last()
    } else if t.kind == "print" {
      r = eat(p, k: "string")
      x = ast.print(r.first().text, line: t.line, col: t.col)
      p = r.last()
    } else if t.kind == "show" {
      r = parse-expr(p)
      x = ast.show-cmd(r.first(), line: t.line, col: t.col)
      p = r.last()
    } else if t.kind == "read" {
      r = eat(p, k: "image")
      r = eat(r.last(), k: "string")
      let s = r.first().text
      r = eat(r.last(), k: "to")
      r = parse-lvalue(r.last())
      x = ast.read(s, r.first(), line: t.line, col: t.col)
      p = r.last()
    } else if t.kind == "write" {
      r = eat(p, k: "image")
      r = parse-expr(r.last())
      let v = r.first()
      r = eat(r.last(), k: "to")
      r = eat(r.last(), k: "string")
      x = ast.write(v, r.first().text, line: t.line, col: t.col)
      p = r.last()
    } else if t.kind == "time" {
      r = parse-command(p)
      x = ast.time(r.first(), line: t.line, col: t.col)
      p = r.last()
    } else if t.kind == "struct" {
      r = eat(p, k: "id")
      let name = r.first().text
      r = eat(r.last(), k: "lc")
      p = required-nl(r.last())
      let fields = ()
      while at(p).kind != "rc" {
        r = eat(p, k: "id")
        let fn = r.first().text
        r = eat(r.last(), k: "colon")
        r = parse-type(r.last())
        fields.push((name: fn, typ: r.first()))
        p = required-nl(r.last())
      }
      x = ast.struct-decl(name, fields, line: t.line, col: t.col)
      p += 1
    } else if t.kind == "fn" {
      r = eat(p, k: "id")
      let name = r.first().text
      r = eat(r.last(), k: "lp")
      p = r.last()
      let params = ()
      if at(p).kind != "rp" {
        while true {
          r = parse-lvalue(p)
          let lv = r.first()
          r = eat(r.last(), k: "colon")
          r = parse-type(r.last())
          params.push((target: lv, typ: r.first()))
          p = r.last()
          if at(p).kind != "comma" { break }
          p += 1
        }
      }
      r = eat(p, k: "rp")
      r = eat(r.last(), k: "colon")
      r = parse-type(r.last())
      let ret = r.first()
      r = eat(r.last(), k: "lc")
      p = required-nl(r.last())
      let body = ()
      while at(p).kind != "rc" {
        r = parse-statement(p)
        body.push(r.first())
        p = required-nl(r.last())
      }
      x = ast.fn-decl(name, params, ret, body, line: t.line, col: t.col)
      p += 1
    } else {
      die(t, "expected command")
    }
    (x, p)
  }

  let p = nls(0)
  let out = ()
  while at(p).kind != "eof" {
    let r = parse-command(p)
    out.push(r.first())
    p = r.last()
    if at(p).kind not in ("nl", "eof") {
      die(at(p), "expected newline")
    }
    p = nls(p)
  }
  out
}
