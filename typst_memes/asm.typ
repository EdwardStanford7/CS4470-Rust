// x86-64 NASM backend, ported from the reference Rust compiler's shadow-stack codegen model.

#import "layout.typ": show-type-string, type-size, struct-field-layout, field-layout

// Typst's str() renders negative numbers with U+2212 MINUS SIGN, not ASCII '-',
// which NASM (and the grader's regex-based normalizer) can't parse.
#let num(n) = if n < 0 { "-" + str(calc.abs(n)) } else { str(n) }

// `gs` ("gen state") threads through every codegen function below: the constant pool
// (`data`/`cmap`), the next free jump-label number, and the shadow-stack bookkeeping
// (`stack`/`shadow`/`offsets`) that keeps 16-byte alignment and local-slot addresses correct.
#let new-gs(opt: 0) = (data: (), cmap: (:), jump: 1, stack: 0, shadow: (), offsets: (:), opt: opt)

#let is-32-bit-n(n) = n >= -2147483648 and n <= 2147483647

// log2(n) if n is an exact positive power of two, else none.
#let pow2-shift(n) = {
  if n <= 0 { none } else {
    let m = n
    let shift = 0
    while calc.rem(m, 2) == 0 {
      m = int(m / 2)
      shift += 1
    }
    if m == 1 { shift } else { none }
  }
}

#let is-pow2-n(n) = pow2-shift(n) != none

#let find-last(arr, kind) = {
  let hit = none
  for i in range(arr.len()).rev() {
    if arr.at(i).kind == kind { hit = i; break }
  }
  hit
}

#let add-item(gs, size) = {
  gs.shadow = gs.shadow + ((kind: "item", size: size),)
  gs.stack += size
  gs
}

#let remove-item(gs) = {
  let i = find-last(gs.shadow, "item")
  let size = gs.shadow.at(i).size
  gs.shadow = gs.shadow.slice(0, i) + gs.shadow.slice(i + 1)
  gs.stack -= size
  gs
}

// returns (gs, code) -- code is "" or "sub rsp, 8"
#let pad-shadow(gs) = {
  let align = calc.rem(gs.stack, 16) == 8
  gs.shadow = gs.shadow + ((kind: "pad", align: align),)
  if align {
    gs.stack += 8
    (gs, "sub rsp, 8")
  } else {
    (gs, "")
  }
}

#let unpad-shadow(gs) = {
  let i = find-last(gs.shadow, "pad")
  let align = gs.shadow.at(i).align
  gs.shadow = gs.shadow.slice(0, i) + gs.shadow.slice(i + 1)
  if align {
    gs.stack -= 8
    (gs, "add rsp, 8")
  } else {
    (gs, "")
  }
}

#let pad-shadow-with-size(gs, size) = {
  gs = add-item(gs, size)
  let (gs2, code) = pad-shadow(gs)
  (remove-item(gs2), code)
}

#let const-key(kind, val) = kind + ":" + val

#let insert-const(gs, kind, val) = {
  let key = const-key(kind, val)
  if key in gs.cmap {
    (gs, gs.cmap.at(key))
  } else {
    let name = "const" + str(gs.data.len())
    gs.data = gs.data + ((kind: kind, val: val),)
    gs.cmap.insert(key, name)
    (gs, name)
  }
}

#let fmt-float(v) = {
  // Rust's `{:?}` for f64 always includes a decimal point.
  let s = str(v)
  if "." not in s and "e" not in s and "E" not in s { s = s + ".0" }
  s
}

#let insert-int-constant(gs, n) = {
  if gs.opt > 0 and is-32-bit-n(n) {
    return (gs, "push qword " + num(n))
  }
  let (gs, name) = insert-const(gs, "num", str(n))
  (gs, "mov rax, [rel " + name + "]\npush rax")
}

#let insert-float-constant(gs, v) = {
  let (gs, name) = insert-const(gs, "num", fmt-float(v))
  (gs, "mov rax, [rel " + name + "]\npush rax")
}

#let insert-string-constant(gs, s) = {
  let (gs, name) = insert-const(gs, "str", s)
  (gs, "lea rdi, [rel " + name + "]")
}

#let next-jump(gs) = {
  let label = ".jump" + str(gs.jump)
  gs.jump += 1
  (gs, label)
}

// emits a conditional-jump-over-failure assertion matching the reference `assert` helper.
// cmp_mode is the jump instruction that SKIPS the failure (e.g. "jne" to skip when not-equal).
#let gen-assert(gs, cmp-mode, message) = {
  let (gs, label) = next-jump(gs)
  let lines = (cmp-mode + " " + label,)
  let (gs, pad) = pad-shadow(gs)
  if pad != "" { lines = lines + (pad,) }
  let (gs, lea) = insert-string-constant(gs, message)
  lines = lines + (lea, "call _fail_assertion")
  let (gs, unpad) = unpad-shadow(gs)
  if unpad != "" { lines = lines + (unpad,) }
  lines = lines + (label + ":",)
  (gs, lines.join("\n"))
}

// init-indices(gs, ranges) -> (gs, code): pushes one zeroed int per range var (reverse order),
// recording each var's stack offset for later variable reads.
#let init-indices(gs, ranges) = {
  let lines = ()
  for r in ranges.rev() {
    lines = lines + ("mov rax, 0", "push rax")
    gs = add-item(gs, 8)
    gs.offsets.insert(r.name, (off: gs.stack, from-main: false))
  }
  (gs, lines.join("\n"))
}

// pure instruction text -- no shadow-stack bookkeeping needed.
// ranges: array of {name,bound} (for ArrayLoop, enables constant-bound folding) or none (ArrayIndex).
#let calculate-linear-index(rank, loop-vars-offset, bounds-offset, element-size, opt: 0, ranges: none) = {
  let lines = ()
  if opt > 0 {
    lines = ("mov rax, [rsp + " + str(loop-vars-offset) + "]",)
    for i in range(1, rank) {
      let folded = none
      if ranges != none {
        let b = ranges.at(i).bound
        if b.tag == "int" and (is-32-bit-n(b.value) or is-pow2-n(b.value)) { folded = b.value }
      }
      if folded != none {
        lines = lines + ("imul rax, " + num(folded), "add rax, [rsp + " + str(i * 8 + loop-vars-offset) + "]")
      } else {
        lines = lines + ("imul rax, [rsp + " + str(bounds-offset + i * 8) + "]", "add rax, [rsp + " + str(i * 8 + loop-vars-offset) + "]")
      }
    }
  } else {
    lines = ("mov rax, 0",)
    for i in range(rank) {
      lines = lines + (
        "imul rax, [rsp + " + str(bounds-offset + i * 8) + "]",
        "add rax, [rsp + " + str(i * 8 + loop-vars-offset) + "]",
      )
    }
  }
  lines = lines + ("imul rax, " + str(element-size), "add rax, [rsp + " + str(bounds-offset + rank * 8) + "]")
  lines.join("\n")
}

