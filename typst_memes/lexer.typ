// Tokenizer: source text -> token stream. Pure, no parser coupling.

#import "diagnostics.typ": error

#let keywords = (
  "array", "assert", "bool", "else", "false", "float", "fn", "if",
  "image", "int", "let", "print", "read", "return", "show", "struct",
  "sum", "then", "time", "to", "true", "void", "write",
)
#let punctuation = ("(": "lp", ")": "rp", "[": "lb", "]": "rb",
  "{": "lc", "}": "rc", ",": "comma", ":": "colon", ".": "dot")
#let operators = ("==", "!=", "<=", ">=", "&&", "||", "+", "-", "*", "/", "%", "<", ">", "!")

#let failure(phase, line, col, message) = error(phase, message, pos: (line: line, col: col))

#let lex(source) = {
  let cs = source.clusters(); let out = (); let i = 0; let line = 1; let col = 0
  while i < cs.len() {
    let chunk-steps=0
    while i < cs.len() and chunk-steps < 4000 {
    chunk-steps += 1
    let c = cs.at(i); let n = if i + 1 < cs.len() { cs.at(i + 1) } else { "" }
    if c == "\n" { if out.len()==0 or out.last().kind!="nl"{out.push((kind:"nl",text:"\n",line:line,col:col))}; i += 1; line += 1; col = 0 }
    else if c == " " or c == "\t" or c == "\r" { i += 1; col += 1 }
    else if c == "/" and n == "/" {
      while i < cs.len() and cs.at(i) != "\n" {let u=cs.at(i).to-unicode();if u < 32 or u > 126{failure("lexical",line,col,"invalid character in comment")}; i += 1; col += 1 }
    } else if c == "/" and n == "*" {
      let l=line;let cc=col;i+=2;col+=2;let closed=false
      while i < cs.len() and not closed {
        if cs.at(i) == "*" and i + 1 < cs.len() and cs.at(i + 1) == "/" { i += 2; col += 2; closed = true }
        else if cs.at(i) == "\n" { i += 1; line += 1; col = 0 }
        else {let u=cs.at(i).to-unicode();if u < 32 or u > 126{failure("lexical",line,col,"invalid character in block comment")}; i += 1; col += 1 }
      }
      if not closed{failure("lexical",l,cc,"unterminated block comment")}
    } else if c == "\\" {
      if n!="\n"{failure("lexical",line,col,"invalid line continuation")};i+=2;line+=1;col=0
    } else if c == "\"" {
      let l = line; let cc = col; i += 1; col += 1; let s = ""; let closed = false
      while i < cs.len() and not closed {
        let x = cs.at(i)
        if x == "\"" { closed = true; i += 1; col += 1 }
        else if x == "\n" { failure("lexical", line, col, "newline in string") }
        else {let u=x.to-unicode();if u < 32 or u > 126{failure("lexical",line,col,"invalid character in string")}; s += x; i += 1; col += 1 }
      }
      if not closed { failure("lexical", l, cc, "unterminated string") }
      out.push((kind:"string",text:s,line:l,col:cc))
    } else if c.match(regex("[A-Za-z_]")) != none {
      let l = line; let cc = col; let s = ""
      while i < cs.len() and cs.at(i).match(regex("[A-Za-z0-9_]")) != none { s += cs.at(i); i += 1; col += 1 }
      out.push((kind:if s in keywords{s}else{"id"},text:s,line:l,col:cc))
    } else if c.match(regex("[0-9]")) != none or (c == "." and n.match(regex("[0-9]")) != none) {
      let l = line; let cc = col; let s = ""; let dots = 0
      while i < cs.len() and (cs.at(i).match(regex("[0-9]")) != none or (cs.at(i) == "." and dots == 0)) {
        if cs.at(i) == "." { dots += 1 }; s += cs.at(i); i += 1; col += 1
      }
      out.push((kind:if dots==1{"floatval"}else{"intval"},text:s,line:l,col:cc))
    } else {
      let two = c + n
      if two in operators { out.push((kind:"op",text:two,line:line,col:col)); i += 2; col += 2 }
      else if c in operators { out.push((kind:"op",text:c,line:line,col:col)); i += 1; col += 1 }
      else if c == "=" { out.push((kind:"eq",text:c,line:line,col:col)); i += 1; col += 1 }
      else if c in punctuation { out.push((kind:punctuation.at(c),text:c,line:line,col:col)); i += 1; col += 1 }
      else { failure("lexical", line, col, "invalid character " + c) }
    }
    }
  }
  out.push((kind:"eof",text:"",line:line,col:col)); out
}

#let token-labels=(nl:"NEWLINE",id:"VARIABLE",eq:"EQUALS",intval:"INTVAL",floatval:"FLOATVAL",op:"OP",string:"STRING",lp:"LPAREN",rp:"RPAREN",lb:"LSQUARE",rb:"RSQUARE",lc:"LCURLY",rc:"RCURLY",comma:"COMMA",colon:"COLON",dot:"DOT",eof:"END_OF_FILE")
#let format-lex(source)={
  lex(source).map(t=>{
    let label=token-labels.at(t.kind,default:upper(t.kind))
    if t.kind in ("nl","eof"){label}else if t.kind=="string"{label+" '\""+t.text+"\"'"}else{label+" '"+t.text+"'"}
  }).join("\n")
}
