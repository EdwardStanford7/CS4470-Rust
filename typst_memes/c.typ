// C intermediate-code backend (the `-i` CLI output, hw8/hw9). There is no
// reference implementation to port here -- unlike asm.typ, this file's exact
// declaration/statement ORDER was reverse-engineered fixture-by-fixture
// against the course's own reference compiler (see plans/DIAGNOSIS-hw9-typedef-order.md
// for the full history: 13 separate ordering bugs were found this way).
// That history is why this file is left in its original dense, one-line-
// per-case style rather than reformatted during the refactor's step 7 --
// every reordering risk here is a correctness risk, not just a style one.
#import "types.typ": normalized-type
#let c-type(t)=if t=="int"{"int64_t"}else if t=="float"{"double"}else if t=="bool"{"bool"}else if t=="void"{"void_t"}else{t}
#let descriptor(t)=if t=="int"{"(IntType)"}else if t=="float"{"(FloatType)"}else if t=="bool"{"(BoolType)"}else if t=="void"{"(VoidType)"}else{"(TupleType)"}
#let cname(i)="_"+str(i)
#let type-env(env)={let d=(:);for (k,v) in env{d.insert(k,v.typ)};d}
#let emit-c(checked)={
  let infer=checked.infer
  let norm = normalized-type
  let type-key(t)=if t=="int"{"int64"}else if t=="float"{"double"}else if t=="void"{"void"}else{t}
  let ct(raw)={let t=norm(raw);if type(t)==dictionary{"_a"+str(t.rank)+"_"+(if type(t.element)==dictionary{ct(t.element)}else{type-key(t.element)+(if t.element in ("int","void"){"_t"}else{""})})}else{c-type(t)}}
  let gen(x,env,next,jump:1)={
    let lines=();if x.tag=="var" and x.name not in env{panic("C backend undefined "+x.name+" among "+env.keys().join(","))};let typ=infer(x,type-env(env));let name=cname(next);next+=1
    if x.tag=="int"{lines.push(c-type(typ)+" "+name+" = "+str(x.value)+";")}
    else if x.tag=="float"{lines.push(c-type(typ)+" "+name+" = "+str(int(x.value))+".0;")}
    else if x.tag=="bool"{lines.push(c-type(typ)+" "+name+" = "+if x.value{"true"}else{"false"}+";")}
    else if x.tag=="void"{lines.push("void_t "+name+" = {};")}
    else if x.tag=="var"{if x.name not in env{panic("C backend undefined "+x.name+" among "+env.keys().join(","))};return (lines:lines,name:env.at(x.name).name,typ:typ,next:next - 1,jump:jump)}
    else if x.tag=="unary"{let a=gen(x.value,env,next - 1,jump:jump);lines+=a.lines;next=a.next;jump=a.jump;name=cname(next);next+=1;lines.push(c-type(typ)+" "+name+" = "+x.op+a.name+";")}
    else if x.tag=="binary" and x.op in ("&&","||"){
      let result=name
      let a=gen(x.left,env,next,jump:jump);lines+=a.lines;next=a.next;jump=a.jump
      lines.push("bool "+result+" = "+a.name+";")
      lines.push("if (0 "+if x.op=="&&"{"=="}else{"!="}+" "+a.name+")")
      lines.push("goto _jump"+str(jump)+";");let done=jump;jump+=1
      let b=gen(x.right,env,next,jump:jump);lines+=b.lines;next=b.next;jump=b.jump
      lines.push(result+" = "+b.name+";");lines.push("_jump"+str(done)+":;");name=result
    }
    else if x.tag=="binary"{let a=gen(x.left,env,next - 1,jump:jump);let b=gen(x.right,env,a.next,jump:a.jump);lines+=a.lines+b.lines;next=b.next;jump=b.jump;name=cname(next);next+=1;let op=if x.op=="%" and infer(x.left,type-env(env))=="float"{"fmod"}else{x.op};let rhs=if op=="fmod"{"fmod("+a.name+", "+b.name+")"}else{a.name+" "+op+" "+b.name};lines.push(c-type(typ)+" "+name+" = "+rhs+";")}
    else if x.tag=="call"{next -=1;let args=();for a in x.args{let r=gen(a,env,next,jump:jump);lines+=r.lines;next=r.next;jump=r.jump;args.push(r.name)};name=cname(next);next+=1;lines.push(ct(typ)+" "+name+" = "+x.name+"("+args.join(", ")+");")}
    else if x.tag=="array"{next -=1;let vals=();for item in x.items{let r=gen(item,env,next,jump:jump);lines+=r.lines;next=r.next;jump=r.jump;vals.push(r.name)};name=cname(next);next+=1;lines.push(ct(typ)+" "+name+";");lines.push(name+".d0 = "+str(x.items.len())+";");lines.push(name+".data = jpl_alloc(sizeof("+ct(typ.element)+") * "+str(x.items.len())+");");for pair in vals.enumerate(){lines.push(name+".data["+str(pair.first())+"] = "+pair.last()+";")}}
    else if x.tag=="struct-lit"{next -=1;let vals=();for item in x.items{let r=gen(item,env,next,jump:jump);lines+=r.lines;next=r.next;jump=r.jump;vals.push(r.name)};name=cname(next);next+=1;lines.push(ct(typ)+" "+name+" = { "+vals.join(", ")+" };")}
    else if x.tag=="dot"{let r=gen(x.base,env,next - 1,jump:jump);lines+=r.lines;next=r.next;jump=r.jump;name=cname(next);next+=1;lines.push(ct(typ)+" "+name+" = "+r.name+"."+x.field+";")}
    else if x.tag=="index"{
      next -= 1
      let base=gen(x.base,env,next,jump:jump);lines+=base.lines;next=base.next;jump=base.jump
      let indices=()
      for ix in x.indices{let r=gen(ix,env,next,jump:jump);lines+=r.lines;next=r.next;jump=r.jump;indices.push(r.name)}
      for pair in indices.enumerate(){
        lines.push("if ("+pair.last()+" >= 0)");lines.push("goto _jump"+str(jump)+";");lines.push("fail_assertion(\"negative array index\");");lines.push("_jump"+str(jump)+":;");jump+=1
        lines.push("if ("+pair.last()+" < "+base.name+".d"+str(pair.first())+")");lines.push("goto _jump"+str(jump)+";");lines.push("fail_assertion(\"index too large\");");lines.push("_jump"+str(jump)+":;");jump+=1
      }
      let offset=cname(next);next+=1;lines.push("int64_t "+offset+" = 0;")
      for pair in indices.enumerate(){lines.push(offset+" *= "+base.name+".d"+str(pair.first())+";");lines.push(offset+" += "+pair.last()+";")}
      name=cname(next);next+=1;lines.push(ct(typ)+" "+name+" = "+base.name+".data["+offset+"];")
    }
    else if x.tag in ("array-loop","sum-loop"){
      let result=name;lines.push(ct(typ)+" "+result+";")
      let bounds=();let loop-env=env
      for pair in x.ranges.enumerate(){let r=gen(pair.last().bound,env,next,jump:jump);lines+=r.lines;next=r.next;jump=r.jump;bounds.push(r.name);if x.tag=="array-loop"{lines.push(result+".d"+str(pair.first())+" = "+r.name+";")};lines.push("if ("+r.name+" > 0)");lines.push("goto _jump"+str(jump)+";");lines.push("fail_assertion(\"non-positive loop bound\");");lines.push("_jump"+str(jump)+":;");jump+=1}
      if x.tag=="array-loop"{let size=cname(next);next+=1;lines.push("int64_t "+size+" = 1;");for b in bounds{lines.push(size+" *= "+b+";")};lines.push(size+" *= sizeof("+ct(typ.element)+");");lines.push(result+".data = jpl_alloc("+size+");")}else{lines.push(result+" = 0;")}
      let vars=();for pair in x.ranges.enumerate().rev(){let v=cname(next);next+=1;lines.push("int64_t "+v+" = 0;");vars.push((range-index:pair.first(),name:v));loop-env.insert(pair.last().name,(name:v,typ:"int"))}
      let body-label=jump;jump+=1;lines.push("_jump"+str(body-label)+":;")
      let body=gen(x.body,loop-env,next,jump:jump);lines+=body.lines;next=body.next;jump=body.jump
      if x.tag=="sum-loop"{lines.push(result+" += "+body.name+";")}else{let offset=cname(next);next+=1;lines.push("int64_t "+offset+" = 0;");for pair in vars.rev(){lines.push(offset+" *= "+result+".d"+str(pair.range-index)+";");lines.push(offset+" += "+pair.name+";")};lines.push(result+".data["+offset+"] = "+body.name+";")}
      for pair in vars{lines.push(pair.name+"++;");lines.push("if ("+pair.name+" < "+bounds.at(pair.range-index)+")");lines.push("goto _jump"+str(body-label)+";");if pair!=vars.last(){lines.push(pair.name+" = 0;")}}
      name=result
    }
    else if x.tag=="if"{let cond=gen(x.cond,env,next - 1,jump:jump);lines+=cond.lines;next=cond.next;jump=cond.jump;name=cname(next);next+=1;lines.push(ct(typ)+" "+name+";");let otherwise=jump;jump+=1;lines.push("if (!"+cond.name+")");lines.push("goto _jump"+str(otherwise)+";");let yes=gen(x.yes,env,next,jump:jump);lines+=yes.lines;next=yes.next;jump=yes.jump;lines.push(name+" = "+yes.name+";");let done=jump;jump+=1;lines.push("goto _jump"+str(done)+";");lines.push("_jump"+str(otherwise)+":;");let no=gen(x.no,env,next,jump:jump);lines+=no.lines;next=no.next;jump=no.jump;lines.push(name+" = "+no.name+";");lines.push("_jump"+str(done)+":;")}
    else{panic("C backend expression not implemented: "+x.tag)}
    (lines:lines,name:name,typ:typ,next:next,jump:jump)
  }
  let desc(raw)={let t=norm(raw);if type(t)==dictionary{"(ArrayType "+desc(t.element)+" "+str(t.rank)+")"}else if t in checked.structs{let fs=checked.structs.at(t);"(TupleType "+fs.map(f=>desc(f.typ)).join(" ")+")"}else{descriptor(t)}}
  let array-chain(raw)={let t=norm(raw);if type(t)==dictionary and t.tag=="array-type"{array-chain(t.element)+(t,)}else{()}}
  let header=("#include <math.h>","#include <stdbool.h>","#include <stdint.h>","#include <stdio.h>","#include \"rt/runtime.h\"","","typedef struct { } void_t;","").join("\n")
  let declared=();let array-decl(t)="typedef struct {\n  "+range(t.rank).map(i=>"int64_t d"+str(i)+";").join("\n  ")+"\n  "+ct(t.element)+" *data;\n} "+ct(t)+";\n\n"
  let declare-type(raw,header,declared)={for t in array-chain(raw){if repr(t) not in declared and ct(t)!="_a2_rgba"{header+=array-decl(t);declared.push(repr(t))}};(header:header,declared:declared)}
  let scan(x,header,declared,tenv:checked.globals)={
    if x.tag=="unary"{let r=scan(x.value,header,declared,tenv:tenv);header=r.header;declared=r.declared}
    else if x.tag=="binary"{let r=scan(x.left,header,declared,tenv:tenv);header=r.header;declared=r.declared;r=scan(x.right,header,declared,tenv:tenv);header=r.header;declared=r.declared}
    else if x.tag=="if"{let r=scan(x.cond,header,declared,tenv:tenv);header=r.header;declared=r.declared;let d=declare-type(infer(x,tenv),header,declared);header=d.header;declared=d.declared;r=scan(x.yes,header,declared,tenv:tenv);header=r.header;declared=r.declared;r=scan(x.no,header,declared,tenv:tenv);header=r.header;declared=r.declared}
    else if x.tag in ("array","struct-lit"){for e in x.items{let r=scan(e,header,declared,tenv:tenv);header=r.header;declared=r.declared}}
    else if x.tag=="call"{for e in x.args{let r=scan(e,header,declared,tenv:tenv);header=r.header;declared=r.declared}}
    else if x.tag=="dot"{let r=scan(x.base,header,declared,tenv:tenv);header=r.header;declared=r.declared}
    else if x.tag=="index"{let r=scan(x.base,header,declared,tenv:tenv);header=r.header;declared=r.declared;for e in x.indices{r=scan(e,header,declared,tenv:tenv);header=r.header;declared=r.declared}}
    else if x.tag=="array-loop"{
      let d=declare-type(infer(x,tenv),header,declared);header=d.header;declared=d.declared
      for r in x.ranges{let r2=scan(r.bound,header,declared,tenv:tenv);header=r2.header;declared=r2.declared}
      let rb=scan(x.body,header,declared,tenv:tenv);header=rb.header;declared=rb.declared
    }
    else if x.tag=="sum-loop"{for r in x.ranges{let r2=scan(r.bound,header,declared,tenv:tenv);header=r2.header;declared=r2.declared};let rb=scan(x.body,header,declared,tenv:tenv);header=rb.header;declared=rb.declared}
    if not (x.tag=="var" and x.name=="args"){declare-type(infer(x,tenv),header,declared)}else{(header:header,declared:declared)}
  }
  let loop-names(x)={
    let out=()
    if x.tag in ("array-loop","sum-loop"){for r in x.ranges{out.push(r.name);out+=loop-names(r.bound)};out+=loop-names(x.body)}
    else if x.tag=="unary"{out+=loop-names(x.value)}
    else if x.tag=="binary"{out+=loop-names(x.left)+loop-names(x.right)}
    else if x.tag=="if"{out+=loop-names(x.cond)+loop-names(x.yes)+loop-names(x.no)}
    else if x.tag in ("array","struct-lit"){for e in x.items{out+=loop-names(e)}}
    else if x.tag=="call"{for e in x.args{out+=loop-names(e)}}
    else if x.tag=="dot"{out+=loop-names(x.base)}
    else if x.tag=="index"{out+=loop-names(x.base);for e in x.indices{out+=loop-names(e)}}
    out
  }
  let global-jump=1
  for c in checked.ast{
    let d=c;while d.tag=="time"{d=d.command}
    if d.tag=="struct"{for f in d.fields{let r=declare-type(f.typ,header,declared);header=r.header;declared=r.declared};header+="typedef struct {\n"+d.fields.map(f=>"    "+ct(f.typ)+" "+f.name+";").join("\n")+"\n} "+d.name+";\n\n"}
    else if d.tag in ("let","show"){let r=scan(d.value,header,declared);header=r.header;declared=r.declared}
    else if d.tag=="assert"{let r=scan(d.cond,header,declared);header=r.header;declared=r.declared}
    else if d.tag=="write"{let r=scan(d.value,header,declared);header=r.header;declared=r.declared}
    else if d.tag=="fn"{
      let r=declare-type(d.returns,header,declared);header=r.header;declared=r.declared
      for p in d.params{let r=declare-type(p.typ,header,declared);header=r.header;declared=r.declared}
      let ftypes=checked.globals;for p in d.params{ftypes.insert(p.target.name,norm(p.typ));if p.target.tag=="array-lvalue"{for dim in p.target.dims{ftypes.insert(dim,"int")}}}
      for s in d.body{let ex=if s.tag in ("let","return"){s.value}else if s.tag=="assert"{s.cond}else{none};if ex!=none{let r=scan(ex,header,declared,tenv:ftypes);header=r.header;declared=r.declared}}
    }
  }
  let functions=""
  let env=(argnum:(name:"args.d0",typ:"int"),args:(name:"args",typ:(tag:"array-type",element:"int",rank:1)))
  for c in checked.ast{let d=c;while d.tag=="time"{d=d.command};let ex=if d.tag in ("let","show","write"){d.value}else if d.tag=="assert"{d.cond}else{none};if ex!=none{for n in loop-names(ex){if n not in env{env.insert(n,(name:n,typ:"int"))}}}}
  let dispatch-command(d,env,lines,next,jump)={
    if d.tag=="show"{let r=gen(d.value,env,next,jump:jump);lines+=r.lines;next=r.next;jump=r.jump;lines.push("show(\""+desc(r.typ)+"\", &"+r.name+");")}
    else if d.tag=="let"{let r=gen(d.value,env,next,jump:jump);lines+=r.lines;next=r.next;jump=r.jump;env.insert(d.target.name,(name:r.name,typ:r.typ));if d.target.tag=="array-lvalue"{for pair in d.target.dims.enumerate(){env.insert(pair.last(),(name:r.name+".d"+str(pair.first()),typ:"int"))}}}
    else if d.tag=="read"{let name=cname(next);next+=1;lines.push("_a2_rgba "+name+" = read_image(\""+d.source+"\");");env.insert(d.target.name,(name:name,typ:(tag:"array-type",element:"rgba",rank:2)));if d.target.tag=="array-lvalue"{for pair in d.target.dims.enumerate(){lines.push("int64_t "+pair.last()+" = "+name+".d"+str(pair.first())+";");env.insert(pair.last(),(name:name+".d"+str(pair.first()),typ:"int"))}}}
    else if d.tag=="write"{let r=gen(d.value,env,next,jump:jump);lines+=r.lines;next=r.next;jump=r.jump;lines.push("write_image("+r.name+", \""+d.dest+"\");")}
    else if d.tag=="assert"{let r=gen(d.cond,env,next,jump:jump);lines+=r.lines;next=r.next;jump=r.jump;lines.push("if (0 != "+r.name+")");lines.push("goto _jump"+str(jump)+";");lines.push("fail_assertion(\""+d.message+"\");");lines.push("_jump"+str(jump)+":;");jump+=1}
    else if d.tag=="print"{lines.push("print(\""+d.message+"\");")}
    (env:env,lines:lines,next:next,jump:jump)
  }
  let gen-time(t,env,lines,next,jump)={
    let start=cname(next);next+=1;lines.push("double "+start+" = get_time();")
    let r=if t.command.tag=="time"{gen-time(t.command,env,lines,next,jump)}else{dispatch-command(t.command,env,lines,next,jump)}
    env=r.env;lines=r.lines;next=r.next;jump=r.jump
    let end=cname(next);next+=1;lines.push("double "+end+" = get_time();");lines.push("print_time("+end+" - "+start+");")
    (env:env,lines:lines,next:next,jump:jump)
  }
  let lines=();let next=0;let jump=global-jump
  for c in checked.ast{
    let d=c;while d.tag=="time"{d=d.command}
    if d.tag=="fn"{
      let fenv=(:);for (k,v) in checked.globals{fenv.insert(k,(name:k,typ:v))}
      let params=();for p in d.params{params.push(ct(p.typ)+" "+p.target.name);fenv.insert(p.target.name,(name:p.target.name,typ:norm(p.typ)));if p.target.tag=="array-lvalue"{for pair in p.target.dims.enumerate(){fenv.insert(pair.last(),(name:p.target.name+".d"+str(pair.first()),typ:"int"))}}}
      for s in d.body{let ex=if s.tag in ("let","return"){s.value}else if s.tag=="assert"{s.cond}else{none};if ex!=none{for n in loop-names(ex){if n not in fenv{fenv.insert(n,(name:n,typ:"int"))}}}}
      let flines=();let fnext=0;let fjump=jump
      for s in d.body{
        if s.tag=="let"{let q=gen(s.value,fenv,fnext,jump:fjump);flines+=q.lines;fnext=q.next;fjump=q.jump;fenv.insert(s.target.name,(name:q.name,typ:q.typ));if s.target.tag=="array-lvalue"{for pair in s.target.dims.enumerate(){fenv.insert(pair.last(),(name:q.name+".d"+str(pair.first()),typ:"int"))}}}
        else if s.tag=="assert"{let q=gen(s.cond,fenv,fnext,jump:fjump);flines+=q.lines;fnext=q.next;fjump=q.jump;flines.push("if (0 != "+q.name+")");flines.push("goto _jump"+str(fjump)+";");flines.push("fail_assertion(\""+s.message+"\");");flines.push("_jump"+str(fjump)+":;");fjump+=1}
        else if s.tag=="return"{let q=gen(s.value,fenv,fnext,jump:fjump);flines+=q.lines;fnext=q.next;fjump=q.jump;flines.push("return "+q.name+";")}
      }
      if type-key(norm(d.returns))=="void" and d.body.find(s=>s.tag=="return")==none{let q=gen((tag:"void"),fenv,fnext,jump:fjump);flines+=q.lines;flines.push("return "+q.name+";")}
      jump=fjump
      functions+=ct(d.returns)+" "+d.name+"("+params.join(", ")+") {\n"+flines.map(s=>"    "+s).join("\n")+"\n}\n\n"
    }
    else if c.tag=="time"{
      let r=gen-time(c,env,lines,next,jump);env=r.env;lines=r.lines;next=r.next;jump=r.jump
    } else {
      let r=dispatch-command(c,env,lines,next,jump);env=r.env;lines=r.lines;next=r.next;jump=r.jump
    }
  }
  header+=functions
  header+"void jpl_main(struct args args) {\n"+lines.map(s=>"    "+s).join("\n")+"\n}\n"
}