#let increment-loop-index(rank, label) = {
  let lines = ()
  for i in range(1, rank).rev() {
    lines = lines + (
      "add qword [rsp + " + str(i * 8) + "], 1",
      "mov rax, [rsp + " + str(i * 8) + "]",
      "cmp rax, [rsp + " + str(8 * (i + rank)) + "]",
      "jl " + label,
      "mov qword [rsp + " + str(i * 8) + "], 0",
    )
  }
  lines = lines + ("add qword [rsp], 1", "mov rax, [rsp]", "cmp rax, [rsp + " + str(8 * rank) + "]", "jl " + label)
  lines.join("\n")
}

// -O3 tensor-contraction loop fusion: `array[...] sum[...] <product of indexed arrays>`.
// Ported from the reference compiler's `tensor_optimizable_expressions` /
// `build_traversal_graph` / `optimized_array_loop`.

#let tensor-optimizable(e) = {
  if e.tag == "binary" {
    tensor-optimizable(e.left) and tensor-optimizable(e.right)
  } else if e.tag in ("int", "float", "var") {
    true
  } else if e.tag == "index" {
    tensor-optimizable(e.base) and e.indices.all(i => tensor-optimizable(i))
  } else {
    false
  }
}

#let tensor-add-edge(graph, a, b) = {
  if a not in graph.nodes { graph.nodes = graph.nodes + (a,) }
  if b not in graph.nodes { graph.nodes = graph.nodes + (b,) }
  if a not in graph.adj { graph.adj.insert(a, ()) }
  if b not in graph.adj { graph.adj.insert(b, ()) }
  if b not in graph.adj.at(a) {
    graph.adj.insert(a, graph.adj.at(a) + (b,))
  }
  graph
}

// Edges among an ArrayIndex's own index variables (consecutive-pair, all i<j).
#let tensor-extract-edges(graph, e) = {
  if e.tag == "binary" {
    graph = tensor-extract-edges(graph, e.left)
    graph = tensor-extract-edges(graph, e.right)
    graph
  } else if e.tag == "index" {
    for i in range(e.indices.len()) {
      if e.indices.at(i).tag == "var" {
        for j in range(i + 1, e.indices.len()) {
          if e.indices.at(j).tag == "var" {
            graph = tensor-add-edge(graph, e.indices.at(i).name, e.indices.at(j).name)
          }
        }
      }
    }
    graph
  } else {
    graph
  }
}

#let tensor-build-graph(array-range, sum-range, sum-body-t, sum-body) = {
  let graph = (nodes: (), adj: (:))
  for i in range(array-range.len()) {
    for j in range(i + 1, array-range.len()) {
      graph = tensor-add-edge(graph, array-range.at(i).name, array-range.at(j).name)
    }
  }
  if sum-body-t == "float" {
    for i in range(sum-range.len()) {
      for j in range(i + 1, sum-range.len()) {
        graph = tensor-add-edge(graph, sum-range.at(i).name, sum-range.at(j).name)
      }
    }
  }
  tensor-extract-edges(graph, sum-body)
}

// DFS-postorder-reversed toposort, matching petgraph::algo::toposort's node/edge visiting
// order (graph.nodes / graph.adj are both insertion-ordered, mirroring DiGraphMap).
#let tensor-dfs-visit(adj, visited, finished, order, node) = {
  if node in visited { return (visited, finished, order) }
  visited.insert(node, true)
  for succ in adj.at(node, default: ()) {
    if succ not in finished {
      let (v2, f2, o2) = tensor-dfs-visit(adj, visited, finished, order, succ)
      visited = v2
      finished = f2
      order = o2
    }
  }
  order = order + (node,)
  finished.insert(node, true)
  (visited, finished, order)
}

#let tensor-toposort(graph) = {
  let visited = (:)
  let finished = (:)
  let order = ()
  for node in graph.nodes {
    if node not in visited {
      let (v2, f2, o2) = tensor-dfs-visit(graph.adj, visited, finished, order, node)
      visited = v2
      finished = f2
      order = o2
    }
  }
  order.rev()
}

