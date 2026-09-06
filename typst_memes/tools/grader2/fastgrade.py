#!/usr/bin/env python3
"""
fastgrade.py -- a faster, ground-truth CS4470 grader runner for the Typst
JPL compiler in typst_memes/.

Why this exists (vs `python3 grader/grader.py`):
  1. The stock grader runs homework assignments SERIALLY, one at a time,
     each with its own thread pool (`--threads N`). This script instead
     pours every single test from every requested assignment into ONE
     shared thread pool, so small assignments don't wait behind big ones
     and total wall-clock time tracks your slowest core-bound test, not
     the sum of all assignments.
  2. Every test is I/O-bound (waiting on a `make run` -> `typst query`
     subprocess), so a large worker count is fine even though this is
     pure Python -- the GIL is released during subprocess.wait().
  3. The instant an assembly-diff test (hw10-15, DiffSpec/OptSpec with
     `-s`/`-O1`/`-O3` flags) fails, this script immediately runs the same
     bisection asm_bisect.py does (first normalized-line divergence, with
     context) and prints it right there -- no separate manual step.

This script imports grader/grader.py and grader/normalize_asm.py directly
(by path) rather than reimplementing test discovery/execution, so results
are guaranteed identical to the real grader -- this is a faster harness
around the same logic, not a competing reimplementation.

Usage:
    tools/grader2/fastgrade.py                     # all of hw2-15, hw1 too
    tools/grader2/fastgrade.py --hw 10,11,12        # just these
    tools/grader2/fastgrade.py --hw current         # grader.py's CURRENT_HW
    tools/grader2/fastgrade.py --workers 24
    tools/grader2/fastgrade.py --no-bisect          # skip auto-bisection
    tools/grader2/fastgrade.py --skip-hw1           # hw1 needs `compare` (ImageMagick) and is not thread-pool based
"""
import argparse
import concurrent.futures as futures
import importlib.util
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parent          # .../typst_memes/tools/grader2
COMPILER_DIR = TOOLS_DIR.parents[1]                  # .../typst_memes
REPO_ROOT = COMPILER_DIR.parent                      # .../CS4470-Rust
GRADER_DIR = REPO_ROOT / "grader"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load {name} from {path}")
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod  # so grader.py's own `from normalize_asm import ...` resolves
    spec.loader.exec_module(mod)
    return mod


# grader.py does `from normalize_asm import ppasm, ppc, commentasm` and
# `from ppsexp import ppsexp, unpp` as plain top-level imports, so its own
# directory must be on sys.path before we load it.
sys.path.insert(0, str(GRADER_DIR))
grader = load_module("grader", GRADER_DIR / "grader.py")
grader.DIR = str(COMPILER_DIR)

# Reuse asm_bisect's building blocks for instant on-failure bisection.
asm_bisect = load_module("asm_bisect", TOOLS_DIR.parent / "asm_bisect.py")


@dataclass
class TestJob:
    hw: str
    part: str
    filespec: object


@dataclass
class TestResult:
    job: TestJob
    status: str
    msg: str
    out: str | None
    err: str | None
    bisect_report: str | None = None
    elapsed: float = 0.0


def is_asm_test(filespec) -> bool:
    return isinstance(filespec, (grader.DiffSpec, grader.OptSpec)) and "-s" in getattr(filespec, "flags", "")


def bisect_flags_for(filespec) -> list[str]:
    """Which `-s [...]` flag strings to try bisecting, in order."""
    if isinstance(filespec, grader.OptSpec):
        return ["-s", f"-s {filespec.flags}"]
    return [filespec.flags]


def run_bisection(filespec) -> str:
    lines = []
    for flags in bisect_flags_for(filespec):
        exp_path = asm_bisect.expected_path_for(filespec.in_file, flags)
        if not exp_path.exists():
            continue
        ok, out = asm_bisect.run_compiler(filespec.in_file, flags)
        if not ok:
            lines.append(f"  [{flags!r}] compiler did not produce output: {out.splitlines()[0] if out else '<empty>'}")
            continue
        expected = asm_bisect.normalized_lines(exp_path.read_text())
        actual = asm_bisect.normalized_lines(out)
        idx = asm_bisect.first_divergence(expected, actual)
        if idx is None:
            lines.append(f"  [{flags!r}] matches exactly ({len(expected)} lines) -- divergence must be in a different flag variant")
            continue
        exp_snip = expected[idx] if idx < len(expected) else "<missing>"
        act_snip = actual[idx] if idx < len(actual) else "<missing>"
        diff_count = sum(1 for e, a in zip(expected, actual) if e != a) + abs(len(expected) - len(actual))
        lines.append(
            f"  [{flags!r}] first divergence @normalized line {idx} ({diff_count} lines differ)\n"
            f"      expected: {exp_snip}\n"
            f"      actual:   {act_snip}"
        )
    return "\n".join(lines) if lines else "  (no expected file found to bisect against)"


def run_one(job: TestJob) -> TestResult:
    t0 = time.monotonic()
    status, msg, out, err = grader.test_one(job.filespec)
    elapsed = time.monotonic() - t0
    bisect_report = None
    if status != "." and is_asm_test(job.filespec):
        try:
            bisect_report = run_bisection(job.filespec)
        except Exception as e:  # bisection is best-effort, never let it mask the real failure
            bisect_report = f"  (bisection itself errored: {e!r})"
    return TestResult(job, status, msg, out, err, bisect_report, elapsed)


