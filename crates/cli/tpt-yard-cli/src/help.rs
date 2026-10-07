//! Command table behind `--help`, `help <command>` and the
//! "did you mean" hint for a mistyped subcommand.

/// `(name, usage, summary)` for every subcommand.
pub const COMMANDS: &[(&str, &str, &str)] = &[
    (
        "validate",
        "validate FILE.json",
        "check a vessel project or hull manifest and report every inconsistency",
    ),
    (
        "plan",
        "plan MANIFEST.json [--json]",
        "block division, erection order, lift checks and schedule in one report",
    ),
    (
        "stability",
        "stability CASE.json [--offsets F.csv] [--draft M] [--kg M] [--fsm TM] [--to-deg DEG] [--flooding-deg DEG] [--csv PREFIX] [--svg F.svg] [--strict] [--force] [--json]",
        "hydrostatics, GZ curve and IMO 2008 criteria from hull offsets or prismatic coefficients",
    ),
    (
        "new-hull",
        "new-hull <wigley|barge|workboat|tug|sailboat|ferry> [--name N] [--loa M] [--beam M] [--draft M] [--depth M] [--kg M] [--out DIR] [--force]",
        "write a runnable hull (offsets CSV + stability case) to start from",
    ),
    (
        "import-hull",
        "import-hull MESH.obj|MESH.stl [--up y|z] [--bow +x|-x] [--scale F] [--stations N] [--levels N] [--draft M] [--kg M] [--name N] [--out DIR] [--force]",
        "slice a hull surface mesh (OBJ or STL) into an offsets CSV + stability case",
    ),
    (
        "schedule",
        "schedule FILE.json [--limit kind=value]... [--json]",
        "critical path and resource levelling for a project's activities",
    ),
    (
        "risk",
        "risk FILE.json [--samples N] [--uncertainty F] [--seed N] [--gate id=hours[:slip]]... [--weather F.json] [--weather-activity ID] [--launch-method slipway|drydock|side|shiplift] [--max-sea-state 0-9] [--vessel-mass-t T] [--way-length-m M] [--friction F] [--breadth-m M] [--json]",
        "Monte Carlo schedule risk with delivery and weather gates",
    ),
    (
        "report",
        "report FILE.json [--json]",
        "weight, CoG and structural summary for a project",
    ),
    (
        "new",
        "new <sea|space|container-ship|submarine|orbital-truss|habitat|solar-array> [NAME] [--out FILE] [--force] [--json]",
        "print a scaffolded project JSON from the workspace templates",
    ),
    (
        "export",
        "export MANIFEST.json [--gltf out.gltf] [--ifc out.ifc] [--force] [--json]",
        "block geometry as glTF 2.0 and/or IFC4 STEP",
    ),
    (
        "html-report",
        "html-report PROJECT.json [--out F.html] [--structure F.json] [--gltf-viewer F.gltf] [--force] [--json]",
        "self-contained HTML calculation package",
    ),
    (
        "pdf-report",
        "pdf-report PROJECT.json [--out F.pdf] [--force] [--json]",
        "PDF calculation package",
    ),
];

/// The full command list.
pub fn overview() -> String {
    let mut s = String::from("tpt-yard — shipyard planning and early-stage naval architecture\n\nUSAGE:\n    tpt-yard <command> [options]\n\nCOMMANDS:\n");
    for (name, _, summary) in COMMANDS {
        s.push_str(&format!("    {name:<12} {summary}\n"));
    }
    s.push_str(
        "\nEvery command takes --json for machine-readable output.\n\
         Output files are never overwritten without --force; missing folders are created.\n\
         Exit codes: 0 ok, 1 a check failed (validate, --strict), 2 usage error, 3 runtime or I/O failure.\n\
         Run `tpt-yard <command> --help` for a command's options.\n",
    );
    s
}

/// One command's usage line and summary; `None` for an unknown name.
pub fn usage_of(cmd: &str) -> Option<String> {
    COMMANDS
        .iter()
        .find(|(name, _, _)| *name == cmd)
        .map(|(_, usage, summary)| format!("{summary}\n\nUSAGE:\n    tpt-yard {usage}\n"))
}

/// The known command closest to a mistyped one: a prefix match, or within
/// two edits.
pub fn suggest(unknown: &str) -> Option<&'static str> {
    // An empty or tiny string is a prefix of everything: no useful hint.
    if unknown.chars().count() < 2 {
        return None;
    }
    COMMANDS
        .iter()
        .map(|(name, _, _)| *name)
        .filter(|name| name.starts_with(unknown) || edit_distance(name, unknown) <= 2)
        .min_by_key(|name| edit_distance(name, unknown))
}

fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != cb);
            cur.push(sub.min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestions_catch_typos_and_prefixes() {
        assert_eq!(suggest("stabilty"), Some("stability"));
        assert_eq!(suggest("sched"), Some("schedule"));
        assert_eq!(suggest("valdiate"), Some("validate"));
        assert_eq!(suggest("zzzzzz"), None);
        assert_eq!(suggest(""), None, "an empty string matches everything");
    }

    #[test]
    fn every_command_has_usage_text() {
        for (name, usage, _) in COMMANDS {
            assert!(usage.starts_with(name), "{name}: {usage}");
            assert!(usage_of(name).is_some());
        }
        assert!(usage_of("nope").is_none());
        assert!(overview().contains("new-hull"));
    }
}