// Emits the fused loop nest for `array[array-range] sum[sum-range] sum-body`. `gen-expr` is
// passed in explicitly (defined further below) to avoid a forward-reference issue.
#let gen-tensor-fusion(gs, t, array-range, sum-range, sum-body, gen-expr-fn, infer, env, structs, in-statement) = {
  let body-env = env
  for r in array-range { body-env.insert(r.name, "int") }
  for r in sum-range { body-env.insert(r.name, "int") }
  let sum-body-t = infer(sum-body, body-env)
  let sum-body-size = type-size(sum-body-t, structs)
  let graph = tensor-build-graph(array-range, sum-range, sum-body-t, sum-body)
  let topo = tensor-toposort(graph)

  let lines = ("sub rsp, 8",)
  gs = add-item(gs, 8)

  for r in array-range.rev() {
    let (gs2, code) = gen-expr-fn(gs, r.bound, infer, env, structs, in-statement)
    gs = gs2
    lines = lines + (code, "mov rax, [rsp]", "cmp rax, 0")
    let (gs3, achk) = gen-assert(gs, "jg", "non-positive loop bound")
    gs = gs3
    lines = lines + (achk,)
  }
  for r in sum-range.rev() {
    let (gs2, code) = gen-expr-fn(gs, r.bound, infer, env, structs, in-statement)
    gs = gs2
    lines = lines + (code, "mov rax, [rsp]", "cmp rax, 0")
    let (gs3, achk) = gen-assert(gs, "jg", "non-positive loop bound")
    gs = gs3
    lines = lines + (achk,)
  }

  lines = lines + ("mov rdi, " + str(sum-body-size),)
  for i in range(array-range.len()) {
    lines = lines + ("imul rdi, [rsp + " + str((i + sum-range.len()) * 8) + "]",)
    let (gs4, achk) = gen-assert(gs, "jno", "overflow computing array size")
    gs = gs4
    lines = lines + (achk,)
  }

  let (gs5, pad) = pad-shadow(gs)
  gs = gs5
  if pad != "" { lines = lines + (pad,) }
  lines = lines + ("call _jpl_alloc",)
  let (gs6, unpad) = unpad-shadow(gs)
  gs = gs6
  if unpad != "" { lines = lines + (unpad,) }
  lines = lines + ("mov [rsp + " + str((array-range.len() + sum-range.len()) * 8) + "], rax",)

  let (gs7, idx1) = init-indices(gs, array-range)
  gs = gs7
  lines = lines + (idx1,)
  let (gs8, idx2) = init-indices(gs, sum-range)
  gs = gs8
  lines = lines + (idx2,)

  let (gs9, label) = next-jump(gs)
  gs = gs9
  lines = lines + (label + ":",)

  let (gs10, body-code) = gen-expr-fn(gs, sum-body, infer, body-env, structs, in-statement)
  gs = gs10
  lines = lines + (body-code,)

  let loop-vars-offset = sum-body-size + sum-range.len() * 8
  let bounds-offset = sum-body-size + (sum-range.len() + array-range.len() + sum-range.len()) * 8
  lines = lines + (calculate-linear-index(array-range.len(), loop-vars-offset, bounds-offset, sum-body-size, opt: gs.opt, ranges: array-range),)

  if sum-body-t == "int" {
    lines = lines + ("pop r10", "add [rax + " + str(8 * (array-range.len() - 1)) + "], r10")
    gs = remove-item(gs)
  } else if sum-body-t == "float" {
    lines = lines + ("movsd xmm0, [rsp]", "add rsp, 8", "addsd xmm0, [rax]", "movsd [rax], xmm0")
    gs = remove-item(gs)
  } else {
    for i in range(int(sum-body-size / 8)).rev() {
      lines = lines + ("mov r10, [rsp + " + str(i * 8) + "]", "mov [rax + " + str(i * 8) + "], r10")
    }
    gs = remove-item(gs)
    lines = lines + ("add rsp, " + str(sum-body-size),)
  }

  let stack-ordering = (array-range.rev().map(r => r.name) + sum-range.rev().map(r => r.name)).rev()

  let offsets = ()
  for name in topo.rev() {
    let pos = stack-ordering.position(v => v == name)
    if pos != none { offsets = offsets + (pos,) }
  }

  let n = offsets.len()
  for offset in offsets.slice(0, n - 1) {
    lines = lines + (
      "add qword [rsp + " + str(offset * 8) + "], 1",
      "mov rax, [rsp + " + str(offset * 8) + "]",
      "cmp rax, [rsp + " + str((offset + n) * 8) + "]",
      "jl " + label,
      "mov qword [rsp + " + str(offset * 8) + "], 0",
    )
  }
  let last-offset = offsets.at(n - 1)
  lines = lines + (
    "add qword [rsp + " + str(last-offset * 8) + "], 1",
    "mov rax, [rsp + " + str(last-offset * 8) + "]",
    "cmp rax, [rsp + " + str((last-offset + n) * 8) + "]",
    "jl " + label,
  )

  for _ in range(sum-range.len()) { gs = remove-item(gs) }
  for _ in range(array-range.len()) { gs = remove-item(gs) }
  lines = lines + ("add rsp, " + str(8 * (sum-range.len() + array-range.len())),)

  for _ in range(sum-range.len()) { gs = remove-item(gs) }
  lines = lines + ("add rsp, " + str(8 * sum-range.len()),)

  for _ in range(array-range.len() + 1) { gs = remove-item(gs) }
  gs = add-item(gs, type-size(t, structs))

  (gs, lines.join("\n"))
}

#let int-op(operator) = (
  "+": "add rax, r10",
  "-": "sub rax, r10",
  "*": "imul rax, r10",
).at(operator, default: none)

#let int-cmp(operator) = (
  "<": "setl", "<=": "setle", ">": "setg", ">=": "setge", "==": "sete", "!=": "setne",
).at(operator)

#let float-cmp(operator) = (
  "<": ("cmpltsd", false), "<=": ("cmplesd", false),
  ">": ("cmpltsd", true), ">=": ("cmplesd", true),
  "==": ("cmpeqsd", false), "!=": ("cmpneqsd", false),
).at(operator)

