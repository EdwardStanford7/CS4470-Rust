#import "compiler.typ": compile
#let path = sys.inputs.at("program", default: "sample.jpl")
#metadata(compile(read(path), backend: "wasmgc")) <wat-output>
