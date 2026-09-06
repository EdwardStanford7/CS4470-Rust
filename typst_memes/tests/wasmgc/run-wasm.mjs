import { readFileSync } from "node:fs";

const wasmPath = process.argv[2];
const bytes = readFileSync(wasmPath);

const output = [];
const push = (s) => output.push(s);

function readString(memArray) {
  // GC array of (mut i8) -> JS array of byte values
  return Buffer.from(memArray).toString("utf8");
}

const importObject = {
  jpl: {
    print: (s) => push(readString(s)),
    fail: (s) => { throw new Error("assertion failed: " + readString(s)); },
    show_i64: (n) => push(String(n)),
    show_f64: (f) => push(String(f)),
    show_bool: (b) => push(b ? "true" : "false"),
    show_ref: (r) => push(String(r)),
    time: () => performance.now() / 1000,
    read_image: (_path) => null,
    write_image: (_obj, _path) => {},
    sqrt: Math.sqrt, exp: Math.exp, sin: Math.sin, cos: Math.cos, tan: Math.tan,
    asin: Math.asin, acos: Math.acos, atan: Math.atan, log: Math.log,
    pow: Math.pow, atan2: Math.atan2,
    to_float: (i) => Number(i), to_int: (f) => BigInt(Math.trunc(f)),
  },
};

const { instance } = await WebAssembly.instantiate(bytes, importObject);
instance.exports.jpl_main();
process.stdout.write(output.join("\n") + (output.length ? "\n" : ""));