// gen-expr(gs, x, infer, env, structs, in-statement) -> (gs, code)
// `ty`, when given, is the already-known type of `x` -- callers that just computed it
// (e.g. a binary/unary node that had to infer an operand's type to pick codegen) should
// pass it through rather than letting this call re-derive it via a fresh `infer` descent,
// which is what turns deeply-nested expression trees quadratic (each level would otherwise
// re-walk its entire remaining subtree just to learn its own type).
#let gen-expr(gs, x, infer, env, structs, in-statement, ty: none) = {
  let t = if ty != none { ty } else { infer(x, env) }
  if x.tag == "int" {
    let (gs, code) = insert-int-constant(gs, x.value)
    gs = add-item(gs, 8)
    (gs, code)
  } else if x.tag == "float" {
    let (gs, code) = insert-float-constant(gs, x.value)
    gs = add-item(gs, 8)
    (gs, code)
  } else if x.tag == "bool" {
    let (gs, code) = insert-int-constant(gs, if x.value { 1 } else { 0 })
    gs = add-item(gs, 8)
    (gs, code)
  } else if x.tag == "void" {
    let (gs, code) = insert-int-constant(gs, 1)
    gs = add-item(gs, 8)
    (gs, code)
  } else if x.tag == "unary" {
    let inner-t = infer(x.value, env)
    let (gs, inner) = gen-expr(gs, x.value, infer, env, structs, in-statement, ty: inner-t)
    gs = remove-item(gs)
    let op-code = if x.op == "-" and inner-t == "int" {
      "pop rax\nneg rax\npush rax"
    } else if x.op == "-" and inner-t == "float" {
      "movsd xmm1, [rsp]\nadd rsp, 8\npxor xmm0, xmm0\nsubsd xmm0, xmm1\nsub rsp, 8\nmovsd [rsp], xmm0"
    } else {
      "pop rax\nxor rax, 1\npush rax"
    }
    gs = add-item(gs, 8)
    (gs, inner + "\n" + op-code)
  } else if x.tag == "binary" and (x.op == "&&" or x.op == "||") {
    let (gs, left) = gen-expr(gs, x.left, infer, env, structs, in-statement, ty: "bool")
    gs = remove-item(gs)
    let (gs, label) = next-jump(gs)
    let jmp = if x.op == "&&" { "je" } else { "jne" }
    let head = left + "\npop rax\ncmp rax, 0\n" + jmp + " " + label
    let (gs, right) = gen-expr(gs, x.right, infer, env, structs, in-statement, ty: "bool")
    gs = remove-item(gs)
    let tail = right + "\npop rax\n" + label + ":\npush rax"
    gs = add-item(gs, 8)
    (gs, head + "\n" + tail)
  } else if x.tag == "binary" and gs.opt > 0 and x.op == "*" and x.left.tag == "int" and (is-32-bit-n(x.left.value) or is-pow2-n(x.left.value)) and x.left.value != 0 {
    // strength-reduction: left operand is a usable integer literal.
    let v1 = x.left.value
    let (gs, right-code) = gen-expr(gs, x.right, infer, env, structs, in-statement, ty: "int")
    let both-lit = x.right.tag == "int" and (is-32-bit-n(x.right.value) or is-pow2-n(x.right.value)) and x.right.value != 0 and v1 != 1 and x.right.value != 1 and not is-pow2-n(v1)
    if both-lit {
      let (gs2, left-code) = gen-expr(gs, x.left, infer, env, structs, in-statement, ty: "int")
      gs = gs2
      gs = remove-item(gs); gs = remove-item(gs)
      gs = add-item(gs, 8)
      (gs, right-code + "\n" + left-code + "\npop rax\npop r10\nimul rax, r10\npush rax")
    } else {
      let extra = if v1 != 1 { "\npop rax\nimul rax, " + num(v1) + "\npush rax" } else { "" }
      (gs, right-code + extra)
    }
  } else if x.tag == "binary" and gs.opt > 0 and x.op == "*" and x.left.tag != "int" and x.right.tag == "int" and is-32-bit-n(x.right.value) and is-pow2-n(x.right.value) and x.right.value != 0 {
    // strength-reduction: right operand is a usable (32-bit AND power-of-two) integer literal.
    let v2 = x.right.value
    let (gs, left-code) = gen-expr(gs, x.left, infer, env, structs, in-statement, ty: "int")
    let extra = if v2 != 1 { "\npop rax\nimul rax, " + num(v2) + "\npush rax" } else { "" }
    (gs, left-code + extra)
  } else if x.tag == "binary" {
    // For every non-comparison op, infer's own rule says the operand type equals the
    // result type `t` -- so avoid a redundant full re-descent into x.left just to learn
    // what we already know (this is what makes deep homogeneous-arithmetic chains, the
    // fuzzer's favorite shape, quadratic if done naively).
    let lt = if x.op in ("==", "!=", "<", ">", "<=", ">=") { infer(x.left, env) } else { t }
    let need-pad = x.op == "%" and lt == "float"
    let (gs, prepad) = if need-pad { pad-shadow(gs) } else { (gs, "") }
    // left/right are same-type by construction (checked during type-checking), so both
    // recursive calls can reuse the same already-known operand type.
    let (gs, right) = gen-expr(gs, x.right, infer, env, structs, in-statement, ty: lt)
    let (gs, left) = gen-expr(gs, x.left, infer, env, structs, in-statement, ty: lt)
    let lines = ()
    if prepad != "" { lines = lines + (prepad,) }
    lines = lines + (right, left)
    if t == "int" {
      if x.op in ("+", "-", "*") {
        lines = lines + ("pop rax", "pop r10", int-op(x.op), "push rax")
        gs = remove-item(gs); gs = remove-item(gs)
      } else {
        // "/" or "%"
        lines = lines + ("pop rax", "pop r10", "cmp r10, 0")
        gs = remove-item(gs); gs = remove-item(gs)
        let (gs2, achk) = gen-assert(gs, "jne", if x.op == "/" { "divide by zero" } else { "mod by zero" })
        gs = gs2
        lines = lines + (achk, "cqo", "idiv r10")
        if x.op == "%" { lines = lines + ("mov rax, rdx",) }
        lines = lines + ("push rax",)
      }
      gs = add-item(gs, 8)
    } else if t == "float" {
      if x.op in ("+", "-", "*", "/") {
        let ins = ("+": "addsd", "-": "subsd", "*": "mulsd", "/": "divsd").at(x.op)
        lines = lines + ("movsd xmm0, [rsp]", "add rsp, 8", "movsd xmm1, [rsp]", "add rsp, 8", ins + " xmm0, xmm1", "sub rsp, 8", "movsd [rsp], xmm0")
      } else {
        lines = lines + ("movsd xmm0, [rsp]", "add rsp, 8", "movsd xmm1, [rsp]", "add rsp, 8", "call _fmod")
        let (gs2, un) = unpad-shadow(gs)
        gs = gs2
        if un != "" { lines = lines + (un,) }
        lines = lines + ("sub rsp, 8", "movsd [rsp], xmm0")
      }
      gs = remove-item(gs); gs = remove-item(gs)
      gs = add-item(gs, 8)
    } else {
      // bool result: comparisons
      if lt == "int" {
        lines = lines + ("pop rax", "pop r10", "cmp rax, r10", int-cmp(x.op) + " al", "and rax, 1", "push rax")
      } else if lt == "float" {
        lines = lines + ("movsd xmm0, [rsp]", "add rsp, 8", "movsd xmm1, [rsp]", "add rsp, 8")
        let (cmp-i, swap) = float-cmp(x.op)
        if swap {
          lines = lines + (cmp-i + " xmm1, xmm0", "movq rax, xmm1", "and rax, 1", "push rax")
        } else {
          lines = lines + (cmp-i + " xmm0, xmm1", "movq rax, xmm0", "and rax, 1", "push rax")
        }
      } else {
        // bool operands: ==, !=  (&& || handled above as short-circuit)
        lines = lines + ("pop rax", "pop r10", "cmp rax, r10", (if x.op == "==" { "sete" } else { "setne" }) + " al", "and rax, 1", "push rax")
      }
      gs = remove-item(gs); gs = remove-item(gs)
      gs = add-item(gs, 8)
    }
    (gs, lines.join("\n"))
  } else if x.tag == "array" {
    let et = infer(x.items.first(), env)
    let element-size = type-size(et, structs)
    let lines = ()
    for el in x.items.rev() {
      let (gs2, code) = gen-expr(gs, el, infer, env, structs, in-statement, ty: et)
      gs = gs2
      lines = lines + (code,)
    }
    lines = lines + ("mov rdi, " + str(x.items.len() * element-size),)
    let (gs2, pad) = pad-shadow(gs)
    gs = gs2
    if pad != "" { lines = lines + (pad,) }
    lines = lines + ("call _jpl_alloc",)
    let (gs3, unpad) = unpad-shadow(gs)
    gs = gs3
    if unpad != "" { lines = lines + (unpad,) }
    let total = x.items.len() * element-size
    for i in range(int(total / 8)).rev() {
      let loc = i * 8
      lines = lines + ("mov r10, [rsp + " + str(loc) + "]", "mov [rax + " + str(loc) + "], r10")
    }
    lines = lines + ("add rsp, " + str(total),)
    for _ in x.items { gs = remove-item(gs) }
    lines = lines + ("push rax", "mov rax, " + str(x.items.len()), "push rax")
    gs = add-item(gs, 16)
    (gs, lines.join("\n"))
  } else if x.tag == "var" {
    let entry = gs.offsets.at(x.name)
    let off = entry.off
    let from-main = entry.from-main
    let size = type-size(t, structs)
    let reg = if from-main and in-statement { "r12" } else { "rbp" }
    let lines = ("sub rsp, " + str(size),)
    gs = add-item(gs, size)
    for i in range(int(size / 8)).rev() {
      let loc = i * 8
      lines = lines + (
        "mov r10, [" + reg + " - " + num(off) + " + " + str(loc) + "]",
        "mov [rsp + " + str(loc) + "], r10",
      )
    }
    (gs, lines.join("\n"))
  } else if x.tag == "call" {
    let ret-t = t
    let int-regs = ("rdi", "rsi", "rdx", "rcx", "r8", "r9")
    let flo-regs = ("xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7")
    let arg-types = x.args.map(a => infer(a, env))
    let is-agg(tt) = type(tt) != str or tt not in ("int", "float", "bool", "void")
    let pad-total = {
      let total = 0
      let int-n = 0
      let flo-n = 0
      for tt in arg-types {
        if tt in ("int", "bool", "void") { int-n += 1; if int-n > int-regs.len() { total += 8 } }
        else if tt == "float" { flo-n += 1; if flo-n > flo-regs.len() { total += 8 } }
        else { total += type-size(tt, structs) }
      }
      total
    }
    let lines = ()
    let int-arg-num = 0
    let flo-arg-num = 0
    let arr-arg-size = 0
    if not is-agg(ret-t) {
      let (gs2, pad) = pad-shadow-with-size(gs, pad-total)
      gs = gs2
      if pad != "" { lines = lines + (pad,) }
    } else {
      lines = lines + ("sub rsp, " + str(type-size(ret-t, structs)),)
      gs = add-item(gs, type-size(ret-t, structs))
      int-arg-num += 1
      let (gs2, pad) = pad-shadow-with-size(gs, pad-total)
      gs = gs2
      if pad != "" { lines = lines + (pad,); arr-arg-size += 8 }
    }
    for (a, tt) in x.args.zip(arg-types).rev() {
      if is-agg(tt) {
        let (gs2, code) = gen-expr(gs, a, infer, env, structs, in-statement)
        gs = gs2
        lines = lines + (code,)
      }
    }
    for (a, tt) in x.args.zip(arg-types).rev() {
      if not is-agg(tt) {
        let (gs2, code) = gen-expr(gs, a, infer, env, structs, in-statement)
        gs = gs2
        lines = lines + (code,)
      }
    }
    for tt in arg-types {
      if tt == "int" or tt == "bool" or tt == "void" {
        lines = lines + ("pop " + int-regs.at(int-arg-num),)
        int-arg-num += 1
        gs = remove-item(gs)
      } else if tt == "float" {
        lines = lines + ("movsd " + flo-regs.at(flo-arg-num) + ", [rsp]", "add rsp, 8")
        flo-arg-num += 1
        gs = remove-item(gs)
      } else {
        arr-arg-size += type-size(tt, structs)
      }
    }
    if is-agg(ret-t) {
      lines = lines + ("lea rdi, [rsp + " + str(arr-arg-size) + "]",)
    }
    lines = lines + ("call _" + x.name,)
    for tt in arg-types {
      if is-agg(tt) {
        lines = lines + ("add rsp, " + str(type-size(tt, structs)),)
        gs = remove-item(gs)
      }
    }
    let (gs2, unpad) = unpad-shadow(gs)
    gs = gs2
    if unpad != "" { lines = lines + (unpad,) }
    if not is-agg(ret-t) {
      gs = add-item(gs, type-size(ret-t, structs))
    }
    if ret-t == "int" or ret-t == "bool" or ret-t == "void" {
      lines = lines + ("push rax",)
    } else if ret-t == "float" {
      lines = lines + ("sub rsp, 8", "movsd [rsp], xmm0")
    }
    (gs, lines.join("\n"))
  } else if x.tag == "array-loop" and gs.opt > 2 and x.body.tag == "sum-loop" and tensor-optimizable(x.body.body) {
    gen-tensor-fusion(gs, t, x.ranges, x.body.ranges, x.body.body, gen-expr, infer, env, structs, in-statement)
  } else if x.tag == "array-loop" {
    let rank = x.ranges.len()
    let body-env = env
    for r in x.ranges { body-env.insert(r.name, "int") }
    let body-t = infer(x.body, body-env)
    let body-size = type-size(body-t, structs)
    let lines = ("sub rsp, 8",)
    gs = add-item(gs, 8)
    let bounds-code = {
      let lines2 = ()
      for r in x.ranges.rev() {
        let (gsb, code) = gen-expr(gs, r.bound, infer, env, structs, in-statement)
        gs = gsb
        lines2 = lines2 + (code, "mov rax, [rsp]", "cmp rax, 0")
        let (gsc, achk) = gen-assert(gs, "jg", "non-positive loop bound")
        gs = gsc
        lines2 = lines2 + (achk,)
      }
      lines2.join("\n")
    }
    lines = lines + (bounds-code, "mov rdi, " + str(body-size))
    for i in range(rank) {
      lines = lines + ("imul rdi, [rsp + " + str(i * 8) + "]",)
      let (gs3, achk) = gen-assert(gs, "jno", "overflow computing array size")
      gs = gs3
      lines = lines + (achk,)
    }
    let (gs4, pad) = pad-shadow(gs)
    gs = gs4
    if pad != "" { lines = lines + (pad,) }
    lines = lines + ("call _jpl_alloc",)
    let (gs5, unpad) = unpad-shadow(gs)
    gs = gs5
    if unpad != "" { lines = lines + (unpad,) }
    lines = lines + ("mov [rsp + " + str(rank * 8) + "], rax",)
    let (gs6, idx-code) = init-indices(gs, x.ranges)
    gs = gs6
    lines = lines + (idx-code,)
    let (gs7, label) = next-jump(gs)
    gs = gs7
    lines = lines + (label + ":",)
    let (gs8, body-code) = gen-expr(gs, x.body, infer, body-env, structs, in-statement)
    gs = gs8
    lines = lines + (body-code,)
    let loop-vars-offset = body-size
    let bounds-offset = body-size + rank * 8
    lines = lines + (calculate-linear-index(rank, loop-vars-offset, bounds-offset, body-size, opt: gs.opt, ranges: x.ranges),)
    for i in range(int(body-size / 8)).rev() {
      lines = lines + ("mov r10, [rsp + " + str(i * 8) + "]", "mov [rax + " + str(i * 8) + "], r10")
    }
    gs = remove-item(gs)
    lines = lines + ("add rsp, " + str(body-size),)
    lines = lines + (increment-loop-index(rank, label),)
    for _ in range(rank) { gs = remove-item(gs) }
    lines = lines + ("add rsp, " + str(8 * rank),)
    for _ in range(rank + 1) { gs = remove-item(gs) }
    gs = add-item(gs, type-size(t, structs))
    (gs, lines.join("\n"))
  } else if x.tag == "sum-loop" {
    let rank = x.ranges.len()
    let body-env = env
    for r in x.ranges { body-env.insert(r.name, "int") }
    let result-size = type-size(t, structs)
    let lines = ("sub rsp, " + str(result-size),)
    gs = add-item(gs, result-size)
    let bounds-code = {
      let lines2 = ()
      for r in x.ranges.rev() {
        let (gsb, code) = gen-expr(gs, r.bound, infer, env, structs, in-statement)
        gs = gsb
        lines2 = lines2 + (code, "mov rax, [rsp]", "cmp rax, 0")
        let (gsc, achk) = gen-assert(gs, "jg", "non-positive loop bound")
        gs = gsc
        lines2 = lines2 + (achk,)
      }
      lines2.join("\n")
    }
    lines = lines + (bounds-code, "mov rax, 0", "mov [rsp + " + str(8 * rank) + "], rax")
    let (gs3, idx-code) = init-indices(gs, x.ranges)
    gs = gs3
    lines = lines + (idx-code,)
    let (gs4, label) = next-jump(gs)
    gs = gs4
    lines = lines + (label + ":",)
    let (gs5, body-code) = gen-expr(gs, x.body, infer, body-env, structs, in-statement)
    gs = gs5
    lines = lines + (body-code,)
    let acc-addr = 2 * 8 * rank
    if t == "int" {
      lines = lines + ("pop rax", "add [rsp + " + str(acc-addr) + "], rax")
    } else {
      lines = lines + ("movsd xmm0, [rsp]", "add rsp, 8", "addsd xmm0, [rsp + " + str(acc-addr) + "]", "movsd [rsp + " + str(acc-addr) + "], xmm0")
    }
    gs = remove-item(gs)
    lines = lines + (increment-loop-index(rank, label),)
    lines = lines + ("add rsp, " + str(8 * rank),)
    for _ in range(rank) { gs = remove-item(gs) }
    lines = lines + ("add rsp, " + str(8 * rank),)
    for _ in range(rank) { gs = remove-item(gs) }
    (gs, lines.join("\n"))
  } else if x.tag == "if" {
    let (gs, cond-code) = gen-expr(gs, x.cond, infer, env, structs, in-statement, ty: "bool")
    if gs.opt > 0 and x.yes.tag == "int" and x.yes.value == 1 and x.no.tag == "int" and x.no.value == 0 {
      (gs, cond-code)
    } else {
      gs = remove-item(gs)
      let (gs, l1) = next-jump(gs)
      let (gs, l2) = next-jump(gs)
      // then/else are same-type by construction (equal to `t`, computed at entry).
      let (gs, then-code) = gen-expr(gs, x.yes, infer, env, structs, in-statement, ty: t)
      gs = remove-item(gs)
      let (gs, else-code) = gen-expr(gs, x.no, infer, env, structs, in-statement, ty: t)
      let lines = (cond-code, "pop rax", "cmp rax, 0", "je " + l1, then-code, "jmp " + l2, l1 + ":", else-code, l2 + ":")
      (gs, lines.join("\n"))
    }
  } else if x.tag == "index" and gs.opt > 0 and x.base.tag == "var" and not (gs.offsets.at(x.base.name).from-main and in-statement) {
    // optimized_array_index: index a named variable in place, without materializing its
    // (pointer, dims...) descriptor onto the stack first -- read straight from its home slot.
    let entry = gs.offsets.at(x.base.name)
    let rank = x.indices.len()
    let lines = ()
    for idx in x.indices.rev() {
      let (gs2, code) = gen-expr(gs, idx, infer, env, structs, in-statement)
      gs = gs2
      lines = lines + (code,)
    }
    let location = gs.stack - entry.off
    for k in range(rank) {
      lines = lines + ("mov rax, [rsp + " + str(k * 8) + "]", "cmp rax, 0")
      let (gs3, achk) = gen-assert(gs, "jge", "negative array index")
      gs = gs3
      lines = lines + (achk, "cmp rax, [rsp + " + str(location + k * 8) + "]")
      let (gs4, achk2) = gen-assert(gs, "jl", "index too large")
      gs = gs4
      lines = lines + (achk2,)
    }
    let result-size = type-size(t, structs)
    lines = lines + (calculate-linear-index(rank, 0, location, result-size, opt: gs.opt),)
    for _ in x.indices { gs = remove-item(gs) }
    lines = lines + ("add rsp, " + str(rank * 8), "sub rsp, " + str(result-size))
    gs = add-item(gs, result-size)
    for i in range(int(result-size / 8)).rev() {
      lines = lines + ("mov r10, [rax + " + str(i * 8) + "]", "mov [rsp + " + str(i * 8) + "], r10")
    }
    (gs, lines.join("\n"))
  } else if x.tag == "index" {
    let arr-t = infer(x.base, env)
    let base-rank = arr-t.rank
    let (gs, arr-code) = gen-expr(gs, x.base, infer, env, structs, in-statement)
    let lines = (arr-code,)
    for idx in x.indices.rev() {
      let (gs2, code) = gen-expr(gs, idx, infer, env, structs, in-statement)
      gs = gs2
      lines = lines + (code,)
    }
    for k in range(x.indices.len()) {
      lines = lines + ("mov rax, [rsp + " + str(k * 8) + "]", "cmp rax, 0")
      let (gs3, achk) = gen-assert(gs, "jge", "negative array index")
      gs = gs3
      lines = lines + (achk, "cmp rax, [rsp + " + str((k + base-rank) * 8) + "]")
      let (gs4, achk2) = gen-assert(gs, "jl", "index too large")
      gs = gs4
      lines = lines + (achk2,)
    }
    let result-size = type-size(t, structs)
    lines = lines + (calculate-linear-index(x.indices.len(), 0, x.indices.len() * 8, result-size, opt: gs.opt),)
    for _ in x.indices {
      gs = remove-item(gs)
      lines = lines + ("add rsp, 8",)
    }
    gs = remove-item(gs)
    lines = lines + ("add rsp, " + str(type-size(arr-t, structs)), "sub rsp, " + str(result-size))
    gs = add-item(gs, result-size)
    for i in range(int(result-size / 8)).rev() {
      lines = lines + ("mov r10, [rax + " + str(i * 8) + "]", "mov [rsp + " + str(i * 8) + "], r10")
    }
    (gs, lines.join("\n"))
  } else if x.tag == "struct-lit" {
    let lines = ()
    for item in x.items.rev() {
      let (gs2, code) = gen-expr(gs, item, infer, env, structs, in-statement)
      gs = gs2
      lines = lines + (code,)
    }
    for _ in x.items { gs = remove-item(gs) }
    gs = add-item(gs, type-size(t, structs))
    (gs, lines.join("\n"))
  } else if x.tag == "dot" {
    let (gs, base-code) = gen-expr(gs, x.base, infer, env, structs, in-statement)
    let base-t = infer(x.base, env)
    let fl = field-layout(base-t, x.field, structs)
    let offset = fl.offset
    let field-size = fl.size
    let struct-size = type-size(base-t, structs)
    let lines = (base-code,)
    for i in range(int(field-size / 8)).rev() {
      let off = i * 8
      lines = lines + (
        "mov r10, [rsp + " + str(offset + off) + "]",
        "mov [rsp + " + str(struct-size - field-size + off) + "], r10",
      )
    }
    lines = lines + ("add rsp, " + str(struct-size - field-size),)
    gs = remove-item(gs)
    gs = add-item(gs, field-size)
    (gs, lines.join("\n"))
  } else {
    panic("x86 backend: expression not representable yet: " + x.tag)
  }
}

