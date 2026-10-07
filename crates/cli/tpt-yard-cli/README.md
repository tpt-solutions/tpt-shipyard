# tpt-yard-cli

Command-line front end for [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard).

```text
tpt-yard validate <project-or-manifest.json>   # consistency check
tpt-yard plan <hull-manifest.json> [--json]    # end-to-end: blocks -> erection -> lift -> schedule
tpt-yard export <hull-manifest.json> --gltf hull.gltf --ifc hull.ifc
                                               # block geometry as glTF 2.0 / IFC4 STEP
tpt-yard schedule <project.json> [--limit crane=200 --limit crew=40] [--json]
                                               # critical path + levelling; --limit states
                                               # yard-wide capacities per resource kind for
                                               # capacity-aware levelling
tpt-yard report <project.json> [--json]        # weight / CoG / structural summary
tpt-yard new-hull <wigley|barge|workboat|tug|sailboat|ferry> [--loa M --beam M --draft M --depth M --kg M] [--out DIR]
                                               # write a runnable hull: offsets CSV + stability case
tpt-yard --help | --version | <command> --help # usage; a mistyped command suggests the nearest one
tpt-yard stability <case.json> [--offsets hull.csv] [--draft M] [--kg M] [--fsm TM]
                   [--to-deg DEG] [--csv PREFIX] [--svg gz.svg] [--strict] [--json]
                                               # hydrostatics + GZ curve + IMO 2008 criteria
                                               # from a hull-offsets CSV or prismatic coefficients;
                                               # --strict exits 1 on a failed criterion
tpt-yard new <sea|space> [NAME] [--json]       # scaffold a project
```

`plan` consumes the hull manifests in `test-data/hull-blocks/`, e.g.

```sh
cargo run -p tpt-yard-cli -- plan test-data/hull-blocks/container-ship-140m.json
```
