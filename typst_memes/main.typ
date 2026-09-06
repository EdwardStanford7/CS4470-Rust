#import "compiler.typ": compile
#set page(width: auto, height: auto, margin: 12pt)
#set text(font: "DejaVu Sans Mono", size: 8pt)
#let path = sys.inputs.at("program", default: "../t2.jpl")
#raw(compile(read(path), backend: "wasmgc"), lang: "wat")
