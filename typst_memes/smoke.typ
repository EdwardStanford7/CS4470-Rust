#import "compiler.typ": lex, parse
#assert.eq(lex("let x = 1\n").map(t => t.kind), ("let", "id", "eq", "intval", "nl", "eof"))
#let ast = parse("let x = 1 + 2 * 3\nshow x\n")
#assert.eq(ast.len(), 2)
#assert.eq(ast.first().value.tag, "binary")
#assert.eq(ast.first().value.right.tag, "binary")
Smoke tests passed.