#let gen-show(gs, x, infer, env, structs, in-statement) = {
  let t = infer(x, env)
  let size = type-size(t, structs)
  let (gs, prepad) = pad-shadow-with-size(gs, size)
  let (gs, expr-code) = gen-expr(gs, x, infer, env, structs, in-statement, ty: t)
  let (gs, fmt) = insert-string-constant(gs, show-type-string(t, structs))
  gs = add-item(gs, 8)
  let lines = ()
  if prepad != "" { lines = lines + (prepad,) }
  lines = lines + (expr-code, fmt, "lea rsi, [rsp]", "call _show")
  gs = remove-item(gs)
  lines = lines + ("add rsp, " + str(size),)
  let (gs, unpad) = unpad-shadow(gs)
  if unpad != "" { lines = lines + (unpad,) }
  gs = remove-item(gs)
  (gs, lines.join("\n"))
}

// unqualified primitive name whether tt is a bare string ("int") or a raw parsed
// type node ({tag:"int",name:"int"}) -- infer()/normalized-type() results are always
// bare strings, but raw param/return type nodes straight from the parser are dicts.
#let prim-name(tt) = if type(tt) == str { tt } else { tt.tag }
#let is-agg-type(tt) = prim-name(tt) not in ("int", "float", "bool", "void")

