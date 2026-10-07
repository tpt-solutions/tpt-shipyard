//! Uniform command-line parsing for `tpt-yard`.
//!
//! One [`Spec`] per command declares its positionals, value flags (with the
//! ones that may repeat) and switches. [`parse`] then gives every command the
//! same behaviour: flags and positionals in any order, `--flag value` or
//! `--flag=value`, unknown flags rejected, a duplicated single-use flag
//! rejected, and `--help` / `-h` honoured only where a flag may appear (never
//! as a flag's value). Usage mistakes are [`CliError::Usage`]; exit codes are
//! `0` ok, `1` a check failed (`validate`, `--strict`), `2` usage, `3` a
//! runtime or I/O failure.

use std::fmt;
use std::path::Path;

/// Why a command ended unsuccessfully; decides the exit code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    /// Bad command line (exit 2): usage text is shown.
    Usage(String),
    /// The command ran and its check failed (exit 1).
    Check(String),
    /// A runtime or I/O failure (exit 3).
    Failed(String),
}

/// Prefix a command puts on an error string to say "the check failed" (the
/// verdict is the output, not a malfunction).
pub const CHECK_PREFIX: &str = "check failed: ";

impl CliError {
    /// Classifies a plain error string from a command body: a
    /// [`CHECK_PREFIX`] string is a failed check, anything else a runtime
    /// failure.
    #[must_use]
    pub fn from_message(msg: String) -> Self {
        match msg.strip_prefix(CHECK_PREFIX) {
            Some(rest) => CliError::Check(rest.to_string()),
            None => CliError::Failed(msg),
        }
    }

    /// The process exit code.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            CliError::Check(_) => 1,
            CliError::Usage(_) => 2,
            CliError::Failed(_) => 3,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Usage(m) | CliError::Check(m) | CliError::Failed(m) => f.write_str(m),
        }
    }
}

impl From<String> for CliError {
    fn from(m: String) -> Self {
        CliError::from_message(m)
    }
}

impl From<&str> for CliError {
    fn from(m: &str) -> Self {
        CliError::from_message(m.to_string())
    }
}

/// A command's accepted command line.
#[derive(Debug, Clone, Copy)]
pub struct Spec {
    /// Command name.
    pub name: &'static str,
    /// Names of the positionals (their count is the exact arity unless
    /// `optional_positionals` says the tail is optional).
    pub positionals: &'static [&'static str],
    /// How many of the trailing positionals may be omitted.
    pub optional_positionals: usize,
    /// Flags that take a value, written without the dashes.
    pub values: &'static [&'static str],
    /// The subset of `values` that may repeat.
    pub repeatable: &'static [&'static str],
    /// Boolean switches, written without the dashes.
    pub switches: &'static [&'static str],
}

const NONE: &[&str] = &[];

