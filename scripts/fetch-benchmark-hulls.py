#!/usr/bin/env python3
"""Fetch the KCS and DTMB 5415 benchmark hulls and convert them to offsets.

The geometry is NOT redistributed with this repository: the SIMMAN 2014
workshop site (FORCE Technology) publishes it without stated licence terms,
so this script downloads it on demand, checks it against pinned SHA-256
hashes, tessellates the IGES NURBS surfaces (entity 128) into an OBJ mesh and
runs `tpt-yard import-hull` on it. Output goes to `test-data/benchmarks/`
(git-ignored). Needs Python 3.9+ with numpy and a built or buildable
`tpt-yard` CLI.

    python scripts/fetch-benchmark-hulls.py [kcs|dtmb5415|all] [--out DIR]
                                            [--grid N] [--cli PATH]

Sources (check the site for terms before any redistribution):
  KCS       https://simman2014.dk/ship-data/moeri-container-ship/
  DTMB 5415 https://simman2014.dk/ship-data/us-navy-combatant/

After conversion the script compares the displacement at the published
design draft against the published particulars and fails if it is off by
more than 2 %. The 5415 file is a 1:24.83 model (INSEAN) with the sonar
dome below the keel line, so its baseline (the lowest point) is the dome
bottom and the design draft is offset accordingly.
"""
import argparse
import hashlib
import io
import subprocess
import sys
import urllib.request
import zipfile
from pathlib import Path

import numpy as np

BASE = "https://simman2014.dk/wp-content/uploads/2015/07/"
RHO = 1.025  # t/m3, the engine's sea-water density

HULLS = {
    "kcs": dict(
        zip="KCS-Hull.zip",
        sha256="b50adace1ddf255fc0b471d3394edaacd27d3bebb298f3b1fc5dad5ae4f7f88c",
        member="KCS_hull_SVA.igs",
        scale=1.0,  # full scale, metres
        lpp=230.0, beam=32.2, draft=10.8, cb=0.651,  # published (Cb on Lpp x Bwl x T)
        label="KCS",
    ),
    "dtmb5415": dict(
        zip="5415-Hull-Bare.zip",
        sha256="daae8a4bca5e586721f2ee71285d390625ec299782d5ee45a4fb22a4e789f50c",
        member="5415.igs",
        scale=24.83,  # the INSEAN model scale; the file is in model metres
        lpp=142.0, beam=19.06, draft=6.15, cb=0.507,
        label="DTMB 5415",
    ),
}


def fetch(spec, cache):
    path = cache / spec["zip"]
    if not path.exists():
        print(f"downloading {BASE}{spec['zip']}")
        # The site rejects urllib's default agent with a 403.
        req = urllib.request.Request(BASE + spec["zip"], headers={"User-Agent": "tpt-shipyard-fetch/1.0"})
        with urllib.request.urlopen(req, timeout=60) as r:
            path.write_bytes(r.read())
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != spec["sha256"]:
        path.unlink()
        sys.exit(f"{spec['zip']}: SHA-256 {digest} differs from the pinned value — "
                 "the upstream file changed; review it and update the pin")
    with zipfile.ZipFile(io.BytesIO(path.read_bytes())) as z:
        return z.read(spec["member"]).decode("latin-1")


def read_surfaces(text):
    """Rational B-spline surfaces (IGES entity 128) from the P section."""
    pd = [l[:64] for l in text.splitlines() if l[72:73] == "P"]
    out = []
    for rec in "".join(pd).replace("D", "E").split(";"):
        f = [x.strip() for x in rec.split(",")]
        if not f or f[0] != "128":
            continue
        k1, k2, m1, m2 = map(int, f[1:5])
        n1, n2 = k1 + 1, k2 + 1
        i = 10
        s = np.array(f[i:i + k1 + m1 + 2], float); i += k1 + m1 + 2
        t = np.array(f[i:i + k2 + m2 + 2], float); i += k2 + m2 + 2
        w = np.array(f[i:i + n1 * n2], float).reshape(n2, n1).T; i += n1 * n2
        c = np.array(f[i:i + 3 * n1 * n2], float).reshape(n2, n1, 3).transpose(1, 0, 2); i += 3 * n1 * n2
        u0, u1, v0, v1 = map(float, f[i:i + 4])
        out.append((m1, m2, s, t, w, c, (u0, u1, v0, v1)))
    if not out:
        sys.exit("no NURBS surfaces found in the IGES file")
    return out