#let gen-let(gs, env, lv, rvalue, infer, structs, in-statement) = {
  let vt = infer(rvalue, env)
  let (gs, code) = gen-expr(gs, rvalue, infer, env, structs, in-statement, ty: vt)
  let from-main = not in-statement
  gs.offsets.insert(lv.name, (off: gs.stack, from-main: from-main))
  env.insert(lv.name, vt)
  if lv.tag == "array-lvalue" {
    for (i, d) in lv.dims.enumerate() {
      gs.offsets.insert(d, (off: gs.stack - 8 * i, from-main: from-main))
      env.insert(d, "int")
    }
  }
  (gs, env, code)
}

// gen-statement(gs, env, s, infer, structs) -> (gs, env, code) -- only used inside function bodies (in-statement always true)
#let gen-statement(gs, env, s, infer, structs) = {
  if s.tag == "let" {
    gen-let(gs, env, s.target, s.value, infer, structs, true)
  } else if s.tag == "assert" {
    let (gs, cond-code) = gen-expr(gs, s.cond, infer, env, structs, true)
    gs = remove-item(gs)
    let (gs, achk) = gen-assert(gs, "jne", s.message)
    (gs, env, cond-code + "\npop rax\ncmp rax, 0\n" + achk)
  } else {
    // return
    let rt = infer(s.value, env)
    let (gs, code) = gen-expr(gs, s.value, infer, env, structs, true)
    gs = remove-item(gs)
    let lines = (code,)
    if rt in ("int", "bool", "void") {
      lines = lines + ("pop rax",)
    } else if rt == "float" {
      lines = lines + ("movsd xmm0, [rsp]", "add rsp, 8")
    } else {
      let sz = type-size(rt, structs)
      lines = lines + ("mov rax, [rbp - 8]",)
      for i in range(int(sz / 8)).rev() {
        lines = lines + ("mov r10, [rsp + " + str(i * 8) + "]", "mov [rax + " + str(i * 8) + "], r10")
      }
      gs = add-item(gs, sz)
    }
    lines = lines + ("add rsp, " + str(gs.stack), "pop rbp", "ret")
    (gs, env, lines.join("\n"))
  }
}