/// Every command's spec (the single source for flag validation).
pub const SPECS: &[Spec] = &[
    Spec {
        name: "validate",
        positionals: &["FILE"],
        optional_positionals: 0,
        values: NONE,
        repeatable: NONE,
        switches: NONE,
    },
    Spec {
        name: "plan",
        positionals: &["MANIFEST"],
        optional_positionals: 0,
        values: NONE,
        repeatable: NONE,
        switches: NONE,
    },
    Spec {
        name: "report",
        positionals: &["FILE"],
        optional_positionals: 0,
        values: NONE,
        repeatable: NONE,
        switches: NONE,
    },
    Spec {
        name: "export",
        positionals: &["MANIFEST"],
        optional_positionals: 0,
        values: &["gltf", "ifc"],
        repeatable: NONE,
        switches: &["force"],
    },
    Spec {
        name: "stability",
        positionals: &["CASE"],
        optional_positionals: 0,
        values: &[
            "offsets",
            "draft",
            "kg",
            "fsm",
            "to-deg",
            "flooding-deg",
            "csv",
            "svg",
        ],
        repeatable: NONE,
        switches: &["strict", "force"],
    },
    Spec {
        name: "new-hull",
        positionals: &["HULL"],
        optional_positionals: 0,
        values: &["name", "loa", "beam", "draft", "depth", "kg", "out"],
        repeatable: NONE,
        switches: &["force"],
    },
    Spec {
        name: "import-hull",
        positionals: &["MESH"],
        optional_positionals: 0,
        values: &[
            "up", "bow", "scale", "stations", "levels", "draft", "kg", "name", "out", "baseline",
            "depth",
        ],
        repeatable: NONE,
        switches: &["force", "half"],
    },
    Spec {
        name: "schedule",
        positionals: &["FILE"],
        optional_positionals: 0,
        values: &["limit"],
        repeatable: &["limit"],
        switches: NONE,
    },
    Spec {
        name: "risk",
        positionals: &["FILE"],
        optional_positionals: 0,
        values: &[
            "samples",
            "uncertainty",
            "seed",
            "gate",
            "weather",
            "weather-activity",
            "launch-method",
            "max-sea-state",
            "vessel-mass-t",
            "way-length-m",
            "friction",
            "breadth-m",
        ],
        repeatable: &["gate"],
        switches: NONE,
    },
    Spec {
        name: "new",
        positionals: &["KIND", "NAME"],
        optional_positionals: 1,
        values: &["out"],
        repeatable: NONE,
        switches: &["force"],
    },
    Spec {
        name: "html-report",
        positionals: &["FILE"],
        optional_positionals: 0,
        values: &["out", "structure", "gltf-viewer"],
        repeatable: NONE,
        switches: &["force"],
    },
    Spec {
        name: "pdf-report",
        positionals: &["FILE"],
        optional_positionals: 0,
        values: &["out"],
        repeatable: NONE,
        switches: &["force"],
    },
];

/// The spec of a command, if it exists.
#[must_use]
pub fn spec_for(name: &str) -> Option<&'static Spec> {
    SPECS.iter().find(|s| s.name == name)
}

/// A parsed command line.
#[derive(Debug, Default)]
pub struct Parsed<'a> {
    /// `--help` / `-h` appeared in a flag position.
    pub help: bool,
    /// Positionals in order.
    pub positionals: Vec<&'a str>,
    values: Vec<(&'a str, &'a str)>,
    switches: Vec<&'a str>,
}

impl<'a> Parsed<'a> {
    /// The positional at `i` (the parser guarantees the required ones).
    #[must_use]
    pub fn pos(&self, i: usize) -> Option<&'a str> {
        self.positionals.get(i).copied()
    }

    /// The value of a single-use flag.
    #[must_use]
    pub fn value(&self, flag: &str) -> Option<&'a str> {
        self.values
            .iter()
            .find(|(f, _)| *f == flag)
            .map(|(_, v)| *v)
    }

    /// Every value of a repeatable flag, in order.
    #[must_use]
    pub fn all(&self, flag: &str) -> Vec<&'a str> {
        self.values
            .iter()
            .filter(|(f, _)| *f == flag)
            .map(|(_, v)| *v)
            .collect()
    }

    /// Whether a switch was given.
    #[must_use]
    pub fn has(&self, flag: &str) -> bool {
        self.switches.contains(&flag)
    }

    /// A finite number.
    ///
    /// # Errors
    ///
    /// [`CliError::Usage`] when the value does not parse or is not finite.
    pub fn number(&self, flag: &str) -> Result<Option<f64>, CliError> {
        self.value(flag).map(|v| number(flag, v)).transpose()
    }

    /// A finite number above zero.
    ///
    /// # Errors
    ///
    /// [`CliError::Usage`] when absent-valued input is not a positive number.
    pub fn positive(&self, flag: &str) -> Result<Option<f64>, CliError> {
        self.value(flag).map(|v| positive(flag, v)).transpose()
    }

    /// A whole number within `[min, max]`.
    ///
    /// # Errors
    ///
    /// [`CliError::Usage`] when the value is not an integer in range.
    pub fn count(&self, flag: &str, min: u64, max: u64) -> Result<Option<u64>, CliError> {
        self.value(flag)
            .map(|v| count(flag, v, min, max))
            .transpose()
    }
}

/// Parses a finite number.
///
/// # Errors
///
/// [`CliError::Usage`] naming the flag.
pub fn number(flag: &str, v: &str) -> Result<f64, CliError> {
    v.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .ok_or_else(|| CliError::Usage(format!("--{flag} needs a finite number, got '{v}'")))
}

