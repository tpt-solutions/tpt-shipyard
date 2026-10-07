#!/usr/bin/env python3
"""Package the release `tpt-yard` binary for one target.

    cargo build --release --locked -p tpt-yard-cli
    python scripts/package-cli.py x86_64-unknown-linux-gnu 0.1.0

Writes `dist/tpt-yard-v<version>-<target>.tar.gz` (`.zip` for Windows
targets) plus a `.sha256` next to it. The archive holds one top-level
directory with the binary, both licences, the README, the project
templates and the JSON schemas.
"""
import hashlib
import sys
import tarfile
import zipfile
from pathlib import Path

if len(sys.argv) != 3:
    sys.exit(__doc__)
target, version = sys.argv[1], sys.argv[2].lstrip("v")
windows = "windows" in target.lower()
root = Path(__file__).resolve().parent.parent
exe = root / "target" / "release" / ("tpt-yard.exe" if windows else "tpt-yard")
if not exe.exists():
    sys.exit(f"{exe} not found: run `cargo build --release --locked -p tpt-yard-cli` first")

name = f"tpt-yard-v{version}-{target}"
files = [(exe, exe.name)]
files += [(root / f, f) for f in ("LICENSE-MIT", "LICENSE-APACHE", "README.md")]
for d in ("templates", "schemas"):
    files += [(p, f"{d}/{p.name}") for p in sorted((root / d).iterdir()) if p.is_file()]

dist = root / "dist"
dist.mkdir(exist_ok=True)
archive = dist / (name + (".zip" if windows else ".tar.gz"))
if windows:
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as z:
        for src, rel in files:
            z.write(src, f"{name}/{rel}")
else:
    with tarfile.open(archive, "w:gz") as t:
        for src, rel in files:
            info = t.gettarinfo(src, f"{name}/{rel}")
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            if rel == exe.name:
                info.mode = 0o755
            with open(src, "rb") as f:
                t.addfile(info, f)
digest = hashlib.sha256(archive.read_bytes()).hexdigest()
Path(str(archive) + ".sha256").write_text(f"{digest}  {archive.name}\n")
print(f"{archive} ({archive.stat().st_size} bytes)\nsha256 {digest}")
