#import "compiler.typ": check
#let path = sys.inputs.at("program")
#metadata(check(read(path)).ast.len()) <check-count>