/// Parses a finite number above zero.
///
/// # Errors
///
/// [`CliError::Usage`] naming the flag.
pub fn positive(flag: &str, v: &str) -> Result<f64, CliError> {
    let n = number(flag, v)?;
    if n > 0.0 {
        Ok(n)
    } else {
        Err(CliError::Usage(format!(
            "--{flag} needs a number above zero, got '{v}'"
        )))
    }
}

/// Parses a whole number within `[min, max]`.
///
/// # Errors
///
/// [`CliError::Usage`] naming the flag and the range.
pub fn count(flag: &str, v: &str, min: u64, max: u64) -> Result<u64, CliError> {
    v.parse::<u64>()
        .ok()
        .filter(|n| (min..=max).contains(n))
        .ok_or_else(|| {
            CliError::Usage(format!(
                "--{flag} needs a whole number from {min} to {max}, got '{v}'"
            ))
        })
}

/// Parses a command line against its spec.
///
/// # Errors
///
/// [`CliError::Usage`] for an unknown flag, a missing or surplus value, a
/// duplicated single-use flag, or the wrong number of positionals.
pub fn parse<'a>(rest: &[&'a str], spec: &Spec) -> Result<Parsed<'a>, CliError> {
    let mut out = Parsed::default();
    let mut it = rest.iter().copied();
    let mut only_positionals = false;
    while let Some(tok) = it.next() {
        if only_positionals || !tok.starts_with('-') || tok == "-" {
            out.positionals.push(tok);
            continue;
        }
        if tok == "--" {
            only_positionals = true;
            continue;
        }
        if tok == "--help" || tok == "-h" {
            out.help = true;
            return Ok(out);
        }
        let Some(long) = tok.strip_prefix("--") else {
            return Err(CliError::Usage(format!(
                "unknown option '{tok}' for '{}'",
                spec.name
            )));
        };
        let (name, inline) = match long.split_once('=') {
            Some((n, v)) => (n, Some(v)),
            None => (long, None),
        };
        if spec.switches.contains(&name) {
            if inline.is_some() {
                return Err(CliError::Usage(format!("--{name} takes no value")));
            }
            if !out.switches.contains(&static_name(spec, name)) {
                out.switches.push(static_name(spec, name));
            }
        } else if spec.values.contains(&name) {
            let value = match inline {
                Some(v) => v,
                None => it
                    .next()
                    .ok_or_else(|| CliError::Usage(format!("--{name} needs a value")))?,
            };
            if !spec.repeatable.contains(&name) && out.value(name).is_some() {
                return Err(CliError::Usage(format!("--{name} given more than once")));
            }
            out.values.push((static_name(spec, name), value));
        } else {
            return Err(CliError::Usage(format!(
                "unknown option '--{name}' for '{}'",
                spec.name
            )));
        }
    }
    let required = spec.positionals.len() - spec.optional_positionals;
    if out.positionals.len() < required {
        return Err(CliError::Usage(format!(
            "{} needs {}",
            spec.name,
            spec.positionals[out.positionals.len()]
        )));
    }
    if out.positionals.len() > spec.positionals.len() {
        return Err(CliError::Usage(format!(
            "unexpected extra argument '{}'",
            out.positionals[spec.positionals.len()]
        )));
    }
    Ok(out)
}

/// The spec's own `'static` copy of a flag name (so `Parsed` can hold it
/// without borrowing the argument list for names).
fn static_name(spec: &Spec, name: &str) -> &'static str {
    spec.values
        .iter()
        .chain(spec.switches)
        .find(|n| **n == name)
        .copied()
        .unwrap_or("")
}

/// Writes an output file: creates missing parent directories and refuses to
/// overwrite an existing file unless `force`.
///
/// # Errors
///
/// A message naming the path.
pub fn write_output(path: &str, data: &[u8], force: bool) -> Result<(), String> {
    let p = Path::new(path);
    if !force && p.exists() {
        return Err(format!("{path} exists; use --force to overwrite"));
    }
    if let Some(parent) = p.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("creating {}: {e}", parent.display()))?;
    }
    std::fs::write(p, data).map_err(|e| format!("writing {path}: {e}"))
}

