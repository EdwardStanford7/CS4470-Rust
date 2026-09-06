# tools/

## asm_bisect.py

Finds the *first* instruction where the compiler's `-s` output diverges
from a grader `.expected` file, instead of eyeballing a whole-file diff.
Uses the exact same normalization as the real grader
(`grader/normalize_asm.py`'s `ppasm`/`normalize_line`), so a clean result
here is a guaranteed grader pass.

Single file:

    tools/asm_bisect.py path/to/test.jpl
    tools/asm_bisect.py path/to/test.jpl --flags "-s -O1"   # diffs against .expected.opt
    tools/asm_bisect.py path/to/test.jpl --context 10
    tools/asm_bisect.py path/to/test.jpl --expected some/other.expected

Prints a context diff centered on the first normalized-line mismatch (both
sides, with `>>` marking the diverging line), a grader-style differing-line
count, and a best-effort proportional correlation to the `.jpl` source
(asm has no source-line tags, so this is approximate, not exact).

Directory (recursively finds every `*.jpl` with a matching expected file):

    tools/asm_bisect.py grader/hw10/ok-fuzzer1
    tools/asm_bisect.py grader/hw11/ok1 --summary   # one line per failure, no context

If the compiler panics/fails outright (e.g. "command not representable
yet"), that's reported as a compiler error rather than a line diff.

Exit code is 0 iff everything matched.
