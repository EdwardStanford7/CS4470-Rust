#import "../compiler.typ": parse
#let path = sys.inputs.at("program")
#let ast = parse(read(path))
#metadata(repr(ast)) <ast-dump>
