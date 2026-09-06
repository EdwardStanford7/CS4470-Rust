#import "compiler.typ": parse
#let path = sys.inputs.at("program")
#metadata(parse(read(path)).len()) <parse-count>