def basis(knots, deg, u, n):
    """B-spline basis matrix (len(u), n) by Cox-de Boor; right end closed."""
    u = np.asarray(u, float)
    b = np.zeros((len(u), len(knots) - 1))
    for i in range(len(knots) - 1):
        inside = (knots[i] <= u) & (u < knots[i + 1])
        if knots[i] < knots[i + 1] and knots[i + 1] == knots[-1]:
            inside |= u == knots[-1]
        b[:, i] = inside
    for d in range(1, deg + 1):
        nb = np.zeros((len(u), len(knots) - 1 - d))
        for i in range(len(knots) - 1 - d):
            a = knots[i + d] - knots[i]
            c = knots[i + d + 1] - knots[i + 1]
            if a > 0:
                nb[:, i] += (u - knots[i]) / a * b[:, i]
            if c > 0:
                nb[:, i] += (knots[i + d + 1] - u) / c * b[:, i + 1]
        b = nb
    return b[:, :n]


def tessellate(surfaces, grid, scale):
    verts, faces = [], []
    for m1, m2, s, t, w, c, (u0, u1, v0, v1) in surfaces:
        u = np.linspace(u0, u1, grid)
        v = np.linspace(v0, v1, grid)
        bu = basis(s, m1, u, c.shape[0])
        bv = basis(t, m2, v, c.shape[1])
        num = np.einsum("ui,vj,ij,ijk->uvk", bu, bv, w, c)
        den = np.einsum("ui,vj,ij->uv", bu, bv, w)
        p = (num / den[..., None]).reshape(-1, 3) * scale
        base = len(verts) + 1
        verts.extend(p.tolist())
        for a in range(grid - 1):
            for b in range(grid - 1):
                i = base + a * grid + b
                faces.append((i, i + grid, i + grid + 1, i + 1))
    return np.array(verts), faces


def write_obj(path, verts, faces):
    with open(path, "w") as f:
        for x, y, z in verts:
            f.write(f"v {x:.5f} {y:.5f} {z:.5f}\n")
        for q in faces:
            f.write("f {} {} {} {}\n".format(*q))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("hull", nargs="?", default="all", choices=["all", *HULLS])
    ap.add_argument("--out", default="test-data/benchmarks")
    ap.add_argument("--grid", type=int, default=24, help="samples per surface edge")
    ap.add_argument("--cli", default=None, help="tpt-yard binary (default: cargo run)")
    a = ap.parse_args()
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    cli = a.cli.split() if a.cli else ["cargo", "run", "-q", "-p", "tpt-yard-cli", "--"]

    for key, spec in HULLS.items():
        if a.hull not in ("all", key):
            continue
        print(f"== {spec['label']}")
        verts, faces = tessellate(read_surfaces(fetch(spec, out)), a.grid, spec["scale"])
        zmin = float(verts[:, 2].min())
        write_obj(out / f"{key}.obj", verts, faces)
        # The lowest mesh point becomes the baseline, so the design draft is
        # measured from there (0 offset for KCS, the dome depth for the 5415).
        draft = spec["draft"] - zmin
        cmd = cli + ["import-hull", str(out / f"{key}.obj"), "--name", key,
                     "--out", str(out), "--force", "--draft", f"{draft:.4f}",
                     "--stations", "41", "--levels", "61"]
        r = subprocess.run(cmd, capture_output=True, text=True)
        if r.returncode:
            sys.exit(r.stderr or r.stdout)
        print(r.stdout.strip())
        # Check the converted hull against the published particulars.
        want = spec["cb"] * spec["lpp"] * spec["beam"] * spec["draft"] * RHO
        got = None
        for line in r.stdout.splitlines():
            if line.startswith("at draft"):
                got = float(line.split("displacement")[1].split("t")[0])
        err = abs(got - want) / want
        print(f"displacement at the design draft: {got:.0f} t vs published "
              f"{want:.0f} t ({100 * err:+.2f} %); baseline offset {-zmin:.3f} m")
        if err > 0.02:
            sys.exit(f"{spec['label']}: displacement is off by more than 2 % — conversion suspect")
    print(f"done; outputs in {out}/ (set your own --kg before trusting stability results)")


if __name__ == "__main__":
    main()
