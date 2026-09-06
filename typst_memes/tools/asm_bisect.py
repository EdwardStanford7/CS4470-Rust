#!/usr/bin/env python3
"""
asm_bisect.py -- find the FIRST instruction where the Typst JPL compiler's
`-s` (or `-s -O1` / `-s -O3`) output diverges from the grader's expected
assembly, instead of eyeballing a whole-file diff.

Usage:
    tools/asm_bisect.py <file.jpl> [--flags "-s -O1"] [--expected PATH] [--context N]
    tools/asm_bisect.py <directory> [--flags "-s"] [--summary]

Single-file mode prints a side-by-side context diff centered on the first
normalized-line divergence, plus a total differing-line count (same notion
of "count" the grader itself reports).

Directory mode walks every *.jpl file that has a matching *.jpl.expected
(or *.jpl.expected.opt when --flags contains -O1/-O3), runs each one, and
prints one line per failing file: the first diverging line number and a
short snippet, so you can spot whether many files break at the "same kind"
of instruction.

Normalization is byte-for-byte the same as the real grader: this script
imports grader/normalize_asm.py directly rather than reimplementing it, so
a "clean" result here means the grader would also call it a pass.
"""
import argparse
import importlib.util
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]   # .../CS4470-Rust
COMPILER_DIR = Path(__file__).resolve().parents[1]  # .../typst_memes
NORMALIZE_PATH = REPO_ROOT / "grader" / "normalize_asm.py"

TIMEOUT = 60


def load_normalize_asm():
    spec = importlib.util.spec_from_file_location("normalize_asm", NORMALIZE_PATH)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load normalize_asm from {NORMALIZE_PATH}")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


normalize_asm = load_normalize_asm()


def run_compiler(jpl_file: Path, flags: str) -> tuple[bool, str]:
    """Mirrors grader.py's run_student(): `make -C <compiler dir> run FLAGS=... TEST=...`."""
    try:
        res = subprocess.run(
            ["make", "--silent", "--no-print-directory", "-C", str(COMPILER_DIR),
             "run", f"FLAGS={flags}", f"TEST={jpl_file.resolve()}"],
            capture_output=True, timeout=TIMEOUT, text=True,
        )
    except subprocess.TimeoutExpired:
        return False, "<compiler timed out>"
    out = res.stdout.strip()
    parts = out.rsplit("\n", 1)
    body, last_line = (parts[0], parts[1]) if len(parts) > 1 else ("", parts[0])
    last_line = last_line.casefold()
    ok = "compilation succeeded" in last_line
    if not ok and "compilation failed" not in last_line:
        # Malformed output (crash, missing marker, etc) -- surface stderr too.
        return False, out + ("\n--- stderr ---\n" + res.stderr.strip() if res.stderr.strip() else "")
    if not ok:
        return False, out
    return True, body


def expected_path_for(jpl_file: Path, flags: str) -> Path:
    opt = ".expected.opt" if ("-O1" in flags or "-O3" in flags or "-O2" in flags) else ".expected"
    return jpl_file.parent / (jpl_file.name + opt)


def normalized_lines(text: str) -> list[str]:
    return list(normalize_asm.ppasm(text.strip().split("\n")))


def first_divergence(expected: list[str], actual: list[str]) -> int | None:
    for i, (e, a) in enumerate(zip(expected, actual)):
        if e != a:
            return i
    if len(expected) != len(actual):
        return min(len(expected), len(actual))
    return None


def print_context_diff(expected: list[str], actual: list[str], idx: int, context: int, label: str):
    lo = max(0, idx - context)
    hi_e = min(len(expected), idx + context + 1)
    hi_a = min(len(actual), idx + context + 1)
    print(f"=== {label}: first divergence at normalized line {idx} ===")
    print(f"--- expected (lines {lo}-{hi_e - 1} of {len(expected)}) ---")
    for i in range(lo, hi_e):
        marker = ">> " if i == idx else "   "
        print(f"{marker}{i:5d} | {expected[i]}")
    print(f"--- actual   (lines {lo}-{hi_a - 1} of {len(actual)}) ---")
    for i in range(lo, hi_a):
        marker = ">> " if i == idx else "   "
        line = actual[i] if i < len(actual) else "<missing>"
        print(f"{marker}{i:5d} | {line}")
    diff_count = sum(1 for e, a in zip(expected, actual) if e != a) + abs(len(expected) - len(actual))
    print(f"total differing lines (grader-style count): {diff_count}")