/// Escapes a string for a JSON string literal body.
#[must_use]
pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Escapes text for an HTML body or attribute.
#[must_use]
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p<'a>(spec: &str, rest: &[&'a str]) -> Result<Parsed<'a>, CliError> {
        parse(rest, spec_for(spec).expect("spec"))
    }

    #[test]
    fn flags_and_positionals_in_any_order() {
        let a = p("schedule", &["--limit", "crane=5", "plan.json"]).unwrap();
        assert_eq!(a.pos(0), Some("plan.json"));
        assert_eq!(a.all("limit"), vec!["crane=5"]);
        let b = p(
            "schedule",
            &["plan.json", "--limit=crane=5", "--limit", "crew=3"],
        )
        .unwrap();
        assert_eq!(b.all("limit"), vec!["crane=5", "crew=3"]);
    }

    /// Regression (8B7): a later argument equal to the path must not be
    /// dropped, and `--help` as a flag value is a value, not a help request.
    #[test]
    fn values_that_look_like_other_things_stay_values() {
        let a = p("export", &["m.json", "--gltf", "m.json"]).unwrap();
        assert_eq!(a.value("gltf"), Some("m.json"));
        assert_eq!(a.pos(0), Some("m.json"));
        let b = p("export", &["m.json", "--ifc", "--help"]).unwrap();
        assert!(!b.help && b.value("ifc") == Some("--help"));
        assert!(p("export", &["--help"]).unwrap().help);
        assert!(p("export", &["m.json", "-h"]).unwrap().help);
    }

    #[test]
    fn unknown_duplicate_and_missing_are_usage_errors() {
        for (spec, args, needle) in [
            (
                "validate",
                vec!["f.json", "--nope"],
                "unknown option '--nope'",
            ),
            (
                "validate",
                vec!["f.json", "-draft"],
                "unknown option '-draft'",
            ),
            ("validate", vec![], "needs FILE"),
            ("validate", vec!["a", "b"], "unexpected extra argument 'b'"),
            ("export", vec!["m", "--gltf"], "--gltf needs a value"),
            (
                "export",
                vec!["m", "--gltf", "a", "--gltf", "b"],
                "more than once",
            ),
            ("export", vec!["m", "--force=yes"], "takes no value"),
        ] {
            let e = p(spec, &args).unwrap_err();
            assert!(
                matches!(&e, CliError::Usage(m) if m.contains(needle)),
                "{args:?}: {e}"
            );
        }
        // `new` takes an optional name.
        assert!(p("new", &["sea"]).is_ok() && p("new", &["sea", "Ship"]).is_ok());
        assert!(p("new", &["sea", "a", "b"]).is_err());
    }

    #[test]
    fn number_helpers_are_strict() {
        assert_eq!(number("x", "2.5").unwrap(), 2.5);
        for bad in ["nan", "inf", "abc", ""] {
            assert!(number("x", bad).is_err(), "{bad}");
        }
        assert!(positive("x", "0").is_err() && positive("x", "-1").is_err());
        assert_eq!(count("n", "5", 1, 10).unwrap(), 5);
        for bad in ["0", "11", "-1", "2.5", "x"] {
            assert!(count("n", bad, 1, 10).is_err(), "{bad}");
        }
    }

    #[test]
    fn exit_codes_and_escapes() {
        assert_eq!(CliError::Usage(String::new()).exit_code(), 2);
        assert_eq!(
            CliError::from_message("check failed: bad".into()).exit_code(),
            1
        );
        assert_eq!(
            CliError::from_message("reading x: gone".into()).exit_code(),
            3
        );
        assert_eq!(json_escape("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
        assert_eq!(
            html_escape("<a href=\"x\">&"),
            "&lt;a href=&quot;x&quot;&gt;&amp;"
        );
    }

    #[test]
    fn output_refuses_overwrite_and_creates_parents() {
        let dir = std::env::temp_dir().join(format!("tpt-yard-args-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let f = dir
            .join("a")
            .join("b")
            .join("out.txt")
            .to_string_lossy()
            .into_owned();
        write_output(&f, b"one", false).unwrap();
        assert!(write_output(&f, b"two", false)
            .unwrap_err()
            .contains("--force"));
        write_output(&f, b"two", true).unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"two");
        std::fs::remove_dir_all(&dir).ok();
    }
}
