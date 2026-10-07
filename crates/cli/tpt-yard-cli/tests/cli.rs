//! End-to-end tests of the `tpt-yard` binary: argument handling, exit
//! codes, JSON validity, file output and paths containing spaces.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tpt-yard"))
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn template() -> String {
    root()
        .join("templates/container-ship.json")
        .to_string_lossy()
        .into_owned()
}

fn manifest() -> String {
    root()
        .join("test-data/hull-blocks/container-ship-140m.json")
        .to_string_lossy()
        .into_owned()
}

/// A scratch directory whose name contains a space (Windows paths do).
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tpt yard cli {tag} {}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> Output {
    bin().args(args).output().expect("runs")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn parses_as_json(text: &str) -> bool {
    tpt_yard_core::json::Value::parse(text.trim()).is_ok()
}

#[test]
fn version_and_help_work_and_exit_zero() {
    let v = run(&["--version"]);
    assert!(v.status.success() && stdout(&v).starts_with("tpt-yard "));
    let h = run(&["stability", "--help"]);
    assert!(h.status.success() && stdout(&h).contains("stability CASE.json"));
}

/// 8B9: usage mistakes exit 2 with the usage text; a missing file exits 3
/// without it; a failed check exits 1.
#[test]
fn exit_codes_distinguish_usage_runtime_and_check_failures() {
    let usage = run(&["validate", "--bogus", "x.json"]);
    assert_eq!(usage.status.code(), Some(2));
    assert!(stderr(&usage).contains("unknown option '--bogus'"));
    assert!(stderr(&usage).contains("validate FILE.json"), "usage shown");

    let missing = run(&["validate", "no-such-file.json"]);
    assert_eq!(missing.status.code(), Some(3), "{}", stderr(&missing));
    assert!(
        !stderr(&missing).contains("validate FILE.json"),
        "no usage for I/O errors"
    );

    let dir = scratch("exit");
    let bad = dir.join("bad.json");
    std::fs::write(&bad, "{\"build_phases\": 7}").unwrap();
    let check = run(&["validate", &bad.to_string_lossy()]);
    assert_eq!(check.status.code(), Some(1), "{}", stderr(&check));

    assert_eq!(run(&["nope"]).status.code(), Some(2));
    assert_eq!(run(&[]).status.code(), Some(2));
}

/// 8B7: flags may precede the path, a later argument equal to the path is
/// kept, `--help` as a flag value is a value, and unknown flags are rejected
/// for every command.
#[test]
fn arguments_are_position_independent_and_strict() {
    let t = template();
    let ok = run(&["schedule", "--limit", "crane=5", &t]);
    assert!(ok.status.success(), "{}", stderr(&ok));
    for cmd in ["validate", "plan", "report"] {
        let extra = run(&[cmd, &t, "--unknown"]);
        assert_eq!(extra.status.code(), Some(2), "{cmd}: {}", stderr(&extra));
        let two = run(&[cmd, &t, &t]);
        assert_eq!(two.status.code(), Some(2), "{cmd} with a surplus argument");
    }
    // `--help` after --weather is the forecast path, not a help request:
    // the run fails reading it (exit 3) instead of printing usage.
    let o = run(&["risk", &t, "--weather", "--help"]);
    assert_eq!(o.status.code(), Some(3), "{}", stderr(&o));
    assert!(stderr(&o).contains("reading --help"), "{}", stderr(&o));
    let typo = run(&["new-hull", "-draft", "1"]);
    assert_eq!(typo.status.code(), Some(2));
    assert!(
        stderr(&typo).contains("unknown option '-draft'"),
        "{}",
        stderr(&typo)
    );
}

/// 8B8: numeric options are validated, not clamped or misreported.
#[test]
fn numeric_options_are_validated() {
    let t = template();
    for args in [
        vec!["risk", t.as_str(), "--samples", "0"],
        vec!["risk", t.as_str(), "--samples", "-3"],
        vec!["risk", t.as_str(), "--uncertainty", "1.5"],
        vec!["risk", t.as_str(), "--uncertainty", "nan"],
        vec!["risk", t.as_str(), "--gate", "5=-3"],
        vec!["risk", t.as_str(), "--gate", "1=10:2"],
        vec!["risk", t.as_str(), "--gate", "99=10"],
        vec!["risk", t.as_str(), "--max-sea-state", "12"],
        vec!["risk", t.as_str(), "--launch-method", "catapult"],
        vec!["schedule", t.as_str(), "--limit", "crane=nan"],
        vec!["schedule", t.as_str(), "--limit", "crane=-5"],
        vec![
            "schedule",
            t.as_str(),
            "--limit",
            "crane=5",
            "--limit",
            "crane=6",
        ],
        vec!["schedule", t.as_str(), "--limit", "teleport=5"],
    ] {
        let o = run(&args);
        assert_eq!(o.status.code(), Some(2), "{args:?}: {}", stderr(&o));
    }
    let ok = run(&["risk", &t, "--samples", "50", "--seed", "7", "--json"]);
    assert!(ok.status.success(), "{}", stderr(&ok));
    assert!(parses_as_json(&stdout(&ok)), "{}", stdout(&ok));
}

/// 8B2: `export --json` is valid JSON for a path with spaces and
/// backslashes; outputs create parent directories and refuse to overwrite.
#[test]
fn export_json_is_valid_with_spaces_and_refuses_overwrite() {
    let dir = scratch("export");
    let gltf = dir.join("sub dir").join("hull.gltf");
    let gltf_s = gltf.to_string_lossy().into_owned();
    let o = run(&["export", &manifest(), "--gltf", &gltf_s, "--json"]);
    assert!(o.status.success(), "{}", stderr(&o));
    let text = stdout(&o);
    let doc = tpt_yard_core::json::Value::parse(text.trim()).expect("valid JSON");
    let written = doc
        .get("written")
        .and_then(|w| w.as_array())
        .expect("written");
    assert_eq!(
        written[0].as_str(),
        Some(gltf_s.as_str()),
        "path survives intact"
    );
    assert!(gltf.exists());
    let again = run(&["export", &manifest(), "--gltf", &gltf_s]);
    assert_eq!(again.status.code(), Some(3));
    assert!(stderr(&again).contains("--force"), "{}", stderr(&again));
    assert!(run(&["export", &manifest(), "--gltf", &gltf_s, "--force"])
        .status
        .success());
    // No output flag is a usage error.
    assert_eq!(run(&["export", &manifest()]).status.code(), Some(2));
}

/// 8B3: the embedded viewer script has no doubled braces (a JS syntax
/// error) and the project name is HTML-escaped.
#[test]
fn html_report_viewer_is_valid_and_names_are_escaped() {
    let dir = scratch("html");
    let gltf = dir.join("hull.gltf");
    assert!(
        run(&["export", &manifest(), "--gltf", &gltf.to_string_lossy()])
            .status
            .success()
    );
    // A project whose name is HTML.
    let project = std::fs::read_to_string(template())
        .unwrap()
        .replace("Container ship 1400 TEU", "<script>alert(1)</script> & co");
    let pfile = dir.join("p.json");
    std::fs::write(&pfile, project).unwrap();
    let out = dir.join("r.html");
    let o = run(&[
        "html-report",
        &pfile.to_string_lossy(),
        "--gltf-viewer",
        &gltf.to_string_lossy(),
        "--out",
        &out.to_string_lossy(),
    ]);
    assert!(o.status.success(), "{}", stderr(&o));
    let html = std::fs::read_to_string(&out).unwrap();
    // (`}}` legitimately closes the nested import-map JSON; the bug was
    // doubled braces around JS blocks and object literals.)
    assert!(
        !html.contains("{{ ") && !html.contains(" }}") && !html.contains("({{"),
        "doubled braces left in the script"
    );
    assert!(html.contains("new THREE.WebGLRenderer({ antialias: true })"));
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt; &amp; co"));
    assert!(!html.contains("<script>alert(1)"));
    // Overwrite needs --force.
    let again = run(&[
        "html-report",
        &pfile.to_string_lossy(),
        "--out",
        &out.to_string_lossy(),
    ]);
    assert_eq!(again.status.code(), Some(3));
}

/// 8B5: `new` keeps stdout pure JSON (the banner is on stderr) and `--out`
/// writes the file directly.
#[test]
fn new_prints_clean_json_and_can_write_a_file() {
    let o = run(&["new", "sea", "Test Boat"]);
    assert!(o.status.success());
    assert!(
        parses_as_json(&stdout(&o)),
        "stdout must be JSON only: {}",
        stdout(&o)
    );
    assert!(stderr(&o).contains("validate"), "banner goes to stderr");
    let dir = scratch("new");
    let file = dir.join("p.json");
    let w = run(&["new", "container-ship", "--out", &file.to_string_lossy()]);
    assert!(w.status.success(), "{}", stderr(&w));
    assert!(stdout(&w).is_empty());
    assert!(run(&["validate", &file.to_string_lossy()]).status.success());
}

/// The reference commands keep working end to end from a path with spaces.
#[test]
fn core_commands_run_on_the_reference_data() {
    let t = template();
    for args in [
        vec!["validate", t.as_str(), "--json"],
        vec!["report", t.as_str(), "--json"],
        vec!["schedule", t.as_str(), "--json"],
    ] {
        let o = run(&args);
        assert!(o.status.success(), "{args:?}: {}", stderr(&o));
        assert!(parses_as_json(&stdout(&o)), "{args:?}: {}", stdout(&o));
    }
    let plan = run(&["plan", &manifest(), "--json"]);
    assert!(plan.status.success(), "{}", stderr(&plan));
    assert!(parses_as_json(&stdout(&plan)));
    let v = run(&["validate", &manifest()]);
    assert!(
        v.status.success() && stdout(&v).contains("workshop fits"),
        "{}",
        stdout(&v)
    );
}

/// 8B9: a failing `--strict` verdict exits 1 (a check, not a malfunction),
/// output files are never silently overwritten, and `--json` carries the
/// verdict fields added for the full IMO set.
#[test]
fn strict_failures_exit_one_and_outputs_are_protected() {
    let dir = scratch("strict");
    let out = dir.to_string_lossy().into_owned();
    let made = run(&["new-hull", "wigley", "--out", &out]);
    assert!(made.status.success(), "{}", stderr(&made));
    let case = dir.join("wigley.json").to_string_lossy().into_owned();
    assert!(run(&["stability", &case, "--strict"]).status.success());
    let fail = run(&["stability", &case, "--kg", "6.4", "--strict"]);
    assert_eq!(fail.status.code(), Some(1), "{}", stderr(&fail));
    assert!(
        !stderr(&fail).contains("USAGE"),
        "a failed check is not a usage error"
    );
    let svg = dir.join("gz.svg").to_string_lossy().into_owned();
    assert!(run(&["stability", &case, "--svg", &svg]).status.success());
    let again = run(&["stability", &case, "--svg", &svg]);
    assert_eq!(again.status.code(), Some(3));
    assert!(run(&["stability", &case, "--svg", &svg, "--force"])
        .status
        .success());
    let json = run(&["stability", &case, "--json"]);
    let text = stdout(&json);
    let doc = tpt_yard_core::json::Value::parse(text.trim()).expect("JSON");
    let imo = doc.get("imo_2008").expect("imo_2008");
    assert!(imo.get("area_30_to_40_deg_m_rad").is_some() && imo.get("gz_at_30_deg_m").is_some());
}