def find_source_context(jpl_file: Path, idx: int, total: int):
    """Best-effort: no line-tagging in the asm, so just show source alongside
    proportionally (asm instruction idx / total maps roughly onto source
    line idx / total source lines) as a rough correlation aid."""
    try:
        src_lines = jpl_file.read_text().splitlines()
    except OSError:
        return
    if not src_lines or total == 0:
        return
    approx = int(len(src_lines) * idx / total)
    approx = max(0, min(len(src_lines) - 1, approx))
    lo, hi = max(0, approx - 3), min(len(src_lines), approx + 4)
    print(f"--- {jpl_file.name} source, rough correlation around line {approx + 1} (approximate only) ---")
    for i in range(lo, hi):
        marker = ">> " if i == approx else "   "
        print(f"{marker}{i + 1:4d} | {src_lines[i]}")


def do_single(jpl_file: Path, flags: str, expected_override: Path | None, context: int):
    exp_path = expected_override or expected_path_for(jpl_file, flags)
    if not exp_path.exists():
        print(f"error: no expected file at {exp_path}", file=sys.stderr)
        return 2
    ok, out = run_compiler(jpl_file, flags)
    if not ok:
        print(f"compiler did not produce output for {jpl_file} with flags {flags!r}:")
        print(out)
        return 1
    expected = normalized_lines(exp_path.read_text())
    actual = normalized_lines(out)
    idx = first_divergence(expected, actual)
    if idx is None:
        print(f"OK: {jpl_file.name} matches {exp_path.name} exactly ({len(expected)} normalized lines)")
        return 0
    print_context_diff(expected, actual, idx, context, f"{jpl_file.name} vs {exp_path.name} (flags={flags!r})")
    find_source_context(jpl_file, idx, max(len(expected), 1))
    return 1


def do_directory(dir_path: Path, flags: str, summary: bool):
    opt = "-O1" in flags or "-O3" in flags or "-O2" in flags
    suffix = ".expected.opt" if opt else ".expected"
    jpl_files = sorted(dir_path.rglob("*.jpl"))
    total = 0
    failed = 0
    for jpl in jpl_files:
        exp_path = jpl.parent / (jpl.name + suffix)
        if not exp_path.exists():
            continue
        total += 1
        ok, out = run_compiler(jpl, flags)
        if not ok:
            failed += 1
            first_line = out.strip().splitlines()[0] if out.strip() else "<no output>"
            print(f"FAIL {jpl.relative_to(dir_path)}: compiler error: {first_line[:80]}")
            continue
        expected = normalized_lines(exp_path.read_text())
        actual = normalized_lines(out)
        idx = first_divergence(expected, actual)
        if idx is None:
            continue
        failed += 1
        diff_count = sum(1 for e, a in zip(expected, actual) if e != a) + abs(len(expected) - len(actual))
        exp_snip = expected[idx] if idx < len(expected) else "<missing>"
        act_snip = actual[idx] if idx < len(actual) else "<missing>"
        print(f"FAIL {jpl.relative_to(dir_path)}: first divergence @line {idx} "
              f"({diff_count} lines differ) expected={exp_snip!r} actual={act_snip!r}")
        if not summary:
            print_context_diff(expected, actual, idx, 2, str(jpl.relative_to(dir_path)))
    print(f"\n{total - failed}/{total} match exactly (flags={flags!r})")
    return 0 if failed == 0 else 1


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("path", type=Path, help="a .jpl file or a directory of tests")
    p.add_argument("--flags", default="-s", help="compiler flags to pass (default: -s)")
    p.add_argument("--expected", type=Path, default=None, help="override expected file (single-file mode only)")
    p.add_argument("--context", type=int, default=5, help="lines of context around divergence (single-file mode)")
    p.add_argument("--summary", action="store_true", help="directory mode: one line per failure, no context diff")
    args = p.parse_args()

    if args.path.is_dir():
        return do_directory(args.path, args.flags, args.summary)
    return do_single(args.path, args.flags, args.expected, args.context)


if __name__ == "__main__":
    sys.exit(main())
