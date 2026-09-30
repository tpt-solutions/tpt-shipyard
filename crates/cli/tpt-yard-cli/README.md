# tpt-yard-cli

Command-line front end for [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard).

```text
tpt-yard validate <project-or-manifest.json>   # consistency check
tpt-yard plan <hull-manifest.json> [--json]    # end-to-end: blocks -> erection -> lift -> schedule
tpt-yard schedule <project.json> [--json]      # critical path + levelling
tpt-yard report <project.json> [--json]        # weight / CoG / structural summary
tpt-yard new <sea|space> [NAME] [--json]       # scaffold a project
```

`plan` consumes the hull manifests in `test-data/hull-blocks/`, e.g.

```sh
cargo run -p tpt-yard-cli -- plan test-data/hull-blocks/container-ship-140m.json
```