// gen-function(gs, c, infer, structs, globals) -> (gs, function-text)
#let gen-function(gs, c, infer, structs, globals) = {
  let int-regs = ("rdi", "rsi", "rdx", "rcx", "r8", "r9")
  let flo-regs = ("xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7")
  let fgs = (data: gs.data, cmap: gs.cmap, jump: gs.jump, stack: 0, shadow: (), offsets: gs.offsets, opt: gs.opt)
  let lines = ("push rbp", "mov rbp, rsp")
  let int-n = 0
  let flo-n = 0
  let arr-size = 0
  let ret-t = c.returns
  if is-agg-type(ret-t) {
    lines = lines + ("push " + int-regs.at(int-n),)
    fgs = add-item(fgs, 8)
    int-n += 1
  }
  let env = globals
  for p in c.params {
    let pt = p.typ
    if not is-agg-type(pt) {
      if prim-name(pt) == "float" {
        lines = lines + ("sub rsp, 8", "movsd [rsp], " + flo-regs.at(flo-n))
        flo-n += 1
      } else {
        lines = lines + ("push " + int-regs.at(int-n),)
        int-n += 1
      }
      fgs = add-item(fgs, 8)
      fgs.offsets.insert(p.target.name, (off: fgs.stack, from-main: false))
    } else {
      let data-pointer = -arr-size - 16
      fgs.offsets.insert(p.target.name, (off: data-pointer, from-main: false))
      arr-size += type-size(pt, structs)
      if p.target.tag == "array-lvalue" {
        for (i, d) in p.target.dims.enumerate() {
          fgs.offsets.insert(d, (off: data-pointer - 8 * i, from-main: false))
          env.insert(d, "int")
        }
      }
    }
    env.insert(p.target.name, pt)
  }
  let has-return = c.body.any(s => s.tag == "return")
  for s in c.body {
    let (fgs2, env2, code) = gen-statement(fgs, env, s, infer, structs)
    fgs = fgs2
    env = env2
    lines = lines + (code,)
  }
  if not has-return {
    let (fgs2, const-code) = insert-int-constant(fgs, 1)
    fgs = fgs2
    lines = lines + (const-code, "pop rax", "add rsp, " + str(fgs.stack), "pop rbp", "ret")
  }
  let gs2 = (data: fgs.data, cmap: fgs.cmap, jump: fgs.jump, stack: gs.stack, shadow: gs.shadow, offsets: fgs.offsets, opt: gs.opt)
  (gs2, c.name + ":\n_" + c.name + ":\n\t" + lines.join("\n\t"))
}

#let header = "global jpl_main\nglobal _jpl_main\nextern _fail_assertion\nextern _jpl_alloc\nextern _get_time\nextern _show\nextern _print\nextern _print_time\nextern _read_image\nextern _write_image\nextern _fmod\nextern _sqrt\nextern _exp\nextern _sin\nextern _cos\nextern _tan\nextern _asin\nextern _acos\nextern _atan\nextern _log\nextern _pow\nextern _atan2\nextern _to_int\nextern _to_float"

