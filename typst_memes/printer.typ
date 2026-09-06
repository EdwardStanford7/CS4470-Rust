// AST / typed-AST pretty printers (the `-p`/`-t` CLI output formats).
//
// format-expr/format-command and their format-typed-* counterparts are
// near-identical tree walks that differ only by whether a type annotation
// is spliced in. Evaluated collapsing them into one generic walk
// parameterized on "annotate node with type or not" during the refactor's
// step 7 -- deliberately deferred: hw3-9's grader tests diff this output
// as exact text (only whitespace-normalized), so a merge risks a subtle
// behavior drift for a purely cosmetic win. Revisit if this file needs to
// change for another reason anyway.

#import "types.typ": normalized-type

#let format-type(t)={if type(t)==dictionary and t.tag=="array-type"{"(ArrayType "+format-type(t.element)+" "+str(t.rank)+")"}else{let n=if type(t)==dictionary and t.tag=="named"{t.name}else if type(t)==dictionary{t.tag}else{t};if n in ("int","float","bool","void"){"("+upper(n.first())+n.slice(1)+"Type)"}else{"(StructType "+n+")"}}}
#let format-lvalue(x)=if x.tag=="var-lvalue"{"(VarLValue "+x.name+")"}else{"(ArrayLValue "+x.name+(if x.dims.len()>0{" "+x.dims.join(" ")}else{""})+")"}
#let format-expr(x)={
  let f=format-expr
  if x.tag=="int"{"(IntExpr "+str(x.value)+")"}
  else if x.tag=="float"{"(FloatExpr "+str(int(x.value))+ ")"}
  else if x.tag=="bool"{"("+if x.value{"True"}else{"False"}+"Expr)"}
  else if x.tag=="void"{"(VoidExpr)"}
  else if x.tag=="var"{"(VarExpr "+x.name+")"}
  else if x.tag=="array"{"(ArrayLiteralExpr"+(if x.items.len()>0{" "+x.items.map(f).join(" ")}else{""})+")"}
  else if x.tag=="index"{"(ArrayIndexExpr "+f(x.base)+(if x.indices.len()>0{" "+x.indices.map(f).join(" ")}else{""})+")"}
  else if x.tag=="dot"{"(DotExpr "+f(x.base)+" "+x.field+")"}
  else if x.tag=="call"{"(CallExpr "+x.name+(if x.args.len()>0{" "+x.args.map(f).join(" ")}else{""})+")"}
  else if x.tag=="struct-lit"{"(StructLiteralExpr "+x.name+(if x.items.len()>0{" "+x.items.map(f).join(" ")}else{""})+")"}
  else if x.tag=="unary"{"(UnopExpr "+x.op+" "+f(x.value)+")"}
  else if x.tag=="binary"{"(BinopExpr "+f(x.left)+" "+x.op+" "+f(x.right)+")"}
  else if x.tag=="if"{"(IfExpr "+f(x.cond)+" "+f(x.yes)+" "+f(x.no)+")"}
  else if x.tag in ("array-loop","sum-loop"){"("+if x.tag=="array-loop"{"ArrayLoopExpr"}else{"SumLoopExpr"}+(if x.ranges.len()>0{" "+x.ranges.map(r=>r.name+" "+f(r.bound)).join(" ")}else{""})+" "+f(x.body)+")"}
  else{panic("cannot format expression "+x.tag)}
}
#let format-statement(s)=if s.tag=="let"{"(LetStmt "+format-lvalue(s.target)+" "+format-expr(s.value)+")"}else if s.tag=="assert"{"(AssertStmt "+format-expr(s.cond)+" \""+s.message+"\")"}else{"(ReturnStmt "+format-expr(s.value)+")"}
#let format-command(c)={
  if c.tag=="read"{"(ReadCmd \""+c.source+"\" "+format-lvalue(c.target)+")"}
  else if c.tag=="write"{"(WriteCmd "+format-expr(c.value)+" \""+c.dest+"\")"}
  else if c.tag=="let"{"(LetCmd "+format-lvalue(c.target)+" "+format-expr(c.value)+")"}
  else if c.tag=="assert"{"(AssertCmd "+format-expr(c.cond)+" \""+c.message+"\")"}
  else if c.tag=="print"{"(PrintCmd \""+c.message+"\")"}
  else if c.tag=="show"{"(ShowCmd "+format-expr(c.value)+")"}
  else if c.tag=="time"{"(TimeCmd "+format-command(c.command)+")"}
  else if c.tag=="struct"{"(StructCmd "+c.name+(if c.fields.len()>0{" "+c.fields.map(f=>f.name+" "+format-type(f.typ)).join(" ")}else{""})+")"}
  else if c.tag=="fn"{let ps=c.params.map(p=>format-lvalue(p.target)+" "+format-type(p.typ)).join(" ");"(FnCmd "+c.name+" (("+ps+")) "+format-type(c.returns)+(if c.body.len()>0{" "+c.body.map(format-statement).join(" ")}else{""})+")"}
}
#let format-ast-nodes(ast)=ast.map(format-command).join("\n")

