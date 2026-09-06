# WebAssembly-GC host ABI

The generated WAT imports these functions from module `jpl`:

| name | signature | purpose |
|---|---|---|
| `print` | `(param (ref $string))` | print UTF-8 text |
| `fail` | `(param (ref $string))` | report assertion failure and trap |
| `show_i64` | `(param i64)` | display an integer |
| `show_f64` | `(param f64)` | display a float |
| `show_bool` | `(param i32)` | display a boolean |
| `time` | `(result f64)` | monotonic clock |
| `read_image` | `(param (ref $string)) (result (ref $obj))` | load an RGBA image |
| `write_image` | `(param (ref $obj)) (param (ref $string))` | store an RGBA image |

Strings in this textual backend are represented by immutable GC arrays of
bytes. JPL arrays are represented by a GC struct containing an element array
and an immutable array of dimensions. Struct declarations become distinct
WebAssembly GC struct types. Images use the ordinary rank-two JPL array object
with `rgba` elements; decoding and encoding remain host responsibilities
behind `read_image` and `write_image`.