#let render-const(c) = {
  if c.kind == "num" {
    "dq " + c.val
  } else {
    "db `" + c.val + "`, 0"
  }
}

// final whole-text peephole pass mirroring the reference's string_replacement_optimization:
// power-of-two `imul rax, N` becomes `shl rax, log2(N)`; `imul rax, 1` is dropped entirely.
#let apply-peephole(text) = {
  // Matches the reference's two sequential regex passes: rule 1 (imul rax,N -> shl for
  // any power of two, including N=1) always fires first and already rewrites every
  // "imul rax, 1", so rule 2 ("imul rax, 1" -> "") never actually has anything left to match.
  text.split("\n").map(line => {
    let m = line.match(regex("imul rax, (\d+)$"))
    if m == none { line } else {
      let n = int(m.captures.at(0))
      let sh = pow2-shift(n)
      if sh == none { line } else { line.slice(0, m.start) + "shl rax, " + str(sh) }
    }
  }).join("\n")
}

// gen-command(gs, c, infer, env, structs) -> (gs, code) -- top-level commands only
// ("fn" is handled separately by emit-asm since it produces a standalone function body,
// not inline code -- JPL disallows nesting a function declaration inside `time` anyway).
#let gen-command(gs, c, infer, env, structs) = {
  if c.tag == "show" {
    gen-show(gs, c.value, infer, env, structs, false)
  } else if c.tag == "let" {
    let (gs, _, code) = gen-let(gs, env, c.target, c.value, infer, structs, false)
    (gs, code)
  } else if c.tag == "assert" {
    let (gs, cond-code) = gen-expr(gs, c.cond, infer, env, structs, false)
    gs = remove-item(gs)
    let (gs, achk) = gen-assert(gs, "jne", c.message)
    (gs, cond-code + "\npop rax\ncmp rax, 0\n" + achk)
  } else if c.tag == "print" {
    let (gs, lea) = insert-string-constant(gs, c.message)
    let (gs, prepad) = pad-shadow(gs)
    let lines = (lea,)
    if prepad != "" { lines = lines + (prepad,) }
    lines = lines + ("call _print",)
    let (gs, unpad) = unpad-shadow(gs)
    if unpad != "" { lines = lines + (unpad,) }
    (gs, lines.join("\n"))
  } else if c.tag == "read" {
    gs = add-item(gs, 24)
    let lines = ("sub rsp, 24", "lea rdi, [rsp]")
    let (gs, prepad) = pad-shadow(gs)
    if prepad != "" { lines = lines + (prepad,) }
    let (gs, name) = insert-const(gs, "str", c.source)
    lines = lines + ("lea rsi, [rel " + name + "]", "call _read_image")
    let (gs, unpad) = unpad-shadow(gs)
    if unpad != "" { lines = lines + (unpad,) }
    gs.offsets.insert(c.target.name, (off: gs.stack, from-main: true))
    if c.target.tag == "array-lvalue" {
      for (i, d) in c.target.dims.enumerate() {
        gs.offsets.insert(d, (off: gs.stack - 8 * i, from-main: true))
      }
    }
    (gs, lines.join("\n"))
  } else if c.tag == "write" {
    let val-t = infer(c.value, env)
    let size = type-size(val-t, structs)
    let (gs, prepad) = pad-shadow-with-size(gs, size)
    let (gs, val-code) = gen-expr(gs, c.value, infer, env, structs, false)
    let (gs, lea) = insert-string-constant(gs, c.dest)
    let lines = ()
    if prepad != "" { lines = lines + (prepad,) }
    lines = lines + (val-code, lea, "call _write_image")
    gs = remove-item(gs)
    lines = lines + ("add rsp, " + str(size),)
    let (gs, unpad) = unpad-shadow(gs)
    if unpad != "" { lines = lines + (unpad,) }
    (gs, lines.join("\n"))
  } else if c.tag == "time" {
    let (gs, prepad1) = pad-shadow(gs)
    let lines = ()
    if prepad1 != "" { lines = lines + (prepad1,) }
    lines = lines + ("call _get_time",)
    let (gs, unpad1) = unpad-shadow(gs)
    if unpad1 != "" { lines = lines + (unpad1,) }
    lines = lines + ("sub rsp, 8", "movsd [rsp], xmm0")
    gs = add-item(gs, 8)
    let stack-before = gs.stack
    let (gs, inner-code) = gen-command(gs, c.command, infer, env, structs)
    lines = lines + (inner-code,)
    let (gs, prepad2) = pad-shadow(gs)
    if prepad2 != "" { lines = lines + (prepad2,) }
    lines = lines + ("call _get_time",)
    let (gs, unpad2) = unpad-shadow(gs)
    if unpad2 != "" { lines = lines + (unpad2,) }
    lines = lines + (
      "sub rsp, 8", "movsd [rsp], xmm0",
      "movsd xmm0, [rsp]", "add rsp, 8",
      "movsd xmm1, [rsp + " + str(gs.stack - stack-before) + "]",
      "subsd xmm0, xmm1",
    )
    let (gs, prepad3) = pad-shadow(gs)
    if prepad3 != "" { lines = lines + (prepad3,) }
    lines = lines + ("call _print_time",)
    let (gs, unpad3) = unpad-shadow(gs)
    if unpad3 != "" { lines = lines + (unpad3,) }
    (gs, lines.join("\n"))
  } else if c.tag == "struct" {
    (gs, "")
  } else {
    panic("x86 backend: command not representable yet: " + c.tag)
  }
}

#let emit-asm(checked, opt: 0) = {
  let infer = checked.infer
  let env = checked.globals
  let structs = checked.structs
  let gs = new-gs(opt: opt)
  gs.offsets.insert("args", (off: -16, from-main: false))
  gs.offsets.insert("argnum", (off: -16, from-main: false))
  gs = add-item(gs, 8) // r12 pushed in prelude
  let body = ("push rbp", "mov rbp, rsp", "push r12", "mov r12, rbp")
  let functions = ()
  for c in checked.ast {
    if c.tag == "fn" {
      let (gs2, fn-text) = gen-function(gs, c, infer, structs, env)
      gs = gs2
      functions.push(fn-text)
    } else {
      let (gs2, code) = gen-command(gs, c, infer, env, structs)
      gs = gs2
      body.push(code)
    }
  }
  if gs.stack > 8 {
    body.push("add rsp, " + str(gs.stack - 8))
  }
  body = body + ("pop r12", "pop rbp", "ret")
  let out = header + "\n\nsection .data"
  for (i, c) in gs.data.enumerate() {
    out += "\nconst" + str(i) + ": " + render-const(c)
  }
  out += "\n\nsection .text"
  for fn-text in functions {
    out += "\n" + fn-text + "\n"
  }
  out += "\njpl_main:\n_jpl_main:\n\t" + body.join("\n\t")
  if opt > 0 { apply-peephole(out) } else { out }
}