#let format-typed-expr(x,infer,env)={
  let f=y=>format-typed-expr(y,infer,env);let ty=" "+format-type(infer(x,env))
  if x.tag=="int"{"(IntExpr"+ty+" "+str(x.value)+")"}
  else if x.tag=="float"{"(FloatExpr"+ty+" "+str(int(x.value))+")"}
  else if x.tag=="bool"{"("+if x.value{"True"}else{"False"}+"Expr"+ty+")"}
  else if x.tag=="void"{"(VoidExpr"+ty+")"}
  else if x.tag=="var"{"(VarExpr"+ty+" "+x.name+")"}
  else if x.tag=="array"{"(ArrayLiteralExpr"+ty+(if x.items.len()>0{" "+x.items.map(f).join(" ")}else{""})+")"}
  else if x.tag=="index"{"(ArrayIndexExpr"+ty+" "+f(x.base)+(if x.indices.len()>0{" "+x.indices.map(f).join(" ")}else{""})+")"}
  else if x.tag=="dot"{"(DotExpr"+ty+" "+f(x.base)+" "+x.field+")"}
  else if x.tag=="call"{"(CallExpr"+ty+" "+x.name+(if x.args.len()>0{" "+x.args.map(f).join(" ")}else{""})+")"}
  else if x.tag=="struct-lit"{"(StructLiteralExpr"+ty+" "+x.name+(if x.items.len()>0{" "+x.items.map(f).join(" ")}else{""})+")"}
  else if x.tag=="unary"{"(UnopExpr"+ty+" "+x.op+" "+f(x.value)+")"}
  else if x.tag=="binary"{"(BinopExpr"+ty+" "+f(x.left)+" "+x.op+" "+f(x.right)+")"}
  else if x.tag=="if"{"(IfExpr"+ty+" "+f(x.cond)+" "+f(x.yes)+" "+f(x.no)+")"}
  else if x.tag in ("array-loop","sum-loop"){
    let body-env=env;for r in x.ranges{body-env.insert(r.name,"int")}
    let head=if x.tag=="array-loop"{"ArrayLoopExpr"}else{"SumLoopExpr"}
    "("+head+ty+(if x.ranges.len()>0{" "+x.ranges.map(r=>r.name+" "+f(r.bound)).join(" ")}else{""})+" "+format-typed-expr(x.body,infer,body-env)+")"
  } else{panic("cannot format typed expression "+x.tag)}
}
#let formatting-bind(env,lv,t)={let e=env;e.insert(lv.name,normalized-type(t));if lv.tag=="array-lvalue"{for d in lv.dims{e.insert(d,"int")}};e}
#let format-typed-statement(s,infer,env)={
  if s.tag=="let"{let text="(LetStmt "+format-lvalue(s.target)+" "+format-typed-expr(s.value,infer,env)+")";let t=infer(s.value,env);(text,formatting-bind(env,s.target,t))}
  else if s.tag=="assert"{("(AssertStmt "+format-typed-expr(s.cond,infer,env)+" \""+s.message+"\")",env)}
  else{("(ReturnStmt "+format-typed-expr(s.value,infer,env)+")",env)}
}
#let format-typed-command(c,checked)={
  let infer=checked.infer;let env=checked.globals
  if c.tag=="read"{"(ReadCmd \""+c.source+"\" "+format-lvalue(c.target)+")"}
  else if c.tag=="write"{"(WriteCmd "+format-typed-expr(c.value,infer,env)+" \""+c.dest+"\")"}
  else if c.tag=="let"{"(LetCmd "+format-lvalue(c.target)+" "+format-typed-expr(c.value,infer,env)+")"}
  else if c.tag=="assert"{"(AssertCmd "+format-typed-expr(c.cond,infer,env)+" \""+c.message+"\")"}
  else if c.tag=="print"{"(PrintCmd \""+c.message+"\")"}
  else if c.tag=="show"{"(ShowCmd "+format-typed-expr(c.value,infer,env)+")"}
  else if c.tag=="time"{"(TimeCmd "+format-typed-command(c.command,checked)+")"}
  else if c.tag=="struct"{format-command(c)}
  else if c.tag=="fn"{
    let e=env;for p in c.params{e=formatting-bind(e,p.target,p.typ)};let ss=()
    for s in c.body{let r=format-typed-statement(s,infer,e);ss.push(r.first());e=r.last()}
    let ps=c.params.map(p=>format-lvalue(p.target)+" "+format-type(p.typ)).join(" ")
    "(FnCmd "+c.name+" (("+ps+")) "+format-type(c.returns)+(if ss.len()>0{" "+ss.join(" ")}else{""})+")"
  }
}
// Takes an already-checked program (compiler.typ's `check()` output) rather
// than calling `check()` itself, so this module has no dependency on the
// parser/checker -- compiler.typ wires `format-typed(source) =
// format-typed-nodes(check(source))`.
#let format-typed-nodes(checked)=checked.ast.map(c=>format-typed-command(c,checked)).join("\n")