def collect_jobs(hw_selector: str, part_selector: str, test_selector: str) -> list[TestJob]:
    homeworks = grader.get_keys(grader.HWS, hw_selector, "homework")
    jobs = []
    for hwname, homework in homeworks.items():
        parts = grader.get_keys(homework, part_selector, f"hw{hwname} part")
        for partname, part in parts.items():
            tests = [t for t in part.all() if t.matches(test_selector)]
            for t in tests:
                jobs.append(TestJob(hwname, partname, t))
    return jobs


def run_hw1(skip: bool) -> tuple[int, int]:
    """hw1 isn't in grader.HWS at all -- it's a standalone shell script that
    needs ImageMagick's `compare`. Run it as one opaque job alongside everything else."""
    if skip:
        return 0, 0
    if not shutil.which("compare"):
        print("hw1: SKIPPED (ImageMagick `compare` not installed)")
        return 0, 0
    script = GRADER_DIR / "hw1" / "test-part"
    res = subprocess.run(["sh", str(script), str(COMPILER_DIR), "all"], capture_output=True, text=True)
    ok = res.returncode == 0
    print(f"hw1: {'PASS' if ok else 'FAIL'} (exit {res.returncode})")
    if not ok:
        tail = "\n".join((res.stdout + res.stderr).splitlines()[-20:])
        print(tail)
    return (0 if ok else 1), 1


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--hw", default="all", help="which homeworks (e.g. '10,11,12', a single number, or 'all'/'current')")
    p.add_argument("--part", default="all", help="which parts within each homework")
    p.add_argument("--test", default="all", help="which specific test name prefix")
    p.add_argument("--workers", type=int, default=24, help="global thread pool size (I/O-bound, safe to oversubscribe cores)")
    p.add_argument("--no-bisect", action="store_true", help="disable automatic bisection on assembly-test failure")
    p.add_argument("--skip-hw1", action="store_true", help="skip hw1 (needs ImageMagick `compare`, not part of grader.HWS)")
    p.add_argument("--quiet", action="store_true", help="only print failures and the final summary, not per-test dots")
    args = p.parse_args()

    hw_selector = "all" if args.hw == "all" else args.hw
    if hw_selector != "all":
        hw_selector = grader.CURRENT_HW if hw_selector == "current" else hw_selector

    t_start = time.monotonic()

    hw1_failures, hw1_total = (0, 0)
    if hw_selector in ("all",) or "1" == hw_selector or (hw_selector != "all" and "1" in hw_selector.split(",")):
        hw1_failures, hw1_total = run_hw1(args.skip_hw1)

    jobs = collect_jobs(hw_selector, args.part, args.test)
    print(f"Collected {len(jobs)} tests across {len(set((j.hw, j.part) for j in jobs))} (hw, part) groups. "
          f"Running with {args.workers} workers...\n")

    results: list[TestResult] = []
    with futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
        for i, res in enumerate(pool.map(run_one, jobs)):
            results.append(res)
            if not args.quiet:
                print(res.status, end="", flush=True)
                if (i + 1) % 100 == 0:
                    print(f"  [{i + 1}/{len(jobs)}]")
            if res.status != ".":
                print()
                print(f"hw{res.job.hw} part {res.job.part} :: {res.msg}")
                if res.bisect_report and not args.no_bisect:
                    print(res.bisect_report)

    print()
    elapsed = time.monotonic() - t_start

    # Aggregate per (hw, part)
    groups: dict[tuple[str, str], list[TestResult]] = {}
    for r in results:
        groups.setdefault((r.job.hw, r.job.part), []).append(r)

    print("=" * 60)
    print("SCOREBOARD")
    print("=" * 60)
    total = 0
    failed = 0
    for hw in sorted({hw for hw, _ in groups}, key=lambda x: (len(x), x)):
        hw_results = [r for (h, _p), rs in groups.items() if h == hw for r in rs]
        hw_total = len(hw_results)
        hw_failed = sum(1 for r in hw_results if r.status != ".")
        total += hw_total
        failed += hw_failed
        pct = (1 - hw_failed / hw_total) * 100 if hw_total else 0.0
        marker = "OK " if hw_failed == 0 else "FAIL"
        print(f"[{marker}] hw{hw:<4} {hw_total - hw_failed:>4}/{hw_total:<4} = {pct:5.1f}%")
    if hw1_total:
        total += hw1_total
        failed += hw1_failures
        print(f"[{'OK ' if hw1_failures == 0 else 'FAIL'}] hw1     {hw1_total - hw1_failures:>4}/{hw1_total:<4}")

    print("-" * 60)
    overall_pct = (1 - failed / total) * 100 if total else 100.0
    print(f"TOTAL: {total - failed}/{total} = {overall_pct:.1f}%  ({elapsed:.1f}s wall clock, {args.workers} workers)")
    sys.exit(1 if failed or hw1_failures else 0)


if __name__ == "__main__":
    main()
