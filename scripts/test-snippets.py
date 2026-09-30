#!/usr/bin/env python3
"""Compile-test every Rust snippet in the book and the root README (review
7F leftover: "compile-test every README and book snippet in CI").

Extracts ```rust fenced blocks from docs/book/src/*.md and README.md, wraps
fragments in `fn main() { ... }` (blocks defining their own main are used
as-is), and compiles each with rustc against the workspace's built rlibs
(`cargo build --workspace` first). Blocks tagged `ignore` or `no_compile`
are skipped.

Exit status 0 = every snippet compiles; 1 = list of failures.
"""

from __future__ import annotations

import glob
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SOURCES = sorted(glob.glob("docs/book/src/*.md")) + ["README.md"]
SKIP_ATTRS = ("ignore", "no_compile")


def workspace_rlibs() -> dict[str, Path]:
    """The newest rlib per workspace crate from target/debug/deps."""
    deps = REPO / "target" / "debug" / "deps"
    best: dict[str, Path] = {}
    for rlib in deps.glob("lib*_*.rlib"):
        m = re.match(r"(lib.+)[-_]([0-9a-f]{8,})\.rlib$", rlib.name)
        if not m:
            continue
        name = m.group(1)[len("lib"):]
        if name not in best or rlib.stat().st_mtime > best[name].stat().st_mtime:
            best[name] = rlib
    return best


def snippets(path: Path):
    text = path.read_text(encoding="utf-8")
    for i, m in enumerate(re.finditer(r"```rust([^\n]*)\n(.*?)```", text, re.S)):
        attrs = m.group(1).strip()
        code = m.group(2)
        if any(a in attrs for a in SKIP_ATTRS):
            continue
        yield i, attrs, code


def externs_for(code: str, rlibs: dict[str, Path]) -> list[tuple[str, Path]]:
    """(crate, rlib) pairs for every workspace crate the snippet imports."""
    used = sorted(set(re.findall(r"use ([a-z_][a-z0-9_]*)\s*::", code)))
    return [(name, rlibs[name]) for name in used if name in rlibs]


def main() -> int:
    subprocess.run(
        ["cargo", "build", "--workspace", "--quiet"], cwd=REPO, check=True
    )
    rlibs = workspace_rlibs()
    if not rlibs:
        print("no rlibs found in target/debug/deps — run from the repo root", file=sys.stderr)
        return 1

    failures = 0
    checked = 0
    # Snippet scratch space INSIDE target/: repo-relative paths for rustc
    # (absolute paths across drives proved unreliable on Windows).
    scratch = REPO / "target" / "snippet-tests"
    scratch.mkdir(parents=True, exist_ok=True)
    for src in SOURCES:
        path = REPO / src
        for i, attrs, code in snippets(path):
            checked += 1
            label = f"{src} block {i} [{attrs or 'plain'}]"
            if "fn main" in code:
                wrapped = code
            elif "?" in code:
                # A fallible fragment: give it a fallible main.
                wrapped = (
                    "fn main() -> Result<(), Box<dyn std::error::Error>> {\n"
                    f"{code}\nOk(())\n}}"
                )
            else:
                wrapped = f"fn main() {{\n{code}\n}}"
            snippet_path = scratch / f"snippet_{i}.rs"
            snippet_path.write_text(wrapped, encoding="utf-8")
            cmd = [
                "rustc",
                "--edition", "2021",
                "--crate-type", "bin",
                "--emit=metadata",
                "-o", "target/snippet-tests/snippet_%d.meta" % i,
                "target/snippet-tests/snippet_%d.rs" % i,
                "-L", "dependency=target/debug/deps",
            ]
            for name, rlib in externs_for(code, rlibs):
                cmd += ["--extern", f"{name}={rlib.relative_to(REPO).as_posix()}"]
            proc = subprocess.run(cmd, capture_output=True, text=True, cwd=REPO)
            snippet_path.unlink(missing_ok=True)
            (scratch / f"snippet_{i}.meta").unlink(missing_ok=True)
            if proc.returncode != 0:
                failures += 1
                print(f"FAIL {label}\n$ {' '.join(cmd)}\n{proc.stderr}\n")
            else:
                print(f"ok   {label}")
    print(f"\n{checked} snippets checked, {failures} failing")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
